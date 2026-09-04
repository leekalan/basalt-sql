use pretty_assertions::assert_eq;

use super::*;

#[test]
fn tokenises_simple_select() {
    let tokens = Lexer::new("SELECT * FROM t;").tokenise().unwrap();
    assert_eq!(tokens.first().unwrap().kind, TokenKind::Select);
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn errors_on_unexpected_char() {
    let err = Lexer::new("SELECT $ FROM t;").tokenise().unwrap_err();
    assert_eq!(err, LexError::UnexpectedChar { ch: '$', offset: 7 });
}

#[test]
fn errors_on_unterminated_string() {
    let err = Lexer::new("SELECT * FROM t WHERE x = 'oops")
        .tokenise()
        .unwrap_err();
    assert_eq!(err, LexError::UnterminatedString { offset: 26 });
}

#[test]
fn tokenises_comparison_operators() {
    let tokens = Lexer::new("< <= > >= <> !=").tokenise().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Lt,
            TokenKind::LtEq,
            TokenKind::Gt,
            TokenKind::GtEq,
            TokenKind::NotEq,
            TokenKind::NotEq,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn tokenises_arithmetic_operators() {
    let tokens = Lexer::new("+ - * /").tokenise().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Add,
            TokenKind::Sub,
            TokenKind::Star,
            TokenKind::Div,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn distinguishes_lt_from_lteq() {
    // Regression test for peek_n: "< " must not be mistaken for "<=".
    let tokens = Lexer::new("a < b").tokenise().unwrap();
    assert_eq!(tokens[1].kind, TokenKind::Lt);
}

#[test]
fn lone_operator_at_end_of_input() {
    // peek_n(2) returns None with only one char left. this exercises
    // the fallback from the two-char block into the single-char block.
    let tokens = Lexer::new("a <").tokenise().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![TokenKind::Ident("a".into()), TokenKind::Lt, TokenKind::Eof]
    );
}

#[test]
fn keywords_are_case_insensitive() {
    let tokens = Lexer::new("select From WHERE").tokenise().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Select,
            TokenKind::From,
            TokenKind::Where,
            TokenKind::Eof
        ]
    );
}

#[test]
fn tokenises_float_and_integer_numbers() {
    let tokens = Lexer::new("42 3.14").tokenise().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number("42".into()),
            TokenKind::Number("3.14".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn comment_does_not_consume_past_newline() {
    let tokens = Lexer::new("SELECT 1 -- comment\nFROM t")
        .tokenise()
        .unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Select,
            TokenKind::Number("1".into()),
            TokenKind::From,
            TokenKind::Ident("t".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn unspaced_double_minus_is_swallowed_as_a_comment() {
    // Subtlety worth pinning down explicitly: "5--3" is NOT "5 - -3".
    // skip_whitespace_and_comments matches "--" before arithmetic ever
    // gets a look, so everything after it on the line is discarded.
    // "5 - -3" (spaced) is required to get two Sub tokens.
    let tokens = Lexer::new("5--3").tokenise().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(kinds, vec![TokenKind::Number("5".into()), TokenKind::Eof]);
}

#[test]
fn empty_input_produces_only_eof() {
    let tokens = Lexer::new("").tokenise().unwrap();
    assert_eq!(tokens, vec![Token::new(TokenKind::Eof, 0)]);
}

#[test]
fn tokenises_null_true_false_keywords() {
    let tokens = Lexer::new("NULL true False").tokenise().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Null,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Eof
        ]
    );
}

#[test]
fn tokenises_is_keyword() {
    let tokens = Lexer::new("IS").tokenise().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Is);
}
