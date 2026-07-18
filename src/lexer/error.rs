//! Lexer-specific error type.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, LexError>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum LexError {
    #[error("unexpected character '{ch}' at offset {offset}")]
    UnexpectedChar { ch: char, offset: usize },

    #[error("unterminated string literal starting at offset {offset}")]
    UnterminatedString { offset: usize },
}

impl LexError {
    pub fn unexpected_char(ch: char, offset: usize) -> Self {
        LexError::UnexpectedChar { ch, offset }
    }

    pub fn unterminated_string(offset: usize) -> Self {
        LexError::UnterminatedString { offset }
    }
}
