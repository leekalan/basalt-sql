# Changelog

All notable changes to this project are documented here.

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

