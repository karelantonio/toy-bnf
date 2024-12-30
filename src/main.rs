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
            short = 'd',
            long = "debug",
            help = "Enable debug output",
            default_value = "false"
        )]
        debug: bool,

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
            short = 'd',
            long = "debug",
            help = "Enable debug output",
            default_value = "false"
        )]
        debug: bool,

        #[arg(
            short = 'P',
            long = "no-pretty",
            help = "Dont pretty-print the matches",
            default_value = "false"
        )]
        no_pretty: bool,

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
            help = "Watch the given rules (use multiple times to watch more than one), by default the same as the initial rule"
        )]
        rules: Vec<String>,
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
        Action::Generate { rule_name, debug } => {
            let tree = ast::parse(&bnf_file)?;
            let engine = engine::Engine::build(&tree, debug)?;
            println!("{}", engine.gen_random(&rule_name)?);
        }

        Action::Match {
            file,
            initial,
            rules,
            no_pretty,
            debug,
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
            let engine = engine::Engine::build(&tree, debug)?;

            let rules = if rules.len() == 0 {
                vec![initial.clone()]
            } else {
                rules
            };

            let matches = engine.match_rule(&initial, &rules, &content)?;

            if no_pretty {
                for (start, end) in matches {
                    println!("Match {start}..{end}: {}", &content[start..end]);
                }
            } else {
                // The colors (Green, blue, yellow and red)
                const COLORS: &[&str] = &["42;30", "44;30", "43;30", "41;30"];
                let mut lastdep = 0;
                let mut bld = String::with_capacity(content.len() * 5);
                for (idx, c) in content.chars().enumerate() {
                    // TODO: Optimize this
                    let mut depth = 0;
                    for (start, end) in matches.iter() {
                        if start <= &idx && &idx < end {
                            depth += 1;
                        }
                    }
                    if depth == lastdep {
                        // Do nothing, just put a char
                        bld.push(c);
                        continue;
                    }
                    // Update the depth and color
                    lastdep = depth;
                    if depth == 0 {
                        // Set no color
                        bld.extend("\x1b[0m".chars());
                    } else {
                        // Set the new color
                        let newcol = COLORS[(depth - 1) % COLORS.len()];
                        bld.extend("\x1b[".chars());
                        bld.extend(newcol.chars());
                        bld.extend("m".chars());
                    }

                    bld.push(c);
                }
                // Clear the color if was not black
                if lastdep != 0 {
                    bld.extend("\x1b[0m".chars());
                }

                println!("Matches:");
                println!("{bld}");
            }
        }
    }

    Ok(())
}
