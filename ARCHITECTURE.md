# Architecture

## Pipeline Roadmap

The program follows a basic pipeline (this will evolve over time):

1. **Lexer**: Converts SQL source text into a stream of [Token]s.
    Holds a cursor to the current token.

2. **Parser**: Converts a stream of tokens into a stream of [Statement]s.

3. **TODO**...

## Design Decisions

### Error Types

Error types will be localised to each module where the main `Error` type
unifies them all into a user facing global `Result` type.

## Known Gaps

- `Row` is not validated against `TableSchema` anywhere
- `Value`'s derived `PartialEq` uses IEEE-754 float equality (`NaN !=
  NaN`, no epsilon tolerance). This will remain unaddressed until
  arithmetic / comparison semantics are designed.
- No three-valued NULL logic (`NULL = x` should yield `NULL`, not
  `true`/`false`).
- `lexer::read_string` doesn't support escaping.
- `lexer::read_number` doesn't reject malformed numbers like `1.2.3`.
- `Catalog` has no constraint enforcement (`NOT NULL`, `UNIQUE`,
  `PRIMARY KEY`, etc.) and no multi-schema namespacing.

## Non-goals (for now)

- Transactions / concurrency control
- Query optimization (cost-based rewrites, index selection)
- Network protocol / server mode
