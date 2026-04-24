# perl-incremental-parsing

Compatibility shim for incremental parsing APIs.

## Overview

`perl-parser` is the single source of truth for incremental parsing logic in this
workspace. This crate remains as a thin re-export layer so existing users of
`perl-incremental-parsing` can migrate without breaking changes.

## Migration

Prefer importing from `perl-parser` directly:

- `perl_parser::incremental`
- `perl_parser::Edit`
- `perl_parser::IncrementalState`
- `perl_parser::apply_edits`

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
