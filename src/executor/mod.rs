//! Executor: runs a [`BoundStatement`] against [`Catalog`] +
//! [`Storage`], producing an [`ExecResult`]. The last pipeline stage.
//! Everything before this is read-only checking. This is where rows
//! actually get read and written.
//!
//! ## NULL and three-valued logic
//!
//! Comparisons and logical operators follow standard SQL three-valued
//! logic: a NULL operand makes a comparison evaluate to UNKNOWN
//! (represented here as [`Value::Null`]; it's exactly the same
//! "don't know" as the literal), and UNKNOWN propagates through
//! `AND`/`OR`/`NOT` per the usual truth tables rather than collapsing
//! straight to `false`. A `WHERE`/filter predicate keeps a row only
//! when it evaluates to `Value::Boolean(true)`. Both `false` and
//! UNKNOWN exclude the row, matching every standard SQL engine.
//!
//! ## Division by zero
//!
//! Matches standard SQL (e.g. PostgreSQL): `/` by zero is a runtime
//! error ([`ExecError::DivisionByZero`]) for both `INTEGER` and
//! `FLOAT` operands. This deliberately does *not* follow IEEE-754
//! float semantics (`1.0 / 0.0 == inf`). SQL engines generally treat
//! division by zero as an error regardless of numeric type.
//!
//! ## Atomicity
//!
//! `UPDATE` and `DELETE` fully evaluate every row's filter (and, for
//! `UPDATE`, every assignment expression) *before* mutating anything.
//! If a runtime error (e.g. division by zero) occurs while evaluating
//! row 50 of 100, none of the first 49 rows have been touched either.
//! This is single-statement atomicity, not a transaction system. See
//! ARCHITECTURE.md's non-goals.

pub mod error;

pub use error::{ExecError, Result};

use crate::analyser::{
    BoundDelete, BoundExpr, BoundInsert, BoundSelect, BoundStatement, BoundUpdate,
};
use crate::catalog::{Catalog, ColumnDef, TableSchema};
use crate::parser::{BinaryOp, CreateTableStatement};
use crate::storage::Storage;
use crate::types::{Row, Value};

#[cfg(test)]
mod tests;

/// The outcome of running one statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecResult {
    /// Rows produced by a `SELECT`.
    Rows(Vec<Row>),
    /// Rows touched by an `INSERT`, `UPDATE`, or `DELETE`.
    RowsAffected(usize),
    /// A `CREATE TABLE` completed.
    TableCreated,
}

/// Runs bound statements against a catalog and its row storage.
/// Takes both by mutable reference rather than owning them. A
/// [`Database`](crate::db::Database) owns them for the process
/// lifetime and hands out an `Executor` per statement.
pub struct Executor<'a> {
    catalog: &'a mut Catalog,
    storage: &'a mut Storage,
}

impl<'a> Executor<'a> {
    pub fn new(catalog: &'a mut Catalog, storage: &'a mut Storage) -> Self {
        Self { catalog, storage }
    }

    pub fn execute(&mut self, statement: BoundStatement) -> Result<ExecResult> {
        match statement {
            BoundStatement::Select(s) => self.exec_select(s),
            BoundStatement::Insert(s) => self.exec_insert(s),
            BoundStatement::Update(s) => self.exec_update(s),
            BoundStatement::Delete(s) => self.exec_delete(s),
            BoundStatement::CreateTable(s) => self.exec_create_table(s),
        }
    }

    fn exec_select(&mut self, select: BoundSelect) -> Result<ExecResult> {
        let rows = self.storage.rows(&select.table)?;
        let mut result = Vec::new();
        for row in rows {
            if row_passes_filter(select.filter.as_ref(), row)? {
                let projected = select
                    .columns
                    .iter()
                    .map(|c| row.values[c.index].clone())
                    .collect();
                result.push(Row::new(projected));
            }
        }
        Ok(ExecResult::Rows(result))
    }

    fn exec_insert(&mut self, insert: BoundInsert) -> Result<ExecResult> {
        // VALUES expressions never contain BoundExpr::Column, the
        // analyser rejects that (see AnalyseError::ColumnInValues),
        // so there's no real row to evaluate against. This empty row
        // is never actually indexed into.
        let no_row = Row::new(Vec::new());
        let mut values = Vec::with_capacity(insert.values.len());
        for expr in &insert.values {
            values.push(eval(expr, &no_row)?);
        }
        self.storage.insert(&insert.table, Row::new(values))?;
        Ok(ExecResult::RowsAffected(1))
    }

