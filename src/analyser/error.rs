//! Analyser-specific error type.

use thiserror::Error;

/// Result alias local to the analyser module.
pub type Result<T> = std::result::Result<T, AnalyseError>;

/// All ways binding a parsed [`Statement`](crate::parser::Statement)
/// against a [`Catalog`](crate::catalog::Catalog) can fail.
///
/// Unlike [`LexError`](crate::lexer::LexError) and
/// [`ParseError`](crate::parser::ParseError), these variants don't
/// carry a byte offset. Position tracking stops at the parser: the
/// [`Expr`](crate::parser::Expr) tree it produces has no offset field
/// on its nodes, so there's nothing to report here. Table and column
/// names are used as the error context instead. Retrofitting offsets
/// onto `Expr` is possible later if better error spans turn out to
/// matter more than the extra field on every node.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum AnalyseError {
    /// `FROM`/`INTO`/`UPDATE` referenced a table that isn't registered
    /// in the catalog.
    #[error("unknown table '{name}'")]
    UnknownTable { name: String },

    /// `CREATE TABLE` named a table that's already registered.
    /// [`Catalog::register_table`](crate::catalog::Catalog::register_table)
    /// would silently overwrite it, so the analyser rejects this
    /// instead rather than allowing silent schema replacement.
    #[error("table '{name}' already exists")]
    TableAlreadyExists { name: String },

    /// An expression, `SELECT` list, or `SET` clause referenced a
    /// column that doesn't exist on the table being queried.
    #[error("unknown column '{column}' on table '{table}'")]
    UnknownColumn { table: String, column: String },

    /// A `CREATE TABLE` column list named the same column twice.
    #[error("duplicate column '{name}' in CREATE TABLE")]
    DuplicateColumn { name: String },

    /// `INSERT ... VALUES` supplied a different number of values than
    /// the table has columns. Values are positional (see
    /// [`InsertStatement`](crate::parser::InsertStatement)), so an
    /// exact count match is required.
    #[error("expected {expected} value(s) for INSERT into '{table}', found {found}")]
    ValueCountMismatch {
        table: String,
        expected: usize,
        found: usize,
    },

    /// An expression's statically-determined type doesn't fit where
    /// it's used — e.g. `-'x'`, `'x' + 1`, `id AND name`, or a `TEXT`
    /// value inserted into an `INTEGER` column. `NULL` literals are
    /// exempt from this check (their type is only known at runtime);
    /// see [`BoundExpr::static_type`](crate::analyser::BoundExpr::static_type).
    #[error("expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    /// `INSERT ... VALUES` referenced a column by name. There's no
    /// source row to resolve a column reference against in a plain
    /// `VALUES` clause — only constant expressions (literals,
    /// arithmetic, unary minus) are allowed. `UPDATE ... SET` is
    /// unaffected: it legitimately reads other columns of the row
    /// being updated (e.g. `SET balance = balance * 2`). Real engines
    /// support `DEFAULT` in `VALUES`; this crate doesn't have that
    /// yet.
    #[error("column '{name}' can't be referenced inside a VALUES expression")]
    ColumnInValues { name: String },
}
