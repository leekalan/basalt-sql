pub mod catalog;
pub mod error;
pub mod lexer;
pub mod types;

pub use error::{Error, Result};

/// The version of the basaltsql crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
