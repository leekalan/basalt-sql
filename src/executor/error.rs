//! Executor-specific error type.

use thiserror::Error;

/// Result alias local to the executor module.
pub type Result<T> = std::result::Result<T, ExecError>;

/// All ways executing a
/// [`BoundStatement`](crate::analyser::BoundStatement) can fail. This
/// is the one pipeline stage where errors are genuinely *runtime* —
/// division by zero can't be caught statically the way a type
/// mismatch can be (see
/// [`AnalyseError::TypeMismatch`](crate::analyser::AnalyseError::TypeMismatch)).
#[derive(Debug, Error, PartialEq, Clone)]
pub enum ExecError {
    /// Integer or float division by zero. Follows standard SQL (e.g.
    /// PostgreSQL) rather than IEEE-754 float semantics: `/` errors
    /// on zero for both `INTEGER` and `FLOAT` operands, it does not
    /// silently produce `inf`/`NaN`.
    #[error("division by zero")]
    DivisionByZero,

    /// An operand's runtime type didn't match what the analyser's
    /// static type check should have already guaranteed. Should be
    /// unreachable in normal operation. Kept as a real error rather
    /// than a panic so a bug here surfaces as a test failure instead
    /// of a crash.
    #[error("internal error: {0}")]
    InternalTypeError(String),

    /// Row storage doesn't have an entry for a table the analyser
    /// already validated exists in the catalog. Should be unreachable
    /// in normal operation.
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
