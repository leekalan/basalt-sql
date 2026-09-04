//! Parser: turns a stream of [Token]s (from the lexer) into a stream of
//! [Statement]s. Purely syntactic — no catalog lookups, no type
//! checking. That validation belongs to a later analyser stage; this
//! stage only asks "is this grammatically valid SQL?".
//!
//! Grammar (loosest to tightest binding for expressions):
//!
//! ```text
//! statement   := select | insert | update | delete | create_table
//!
//! select      := SELECT (STAR | ident (',' ident)*) FROM ident where?
//! insert      := INSERT INTO ident VALUES '(' expr (',' expr)* ')'
//! update      := UPDATE ident SET assignment (',' assignment)* where?
//! delete      := DELETE FROM ident where?
//! create_table:= CREATE TABLE ident '(' column_decl (',' column_decl)* ')'
//!
//! assignment  := ident '=' expr
//! column_decl := ident type_name (NOT NULL)?
//! where       := WHERE expr
//!
//! expr        := or_expr
//! or_expr     := and_expr (OR and_expr)*
//! and_expr    := not_expr (AND not_expr)*
//! not_expr    := NOT not_expr | comparison
//! comparison  := term (comp_op term)?
//! term        := factor ((PLUS | MINUS) factor)*
//! factor      := unary ((STAR | SLASH) unary)*
//! unary       := MINUS unary | primary
//! primary     := ident | literal | '(' expr ')'
//! literal     := number | string
//! comp_op     := '=' | '<>' | '<' | '<=' | '>' | '>='
//! ```

pub mod ast;
mod error;

pub use ast::*;
pub use error::{ParseError, Result};

use crate::lexer::{Token, TokenKind};
use crate::types::{DataType, Value};

#[cfg(test)]
mod tests;

