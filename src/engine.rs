//! The engine is where the BNF gets used
//! See the [`Engine`] docs for more information

use crate::ast::{Atom, Rule};
use std::collections::{BTreeMap, BTreeSet};

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Expression contains duplicated names: {0:?}")]
    DuplicatedNames(Vec<String>),

    #[error("Some rules ({0}) reference inexistent non-terminals ({1})")]
    InexistentNonTerminals(String, String),

    #[error("A rule ({0}) may cause an infinite recursion")]
    InfinityRecursion(String),
}

pub struct Engine {
    tree: BTreeMap<String, Rule>,
}

impl Engine {
    /// Generate a new random (valid) string
    pub fn gen_random(&self, rule: String) -> String {
        todo!();
    }

    /// Check if the given rule causes a recursion error
    /// TODO: Currently this checks for cicles in O(n) time each rule, in total O( n^2 )
    /// This could be improved to check if exists a cycle in one pass in O(n)
    fn check_recursion(idx: usize, rule: &Rule, in_stack: &mut [bool], rules: &[Rule]) -> bool {
        // Mark this as visited
        in_stack[idx] = true;

        for variant in rule.variants.iter() {
            if variant.items.len() == 0 {
                continue;
            }

            if let crate::ast::Atom::NonTerminal { name } = &variant.items[0] {
                for (cidx, rule) in rules.iter().enumerate() {
                    if rule.name != *name {
                        continue;
                    }

                    // If is in stack then return
                    if in_stack[cidx] {
                        return true;
                    }

                    if Self::check_recursion(cidx, rule, in_stack, rules) {
                        return true;
                    }
                }
            }
        }

        in_stack[idx] = false;

        return false;
    }

    /// Create a new instance of this engine and verify if there is any possible error at
    /// run time
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
                    let Atom::NonTerminal { name } = item else {
                        continue;
                    };
                    if names.contains(name) {
                        continue;
                    };
                    return Err(BuildError::InexistentNonTerminals(
                        rule.name.clone(),
                        name.clone(),
                    ));
                }
            }
        }

        // Check if a rule causes infinite recursion
        let mut in_stack = vec![false; ast.len()];
        for (idx, rule) in ast.iter().enumerate() {
            if Self::check_recursion(idx, rule, &mut in_stack, ast) {
                return Err(BuildError::InfinityRecursion(rule.name.clone()));
            }
        }

        let mut all = BTreeMap::new();

        for rule in ast {
            all.insert(rule.name.clone(), rule.clone());
        }

        Ok(Self { tree: all })
    }
}
