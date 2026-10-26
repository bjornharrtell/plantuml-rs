//! Symbols of the token stream of an EBNF rule (`Symbol`).

/// The kind of a [`Token`](super::token::Token).
///
/// The names follow the Java enum, which is why `RepetitionSymbol` ends with the type name.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Symbol {
    /// An identifier written without quotes.
    Literal,
    /// `=`
    Definition,
    /// `,`
    Concatenation,
    /// `;`
    #[allow(dead_code)]
    Termination,
    /// `|`
    Alternation,
    /// `[`
    OptionalOpen,
    /// `]`
    OptionalClose,
    /// The `[` ... `]` group itself, synthetized by the shunting yard.
    Optional,
    /// `*`
    RepetitionSymbol,
    /// `{`
    RepetitionOpen,
    /// `}`
    RepetitionClose,
    /// `}-`
    RepetitionMinusClose,
    /// The `{` ... `}` group itself.
    RepetitionZeroOrMore,
    /// The `{` ... `}-` group itself.
    RepetitionOneOrMore,
    /// `(`
    GroupingOpen,
    /// `)`
    GroupingClose,
    /// `" "`
    TerminalString1,
    /// `' '`
    TerminalString2,
    /// `(* ... *)` inside a rule.
    CommentToken,
    /// A comment that followed the token it belongs to.
    CommentBelow,
    /// A comment that preceded the token it belongs to.
    CommentAbove,
    /// `? ... ?`
    SpecialSequence,
    /// `-`
    Not,
    /// A `(...)` group of the regex diagram.
    #[allow(dead_code)]
    RegexGroup,
}

impl Symbol {
    /// Binding strength used by the shunting yard (`getPriority`).
    ///
    /// Java throws for the other symbols; returning 0 keeps [`priority`](Self::priority) total.
    pub fn priority(self) -> i32 {
        match self {
            Symbol::RepetitionSymbol => 3,
            Symbol::Concatenation => 2,
            Symbol::Alternation => 1,
            _ => 0,
        }
    }

    pub fn is_operator(self) -> bool {
        matches!(
            self,
            Symbol::Concatenation | Symbol::Alternation | Symbol::RepetitionSymbol
        )
    }

    /// Java's `isFunction`, kept for completeness of the symbol model.
    #[allow(dead_code)]
    pub fn is_function(self) -> bool {
        matches!(self, Symbol::Optional | Symbol::RepetitionZeroOrMore)
    }

    /// Whether the token holds text shown in a box (`LITERAL`, terminals, `? ?`).
    pub fn is_text(self) -> bool {
        matches!(
            self,
            Symbol::Literal
                | Symbol::TerminalString1
                | Symbol::TerminalString2
                | Symbol::SpecialSequence
        )
    }
}
