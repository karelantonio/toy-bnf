use logos::Logos;

#[derive(thiserror::Error, Debug)]
pub enum LexError {
    #[error("Unexpected character at line: {0}, near: {0}")]
    Unexpected(usize, String),
}

#[derive(Logos, Debug)]
#[logos(skip "[ \r\t]+")]
pub enum Tk<'a> {
    #[token("\n")]
    Nl,

    #[regex("[a-zA-Z_][a-zA-Z_0-9]*")]
    Id(&'a str),

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("::=")]
    Assign,

    #[token("|")]
    Pipe,

    #[regex("\"(\\\\.|[^\"])*\"")]
    Terminal(&'a str),
}

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
