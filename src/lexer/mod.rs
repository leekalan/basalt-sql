//! Tokenizer: turns raw SQL source text into a stream of [Token]s.
//! Purely lexical analysis with no grammar.

pub mod error;
pub use error::{LexError, Result};

/// The category of a single token, plus any data it carries (identifier
/// text, literal value, etc).
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Select,
    From,
    Where,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    Create,
    Table,
    And,
    Or,
    Not,
    /// A user-defined name (table, column, etc). This can be anything
    /// alphabetic that isn't a reserved keyword.
    Ident(String),
    /// A numeric literal stored as the original source text (not yet
    /// parsed into [i64] or [f64]).
    Number(String),
    /// A single-quoted string literal with quotes stripped and
    /// contents unescaped.
    StringLit(String),
    Comma,
    Star,
    LParen,
    RParen,
    Semicolon,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    /// Marks the end of input. Always the last token produced.
    Eof,
}

/// A single lexical token: its kind plus the byte offset in the source
/// where it starts.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub offset: usize,
}

impl Token {
    pub fn new(kind: TokenKind, offset: usize) -> Self {
        Self { kind, offset }
    }
}

/// Converts SQL source text into a stream of [Token]s. Holds a cursor
/// (`pos`) into the source and advances it one token at a time.
pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a lexer over `src` ran by calling [Lexer::tokenize] to run it.
    pub fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    /// Consumes the lexer and tokenizes the entire input returning all
    /// tokens including a trailing [TokenKind::Eof]. Fails on the
    /// first lexical error encountered (no error recovery and the parser
    /// stage never sees a partial/invalid token stream).
    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Scans and returns the next token then advances the cursor past it.
    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace_and_comments();
        let start = self.pos;

        let Some(c) = self.peek() else {
            return Ok(Token::new(TokenKind::Eof, start));
        };

        let kind = match c {
            ',' => {
                self.pos += 1;
                TokenKind::Comma
            }
            '*' => {
                self.pos += 1;
                TokenKind::Star
            }
            '(' => {
                self.pos += 1;
                TokenKind::LParen
            }
            ')' => {
                self.pos += 1;
                TokenKind::RParen
            }
            ';' => {
                self.pos += 1;
                TokenKind::Semicolon
            }
            '=' => {
                self.pos += 1;
                TokenKind::Eq
            }
            '\'' => self.read_string()?,
            c if c.is_ascii_digit() => self.read_number(),
            c if c.is_alphabetic() || c == '_' => self.read_ident_or_keyword(),
            other => {
                return Err(LexError::unexpected_char(other, start));
            }
        };

        Ok(Token::new(kind, start))
    }

    /// Returns the next character without consuming it.
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    /// Advances past whitespace and `--`-style line comments. Called
    /// before scanning every token so neither has to be handled inside
    /// individual token-reading functions.
    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else if self.src[self.pos..].starts_with("--") {
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.pos += c.len_utf8();
                }
            } else {
                break;
            }
        }
    }

    /// Reads a `'...'` string literal, starting at the opening quote.
    /// Does not yet support escaping.
    fn read_string(&mut self) -> Result<TokenKind> {
        let start = self.pos;
        self.pos += 1; // opening quote
        let mut s = String::new();
        loop {
            match self.peek() {
                Some('\'') => {
                    self.pos += 1;
                    break;
                }
                Some(c) => {
                    s.push(c);
                    self.pos += c.len_utf8();
                }
                None => return Err(LexError::unterminated_string(start)),
            }
        }
        Ok(TokenKind::StringLit(s))
    }

    /// Reads a numeric literal.
    fn read_number(&mut self) -> TokenKind {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        TokenKind::Number(self.src[start..self.pos].to_string())
    }

    /// Reads an identifier or keyword and classifies it. Reserved words
    /// are matched case-insensitively against `TokenKind` variants,
    /// everything else becomes `TokenKind::Ident`.
    fn read_ident_or_keyword(&mut self) -> TokenKind {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let word = &self.src[start..self.pos];
        match word.to_ascii_uppercase().as_str() {
            "SELECT" => TokenKind::Select,
            "FROM" => TokenKind::From,
            "WHERE" => TokenKind::Where,
            "INSERT" => TokenKind::Insert,
            "INTO" => TokenKind::Into,
            "VALUES" => TokenKind::Values,
            "UPDATE" => TokenKind::Update,
            "SET" => TokenKind::Set,
            "DELETE" => TokenKind::Delete,
            "CREATE" => TokenKind::Create,
            "TABLE" => TokenKind::Table,
            "AND" => TokenKind::And,
            "OR" => TokenKind::Or,
            "NOT" => TokenKind::Not,
            _ => TokenKind::Ident(word.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn tokenizes_simple_select() {
        let tokens = Lexer::new("SELECT * FROM t;").tokenize().unwrap();
        assert_eq!(tokens.first().unwrap().kind, TokenKind::Select);
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn errors_on_unexpected_char() {
        let err = Lexer::new("SELECT $ FROM t;").tokenize().unwrap_err();
        assert_eq!(err, LexError::UnexpectedChar { ch: '$', offset: 7 });
    }

    #[test]
    fn errors_on_unterminated_string() {
        let err = Lexer::new("SELECT * FROM t WHERE x = 'oops")
            .tokenize()
            .unwrap_err();
        assert_eq!(err, LexError::UnterminatedString { offset: 26 });
    }
}
