pub mod analyser;
pub mod catalog;
pub mod db;
pub mod error;
pub mod executor;
pub mod lexer;
pub mod parser;
pub mod storage;
pub mod types;

pub use error::{Error, Result};

/// The version of the basaltsql crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
