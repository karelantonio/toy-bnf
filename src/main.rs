//! Simple program to run and test your grammars against text files inside your
//! terminal. Inspired by https://bnfplayground.pauliankline.com

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{fs::read_to_string, path::absolute};

mod ast;
mod engine;
mod lex;

/// What to do
#[derive(Subcommand, Clone)]
enum Action {
    #[command(name = "dump-ast")]
    DumpAst {
        #[arg(
            short = 'w',
            long = "wide",
            help = "Print the ast in wide mode",
            default_value = "false"
        )]
        wide: bool,
    },

    #[command(name = "dump-lex")]
    DumpLex {
        #[arg(
            short = 'w',
            long = "wide",
            help = "Dump the lex in wide mode",
            default_value = "false"
        )]
        wide: bool,
    },

    #[command(name = "generate")]
    Generate {
        #[arg(
            short = 'n',
            long = "name",
            name = "rule-name",
            help = "The rule name that you want to generate"
        )]
        rule_name: String,
    },

    #[command(name = "match")]
    Match {
        #[arg(
            short = 'f',
            long = "file",
            name = "file",
            help = "The file that contains the data you want to match, by default read from stdin"
        )]
        file: Option<String>,

        #[arg(
            short = 'i',
            long = "initial",
            name = "initial-rule",
            help = "The initial rule"
        )]
        initial: String,

        #[arg(
            short = 'W',
            long = "watch",
            name = "rule",
            help = "Watch the given rule, by default the same as the initial rule"
        )]
        rule: Option<String>,
    },
}

/// The command line arguments
#[derive(Parser)]
#[command(name = "Toy BNF")]
struct Args {
    /// The path to the grammar file
    #[arg(name = "path", help = "The path to the BNF file")]
    path: String,

    /// What to do
    #[command(subcommand, name = "action")]
    action: Action,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Parse the BNF
    let path = absolute(args.path)?;
    let bnf_file = read_to_string(path)?;

    match args.action {
        Action::DumpLex { wide } => {
            let tokens = lex::tokenize(&bnf_file)?;
            if wide {
                println!("Lex tokens: {tokens:#?}");
            } else {
                println!("Lex tokens: {tokens:?}");
            }
        }
        Action::DumpAst { wide } => {
            let tree = ast::parse(&bnf_file)?;
            if wide {
                println!("Ast tree: {tree:#?}");
            } else {
                println!("Ast tree: {tree:?}");
            }
        }
        Action::Generate { rule_name } => {
            let tree = ast::parse(&bnf_file)?;
            let engine = engine::Engine::build(&tree)?;
            println!("{}", engine.gen_random(&rule_name)?);
        }

        Action::Match {
            file,
            initial,
            rule,
        } => {
            let file = file.unwrap_or("/dev/stdin".into());
            // Resolve the file first
            let file = if file == "-" {
                "/dev/stdin".into()
            } else {
                absolute(&file)?
            };
            let content = read_to_string(file)?;
            // Create the engine
            let tree = ast::parse(&bnf_file)?;
            let engine = engine::Engine::build(&tree)?;
            for (start, end) in
                engine.match_rule(&initial, &rule.unwrap_or(initial.clone()), &content)?
            {
                println!("Match {start}..{end}: {}", &content[start..end]);
            }
        }
    }

    Ok(())
}
