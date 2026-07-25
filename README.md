# BasaltSQL

A from-scratch reimplementation of a SQL query engine in Rust with a defined
and user facing pipeline.

This is a learning and exploration project, not a production database. The
goal is to build each stage of a real query-processing pipeline
(tokenizer → AST → logical plan → row-iterator execution → storage)
and understand how the pieces fit together.

## Status

- [x] Crate scaffold and error-handling architecture
- [x] `types`: `Value`, `Row`, `DataType`
- [x] `catalog`: table/column schema registration and lookup
- [x] `lexer`: full tokeniser with keywords, identifiers, numeric/string
    literals, punctuation, and `--` comments
- [x] `parser`: recursive-descent parser producing `Statement`s with a full
    expression parser: logical (`OR`/`AND`/`NOT`), comparison, and
    arithmetic (with correct precedence, left-associativity, unary
    minus, and parenthesization)
- [ ] `analyzer`: bind statements against the `Catalog`
- [ ] `executor`: run bound statements against in-memory storage

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the pipeline design and the
reasoning behind the module boundaries and error-handling approach. It
also lists known gaps

## Getting started

```bash
cargo build
cargo test
cargo run
```

## Changelog

See [Changelog](./Changelog.md).

## License

MIT, see [LICENSE](./LICENSE).
