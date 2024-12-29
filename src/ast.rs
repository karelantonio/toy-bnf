#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("While parsing rule atom")]
    InAtom(#[source] Box<ParseError>),

    #[error("While parsing rule's {0}-th variant")]
    InVariant(usize, #[source] Box<ParseError>),

    #[error("Error while parsing rule: {0}")]
    InRule(String, #[source] Box<ParseError>),

    #[error("Error while parsing file")]
    InFile(#[source] Box<ParseError>),

    #[error("Error at line {0}: unexpected: {1}, expecting one of: {2}")]
    UnexpectedHint(usize, String, String),

    #[error("Lex error")]
    Lex(#[from] crate::lex::LexError),
}

#[derive(Debug)]
pub enum Atom {
    Terminal { content: String },
    NonTerminal { name: String },
}

#[derive(Debug)]
pub struct RuleVariant {
    items: Vec<Atom>,
}

#[derive(Debug)]
pub struct Rule {
    name: String,
    variants: Vec<RuleVariant>,
}

type ParseResult = std::result::Result<Vec<Rule>, ParseError>;
use crate::lex::{tokenize, LexError, Tk};
use logos::Logos;

impl Rule {
    pub fn parse(data: &str) -> ParseResult {
        let data = tokenize(data)?;
        // Now parse
        Parser::parse(&data)
    }
}

struct Parser<'a, 'b> {
    data: &'a [Tk<'b>],
    lineno: usize,
}

impl<'a, 'b> Parser<'a, 'b> {
    // To remove some boilerplate
    fn unexpected(&self, expecting: &str) -> ParseError {
        let tk = match self.data {
            [unex, ..] => format!("{unex:?}"),
            [] => "EOF".into(),
        };
        ParseError::UnexpectedHint(self.lineno, tk, expecting.into())
    }

    fn reduce_atom(&mut self) -> Result<Atom, ParseError> {
        // Check if is a terminal or not
        match self.data {
            [Tk::Lt, ..] => self.data = &self.data[1..],
            [Tk::Terminal(term), ..] => {
                self.data = &self.data[1..];
                return Ok(Atom::Terminal {
                    content: term.to_string(),
                });
            }
            _ => return Err(ParseError::InAtom(self.unexpected("'<' or \"...\"").into())),
        }

        // Parse the non-terminal
        let name = match self.data {
            [Tk::Id(name), ..] => {
                self.data = &self.data[1..];
                name.to_string()
            }
            _ => {
                return Err(ParseError::InAtom(
                    self.unexpected("non-terminal identifier").into(),
                ))
            }
        };

        // The final >
        match self.data {
            [Tk::Gt, ..] => {
                self.data = &self.data[1..];
                // Ok
            }
            _ => return Err(ParseError::InAtom(self.unexpected("'>'").into())),
        }

        return Ok(Atom::NonTerminal { name });
    }

    fn reduce_variant(&mut self, idx: usize, vari: &mut RuleVariant) -> Result<(), ParseError> {
        // Pop the atom
        let atom = self
            .reduce_atom()
            .map_err(|e| ParseError::InVariant(idx, e.into()))?;
        vari.items.push(atom);

        // Check if more
        match self.data {
            [Tk::Lt, ..] | [Tk::Terminal(_), ..] => self.reduce_variant(idx, vari),
            _ => Ok(()),
        }
    }

    fn reduce_variants(&mut self, outp: &mut Vec<RuleVariant>) -> Result<(), ParseError> {
        // Pop the variant
        let mut variant = RuleVariant { items: Vec::new() };
        self.reduce_variant(outp.len() + 1, &mut variant)?;
        outp.push(variant);

        match self.data {
            [Tk::Pipe, ..] => {
                self.data = &self.data[1..];
                // Ok
            }
            [Tk::Nl, Tk::Pipe, ..] => {
                self.data = &self.data[2..];
                // Also Ok
            }
            _ => {
                return Ok(());
            }
        }

        self.reduce_variants(outp)
    }

    fn reduce_rule(&mut self, prev: Option<&str>) -> Result<Rule, ParseError> {
        let noname_msg = match prev {
            Some(name) => format!("(name not reached, previous was: {name}"),
            Option::None => format!("(name not reached, it is the first)"),
        };

        // The <
        match self.data {
            [Tk::Lt, ..] => {
                self.data = &self.data[1..];
                // Ok
            }
            _ => {
                return Err(ParseError::InRule(
                    noname_msg,
                    self.unexpected("'>'").into(),
                ))
            }
        }

        // The identifier
        let name = match self.data {
            [Tk::Id(name), ..] => {
                self.data = &self.data[1..];
                name.to_string()
                // Ok
            }
            _ => {
                return Err(ParseError::InRule(
                    noname_msg,
                    self.unexpected("'>'").into(),
                ))
            }
        };

        // The >
        match self.data {
            [Tk::Gt, ..] => {
                self.data = &self.data[1..];
                // Ok
            }
            _ => return Err(ParseError::InRule(name, self.unexpected("'>'").into())),
        }

        // The ::=
        match self.data {
            [Tk::Assign, ..] => {
                self.data = &self.data[1..];
                // Ok
            }
            _ => return Err(ParseError::InRule(name, self.unexpected("'::='").into())),
        }

        // The variants
        let mut variants = Vec::new();
        self.reduce_variants(&mut variants)
            .map_err(|e| ParseError::InRule(name.clone(), e.into()))?;

        // Done
        Ok(Rule { name, variants })
    }

    fn reduce_rules(&mut self, rules: &mut Vec<Rule>) -> Result<(), ParseError> {
        match self.data {
            [Tk::Nl, ..] => {
                self.data = &self.data[1..];
                self.reduce_rules(rules)
            }
            [Tk::Lt, ..] => {
                // Previous name
                let prev: Option<&str> = if rules.len() == 0 {
                    None
                } else {
                    Some(&(*rules)[0].name)
                };

                let res = self.reduce_rule(prev)?;
                rules.push(res);
                self.reduce_rules(rules)
            }
            [_, ..] => Err(ParseError::InFile(
                self.unexpected("'<' of new line").into(),
            )),
            [] => Ok(()),
        }
    }

    /// The actual BNF:
    ///
    /// <terminal> ::= QUOTED_TEXT
    ///
    /// <non-terminal> ::= "<" ID ">"
    ///
    /// <atom> ::= <terminal>
    ///         | <non-terminal>
    ///
    /// <rule-variant>  ::= atom <rule-variant>
    ///                  | atom
    ///
    /// <rule-variants> ::= <rule-variant> "|" <rule-variants>
    ///                  |  <rule-variant>
    ///
    /// <rule> ::= "<" ID ">" "::=" <rule-variants>
    ///
    /// <rules> ::= <rule> <NL> <rule>
    ///          | <rule>
    ///
    fn parse(data: &[Tk<'b>]) -> Result<Vec<Rule>, ParseError> {
        let mut out = Vec::new();
        Parser { data, lineno: 1 }.reduce_rules(&mut out)?;
        Ok(out)
    }
}
