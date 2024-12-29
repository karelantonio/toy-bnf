use anyhow::Result;
use logos::Logos;
use std::{env::args,fs::read_to_string};

mod lex;
mod ast;

fn main() -> Result<()> {

    let mut cli = args();
    let _ = cli.next();

    let Some(file) = cli.next() else {
        anyhow::bail!("No file specified");
    };
    println!("Evaluating file: {file}");
    let content = read_to_string(file)?;

    // Parse the content
    let res = ast::Rule::parse(&content)?;
    println!("{res:#?}");

    Ok(())
}
