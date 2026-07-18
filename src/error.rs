//! Crate-wide error type. Each variant wraps a specific module's error type.

use thiserror::Error;

/// Crate-wide result alias. Any function that can fail across module
/// boundaries returns this rather than a module-local `Result`.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error type. This is a thin wrapper around each pipeline
/// stage's own error enum (see [`LexError`](crate::lexer::LexError)).
/// It exists so callers outside a specific module only need to match one type.
#[derive(Debug, Error)]
pub enum Error {
    /// A lexical error while tokenizing source SQL text.
    #[error(transparent)]
    Lex(#[from] crate::lexer::LexError),

    /// A wrapped standard I/O error, for future file-backed storage.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
