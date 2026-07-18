//! Types: The layout and instance of data in a table.
//! 
//! - [DataType] describes what a column *can* hold (schema-level),
//! - [Value] is what a cell *actually* holds (runtime data) and `Row` is
//!   an ordered collection of [Value]s.
//! 
//! Names live in [TableSchema](crate::catalog::TableSchema),
//! not on the row itself.

/// The type a column is declared to hold.
/// Used in [ColumnDef](crate::catalog::ColumnDef).
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Integer,
    Float,
    Text,
    Boolean,
}

/// A single cell's runtime value. Note: `PartialEq` here uses standard
/// Rust/IEEE-754 float comparison, so Value::Float(f64::NAN) ==
/// Value::Float(f64::NAN)` is `false` and `0.1 + 0.2 != 0.3`. Real SQL
/// engines make a deliberate choice about float/NULL comparison
/// semantics. This type doesn't encode that choice yet.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Null,
}

/// An ordered list of column values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub values: Vec<Value>,
}

impl Row {
    /// Builds a row from an already-ordered list of values.
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }
}
