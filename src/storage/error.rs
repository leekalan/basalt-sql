//! Storage-specific error type.

use thiserror::Error;

/// Result alias local to the storage module.
pub type Result<T> = std::result::Result<T, StorageError>;

/// All ways interacting with row storage can fail. In practice this
/// should be unreachable in normal operation since the analyser validates
/// table existence against the [`Catalog`](crate::catalog::Catalog)
/// before the executor ever touches storage, so `MissingTable` firing
/// points at a bug (e.g. `CREATE TABLE` updating the catalog but not
/// storage) rather than bad user input.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum StorageError {
    #[error("internal error: table '{name}' missing from storage")]
    MissingTable { name: String },
}
