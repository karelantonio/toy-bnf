use crate::ast::Rule;
use std::collections::{BTreeMap, BTreeSet};

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Expression contains duplicated names: {0:?}")]
    DuplicatedNames(Vec<String>),

    #[error("Some rules ({0}) reference inexistent non-terminals ({1})")]
    InexistentNonTerminals(String, String),

    #[error("A rule ({0}) matches an empty string, which may cause an infinite recursion")]
    MatchesEmptyString(String),
}

pub struct Engine;

impl Engine {
    pub fn build(ast: &[Rule]) -> Result<Engine, BuildError> {
        // First check if names are duplicated
        let mut names = BTreeSet::<String>::new();
        let mut dup = Vec::new();

        for rule in ast {
            if names.contains(&rule.name) {
                // Error, duplicated
                dup.push(rule.name.clone());
            } else {
                names.insert(rule.name.clone());
            }
        }

        // If any duplicated, abort
        if !dup.is_empty() {
            return Err(BuildError::DuplicatedNames(dup));
        }

        // Check if all the non-terminals are valid
        for rule in ast {
            for variant in rule.variants.iter() {
                for item in variant.items.iter() {
                    if let crate::ast::Atom::NonTerminal { name } = item {
                        if !names.contains(name) {
                            return Err(BuildError::InexistentNonTerminals(rule.name.clone(), name.clone()));
                        }
                    }
                }
            }
        }

        todo!();
    }
}