    fn exec_update(&mut self, update: BoundUpdate) -> Result<ExecResult> {
        // Evaluate then apply: see module doc comment on atomicity.
        // All SET expressions are evaluated against pre-update row
        // values, matching standard SQL. A single UPDATE's SET list
        // sees the old row, not partially-updated values from earlier
        // assignments in the same list.
        let existing = self.storage.rows(&update.table)?;
        let mut planned: Vec<(usize, Vec<(usize, Value)>)> = Vec::new();
        for (row_index, row) in existing.iter().enumerate() {
            if row_passes_filter(update.filter.as_ref(), row)? {
                let mut changes = Vec::with_capacity(update.assignments.len());
                for (column, expr) in &update.assignments {
                    changes.push((column.index, eval(expr, row)?));
                }
                planned.push((row_index, changes));
            }
        }
        let affected = planned.len();
        let rows = self.storage.rows_mut(&update.table)?;
        for (row_index, changes) in planned {
            for (col_index, value) in changes {
                rows[row_index].values[col_index] = value;
            }
        }
        Ok(ExecResult::RowsAffected(affected))
    }

    fn exec_delete(&mut self, delete: BoundDelete) -> Result<ExecResult> {
        // Evaluate-then-apply, same reasoning as exec_update: decide
        // what to remove before removing anything.
        let existing = self.storage.rows(&delete.table)?;
        let mut keep = Vec::with_capacity(existing.len());
        for row in existing {
            keep.push(!row_passes_filter(delete.filter.as_ref(), row)?);
        }
        let removed = keep.iter().filter(|k| !**k).count();
        let rows = self.storage.rows_mut(&delete.table)?;
        let mut flags = keep.into_iter();
        rows.retain(|_| flags.next().unwrap());
        Ok(ExecResult::RowsAffected(removed))
    }

    fn exec_create_table(&mut self, stmt: CreateTableStatement) -> Result<ExecResult> {
        let schema = TableSchema {
            columns: stmt
                .columns
                .into_iter()
                .map(|c| ColumnDef {
                    name: c.name,
                    ty: c.ty,
                    nullable: c.nullable,
                })
                .collect(),
        };
        self.catalog.register_table(stmt.table.clone(), schema);
        self.storage.create_table(&stmt.table);
        Ok(ExecResult::TableCreated)
    }
}

/// A row is kept by a filter only if it evaluates to exactly
/// `Value::Boolean(true)`. `false` and UNKNOWN (`Value::Null`) both
/// exclude it, per standard SQL three-valued logic. No filter means
/// every row passes.
fn row_passes_filter(filter: Option<&BoundExpr>, row: &Row) -> Result<bool> {
    match filter {
        Some(expr) => Ok(matches!(eval(expr, row)?, Value::Boolean(true))),
        None => Ok(true),
    }
}

/// Evaluates a bound expression against a row. Assumes the expression
/// already passed analyser type checks, the one thing analysis
/// **can't** catch statically is division by zero, so that's the only
/// error well-typed input can actually produce here.
fn eval(expr: &BoundExpr, row: &Row) -> Result<Value> {
    match expr {
        BoundExpr::Column { index, .. } => Ok(row.values[*index].clone()),
        BoundExpr::Literal(value) => Ok(value.clone()),
        BoundExpr::Not(inner) => eval_not(eval(inner, row)?),
        BoundExpr::Neg(inner) => eval_neg(eval(inner, row)?),
        BoundExpr::BinaryOp { left, op, right } => {
            let l = eval(left, row)?;
            let r = eval(right, row)?;
            match op {
                BinaryOp::And => eval_and(l, r),
                BinaryOp::Or => eval_or(l, r),
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq => eval_cmp(*op, l, r),
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                    eval_arith(*op, l, r)
                }
            }
        }
    }
}

/// Maps a BOOLEAN-typed `Value` to `Some(bool)`, or `None` for SQL
/// NULL (UNKNOWN), the natural representation of three-valued logic
/// in Rust. Errors on anything else. The analyser guarantees logical
/// operators only ever see BOOLEAN or NULL operands.
fn as_tri_bool(v: Value) -> Result<Option<bool>> {
    match v {
        Value::Boolean(b) => Ok(Some(b)),
        Value::Null => Ok(None),
        other => Err(ExecError::InternalTypeError(format!(
            "expected BOOLEAN or NULL, got {other:?}"
        ))),
    }
}

fn tri_bool_to_value(b: Option<bool>) -> Value {
    match b {
        Some(b) => Value::Boolean(b),
        None => Value::Null,
    }
}

/// SQL `NOT`. `NOT UNKNOWN` is `UNKNOWN`.
fn eval_not(v: Value) -> Result<Value> {
    Ok(tri_bool_to_value(as_tri_bool(v)?.map(|b| !b)))
}

