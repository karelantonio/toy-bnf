
use crate::ast::Rule;
use std::collections::BTreeMap;

#[derive(thiserror::Error, Debug)]
pub enum BuildError {

    #[error("Expression contains duplicated names: {0:?}")]
    DuplicatedNames(Vec<String>),

    #[error("Some rules ({0}) reference inexistent non-terminals ({1})")]
    InexistentNonTerminals(String, String),

    #[error("A rule ({0}) matches an empty string, which may cause an infinite recursion")]
    MatchesEmptyString(String),

}

pub struct Engine {
    mappings: BTreeMap<String, usize>,
    rmappings: BTreeMap<usize, String>,
}

impl Engine {

    pub fn build(ast: &[Rule]) -> Result<Engine, BuildError> {
        // First check if names are duplicated
        todo!();
    }

}
