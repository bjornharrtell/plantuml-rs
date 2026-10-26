//! One token of an EBNF rule (`Token`).

use super::symbol::Symbol;

/// A [`Symbol`] with the text it carries, when it carries one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    symbol: Symbol,
    data: Option<String>,
}

impl Token {
    /// A token without text, like `=`, `,` or `[`.
    pub fn new(symbol: Symbol) -> Self {
        Self { symbol, data: None }
    }

    /// A token carrying text, like an identifier, a terminal string or a comment.
    ///
    /// The source of a token is never empty: the tokenizer replaces an empty terminal string with
    /// a space, as Java does.
    pub fn with_data(symbol: Symbol, data: impl Into<String>) -> Self {
        Self {
            symbol,
            data: Some(data.into()),
        }
    }

    pub fn symbol(&self) -> Symbol {
        self.symbol
    }

    /// The text of the token, empty when it carries none.
    pub fn text(&self) -> &str {
        self.data.as_deref().unwrap_or_default()
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            None => write!(f, "{:?}", self.symbol),
            Some(d) => write!(f, "{:?} {}", self.symbol, d),
        }
    }
}
