use anyhow::Result;
use std::{
    fs::read_to_string,
    path::absolute,
};
use clap::Parser;

mod lex;
mod ast;
mod engine;

/// The command line arguments
#[derive(Parser)]
#[command(name="Toy BNF")]
pub struct Args {
    #[arg(help="The path to the BNF file")]
    path: String,

    #[arg(short='l', long="dump-lex", help="Dump the BNF lex tokens (to stderr)")]
    dump_lex: bool,

    #[arg(short='a', long="dump-ast", help="Dump the BNF ast tree (to stderr)")]
    dump_ast: bool,
}

fn main() -> Result<()> {

    let args = Args::parse();

    // Parse the BNF
    let path = absolute(args.path)?;
    let file = read_to_string(path)?;

    if args.dump_lex {
        let tokens = lex::tokenize(&file)?;
        eprintln!("Lex tokens: {tokens:#?}");
    }

    let tree = ast::Rule::parse(&file)?;
    if args.dump_ast {
        eprintln!("Ast tree: {tree:#?}");
    }

    Ok(())
}