/// SQL `AND` truth table: `false` dominates (short-circuits to
/// `false` even against UNKNOWN), otherwise UNKNOWN propagates unless
/// both sides are `true`.
fn eval_and(l: Value, r: Value) -> Result<Value> {
    let (l, r) = (as_tri_bool(l)?, as_tri_bool(r)?);
    let result = match (l, r) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    };
    Ok(tri_bool_to_value(result))
}

/// SQL `OR` truth table: `true` dominates, otherwise UNKNOWN
/// propagates unless both sides are `false`.
fn eval_or(l: Value, r: Value) -> Result<Value> {
    let (l, r) = (as_tri_bool(l)?, as_tri_bool(r)?);
    let result = match (l, r) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    };
    Ok(tri_bool_to_value(result))
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Comparison with SQL NULL semantics: either side NULL makes the
/// whole comparison UNKNOWN, never `true`/`false`.
fn eval_cmp(op: BinaryOp, l: Value, r: Value) -> Result<Value> {
    if l == Value::Null || r == Value::Null {
        return Ok(Value::Null);
    }
    let ordering = match (&l, &r) {
        (Value::Integer(a), Value::Integer(b)) => a.partial_cmp(b),
        (Value::Text(a), Value::Text(b)) => a.partial_cmp(b),
        (Value::Boolean(a), Value::Boolean(b)) => a.partial_cmp(b),
        _ => match (as_f64(&l), as_f64(&r)) {
            (Some(a), Some(b)) => a.partial_cmp(&b),
            _ => {
                return Err(ExecError::InternalTypeError(format!(
                    "cannot compare {l:?} and {r:?}"
                )));
            }
        },
    };
    let Some(ordering) = ordering else {
        // NaN: every comparison is false except `<>`, matching
        // IEEE-754 and Value's documented PartialEq semantics (see
        // types.rs and ARCHITECTURE.md).
        return Ok(Value::Boolean(matches!(op, BinaryOp::NotEq)));
    };
    let result = match op {
        BinaryOp::Eq => ordering.is_eq(),
        BinaryOp::NotEq => ordering.is_ne(),
        BinaryOp::Lt => ordering.is_lt(),
        BinaryOp::LtEq => ordering.is_le(),
        BinaryOp::Gt => ordering.is_gt(),
        BinaryOp::GtEq => ordering.is_ge(),
        other => {
            return Err(ExecError::InternalTypeError(format!(
                "eval_cmp got non-comparison operator {other:?}"
            )));
        }
    };
    Ok(Value::Boolean(result))
}

/// Arithmetic with SQL NULL semantics (either side NULL propagates to
/// NULL) and standard SQL division by zero semantics. See the module
/// doc comment.
fn eval_arith(op: BinaryOp, l: Value, r: Value) -> Result<Value> {
    if l == Value::Null || r == Value::Null {
        return Ok(Value::Null);
    }
    if let (Value::Integer(a), Value::Integer(b)) = (&l, &r) {
        let (a, b) = (*a, *b);
        let result = match op {
            BinaryOp::Add => a.checked_add(b),
            BinaryOp::Sub => a.checked_sub(b),
            BinaryOp::Mul => a.checked_mul(b),
            BinaryOp::Div => {
                if b == 0 {
                    return Err(ExecError::DivisionByZero);
                }
                a.checked_div(b)
            }
            other => {
                return Err(ExecError::InternalTypeError(format!(
                    "eval_arith got non-arithmetic operator {other:?}"
                )))
            }
        };
        return result.map(Value::Integer).ok_or(ExecError::IntegerOverflow);
    }
    let (Some(a), Some(b)) = (as_f64(&l), as_f64(&r)) else {
        return Err(ExecError::InternalTypeError(format!(
            "cannot do arithmetic on {l:?} and {r:?}"
        )));
    };
    if matches!(op, BinaryOp::Div) && b == 0.0 {
        return Err(ExecError::DivisionByZero);
    }
    let result = match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => a / b,
        other => {
            return Err(ExecError::InternalTypeError(format!(
                "eval_arith got non-arithmetic operator {other:?}"
            )));
        }
    };
    Ok(Value::Float(result))
}

fn eval_neg(v: Value) -> Result<Value> {
    match v {
        Value::Integer(n) => Ok(Value::Integer(-n)),
        Value::Float(f) => Ok(Value::Float(-f)),
        Value::Null => Ok(Value::Null),
        other => Err(ExecError::InternalTypeError(format!(
            "NEG expected a numeric type or NULL, got {other:?}"
        ))),
    }
}
