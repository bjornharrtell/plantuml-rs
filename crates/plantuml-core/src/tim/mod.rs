//! The preprocessor: variables, functions, conditionals, loops, includes (`tim`).

pub mod builtin;
pub mod context;
pub mod eater;
pub mod expr;
pub mod functions;
pub mod iterator;
pub mod line_type;
pub mod loader;
pub mod memory;
pub mod resolver;
pub mod themes;
pub mod trie;

use crate::text::StringLocated;

/// An error found while preprocessing (`EaterException`).
#[derive(Debug, Clone, PartialEq)]
pub struct EaterException {
    pub message: String,
    pub location: StringLocated,
    /// Column of the error when it comes from JSON data that could not be parsed.
    pub json_column: Option<usize>,
}

impl EaterException {
    pub fn new(message: impl Into<String>, location: &StringLocated) -> Self {
        Self {
            message: message.into(),
            location: location.clone(),
            json_column: None,
        }
    }
}

impl std::fmt::Display for EaterException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type Result<T> = std::result::Result<T, EaterException>;
