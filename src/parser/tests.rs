use pretty_assertions::assert_eq;

use super::*;
use crate::lexer::Lexer;

/// Lexes and parses `src`, panicking (with the underlying error) if
/// either stage fails. Kept out of the public API, for tests only.
fn parse(src: &str) -> Vec<Statement> {
    let tokens = Lexer::new(src).tokenise().expect("lex failed");
    Parser::new(tokens).parse().expect("parse failed")
}

#[test]
fn parses_simple_select() {
    let stmts = parse("SELECT * FROM t;");
    assert_eq!(
        stmts,
        vec![Statement::Select(SelectStatement {
            columns: SelectColumns::All,
            table: "t".to_string(),
            filter: None,
        })]
    );
}

#[test]
fn parses_select_with_column_list_and_where() {
    let stmts = parse("SELECT a, b FROM t WHERE a = 1 AND b = 'x';");
    assert_eq!(
        stmts,
        vec![Statement::Select(SelectStatement {
            columns: SelectColumns::List(vec!["a".into(), "b".into()]),
            table: "t".into(),
            filter: Some(Expr::BinaryOp {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Column("a".into())),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal(Value::Integer(1))),
                }),
                op: BinaryOp::And,
                right: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Column("b".into())),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal(Value::Text("x".into()))),
                }),
            }),
        })]
    );
}

#[test]
fn and_binds_tighter_than_or() {
    // Should parse as: a = 1 OR (b = 2 AND c = 3)
    let stmts = parse("SELECT * FROM t WHERE a = 1 OR b = 2 AND c = 3;");
    let Statement::Select(select) = &stmts[0] else {
        panic!("expected select");
    };
    let Some(Expr::BinaryOp { op, left, right }) = &select.filter else {
        panic!("expected top-level binary op");
    };
    assert_eq!(*op, BinaryOp::Or);
    assert_eq!(**left, Expr::Column("a".into()).eq_literal(1));
    assert!(matches!(
        **right,
        Expr::BinaryOp {
            op: BinaryOp::And,
            ..
        }
    ));
}

#[test]
fn parenthesized_expr_overrides_precedence() {
    // Forces OR to bind first: (a = 1 OR b = 2) AND c = 3
    let stmts = parse("SELECT * FROM t WHERE (a = 1 OR b = 2) AND c = 3;");
    let Statement::Select(select) = &stmts[0] else {
        panic!("expected select");
    };
    let Some(Expr::BinaryOp { op, .. }) = &select.filter else {
        panic!("expected top-level binary op");
    };
    assert_eq!(*op, BinaryOp::And);
}

#[test]
fn parses_not() {
    let stmts = parse("SELECT * FROM t WHERE NOT a = 1;");
    let Statement::Select(select) = &stmts[0] else {
        panic!("expected select");
    };
    assert!(matches!(select.filter, Some(Expr::Not(_))));
}

#[test]
fn parses_insert() {
    let stmts = parse("INSERT INTO t VALUES (1, 'x', 2.5);");
    assert_eq!(
        stmts,
        vec![Statement::Insert(InsertStatement {
            table: "t".into(),
            values: vec![
                Value::Integer(1),
                Value::Text("x".into()),
                Value::Float(2.5),
            ],
        })]
    );
}

#[test]
fn parses_update() {
    let stmts = parse("UPDATE t SET a = 1, b = 'y' WHERE a = 2;");
    assert_eq!(
        stmts,
        vec![Statement::Update(UpdateStatement {
            table: "t".into(),
            assignments: vec![
                ("a".into(), Value::Integer(1)),
                ("b".into(), Value::Text("y".into())),
            ],
            filter: Some(Expr::BinaryOp {
                left: Box::new(Expr::Column("a".into())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(Value::Integer(2))),
            }),
        })]
    );
}

#[test]
fn parses_delete() {
    let stmts = parse("DELETE FROM t WHERE a = 1;");
    assert_eq!(
        stmts,
        vec![Statement::Delete(DeleteStatement {
            table: "t".into(),
            filter: Some(Expr::BinaryOp {
                left: Box::new(Expr::Column("a".into())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(Value::Integer(1))),
            }),
        })]
    );
}

#[test]
fn parses_create_table() {
    let stmts = parse("CREATE TABLE t (id INTEGER NOT NULL, name TEXT);");
    assert_eq!(
        stmts,
        vec![Statement::CreateTable(CreateTableStatement {
            table: "t".into(),
            columns: vec![
                ColumnDecl {
                    name: "id".into(),
                    ty: DataType::Integer,
                    nullable: false,
                },
                ColumnDecl {
                    name: "name".into(),
                    ty: DataType::Text,
                    nullable: true,
                },
            ],
        })]
    );
}

