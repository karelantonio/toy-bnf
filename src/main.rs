//! Simple program to run and test your grammars against text files inside your
//! terminal. Inspired by https://bnfplayground.pauliankline.com

use anyhow::Result;
use clap::Parser;
use std::{fs::read_to_string, path::absolute};

mod ast;
mod engine;
mod lex;

/// The command line arguments
#[derive(Parser)]
#[command(name = "Toy BNF")]
pub struct Args {
    /// The path to the grammar file
    #[arg(help = "The path to the BNF file")]
    path: String,

    /// Dump the lex tokens
    #[arg(
        short = 'l',
        long = "dump-lex",
        help = "Dump the BNF lex tokens (to stderr)"
    )]
    dump_lex: bool,

    /// Dump the ast tree
    #[arg(
        short = 'a',
        long = "dump-ast",
        help = "Dump the BNF ast tree (to stderr)"
    )]
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

    let _engine = engine::Engine::build(&tree)?;

    Ok(())
}
