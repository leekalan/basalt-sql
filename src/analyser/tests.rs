use pretty_assertions::assert_eq;

use super::*;
use crate::catalog::ColumnDef;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::types::Value;

fn test_catalog() -> Catalog {
    let mut catalog = Catalog::new();
    catalog.register_table(
        "users",
        TableSchema {
            columns: vec![
                ColumnDef {
                    name: "id".into(),
                    ty: DataType::Integer,
                    nullable: false,
                },
                ColumnDef {
                    name: "name".into(),
                    ty: DataType::Text,
                    nullable: true,
                },
                ColumnDef {
                    name: "balance".into(),
                    ty: DataType::Float,
                    nullable: true,
                },
            ],
        },
    );
    catalog
}

fn analyse(catalog: &Catalog, sql: &str) -> Result<BoundStatement> {
    let tokens = Lexer::new(sql).tokenise().expect("lex failed");
    let stmt = Parser::new(tokens).parse().expect("parse failed").remove(0);
    Analyser::new(catalog).analyse(stmt)
}

#[test]
fn binds_select_star_to_all_columns_in_schema_order() {
    let catalog = test_catalog();
    let bound = analyse(&catalog, "SELECT * FROM users;").unwrap();
    let BoundStatement::Select(select) = bound else {
        panic!("expected select")
    };
    assert_eq!(
        select.columns,
        vec![
            BoundColumn {
                name: "id".into(),
                index: 0,
                ty: DataType::Integer
            },
            BoundColumn {
                name: "name".into(),
                index: 1,
                ty: DataType::Text
            },
            BoundColumn {
                name: "balance".into(),
                index: 2,
                ty: DataType::Float
            },
        ]
    );
    assert_eq!(select.filter, None);
}

#[test]
fn binds_select_with_explicit_column_list() {
    let catalog = test_catalog();
    let bound = analyse(&catalog, "SELECT name, id FROM users;").unwrap();
    let BoundStatement::Select(select) = bound else {
        panic!("expected select")
    };
    assert_eq!(
        select.columns,
        vec![
            BoundColumn {
                name: "name".into(),
                index: 1,
                ty: DataType::Text
            },
            BoundColumn {
                name: "id".into(),
                index: 0,
                ty: DataType::Integer
            },
        ]
    );
}

#[test]
fn errors_on_unknown_table() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "SELECT * FROM ghosts;").unwrap_err();
    assert_eq!(
        err,
        AnalyseError::UnknownTable {
            name: "ghosts".into()
        }
    );
}

#[test]
fn errors_on_unknown_column_in_select_list() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "SELECT nickname FROM users;").unwrap_err();
    assert_eq!(
        err,
        AnalyseError::UnknownColumn {
            table: "users".into(),
            column: "nickname".into()
        }
    );
}

#[test]
fn errors_on_unknown_column_in_where() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "SELECT * FROM users WHERE nickname = 1;").unwrap_err();
    assert_eq!(
        err,
        AnalyseError::UnknownColumn {
            table: "users".into(),
            column: "nickname".into()
        }
    );
}

#[test]
fn binds_insert_with_matching_types() {
    let catalog = test_catalog();
    let bound = analyse(&catalog, "INSERT INTO users VALUES (1, 'Ada', 10.5);").unwrap();
    let BoundStatement::Insert(insert) = bound else {
        panic!("expected insert")
    };
    assert_eq!(
        insert.values,
        vec![
            BoundExpr::Literal(Value::Integer(1)),
            BoundExpr::Literal(Value::Text("Ada".into())),
            BoundExpr::Literal(Value::Float(10.5)),
        ]
    );
}

#[test]
fn errors_on_insert_value_count_mismatch() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "INSERT INTO users VALUES (1, 'Ada');").unwrap_err();
    assert_eq!(
        err,
        AnalyseError::ValueCountMismatch {
            table: "users".into(),
            expected: 3,
            found: 2
        }
    );
}

#[test]
fn errors_on_insert_type_mismatch() {
    let catalog = test_catalog();
    let err = analyse(
        &catalog,
        "INSERT INTO users VALUES ('not a number', 'Ada', 1.0);",
    )
    .unwrap_err();
    assert!(matches!(err, AnalyseError::TypeMismatch { .. }));
}

#[test]
fn binds_insert_with_computed_expression() {
    let catalog = test_catalog();
    let bound = analyse(
        &catalog,
        "INSERT INTO users VALUES (1 + 1, 'Ada', 10.0 / 2);",
    )
    .unwrap();
    let BoundStatement::Insert(insert) = bound else {
        panic!("expected insert")
    };
    assert_eq!(insert.values[0].static_type(), Some(DataType::Integer));
    assert_eq!(insert.values[2].static_type(), Some(DataType::Float));
}

