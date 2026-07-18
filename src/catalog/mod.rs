use crate::types::DataType;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub ty: DataType,
    pub nullable: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
}

#[derive(Default)]
pub struct Catalog {
    tables: HashMap<String, TableSchema>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_table(&mut self, name: impl Into<String>, schema: TableSchema) {
        self.tables.insert(name.into(), schema);
    }

    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

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
