# tree-sitter-perl-c

Legacy Tree-sitter Perl bindings using the C scanner implementation.

## Scope

- Exposes the C-based Tree-sitter Perl language and parser constructors.
- Provides simple helpers to parse Perl strings/files with the legacy scanner.
- Preserved for compatibility and comparative benchmarking.

## Public Surface

- `language`, `try_create_parser`, `create_parser`.
- `parse_perl_code`, `parse_perl_file`.
- `get_scanner_config`.

## Status

Legacy crate. New parser and LSP development is centered on Rust-native workspace crates.

## License

Apache-2.0 OR MIT.
