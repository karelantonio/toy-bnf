//! The engine is where the BNF gets used
//! See the [`Engine`] docs for more information

use crate::ast::{Atom, Rule};
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::collections::{BTreeMap, BTreeSet};

#[derive(thiserror::Error, Debug)]
pub enum GenerateError {
    #[error("The rule {0} does not exist")]
    BadRule(String),
}

#[derive(thiserror::Error, Debug)]
pub enum MatchError {
    #[error("That initial rule: {0} does not exist")]
    BadInitialRule(String),

    #[error("That Watch rule: {0} does not exist")]
    BadWatchRule(String),

    #[error("No matches")]
    NoMatches,
}

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
    debug: bool,
}

impl Engine {
    fn gen_random_variant(&self, rule: &Rule, rng: &mut ThreadRng) -> String {
        // Choose 1 variant
        let var: usize = rng.gen_range(0..rule.variants.len());
        let var = &rule.variants[var];
        let mut res = String::with_capacity(1024);

        for item in var.items.iter() {
            match item {
                Atom::Terminal { content } => {
                    res += &content;
                }
                Atom::NonTerminal { name } => {
                    res += &self.gen_random_variant(&self.tree[name], rng);
                }
            }
        }

        res
    }

    /// Generate a new random (valid) string
    pub fn gen_random(&self, rule: &str) -> Result<String, GenerateError> {
        let mut rng = thread_rng();

        let Some(rule) = self.tree.get(rule) else {
            return Err(GenerateError::BadRule(rule.into()));
        };

        Ok(self.gen_random_variant(rule, &mut rng))
    }

    /// Match against the given rule (and optionally save the range)
    /// rule: The current rule to match against
    /// to_watch: The rule which may be watched (saved to the vector)
    /// data: The input data (already sliced to the current offset)
    /// offset: The current offset (to add to the output vector)
    /// outp: The output vector
    fn match_against(
        &self,
        rule: &Rule,
        to_watch: &[String],
        data: &str,
        offset: usize,
        outp: &mut Vec<(usize, usize)>,
    ) -> Result<usize, ()> {
        if self.debug {
            // Should probably use the `log` crate
            eprintln!(
                "[*] Matching agains: {rule:?}, near: {:?}",
                &data[..data.len().min(5)]
            );
        }

        let mut sub = Vec::new();

        'varloop: for variant in rule.variants.iter() {
            if self.debug {
                eprintln!("Trying variant: {variant:?}");
            }

            sub.clear();

            let mut data = data;
            let mut proc = 0;

            // Only save the values when all the atoms in the variant have succeed
            for item in variant.items.iter() {
                match item {
                    Atom::Terminal { content } => {
                        if !data.starts_with(content) {
                            if self.debug {
                                eprintln!("Terminal {content:?} did not match near: {:?}, skipping variant", &data[..data.len().min(5)]);
                            }

                            continue 'varloop;
                        }

                        if self.debug {
                            eprintln!("Terminal {content:?} matched completely");
                        }

                        data = &data[content.len()..];
                        proc += content.len();
                    }
                    Atom::NonTerminal { name } => {
                        let subrule = &self.tree[name];
                        let Ok(processed) =
                            self.match_against(subrule, to_watch, data, offset + proc, &mut sub)
                        else {
                            continue 'varloop;
                        };
                        data = &data[processed..];
                        proc += processed;
                    }
                }
            }

            if self.debug {
                eprintln!("Done matching against {rule:?}");
            }

            // All atoms matched, save and return the slice
            // If this rule matched, add to the output vector
            if to_watch.contains(&rule.name) {
                outp.push((offset, offset + proc));
            }
            outp.extend(sub);
            return Ok(proc);
        }

        Err(())
    }

    /// Get the matches of a rule in the given data, starting from the given rule
    pub fn match_rule(
        &self,
        initial: &str,
        to_watch: &[String],
        data: &str,
    ) -> Result<Vec<(usize, usize)>, MatchError> {
        if !self.tree.contains_key(initial) {
            return Err(MatchError::BadInitialRule(initial.into()));
        }

        for rule in to_watch {
            if !self.tree.contains_key(rule) {
                return Err(MatchError::BadWatchRule(rule.clone()));
            }
        }

        let mut outp = Vec::new();
        let _ = self
            .match_against(&self.tree[initial], to_watch, data, 0, &mut outp)
            .map_err(|_| MatchError::NoMatches)?;
        Ok(outp)
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
    pub fn build(ast: &[Rule], debug: bool) -> Result<Engine, BuildError> {
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

        Ok(Self { tree: all, debug })
    }
}
