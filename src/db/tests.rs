use pretty_assertions::assert_eq;

use super::*;
use crate::error::Error;
use crate::executor::ExecError;
use crate::types::{Row, Value};

#[test]
fn create_insert_select_round_trip() {
    let mut db = Database::new();
    db.execute("CREATE TABLE users (id INTEGER, name TEXT, balance FLOAT);")
        .unwrap();
    db.execute("INSERT INTO users VALUES (1, 'Ada', 10.5);")
        .unwrap();
    db.execute("INSERT INTO users VALUES (2, 'Bo', 3.0);")
        .unwrap();

    let result = db
        .execute("SELECT * FROM users WHERE balance > 5;")
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        ExecResult::Rows(vec![Row::new(vec![
            Value::Integer(1),
            Value::Text("Ada".into()),
            Value::Float(10.5),
        ])])
    );
}

#[test]
fn select_with_column_list_and_computed_where() {
    let mut db = Database::new();
    db.execute("CREATE TABLE t (id INTEGER, qty INTEGER);")
        .unwrap();
    db.execute("INSERT INTO t VALUES (1, 4);").unwrap();
    db.execute("INSERT INTO t VALUES (2, 10);").unwrap();

    let result = db
        .execute("SELECT id FROM t WHERE qty * 2 > 15;")
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        ExecResult::Rows(vec![Row::new(vec![Value::Integer(2)])])
    );
}

#[test]
fn update_and_delete() {
    let mut db = Database::new();
    db.execute("CREATE TABLE t (id INTEGER, qty INTEGER);")
        .unwrap();
    db.execute("INSERT INTO t VALUES (1, 10);").unwrap();
    db.execute("INSERT INTO t VALUES (2, 20);").unwrap();

    let updated = db
        .execute("UPDATE t SET qty = qty * 2 WHERE id = 1;")
        .unwrap()
        .unwrap();
    assert_eq!(updated, ExecResult::RowsAffected(1));

    // Both rows now have qty > 15: id 1 -> 20, id 2 -> 20.
    let deleted = db
        .execute("DELETE FROM t WHERE qty > 15;")
        .unwrap()
        .unwrap();
    assert_eq!(deleted, ExecResult::RowsAffected(2));

    let remaining = db.execute("SELECT * FROM t;").unwrap().unwrap();
    assert_eq!(remaining, ExecResult::Rows(vec![]));
}

#[test]
fn division_by_zero_is_a_runtime_error() {
    let mut db = Database::new();
    db.execute("CREATE TABLE t (id INTEGER, qty INTEGER);")
        .unwrap();
    db.execute("INSERT INTO t VALUES (1, 10);").unwrap();

    let err = db.execute("SELECT * FROM t WHERE id / 0 = 1;").unwrap_err();
    assert!(matches!(err, Error::Exec(ExecError::DivisionByZero)));
}

#[test]
fn analyser_rejects_before_execution_touches_storage() {
    let mut db = Database::new();
    db.execute("CREATE TABLE t (id INTEGER);").unwrap();

    let err = db
        .execute("INSERT INTO t VALUES ('not a number');")
        .unwrap_err();
    assert!(matches!(err, Error::Analyse(_)));

    // No row was inserted, since analysis fails before execution.
    let rows = db.execute("SELECT * FROM t;").unwrap().unwrap();
    assert_eq!(rows, ExecResult::Rows(vec![]));
}

#[test]
fn execute_all_runs_a_batch_in_order() {
    let mut db = Database::new();
    let results = db
        .execute_all(
            "CREATE TABLE t (id INTEGER); \
                INSERT INTO t VALUES (1); \
                INSERT INTO t VALUES (2);",
        )
        .unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0], ExecResult::TableCreated);
    assert_eq!(results[1], ExecResult::RowsAffected(1));
    assert_eq!(results[2], ExecResult::RowsAffected(1));
}

#[test]
fn execute_returns_none_for_empty_input() {
    let mut db = Database::new();
    assert_eq!(db.execute("   ").unwrap(), None);
}

#[test]
fn three_valued_logic_is_reachable_from_real_sql() {
    let mut db = Database::new();
    db.execute("CREATE TABLE t (id INTEGER, flag BOOLEAN);")
        .unwrap();
    db.execute("INSERT INTO t VALUES (1, NULL);").unwrap();
    db.execute("INSERT INTO t VALUES (2, TRUE);").unwrap();
    db.execute("INSERT INTO t VALUES (3, FALSE);").unwrap();

    // `flag = TRUE` excludes both the NULL row (UNKNOWN) and the
    // FALSE row, only an actual `true` passes a WHERE filter.
    let result = db
        .execute("SELECT id FROM t WHERE flag = TRUE;")
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        ExecResult::Rows(vec![Row::new(vec![Value::Integer(2)])])
    );
}
