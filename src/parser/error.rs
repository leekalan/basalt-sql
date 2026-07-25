//! Parser-specific error type.

use thiserror::Error;

use crate::lexer::TokenKind;

/// Result alias local to the parser module.
pub type Result<T> = std::result::Result<T, ParseError>;

/// All ways parsing a token stream into an AST can fail. Each variant
/// carries the byte offset of the offending token (taken from
/// [`Token::offset`](crate::lexer::Token::offset)) so callers can point
/// at the exact source location.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum ParseError {
    /// A specific token was required but a different one was found.
    #[error("expected {expected} but found {found:?} at offset {offset}")]
    UnexpectedToken {
        /// Human-readable description of what was expected, e.g.
        /// `"FROM"` or `"identifier"`.
        expected: String,
        found: TokenKind,
        offset: usize,
    },

    /// The token stream ended before a statement was complete.
    #[error("unexpected end of input, expected {expected}")]
    UnexpectedEof { expected: String },

    /// A `Number` token's text couldn't be parsed as an integer or
    /// float. Should be rare in practice: [`Lexer::read_number`]
    /// (crate::lexer::Lexer::read_number) only accepts digits and `.`,
    /// but that doesn't guarantee well-formed numeric text (e.g.
    /// `1.2.3`, or a lone `.`).
    #[error("invalid numeric literal '{text}' at offset {offset}")]
    InvalidNumber { text: String, offset: usize },

    /// A column type name in a `CREATE TABLE` declaration wasn't one
    /// of the recognised [`DataType`](crate::types::DataType) names.
    /// Type names aren't reserved keywords in the lexer, so any
    /// identifier is accepted here and checked at this stage instead.
    #[error("unknown type '{name}' at offset {offset}")]
    UnknownType { name: String, offset: usize },
}
