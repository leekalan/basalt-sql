//! Abstract syntax tree produced by the [`Parser`](super::Parser).
//! Statements own all their data (no borrows from the token stream) so
//! they can outlive parsing and be passed on to later pipeline stages
//! (analyser, executor).

use crate::types::{DataType, Value};

/// A single parsed SQL statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateTable(CreateTableStatement),
}

/// `SELECT <columns> FROM <table> [WHERE <filter>]`
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStatement {
    pub columns: SelectColumns,
    pub table: String,
    pub filter: Option<Expr>,
}

/// The column list of a `SELECT`. Kept distinct from a plain
/// [Vec<String>] so `SELECT *` doesn't need to be resolved against the
/// catalog until the analyser stage.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectColumns {
    /// `SELECT *`
    All,
    /// `SELECT a, b, c`
    List(Vec<String>),
}

/// `INSERT INTO <table> VALUES (<expr> ...)`
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    pub table: String,
    pub values: Vec<Expr>,
}

/// `UPDATE <table> SET <col> = <expr>, ... [WHERE <filter>]`
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateStatement {
    pub table: String,
    pub assignments: Vec<(String, Expr)>,
    pub filter: Option<Expr>,
}

/// `DELETE FROM <table> [WHERE <filter>]`
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteStatement {
    pub table: String,
    pub filter: Option<Expr>,
}

/// `CREATE TABLE <table> (<columns>)`
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTableStatement {
    pub table: String,
    pub columns: Vec<ColumnDecl>,
}

/// A single column declaration inside `CREATE TABLE`, e.g.
/// `id INTEGER NOT NULL`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDecl {
    pub name: String,
    pub ty: DataType,
    /// `true` unless the declaration is followed by `NOT NULL`.
    pub nullable: bool,
}

/// A `WHERE` clause expression tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A bare column reference, e.g. `age` in `WHERE age > 3`.
    Column(String),
    /// A literal value, e.g. `3` or `'x'`.
    Literal(Value),
    /// `NOT <expr>`
    Not(Box<Expr>),
    /// `-<expr>`, e.g. `-price` or `-1`.
    Neg(Box<Expr>),
    /// `<left> <op> <right>`
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

/// Operators usable inside an [`Expr::BinaryOp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    And,
    Or,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Add,
    Sub,
    Mul,
    Div,
}
