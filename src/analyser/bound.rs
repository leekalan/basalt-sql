//! The "bound" AST: a [`Statement`](crate::parser::Statement) tree
//! after it's been checked against a [`Catalog`](crate::catalog::Catalog).
//! Column references are resolved to a concrete index + type, so the
//! executor never needs to re resolve names. It indexes into a
//! [`Row`](crate::types::Row).

use crate::parser::{BinaryOp, CreateTableStatement};
use crate::types::{DataType, Value};

/// A statement after binding. Mirrors
/// [`Statement`](crate::parser::Statement) one for one, except
/// `CreateTable` is passed through unchanged as there's no schema to
/// bind column references against when the schema is what's being
/// defined.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundStatement {
    Select(BoundSelect),
    Insert(BoundInsert),
    Update(BoundUpdate),
    Delete(BoundDelete),
    CreateTable(CreateTableStatement),
}

/// A resolved reference to a table column: its declared name, its
/// position in the table's schema (used to index into a
/// [`Row`](crate::types::Row)), and its declared type.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundColumn {
    pub name: String,
    pub index: usize,
    pub ty: DataType,
}

/// `SELECT <columns> FROM <table> [WHERE <filter>]`, bound.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundSelect {
    pub table: String,
    /// Resolved in schema order for `SELECT *`, or as written for an
    /// explicit column list.
    pub columns: Vec<BoundColumn>,
    pub filter: Option<BoundExpr>,
}

/// `INSERT INTO <table> VALUES (<values>)`, bound.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundInsert {
    pub table: String,
    /// Positional, one per column in schema order. Never contains a
    /// [`BoundExpr::Column`] — see
    /// [`AnalyseError::ColumnInValues`](crate::analyser::AnalyseError::ColumnInValues).
    pub values: Vec<BoundExpr>,
}

/// `UPDATE <table> SET <col> = <value>, ... [WHERE <filter>]`, bound.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundUpdate {
    pub table: String,
    pub assignments: Vec<(BoundColumn, BoundExpr)>,
    pub filter: Option<BoundExpr>,
}

/// `DELETE FROM <table> [WHERE <filter>]`, bound.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundDelete {
    pub table: String,
    pub filter: Option<BoundExpr>,
}

/// [`Expr`](crate::parser::Expr) after binding: column references
/// carry their resolved index and type instead of a bare name.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundExpr {
    Column {
        name: String,
        index: usize,
        ty: DataType,
    },
    Literal(Value),
    Not(Box<BoundExpr>),
    Neg(Box<BoundExpr>),
    BinaryOp {
        left: Box<BoundExpr>,
        op: BinaryOp,
        right: Box<BoundExpr>,
    },
    IsNull {
        expr: Box<BoundExpr>,
        negated: bool,
    },
}

impl BoundExpr {
    /// This expression's type, if statically determinable.
    ///
    /// Returns `None` for anything built from a [`Value::Null`]
    /// literal, NULL's type is only meaningful at runtime (SQL's
    /// three valued logic), so it's treated as compatible with
    /// anything during binding rather than rejected. Every type check
    /// in this module destructures to `None => skip the check` for
    /// exactly this reason.
    pub fn static_type(&self) -> Option<DataType> {
        match self {
            BoundExpr::Column { ty, .. } => Some(ty.clone()),
            BoundExpr::Literal(Value::Integer(_)) => Some(DataType::Integer),
            BoundExpr::Literal(Value::Float(_)) => Some(DataType::Float),
            BoundExpr::Literal(Value::Text(_)) => Some(DataType::Text),
            BoundExpr::Literal(Value::Boolean(_)) => Some(DataType::Boolean),
            BoundExpr::Literal(Value::Null) => None,
            BoundExpr::Not(_) => Some(DataType::Boolean),
            // Always a definite BOOLEAN, never UNKNOWN — `x IS NULL`
            // is one of the few SQL constructs guaranteed not to
            // itself produce NULL, regardless of what `expr` is.
            BoundExpr::IsNull { .. } => Some(DataType::Boolean),
            BoundExpr::Neg(inner) => inner.static_type(),
            BoundExpr::BinaryOp { op, left, right } => match op {
                BinaryOp::And
                | BinaryOp::Or
                | BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq => Some(DataType::Boolean),
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                    match (left.static_type(), right.static_type()) {
                        (Some(DataType::Float), _) | (_, Some(DataType::Float)) => {
                            Some(DataType::Float)
                        }
                        (Some(DataType::Integer), Some(DataType::Integer)) => {
                            Some(DataType::Integer)
                        }
                        _ => None,
                    }
                }
            },
        }
    }
}
