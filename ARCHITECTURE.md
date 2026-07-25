# Architecture

## Pipeline Roadmap

The program follows a basic pipeline (this will evolve over time):

1. **Lexer**: Converts SQL source text into a stream of [Token]s.
    Holds a cursor to the current token.

2. **Parser**: Converts a stream of [Token]s into a stream of
    [Statement]s. Holds a cursor to the current token. Purely
    syntactic (grammar only). No catalog lookups or type checking.
    Recursive-descent with a precedence-climbing sub-parser for
    expressions:

```text
    OR  <  AND  <  NOT  <  comparison  <  (+ -)  <  (* /)  <  unary (-)
    (loosest)                                              (tightest)
```

3. **TODO**: Analyzer (bind statements against the `Catalog`: table/
    column existence, basic type checks) → Executor (run a bound
    statement against in-memory storage) → Driver/REPL.

## Design Decisions

### Error Types

Error types are localised to each module; the crate-wide `Error` type
unifies them into a single user-facing `Result`. `LexError` and
`ParseError` follow the same shape: each variant carries the byte
offset of the offending source position.

### Parser / Lexer boundary

Not every SQL keyword is a reserved word at the lexer level. `NULL`
(as in `NOT NULL`) and column type names (`INTEGER`, `TEXT`, ...) are
not in the lexer's keyword table, so they arrive at the parser as plain
`Ident` tokens and are matched by text there instead. This keeps the
lexer's keyword list small and lets the parser decide contextually
(e.g. `NULL` is only meaningful directly after `NOT` in a column
declaration).

### `Star` is reused as the multiply operator
 
`TokenKind::Star` means "all columns" in `SELECT *` and "multiply" in
an expression (`a * b`). There's no ambiguity in practice: the column
list is fully consumed by `parse_select_columns` before expression
parsing ever begins, so the parser never has to guess which meaning is
intended based on context.

### `VALUES` / `SET` take literals, not expressions
 
`InsertStatement.values` and `UpdateStatement.assignments` are typed as
`Vec<Value>` / `Vec<(String, Value)>`, not `Vec<Expr>`. This is
deliberate for now: there's no evaluator yet, so accepting a full
expression there (e.g. `SET price = price + 1`) would parse but have
nowhere to be evaluated. They do accept a leading unary `-` (via
`parse_signed_literal`) since negative literals are common enough to
support directly. Upgrading these to `Expr` is planned but deferred
until the executor exists — see Known Gaps.


## Known Gaps

- `Row` is not validated against `TableSchema` anywhere.
- `Value`'s derived `PartialEq` uses IEEE-754 float equality (`NaN !=
  NaN`, no epsilon tolerance). Unaddressed until arithmetic /
  comparison semantics are designed at the executor level.
- No three-valued NULL logic (`NULL = x` should yield `NULL`, not
  `true`/`false`).
- `lexer::read_string` doesn't support escaping.
- `lexer::read_number` doesn't reject malformed numbers like `1.2.3`
  at lex time — it lexes as a single `Number` token and is only
  rejected later, by the parser, as `ParseError::InvalidNumber` when
  converting the token text to `i64`/`f64`.
- `5--3` lexes as `5` followed by a comment, **not** `5 - -3` — the
  lexer matches `--` as a line comment before arithmetic ever gets a
  look. Spacing (`5 - -3`) is required. Documented and pinned down by
  a lexer test rather than treated as a bug.
- No `%` (modulo) or `||` (string concatenation) tokens/operators.
- No `LIKE`, `IN`, `BETWEEN`, or `IS [NOT] NULL` — these need new
  reserved words at the lexer level plus new `Expr` variants, larger
  in scope than the arithmetic/comparison work done so far.
- No literal support for `NULL` or boolean (`TRUE`/`FALSE`) values in
  expressions or `INSERT`/`UPDATE` — only numbers and strings.
- `SET`/`VALUES` can't take computed expressions (see Design
  Decisions above) — only literals, optionally negated.
- `Catalog` has no constraint enforcement (`NOT NULL`, `UNIQUE`,
  `PRIMARY KEY`, etc. are parsed into `ColumnDecl.nullable` but never
  checked against actual rows) and no multi-schema namespacing.

## Non-goals (for now)

- Transactions / concurrency control
- Query optimization (cost-based rewrites, index selection)
- Network protocol / server mode
