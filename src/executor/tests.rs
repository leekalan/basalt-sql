use pretty_assertions::assert_eq;

use super::*;

#[test]
fn and_truth_table_matches_three_valued_logic() {
    let t = Value::Boolean(true);
    let f = Value::Boolean(false);
    let u = Value::Null;

    assert_eq!(
        eval_and(t.clone(), t.clone()).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        eval_and(t.clone(), f.clone()).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        eval_and(f.clone(), t.clone()).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        eval_and(f.clone(), f.clone()).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(eval_and(t.clone(), u.clone()).unwrap(), Value::Null);
    assert_eq!(eval_and(u.clone(), t.clone()).unwrap(), Value::Null);
    // False dominates even against UNKNOWN.
    assert_eq!(
        eval_and(f.clone(), u.clone()).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        eval_and(u.clone(), f.clone()).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(eval_and(u.clone(), u.clone()).unwrap(), Value::Null);
}

#[test]
fn or_truth_table_matches_three_valued_logic() {
    let t = Value::Boolean(true);
    let f = Value::Boolean(false);
    let u = Value::Null;

    assert_eq!(eval_or(t.clone(), t.clone()).unwrap(), Value::Boolean(true));
    assert_eq!(
        eval_or(f.clone(), f.clone()).unwrap(),
        Value::Boolean(false)
    );
    // True dominates even against UNKNOWN.
    assert_eq!(eval_or(t.clone(), u.clone()).unwrap(), Value::Boolean(true));
    assert_eq!(eval_or(u.clone(), t.clone()).unwrap(), Value::Boolean(true));
    assert_eq!(eval_or(f.clone(), u.clone()).unwrap(), Value::Null);
    assert_eq!(eval_or(u.clone(), f.clone()).unwrap(), Value::Null);
    assert_eq!(eval_or(u.clone(), u.clone()).unwrap(), Value::Null);
}

#[test]
fn not_unknown_is_unknown() {
    assert_eq!(eval_not(Value::Null).unwrap(), Value::Null);
    assert_eq!(
        eval_not(Value::Boolean(true)).unwrap(),
        Value::Boolean(false)
    );
}

#[test]
fn comparison_with_null_is_unknown_not_false() {
    let result = eval_cmp(BinaryOp::Eq, Value::Integer(1), Value::Null).unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn integer_division_by_zero_errors() {
    let err = eval_arith(BinaryOp::Div, Value::Integer(1), Value::Integer(0)).unwrap_err();
    assert_eq!(err, ExecError::DivisionByZero);
}

#[test]
fn float_division_by_zero_errors_not_infinity() {
    // Deliberately NOT IEEE-754 semantics — see module doc comment.
    let err = eval_arith(BinaryOp::Div, Value::Float(1.0), Value::Float(0.0)).unwrap_err();
    assert_eq!(err, ExecError::DivisionByZero);
}

#[test]
fn mixed_integer_float_arithmetic_promotes_to_float() {
    let result = eval_arith(BinaryOp::Add, Value::Integer(1), Value::Float(2.5)).unwrap();
    assert_eq!(result, Value::Float(3.5));
}

#[test]
fn null_propagates_through_arithmetic() {
    let result = eval_arith(BinaryOp::Add, Value::Integer(1), Value::Null).unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn nan_comparisons_are_all_false_except_not_eq() {
    let nan = Value::Float(f64::NAN);
    assert_eq!(
        eval_cmp(BinaryOp::Eq, nan.clone(), nan.clone()).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        eval_cmp(BinaryOp::NotEq, nan.clone(), nan.clone()).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        eval_cmp(BinaryOp::Lt, nan.clone(), nan).unwrap(),
        Value::Boolean(false)
    );
}
