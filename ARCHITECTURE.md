# Architecture

## Pipeline Roadmap

The full pipeline:

1. **Lexer**: turns SQL source text into a stream of [Token]s. Holds a
    cursor to the current character. Uses one character of lookahead
    (`peek_n`) to tell two-character operators (`<=`, `>=`, `<>`,
    `!=`) apart from their one-character prefixes.

2. **Parser**: turns a stream of [Token]s into a stream of
    [Statement]s. Purely syntactic, grammar only, no catalog lookups
    or type checking. Recursive descent, with a precedence-climbing
    sub-parser for expressions:

    ```text
    OR < AND < NOT < comparison / IS NULL < (+ -) < (* /) < unary (-)
    (loosest)                                              (tightest)
    ```

3. **Analyzer**: binds a [Statement] against the [Catalog], producing
    a [BoundStatement]. Resolves column names to `(index, DataType)`
    pairs. Catches what it can from types alone: unknown
    tables/columns, type mismatches, wrong INSERT value counts.

4. **Executor**: runs a [BoundStatement] against [Catalog] +
    [Storage], producing an [ExecResult]. Catches what can only be
    known at runtime: division by zero, integer overflow, NOT NULL
    violations etc.

5. **Database**: owns a `Catalog` + `Storage` for the process
    lifetime. Sequences stages 1-4 together behind `execute(sql: &str)`.

6. **main.rs**: a small REPL. Reads lines from stdin and buffers until a
    `;` terminated statement, runs it through `Database` and prints the
    result.

## Design Decisions

### Error Types

Error types are local to each module. The crate-wide `Error` type
wraps each one so callers only need to match one type. `LexError` and
`ParseError` carry a byte offset into the source. `AnalyzeError` and
`ExecError` don't: the `Expr` tree has no offset field on its nodes,
so table/column names are used as error context instead.

### Bound tree, not validate-in-place

The analyzer produces a separate `BoundStatement`/`BoundExpr` tree
instead of just returning `Result<()>` against the original
`Statement`. This means the executor never has to re-resolve a column
name against the catalog: `BoundExpr::Column` already carries its
schema index. Costs a second AST-shaped type, but keeps every stage's
output typed the same way.

### Parser / Lexer boundary

Column type names (`INTEGER`, `TEXT`, ...) are still not reserved
words at the lexer level. They arrive at the parser as plain `Ident`
tokens and get matched by text in `parse_data_type`. Keeps the lexer's
keyword list small and lets the parser decide contextually.

`NULL`, `TRUE`, `FALSE`, and `IS` are real keywords now. `NULL` used
to be handled as a special-cased identifier inside `parse_column_decl`
before it had its own token. That hack is gone.

### `Star` is reused as the multiply operator

`TokenKind::Star` means "all columns" in `SELECT *` and "multiply" in
an expression (`a * b`). No ambiguity in practice: the column list is
fully consumed by `parse_select_columns` before expression parsing
starts.

### `VALUES` and `SET` take full expressions, except columns in `VALUES`

`InsertStatement.values` and `UpdateStatement.assignments` are
`Expr` typed, same as `WHERE`. `INSERT INTO t VALUES (1 + 2)` and
`UPDATE t SET price = price * 2` both work.

One exception: `bind_expr` takes an `allow_columns` flag, `false` only
for `INSERT ... VALUES`. There's no row to resolve a column reference
against in a plain VALUES clause, so `INSERT INTO users VALUES (id +
1)` is rejected at bind time (`AnalyzeError::ColumnInValues`). `SET`
keeps `allow_columns: true` since `SET balance = balance * 2` reads
the row being updated, which is valid.

### `CREATE TABLE` binds against itself, not an existing schema

Every other statement binds against an existing table's schema.
`CREATE TABLE` is defining one, so `analyze_create_table` checks two
different things instead: the table isn't already registered
(`AnalyzeError::TableAlreadyExists`), and no column name is declared
twice (`AnalyzeError::DuplicateColumn`).

### `IS NULL` / `IS NOT NULL` always resolve to BOOLEAN

Unlike `x = NULL`, which is always UNKNOWN under three-valued logic,
`x IS NULL` always returns `true` or `false`, never UNKNOWN. Kept as
its own `Expr`/`BoundExpr` variant instead of a `BinaryOp` for exactly
this reason.

### Three-valued NULL logic

Comparisons and logical operators (`AND`/`OR`/`NOT`) follow standard
SQL three-valued logic. A NULL operand makes a comparison UNKNOWN, not
`true`/`false`, and UNKNOWN propagates through `AND`/`OR`/`NOT` using
the real truth tables (`false AND UNKNOWN` is `false`, `true OR
UNKNOWN` is `true`). Implemented with `Option<bool>` internally (`None`
means UNKNOWN), represented externally as `Value::Null`. A
`WHERE`/filter predicate only keeps a row when it evaluates to exactly
`Value::Boolean(true)`.

### Division by zero errors, doesn't follow IEEE-754

`/` by zero errors (`ExecError::DivisionByZero`) for both INTEGER and
FLOAT operands. Matches standard SQL (Postgres does the same), not
IEEE-754 float semantics, where `1.0 / 0.0` would silently give `inf`.

### Integer arithmetic is checked, not wrapping

`+`, `-`, `*`, `/` on INTEGER use `checked_*` and return
`ExecError::IntegerOverflow` on overflow, including the `i64::MIN /
-1` edge case. Used to wrap silently with `wrapping_*`. Fixed since
silent wraparound corrupts data quietly.

### NOT NULL and overflow checks live in the executor, not the analyzer

Both can only be known from an actual runtime value, not from static
type. `1 + NULL` evaluates to NULL at runtime even though neither
operand is a bare NULL literal, so checking `Expr` structure at bind
time would miss it. Same reasoning as division by zero: the analyzer
catches what's knowable from type alone, the executor catches what's
only knowable from the value.

### Evaluate-then-apply for `UPDATE`/`DELETE`

Both fully evaluate every row's filter (and, for `UPDATE`, every
assignment expression and NOT NULL check) before mutating anything.
Planned changes go into a `Vec` first, then get applied in a second
pass. If a runtime error happens partway through, zero rows have been
touched. This is atomicity for a single statement, not a transaction
system.

## Known Gaps

- `Value`'s derived `PartialEq` uses IEEE-754 float equality (`NaN !=
  NaN`, no epsilon tolerance). `eval_cmp` copies this on purpose:
  every comparison on NaN is `false` except `<>`.
- `lexer::read_string` doesn't support escaping.
- `lexer::read_number` doesn't reject malformed numbers like `1.2.3`
  at lex time. Caught later by the parser as `ParseError::InvalidNumber`.
- `5--3` lexes as `5` plus a swallowed comment, not `5 - -3`. Spacing
  is required. Documented and pinned down by a lexer test, not
  planned to change.
- No `%` or `||` operators.
- No `LIKE`, `IN`, `BETWEEN`.
- `Catalog` has no `UNIQUE` or `PRIMARY KEY` enforcement, and no
  multi-schema namespacing.
- No transactions across a batch. `Database::execute_all` stops at
  the first error but doesn't roll back statements already applied.
- The REPL finds the end of a statement by looking for a trailing
  `;`. A `;` inside a string literal (`INSERT INTO t VALUES
  ('a;b');`) ends the statement early.

## Non-goals (for now)

- Transactions / concurrency control
- Query optimization (cost-based rewrites, index selection)
- Network protocol / server mode
