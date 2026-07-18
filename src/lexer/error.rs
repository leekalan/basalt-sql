//! Lexer-specific error type.

use thiserror::Error;

/// Result alias local to the lexer module.
pub type Result<T> = std::result::Result<T, LexError>;

/// All ways tokenizing can fail. Each variant carries the byte offset
/// into the source string where the failure occurred, so callers can point
/// at the exact character.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum LexError {
    /// A character was encountered that doesn't start any valid token
    /// (e.g. `$`, `@`, or other symbols outside the supported grammar).
    #[error("unexpected character '{ch}' at offset {offset}")]
    UnexpectedChar {
        /// The offending character.
        ch: char,
        /// Byte offset into the source where it was found.
        offset: usize,
    },

    /// A string literal was opened with `'` but the input ended before
    /// a closing `'` was found.
    #[error("unterminated string literal starting at offset {offset}")]
    UnterminatedString {
        /// Byte offset of the opening quote.
        offset: usize,
    },
}

impl LexError {
    pub fn unexpected_char(ch: char, offset: usize) -> Self {
        LexError::UnexpectedChar { ch, offset }
    }

    pub fn unterminated_string(offset: usize) -> Self {
        LexError::UnterminatedString { offset }
    }
}
