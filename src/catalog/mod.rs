//! Catalog: tracks table and column metadata (schemas) so later stages can
//! validate a query (does this table exist? does this column exist?
//! what type is it?) before executing it.
//! The catalog holds no row data. It is schema only.

use crate::types::DataType;
use std::collections::HashMap;

/// Metadata for a single column: its name, declared type, and whether
/// `NULL` is permitted. `nullable` isn't enforced anywhere yet. 
/// This is schema storage only for now.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub ty: DataType,
    pub nullable: bool,
}

/// The full schema for one table: an ordered list of its columns.
/// Column order here is significant. It is what `SELECT *` and
/// positional `INSERT ... VALUES (...)` will rely on later.
#[derive(Debug, Clone, Default)]
pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
}

/// Registry of all known tables, keyed by name.
/// No support yet for multiple databases/schemas and
/// no identifier case-folding rules (unlike real engines, which usually
/// lowercase unquoted identifiers and are case-sensitive for quoted
/// ones). Table names are matched exactly as given.
#[derive(Default)]
pub struct Catalog {
    tables: HashMap<String, TableSchema>,
}

impl Catalog {
    /// Creates an empty catalog with no tables registered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a table's schema under `name`, overwriting any existing
    /// schema of the same name. This is a direct, unchecked insert.
    pub fn register_table(&mut self, name: impl Into<String>, schema: TableSchema) {
        self.tables.insert(name.into(), schema);
    }

    /// Returns whether a table with this exact name is registered.
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    /// Returns the schema for `name` if registered.
    pub fn schema(&self, name: &str) -> Option<&TableSchema> {
        self.tables.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_table() {
        let mut catalog = Catalog::new();
        catalog.register_table("users", TableSchema::default());

        assert!(catalog.has_table("users"));
        assert!(!catalog.has_table("invalid"));
    }
}