#[test]
fn parses_multiple_statements() {
    let stmts = parse("SELECT * FROM t; DELETE FROM t;");
    assert_eq!(stmts.len(), 2);
}

#[test]
fn errors_on_unknown_type() {
    let tokens = Lexer::new("CREATE TABLE t (id WIDGET);")
        .tokenise()
        .unwrap();
    let err = Parser::new(tokens).parse().unwrap_err();
    match err {
        ParseError::UnknownType { name, .. } => assert_eq!(name, "WIDGET"),
        other => panic!("expected UnknownType, got {other:?}"),
    }
}

#[test]
fn errors_on_unexpected_token() {
    let tokens = Lexer::new("SELECT FROM t;").tokenise().unwrap();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn errors_on_unexpected_eof() {
    let tokens = Lexer::new("SELECT * FROM").tokenise().unwrap();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedEof { .. }));
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    // a + b * c  =>  a + (b * c)
    let stmts = parse("SELECT * FROM t WHERE a + b * c = 1;");
    let Statement::Select(select) = &stmts[0] else {
        panic!("expected select")
    };
    let Some(Expr::BinaryOp {
        op: BinaryOp::Eq,
        left,
        ..
    }) = &select.filter
    else {
        panic!("expected top-level Eq");
    };
    let Expr::BinaryOp { op, right, .. } = left.as_ref() else {
        panic!("expected term-level binary op on the left of Eq");
    };
    assert_eq!(*op, BinaryOp::Add);
    assert!(matches!(
        right.as_ref(),
        Expr::BinaryOp {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn parens_override_arithmetic_precedence() {
    // (a + b) * c
    let stmts = parse("SELECT * FROM t WHERE (a + b) * c = 1;");
    let Statement::Select(select) = &stmts[0] else {
        panic!("expected select")
    };
    let Some(Expr::BinaryOp { left, .. }) = &select.filter else {
        panic!("expected top-level binary op");
    };
    let Expr::BinaryOp { op, .. } = left.as_ref() else {
        panic!("expected multiplication on the left")
    };
    assert_eq!(*op, BinaryOp::Mul);
}

#[test]
fn arithmetic_is_left_associative() {
    // a / b * c  =>  (a / b) * c, not a / (b * c)
    let stmts = parse("SELECT * FROM t WHERE a / b * c = 1;");
    let Statement::Select(select) = &stmts[0] else {
        panic!("expected select")
    };
    let Some(Expr::BinaryOp { left, .. }) = &select.filter else {
        panic!("expected top-level binary op");
    };
    let Expr::BinaryOp {
        op,
        left: inner_left,
        ..
    } = left.as_ref()
    else {
        panic!("expected multiplication on the left")
    };
    assert_eq!(*op, BinaryOp::Mul);
    assert!(matches!(
        inner_left.as_ref(),
        Expr::BinaryOp {
            op: BinaryOp::Div,
            ..
        }
    ));
}

#[test]
fn parses_unary_minus() {
    let stmts = parse("SELECT * FROM t WHERE a = -1;");
    let Statement::Select(select) = &stmts[0] else {
        panic!("expected select")
    };
    assert_eq!(
        select.filter,
        Some(Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Neg(Box::new(Expr::Literal(Value::Integer(1))))),
        })
    );
}

#[test]
fn parses_negative_literal_in_insert() {
    let stmts = parse("INSERT INTO t VALUES (-1, -2.5);");
    assert_eq!(
        stmts,
        vec![Statement::Insert(InsertStatement {
            table: "t".into(),
            values: vec![Value::Integer(-1), Value::Float(-2.5)],
        })]
    );
}

#[test]
fn errors_on_negating_string_literal() {
    let tokens = Lexer::new("INSERT INTO t VALUES (-'x');")
        .tokenise()
        .unwrap();
    let err = Parser::new(tokens).parse().unwrap_err();
    assert!(matches!(err, ParseError::NonNumericNegation { .. }));
}

#[test]
fn parses_full_arithmetic_and_logical_expression() {
    // Smoke test — each operator's precedence is covered individually above.
    let stmts = parse("SELECT * FROM t WHERE a * b + c = d AND -e < f;");
    assert_eq!(stmts.len(), 1);
}

// Small test-only helper to keep the precedence tests above
// readable without repeating the full `Expr::BinaryOp` shape.
impl Expr {
    fn eq_literal(self, n: i64) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(Value::Integer(n))),
        }
    }
}