#[test]
fn errors_on_column_reference_in_values() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "INSERT INTO users VALUES (id, 'Ada', 1.0);").unwrap_err();
    assert_eq!(err, AnalyseError::ColumnInValues { name: "id".into() });
}

#[test]
fn binds_update_with_computed_assignment() {
    let catalog = test_catalog();
    let bound = analyse(
        &catalog,
        "UPDATE users SET balance = balance * 2 WHERE id = 1;",
    )
    .unwrap();
    let BoundStatement::Update(update) = bound else {
        panic!("expected update")
    };
    assert_eq!(update.assignments.len(), 1);
    assert_eq!(
        update.assignments[0].0,
        BoundColumn {
            name: "balance".into(),
            index: 2,
            ty: DataType::Float
        }
    );
    assert!(update.filter.is_some());
}

#[test]
fn errors_on_update_unknown_column() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "UPDATE users SET nickname = 'x';").unwrap_err();
    assert_eq!(
        err,
        AnalyseError::UnknownColumn {
            table: "users".into(),
            column: "nickname".into()
        }
    );
}

#[test]
fn binds_delete_with_filter() {
    let catalog = test_catalog();
    let bound = analyse(&catalog, "DELETE FROM users WHERE id = 1;").unwrap();
    let BoundStatement::Delete(delete) = bound else {
        panic!("expected delete")
    };
    assert_eq!(delete.table, "users");
    assert!(delete.filter.is_some());
}

#[test]
fn binds_create_table() {
    let catalog = test_catalog();
    let bound = analyse(&catalog, "CREATE TABLE products (id INTEGER, price FLOAT);").unwrap();
    assert!(matches!(bound, BoundStatement::CreateTable(_)));
}

#[test]
fn errors_on_create_table_already_exists() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "CREATE TABLE users (id INTEGER);").unwrap_err();
    assert_eq!(
        err,
        AnalyseError::TableAlreadyExists {
            name: "users".into()
        }
    );
}

#[test]
fn errors_on_create_table_duplicate_column() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "CREATE TABLE products (id INTEGER, id TEXT);").unwrap_err();
    assert_eq!(err, AnalyseError::DuplicateColumn { name: "id".into() });
}

#[test]
fn negating_a_numeric_literal_is_fine() {
    let catalog = test_catalog();
    let bound = analyse(&catalog, "INSERT INTO users VALUES (-1, 'Ada', -2.5);").unwrap();
    let BoundStatement::Insert(insert) = bound else {
        panic!("expected insert")
    };
    assert_eq!(insert.values[0].static_type(), Some(DataType::Integer));
}

#[test]
fn errors_on_negating_non_numeric_column() {
    // This is the check deferred out of the parser earlier (see
    // ARCHITECTURE.md): `-name` parses fine syntactically since
    // `name` is just an identifier at parse time, and is rejected
    // here, at bind time, once we know `name` is TEXT.
    let catalog = test_catalog();
    let err = analyse(&catalog, "SELECT * FROM users WHERE -name = 'x';").unwrap_err();
    assert!(matches!(err, AnalyseError::TypeMismatch { .. }));
}

#[test]
fn errors_on_and_with_non_boolean_operand() {
    let catalog = test_catalog();
    // `id` is INTEGER, not BOOLEAN, so it can't stand alone as an
    // AND operand.
    let err = analyse(&catalog, "SELECT * FROM users WHERE id AND name = 'x';").unwrap_err();
    assert!(matches!(err, AnalyseError::TypeMismatch { .. }));
}

#[test]
fn errors_on_comparing_text_and_number() {
    let catalog = test_catalog();
    let err = analyse(&catalog, "SELECT * FROM users WHERE name > 5;").unwrap_err();
    assert!(matches!(err, AnalyseError::TypeMismatch { .. }));
}

#[test]
fn null_literal_is_exempt_from_static_type_checks() {
    let catalog = test_catalog();
    let schema = catalog.schema("users").unwrap();
    let analyser = Analyser::new(&catalog);
    let expr = Expr::BinaryOp {
        left: Box::new(Expr::Column("id".into())),
        op: BinaryOp::Eq,
        right: Box::new(Expr::Literal(Value::Null)),
    };
    assert!(analyser.bind_expr(&expr, schema, "users", true).is_ok());
}
