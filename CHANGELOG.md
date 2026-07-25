# Changelog

All notable changes to this project are documented here.

## Second milestone
 
### Added
 
- `parser`: recursive-descent parser converting a token stream into
  `Statement`s (`SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE TABLE`)
- `parser::ast`: `Statement`, `Expr`, `BinaryOp`, and per-statement AST
  types
- `parser::error::ParseError` covering unexpected tokens, unexpected
  EOF, invalid numeric literals, and unknown column types
- Precedence-climbing expression parser for `WHERE` clauses (`OR` <
  `AND` < `NOT` < comparison), with parenthesized sub-expressions
- Top-level `Error` now wraps `ParseError` alongside `LexError`
- `lexer`: single-character `Lt` (`<`) and `Gt` (`>`) tokens — these
  variants existed since the second milestone but had no match arm, so
  a lone `<` or `>` always failed with `UnexpectedChar` until now
- `parser`: full arithmetic expression support — `term`/`factor`
  precedence tiers (`+`/`-` looser than `*`/`/`), left-associative,
  parenthesizable, sitting between comparison and unary in precedence
- `parser`: unary minus (`Expr::Neg`), right-recursive so `- -a`
  parses as double negation
- Lexer test coverage for the two-character lookahead logic (`<=` vs
  `<`, `!=`/`<>` both mapping to `NotEq`, operator-at-EOF), keyword
  case-insensitivity, and comment-boundary edge cases

### Changed
 
- `Lexer::tokenize` renamed to `Lexer::tokenise` (and `LexError`
  offsets/messages updated to match); all call sites in the parser and
  its tests updated accordingly
- `lexer::error` submodule made private (`mod error` instead of `pub
  mod error`), re-exported via `pub use error::{LexError, Result}` —
  no change to the public API

### Known gaps
 
- No `%` or `||` operators.
- No `LIKE`/`IN`/`BETWEEN`/`IS NULL`.
- `5--3` lexes as `5` + a swallowed comment, not `5 - -3`. Documented,
  not planned to change.

## First milestone

### Added

- A crate scaffold
- Core `types`
- A `catalog` for table registration and lookup
- Full tokeniser

### Known gaps
- `read_string` doesn't support escapeing.
- `read_number` doesn't reject malformed numbers like `1.2.3` at lex time.
- `Row` isn't validated against `TableSchema` anywhere yet.
