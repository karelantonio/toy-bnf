//! Contains definitions for the tokens used in the parsing process
//! For informationa about the ast, see the [`crate::ast`] module
use logos::Logos;

/// Error that may happen while lexing the input
/// (Well, just one error, an unexpected character)
#[derive(thiserror::Error, Debug, Clone)]
pub enum LexError {
    #[error("Unexpected character at line: {0}, near: {1}")]
    Unexpected(usize, String),
}

/// A token is a simple abstraction over the input data
#[derive(Logos, Debug)]
#[logos(skip "[ \r\t]+")]
pub enum Tk<'a> {
    /// A newline character, por linenumber purposes
    #[token("\n")]
    Nl,

    /// An identifier
    /// (Shall we allow dashes? (-) )
    #[regex("[a-zA-Z_][a-zA-Z_0-9]*")]
    Id(&'a str),

    /// Lower than character, used in the rule names and non-terminals
    #[token("<")]
    Lt,

    /// A greater than character, using in the rule names and non-terminals
    #[token(">")]
    Gt,

    /// Assign a set or variants to a rule
    #[token("::=")]
    Assign,

    /// Separate the variants of a rule
    #[token("|")]
    Pipe,

    /// An actual terminal value
    #[regex("\"(\\\\.|[^\"])*\"")]
    Terminal(&'a str),
}

/// This makes the work of tokenize the input haystack
/// This is a simple abstraction over the input data
pub fn tokenize<'a>(data: &'a str) -> Result<Vec<Tk<'a>>, LexError> {
    let mut lexer = Tk::lexer(data);
    let mut res = Vec::new();
    let mut line = 0;

    while let Some(tk) = lexer.next() {
        match tk {
            Ok(tk) => {
                if let Tk::Terminal(data) = tk {
                    res.push(Tk::Terminal(&data[1..data.len() - 1]));
                } else if let Tk::Nl = tk {
                    res.push(tk);
                    line += 1;
                } else {
                    res.push(tk);
                }
            }
            Err(_) => {
                return Err(LexError::Unexpected(line, lexer.slice().to_string()));
            }
        }
    }

    Ok(res)
}
