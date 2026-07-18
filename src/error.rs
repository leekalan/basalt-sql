//! Crate-wide error type. Each variant wraps a specific module's error type.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Lex(#[from] crate::lexer::LexError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