/// Converts a token stream into a stream of [Statement]s. Holds a
/// cursor (`pos`) into the tokens and advances it one token at a time,
/// mirroring the lexer's own cursor-based design.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Creates a parser over `tokens`. `tokens` is expected to end with
    /// a trailing [TokenKind::Eof], as produced by
    /// [Lexer::tokenise](crate::lexer::Lexer::tokenise).
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Consumes the parser and parses the entire token stream into zero
    /// or more statements, separated (and optionally terminated) by
    /// `;`. Fails on the first syntax error encountered.
    pub fn parse(mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();
        loop {
            while self.check(&TokenKind::Semicolon) {
                self.advance();
            }
            if self.check(&TokenKind::Eof) {
                break;
            }
            statements.push(self.parse_statement()?);
            if self.check(&TokenKind::Semicolon) {
                self.advance();
            }
        }
        Ok(statements)
    }

    /// Dispatches to the right statement parser based on the leading
    /// keyword.
    fn parse_statement(&mut self) -> Result<Statement> {
        match self.peek_kind() {
            TokenKind::Select => self.parse_select().map(Statement::Select),
            TokenKind::Insert => self.parse_insert().map(Statement::Insert),
            TokenKind::Update => self.parse_update().map(Statement::Update),
            TokenKind::Delete => self.parse_delete().map(Statement::Delete),
            TokenKind::Create => self.parse_create_table().map(Statement::CreateTable),
            _ => Err(self.error("a statement (SELECT, INSERT, UPDATE, DELETE, or CREATE TABLE)")),
        }
    }

    // --- STATEMENTS ---

    fn parse_select(&mut self) -> Result<SelectStatement> {
        self.expect(TokenKind::Select, "SELECT")?;
        let columns = self.parse_select_columns()?;
        self.expect(TokenKind::From, "FROM")?;
        let table = self.expect_ident()?;
        let filter = self.parse_optional_where()?;
        Ok(SelectStatement {
            columns,
            table,
            filter,
        })
    }

    fn parse_select_columns(&mut self) -> Result<SelectColumns> {
        if self.check(&TokenKind::Star) {
            self.advance();
            return Ok(SelectColumns::All);
        }
        let mut columns = vec![self.expect_ident()?];
        while self.check(&TokenKind::Comma) {
            self.advance();
            columns.push(self.expect_ident()?);
        }
        Ok(SelectColumns::List(columns))
    }

    fn parse_insert(&mut self) -> Result<InsertStatement> {
        self.expect(TokenKind::Insert, "INSERT")?;
        self.expect(TokenKind::Into, "INTO")?;
        let table = self.expect_ident()?;
        self.expect(TokenKind::Values, "VALUES")?;
        self.expect(TokenKind::LParen, "(")?;
        let mut values = vec![self.parse_expr()?];
        while self.check(&TokenKind::Comma) {
            self.advance();
            values.push(self.parse_expr()?);
        }
        self.expect(TokenKind::RParen, ")")?;
        Ok(InsertStatement { table, values })
    }

    fn parse_update(&mut self) -> Result<UpdateStatement> {
        self.expect(TokenKind::Update, "UPDATE")?;
        let table = self.expect_ident()?;
        self.expect(TokenKind::Set, "SET")?;
        let mut assignments = vec![self.parse_assignment()?];
        while self.check(&TokenKind::Comma) {
            self.advance();
            assignments.push(self.parse_assignment()?);
        }
        let filter = self.parse_optional_where()?;
        Ok(UpdateStatement {
            table,
            assignments,
            filter,
        })
    }

    fn parse_assignment(&mut self) -> Result<(String, Expr)> {
        let column = self.expect_ident()?;
        self.expect(TokenKind::Eq, "=")?;
        let value = self.parse_expr()?;
        Ok((column, value))
    }

    fn parse_delete(&mut self) -> Result<DeleteStatement> {
        self.expect(TokenKind::Delete, "DELETE")?;
        self.expect(TokenKind::From, "FROM")?;
        let table = self.expect_ident()?;
        let filter = self.parse_optional_where()?;
        Ok(DeleteStatement { table, filter })
    }

    fn parse_create_table(&mut self) -> Result<CreateTableStatement> {
        self.expect(TokenKind::Create, "CREATE")?;
        self.expect(TokenKind::Table, "TABLE")?;
        let table = self.expect_ident()?;
        self.expect(TokenKind::LParen, "(")?;
        let mut columns = vec![self.parse_column_decl()?];
        while self.check(&TokenKind::Comma) {
            self.advance();
            columns.push(self.parse_column_decl()?);
        }
        self.expect(TokenKind::RParen, ")")?;
        Ok(CreateTableStatement { table, columns })
    }

    fn parse_column_decl(&mut self) -> Result<ColumnDecl> {
        let name = self.expect_ident()?;
        let ty = self.parse_data_type()?;
        let mut nullable = true;
        if self.check(&TokenKind::Not) {
            self.advance();
            self.expect(TokenKind::Null, "NULL")?;
            nullable = false;
        }
        Ok(ColumnDecl { name, ty, nullable })
    }

    /// Type names (`INTEGER`, `FLOAT`, `TEXT`, `BOOLEAN`, ...) aren't
    /// reserved keywords in the lexer either, so they arrive as a plain
    /// `Ident` and are matched here rather than as their own TokenKind.
    fn parse_data_type(&mut self) -> Result<DataType> {
        let offset = self.peek().offset;
        let name = self.expect_ident()?;
        match name.to_ascii_uppercase().as_str() {
            "INTEGER" | "INT" => Ok(DataType::Integer),
            "FLOAT" | "REAL" | "DOUBLE" => Ok(DataType::Float),
            "TEXT" | "VARCHAR" | "STRING" => Ok(DataType::Text),
            "BOOLEAN" | "BOOL" => Ok(DataType::Boolean),
            _ => Err(ParseError::UnknownType { name, offset }),
        }
    }

    fn parse_optional_where(&mut self) -> Result<Option<Expr>> {
        if self.check(&TokenKind::Where) {
            self.advance();
            Ok(Some(self.parse_expr()?))
        } else {
            Ok(None)
        }
    }

    // --- Expressions ---

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_not()?;
        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr> {
        if self.check(&TokenKind::Not) {
            self.advance();
            let inner = self.parse_not()?;
            Ok(Expr::Not(Box::new(inner)))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let left = self.parse_term()?;
        if let Some(op) = self.match_comparison_op() {
            let right = self.parse_term()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    /// Consumes and returns a comparison operator token if the current
    /// token is one, otherwise leaves the cursor untouched.
    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_kind() {
            TokenKind::Eq => BinaryOp::Eq,
            TokenKind::NotEq => BinaryOp::NotEq,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::LtEq => BinaryOp::LtEq,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::GtEq => BinaryOp::GtEq,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_term(&mut self) -> Result<Expr> {
        let mut left = self.parse_factor()?;
        while let Some(op) = self.match_term_op() {
            let right = self.parse_factor()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn match_term_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_kind() {
            TokenKind::Add => BinaryOp::Add,
            TokenKind::Sub => BinaryOp::Sub,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_factor(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.match_factor_op() {
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn match_factor_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_kind() {
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::Div => BinaryOp::Div,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    /// `-<unary>`, right-recursive so `- - a` parses (as double negation)
    /// rather than needing special-casing.
    fn parse_unary(&mut self) -> Result<Expr> {
        if self.check(&TokenKind::Sub) {
            self.advance();
            let operand = self.parse_unary()?;
            Ok(Expr::Neg(Box::new(operand)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.peek_kind().clone() {
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(TokenKind::RParen, ")")?;
                Ok(inner)
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Column(name))
            }
            TokenKind::Number(_)
            | TokenKind::StringLit(_)
            | TokenKind::Null
            | TokenKind::True
            | TokenKind::False => Ok(Expr::Literal(self.parse_literal()?)),
            _ => Err(self.error("a column, literal, or '('")),
        }
    }

    fn parse_literal(&mut self) -> Result<Value> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Number(text) => {
                self.advance();
                parse_number_literal(text, tok.offset)
            }
            TokenKind::StringLit(text) => {
                self.advance();
                Ok(Value::Text(text.clone()))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Value::Null)
            }
            TokenKind::True => {
                self.advance();
                Ok(Value::Boolean(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Value::Boolean(false))
            }
            _ => Err(self.error("a literal value")),
        }
    }

    // --- HELPERS ---

    /// Returns the current token without consuming it.
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    /// Returns the current token and advances the cursor, unless
    /// already at the trailing [TokenKind::Eof] (which is never
    /// consumed, so repeated calls at end-of-input are safe).
    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    /// Consumes the current token if it matches `kind`, otherwise
    /// fails with an error naming `expected`.
    fn expect(&mut self, kind: TokenKind, expected: &str) -> Result<Token> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(self.error(expected))
        }
    }

    /// Consumes the current token if it's an identifier and returns its
    /// text, otherwise fails.
    fn expect_ident(&mut self) -> Result<String> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("identifier")),
        }
    }

    /// Builds the appropriate error variant for the current token:
    /// [ParseError::UnexpectedEof] at end of input, otherwise
    /// [ParseError::UnexpectedToken].
    fn error(&self, expected: &str) -> ParseError {
        let tok = self.peek();
        if tok.kind == TokenKind::Eof {
            ParseError::UnexpectedEof {
                expected: expected.to_string(),
            }
        } else {
            ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: tok.kind.clone(),
                offset: tok.offset,
            }
        }
    }
}

/// Parses a [TokenKind::Number]'s source text into a [Value]. Numbers
/// containing `.` become [Value::Float], everything else
/// [Value::Integer].
fn parse_number_literal(text: &str, offset: usize) -> Result<Value> {
    if text.contains('.') {
        text.parse::<f64>()
            .map(Value::Float)
            .map_err(|_| ParseError::InvalidNumber {
                text: text.to_string(),
                offset,
            })
    } else {
        text.parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| ParseError::InvalidNumber {
                text: text.to_string(),
                offset,
            })
    }
}
