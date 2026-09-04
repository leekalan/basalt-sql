use pretty_assertions::assert_eq;

use super::*;

#[test]
fn insert_and_read_rows() {
    let mut storage = Storage::new();
    storage.create_table("t");
    storage.insert("t", Row::new(vec![])).unwrap();
    storage.insert("t", Row::new(vec![])).unwrap();
    assert_eq!(storage.rows("t").unwrap().len(), 2);
}

#[test]
fn errors_on_missing_table() {
    let storage = Storage::new();
    let err = storage.rows("ghost").unwrap_err();
    assert_eq!(
        err,
        StorageError::MissingTable {
            name: "ghost".into()
        }
    );
}

#[test]
fn create_table_is_idempotent_and_keeps_existing_rows() {
    let mut storage = Storage::new();
    storage.create_table("t");
    storage.insert("t", Row::new(vec![])).unwrap();
    storage.create_table("t");
    assert_eq!(storage.rows("t").unwrap().len(), 1);
}
