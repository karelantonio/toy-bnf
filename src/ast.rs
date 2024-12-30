//! Contains definitions and functions to parse a list of tokens into a more useful
//! abstract syntax tree.
//! See the [`Atom`], [`RuleVariant`], [`RuleVariant`], [`Rule`] and (module-private) [`Parser`] types
//! In short, this Ast is built upon three blocks:
//!  * Terminals (See the [`Atom`] docs)
//!  * Non-Terminals (See the [`Atom`] docs)
//!  * Rules, including (See the [`Rule`] docs)

/// Errors that may happen while parsing
/// This includes the Lex which is not actually part of the parsing
/// but
#[derive(thiserror::Error, Debug, Clone)]
pub enum ParseError {
    /// The errors ocurred under the reduce_atom stage
    #[error("While parsing rule atom")]
    InAtom(#[source] Box<ParseError>),

    /// Errors that may happend under the reduce_variant stage
    #[error("While parsing rule's {0}-th variant")]
    InVariant(usize, #[source] Box<ParseError>),

    /// Errors that may happend under the reduce_rule stage
    #[error("Error while parsing rule: {0}")]
    InRule(String, #[source] Box<ParseError>),

    /// Errors that may happen under the reduce_rules stage
    #[error("Error while parsing file")]
    InFile(#[source] Box<ParseError>),

    /// Unexpecte token, and several hints
    #[error("Error at line {0}: unexpected: {1}, expecting one of: {2}")]
    UnexpectedHint(usize, String, String),

    /// Error while lexing the data
    #[error("Lex error")]
    Lex(#[from] LexError),
}

/// An atom is the basic unit of information in this Tree
/// It may be:
///   - A terminal element (which is the lowest element in the tree)
///   - A non-terminal element (a reference to another rule)
/// For instance, the rule:
/// <fn_call> ::= <id> "(" <params> ")"
/// Has four atoms:
///  * <id> Is a reference to a rule called `id` (identifier), it is a non-terminal
///  * "(" Is a terminal element, here the parsing dfs ends
///  * <params> Is a reference to another rule called `param` which may be a list of numbers, or
///     any thing you have defined
///  * ")" Is the last terminal element
#[derive(Debug, Clone)]
pub enum Atom {
    Terminal { content: String },
    NonTerminal { name: String },
}

/// It is a set of terminals and non-terminals that a rule may match
/// More information in [`Rule`]'s docs
#[derive(Debug, Clone)]
pub struct RuleVariant {
    pub items: Vec<Atom>,
}

/// A rule is a set of terminals and non-terminals, usually grouped into variants
/// A rule matches a string of text IFF at least one of its variants matched this string
///
/// For example:
/// ```
/// <yes_or_no> ::= "Yes" | "No"
/// ```
///
/// This rule has two variants, the first one only matches when it finds the literal "Yes", and the
/// second one when it finds the literal "No", those are called Terminals
///
/// Another example:
///
/// ```
/// <echo_yn> ::= "echo" <yes_or_on>
/// ```
///
/// This has only one variant so its simpler.
/// This contains two atoms (an atomis either a terminal or a non-terminal) The first one is a
/// terminal, so matches if and only if it finds a literal "echo" in the data, the second one is a
/// reference to the `yes_or_no` rule previously seen.
///
/// This rule matches only if it find a "echo" string and then any "Yes" or "No"
///
/// We can use this mix of rules, terminals and non terminals to make some interesting stuff.
///
/// For instance, this is a set of rules that match a date like 10/5/2080:
/// ```
///
/// <non_zero_digit> ::= "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
///
/// <digit> ::= "0" | <non_zero_digit>
///
/// <many_digits> ::= <digit> <many_digits>
///                 | <digit>
///
/// <number> ::= <non_zero_digit> <many_digits>
///            | <digit>
///
/// <date> ::= <number> "/" <number> "/" <date>
/// ```
#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub variants: Vec<RuleVariant>,
}

type ParseResult = std::result::Result<Vec<Rule>, ParseError>;
use crate::lex::{tokenize, LexError, Tk};

/// Lex and parse the given data
pub fn parse(data: &str) -> ParseResult {
    let data = tokenize(data)?;
    // Now parse
    Parser::parse(&data)
}

/// To make our life easier
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

    fn un_escape(&self, data: &str) -> String {
        let mut out = String::with_capacity(data.len());
        
        let mut was_escaped = false;
        for c in data.chars() {
            if was_escaped {
                if c=='n' {
                    out.push('\n');
                }else if c=='t' {
                    out.push('\t');
                }else if c=='r' {
                    out.push('\r');
                }else if c=='r' {
                    out.push('\r');
                }else{
                    out.push(c);
                }
                was_escaped = false;
            }else if c=='\\' {
                was_escaped = true;
            }else{
                out.push(c);
            }
        }
        out
    }

    /// Pop an atom from the input data
    /// See [`Atom`]
    fn reduce_atom(&mut self) -> Result<Atom, ParseError> {
        // Check if is a terminal or not
        match self.data {
            [Tk::Lt, ..] => self.data = &self.data[1..],
            [Tk::Terminal(term), ..] => {
                self.data = &self.data[1..];
                return Ok(Atom::Terminal {
                    content: self.un_escape(term),
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

    /// Pop a variant from the input data
    /// See: [`Rule`], [`RuleVariant`]
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

    /// Pop a set of variants from the input data
    /// See: [`Rule`], [`RuleVariant`]
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
                self.lineno += 1;
                self.data = &self.data[2..];
                // Also Ok
            }
            _ => {
                return Ok(());
            }
        }

        self.reduce_variants(outp)
    }

    /// Pop a rule from the input
    /// See: [`Rule`], [`RuleVariant`], [`Atom`]
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

    /// Pop all the rules in the haystack
    fn reduce_rules(&mut self, rules: &mut Vec<Rule>) -> Result<(), ParseError> {
        match self.data {
            [Tk::Nl, ..] => {
                self.lineno += 1;
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

    /// Parse the data
    ///
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
