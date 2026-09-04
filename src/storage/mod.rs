//! Storage: holds actual row data, keyed by table name. Knows nothing
//! about SQL, only an in-memory `Vec<Row>` per table. Schema lives in
//! [`Catalog`](crate::catalog::Catalog), which is explicitly
//! schema-only ("The catalog holds no row data". See catalog.rs);
//! this module is the other half.

mod error;
pub use error::{Result, StorageError};

use std::collections::HashMap;

use crate::types::Row;

#[cfg(test)]
mod tests;

/// In-memory row storage, one `Vec<Row>` per table.
#[derive(Default)]
pub struct Storage {
    tables: HashMap<String, Vec<Row>>,
}

impl Storage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an empty row set for a new table. An
    /// already registered table's rows are left untouched rather than
    /// cleared, though in practice this is only reachable for
    /// not yet existing tables: the analyser rejects `CREATE TABLE`
    /// for a name that's already registered before the executor ever
    /// calls this.
    pub fn create_table(&mut self, name: &str) {
        self.tables.entry(name.to_string()).or_default();
    }

    pub fn insert(&mut self, table: &str, row: Row) -> Result<()> {
        self.rows_mut(table)?.push(row);
        Ok(())
    }

    pub fn rows(&self, table: &str) -> Result<&[Row]> {
        self.tables
            .get(table)
            .map(Vec::as_slice)
            .ok_or_else(|| StorageError::MissingTable {
                name: table.to_string(),
            })
    }

    pub fn rows_mut(&mut self, table: &str) -> Result<&mut Vec<Row>> {
        self.tables
            .get_mut(table)
            .ok_or_else(|| StorageError::MissingTable {
                name: table.to_string(),
            })
    }
}
