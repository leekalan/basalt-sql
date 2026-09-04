# Changelog

All notable changes to this project are documented here.

## Fifth milestone

### Added

- `lexer`: `NULL`, `TRUE`, `FALSE`, and `IS` keywords/tokens.
- `parser`: `NULL`/`TRUE`/`FALSE` literals in expressions and
  `INSERT`/`SET` values. `parse_column_decl`'s old hack (matching
  `NULL` as a plain identifier after `NOT`) is gone, it uses the
  `Null` token directly now.
- `parser`/`analyzer`/`executor`: `IS NULL` and `IS NOT NULL`, as
  their own `Expr`/`BoundExpr::IsNull` variant. Always resolves to a
  definite BOOLEAN, never UNKNOWN, unlike `x = NULL`.
- `executor::ExecError::NotNullViolation`: inserting or updating a
  column declared `NOT NULL` with a NULL value now errors. Checked
  against the value actually written, after evaluation, so `SET x =
  1 + NULL` is caught too, not just a bare `NULL` literal.
- `executor::ExecError::IntegerOverflow`: INTEGER `+ - * /` now use
  `checked_*` instead of `wrapping_*`. Overflow errors instead of
  silently wrapping. Covers the `i64::MIN / -1` edge case too.
- `main.rs`: a basic REPL. Reads stdin line by line, buffers until a
  `;`-terminated statement, runs it through `Database::execute_all`,
  prints rows or the affected-row count.

### Known gaps

- Three-valued NULL logic and NULL literals were fully implemented in
  the executor and unit-tested before this milestone, but weren't
  reachable from real SQL text since there was no `NULL` keyword.
  **Resolved.**
- `ColumnDef.nullable` was parsed and stored but never enforced.
  **Resolved.**
- Integer arithmetic overflow wrapped silently. **Resolved.**
- No `LIKE`, `IN`, `BETWEEN`. Still open.
- The REPL splits on a literal `;`, so one inside a string literal
  ends the statement early. Known limitation, not planned to fix
  soon.

## Fourth milestone

### Added

- `analyzer`: binds a parsed `Statement` against the `Catalog`,
  producing a `BoundStatement`/`BoundExpr` tree with column references
  resolved to `(index, DataType)`. Catches unknown tables/columns,
  `INSERT` value count mismatches, `CREATE TABLE` on an already
  existing table, duplicate columns in a `CREATE TABLE`, and static
  type errors (`-'x'`, `'x' + 1`, `id AND name`, wrong-typed `INSERT`
  values). See ARCHITECTURE.md for the full binding rules.
- `analyzer::AnalyzeError::ColumnInValues`: `INSERT ... VALUES` can no
  longer reference a table column (`INSERT INTO t VALUES (id + 1)`).
  There's no row to resolve it against. `UPDATE ... SET` is
  unaffected and still allows this (`SET balance = balance * 2`).
- `storage`: new module, `Storage`. In-memory `Vec<Row>` per table.
  Kept separate from `Catalog`, which stays schema-only.
- `executor`: runs a `BoundStatement` against `Catalog` + `Storage`.
  Implements standard SQL three-valued NULL logic for comparisons and
  `AND`/`OR`/`NOT`, and standard-SQL (not IEEE-754) division-by-zero
  errors for both INTEGER and FLOAT. `UPDATE`/`DELETE` evaluate every
  affected row before mutating any of them (single-statement
  atomicity, see ARCHITECTURE.md).
- `db::Database`: the crate's main entry point. Owns a `Catalog` +
  `Storage` and runs the full lex, parse, analyze, execute pipeline
  through `execute(sql: &str)` / `execute_all(sql: &str)`.
- Top-level `Error` now wraps `AnalyzeError` and `ExecError` alongside
  `LexError`/`ParseError`.

### Known gaps

- No `NULL`/`TRUE`/`FALSE` literal syntax yet. Three-valued logic and
  NULL propagation were fully implemented and unit-tested, but not
  reachable end to end through `Database`. **Resolved in the fifth
  milestone.**
- `ColumnDef.nullable` still isn't enforced. **Resolved in the fifth
  milestone.**
- Integer arithmetic overflow wraps rather than erroring. **Resolved
  in the fifth milestone.**
- No rollback across an `execute_all` batch on error. Still open.

## Third milestone

### Added

- `lexer`: single-character `Lt` (`<`) and `Gt` (`>`) tokens. These
  variants existed since the second milestone but had no match arm,
  so a lone `<` or `>` always failed with `UnexpectedChar` until now.
- `parser`: full arithmetic expression support, `term`/`factor`
  precedence tiers (`+`/`-` looser than `*`/`/`), left-associative,
  parenthesizable, sitting between comparison and unary in precedence.
- `parser`: unary minus (`Expr::Neg`), right-recursive so `- -a`
  parses as double negation.
- `parser`: `InsertStatement.values` (`Vec<Expr>`) and
  `UpdateStatement.assignments` (`Vec<(String, Expr)>`) accept full
  expressions, matching `WHERE`. `INSERT INTO t VALUES (1 + 2)` and
  `UPDATE t SET price = price * 2` both parse.
- Lexer test coverage for the two-character lookahead logic (`<=` vs
  `<`, `!=`/`<>` both mapping to `NotEq`, operator at EOF), keyword
  case-insensitivity, and comment-boundary edge cases.

### Changed

- `Lexer::tokenize` renamed to `Lexer::tokenise`. All call sites in
  the parser and its tests updated to match.
- `lexer::error` submodule made private (`mod error` instead of `pub
  mod error`), re-exported via `pub use error::{LexError, Result}`.
  No change to the public API.

### Known gaps

- No `%` or `||` operators. Still open.
- No `LIKE`/`IN`/`BETWEEN`/`IS NULL`. `IS [NOT] NULL` resolved in the
  fifth milestone, the rest still open.
- Type-sanity checking (rejecting `-'x'` or `'x' + 1`) deferred to the
  analyzer. **Resolved in the fourth milestone.**
- `5--3` lexes as `5` plus a swallowed comment, not `5 - -3`.
  Documented, not planned to change.

## Second milestone

### Added

- `parser`: recursive-descent parser converting a token stream into
  `Statement`s (`SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE
  TABLE`).
- `parser::ast`: `Statement`, `Expr`, `BinaryOp`, and per-statement
  AST types.
- `parser::error::ParseError` covering unexpected tokens, unexpected
  EOF, invalid numeric literals, and unknown column types.
- Precedence-climbing expression parser for `WHERE` clauses (`OR` <
  `AND` < `NOT` < comparison), with parenthesized sub-expressions.
- Top-level `Error` now wraps `ParseError` alongside `LexError`.

### Known gaps

- The lexer only emitted `Eq` (`=`) for comparisons. **Resolved in
  the third milestone.**
- No `NULL` or boolean literals in expressions/values. **Resolved in
  the fifth milestone.**
- No unary minus / negative number literals. **Resolved in the third
  milestone.**

## First milestone

### Added

- A crate scaffold.
- Core `types`.
- A `catalog` for table registration and lookup.
- Full tokeniser.

### Known gaps

- `read_string` doesn't support escaping. Still open.
- `read_number` doesn't reject malformed numbers like `1.2.3` at lex
  time. Still open.
- `Row` isn't validated against `TableSchema` anywhere. **Resolved in
  the fourth milestone**, the analyzer's `INSERT`/`UPDATE` type checks
  cover this.