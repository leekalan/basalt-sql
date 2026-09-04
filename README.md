# BasaltSQL

A from-scratch reimplementation of a SQL query engine in Rust with a defined
and user facing pipeline.

This is a learning and exploration project, not a production database. The
goal is to build each stage of a real query-processing pipeline
(tokeniser, parser, binder, executor, storage) and understand how the
pieces fit together.

## Status

- [x] Crate scaffold and error-handling architecture
- [x] `types`: `Value`, `Row`, `DataType`
- [x] `catalog`: table/column schema registration and lookup
- [x] `lexer`: full tokeniser. Keywords, identifiers, numeric/string
    literals, punctuation, `--` comments, all comparison operators
    (`=`, `<>`/`!=`, `<`, `<=`, `>`, `>=`), arithmetic operators (`+`,
    `-`, `*`, `/`), and `NULL`/`TRUE`/`FALSE`/`IS` keywords
- [x] `parser`: recursive-descent parser producing `Statement`s for
    `SELECT`/`INSERT`/`UPDATE`/`DELETE`/`CREATE TABLE`, with a full
    expression parser: logical, comparison, `IS [NOT] NULL`, and
    arithmetic, with correct precedence, left-associativity, unary
    minus, and parenthesization
- [x] `analyzer`: binds statements against the `Catalog`. Resolves
    column references, rejects unknown tables/columns and static type
    errors, produces a `BoundStatement` tree
- [x] `storage` + `executor`: in-memory row storage and a full
    evaluator. Three-valued NULL logic, standard-SQL division-by-zero
    errors, checked integer arithmetic (no silent overflow), NOT NULL
    enforcement, single-statement atomicity for `UPDATE`/`DELETE`
- [x] `db::Database`: the full pipeline behind one call,
    `Database::new().execute("SELECT * FROM t WHERE x > 1;")`
- [x] `main.rs`: a basic REPL

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the pipeline design, the
reasoning behind each stage's boundaries, and the full list of known
gaps.

## Getting started

```bash
cargo build
cargo test
cargo run
```

## Changelog

See [CHANGELOG.md](./CHANGELOG.md).

## License

MIT, see [LICENSE](./LICENSE).