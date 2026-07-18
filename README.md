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
- [ ] `parser`: not started

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the pipeline design and the
reasoning behind the module boundaries and error-handling approach.


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
