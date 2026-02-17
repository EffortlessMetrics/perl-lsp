# perl-builtins

Builtin symbol metadata and signatures for Perl analysis.

## Scope

- Provides builtin function metadata used by parser and LSP features.
- Exposes static signature data for completion and semantic workflows.
- Uses generated/static lookup tables for fast access.

## Public Surface

- `builtin_signatures` module.
- `builtin_signatures_phf` module.

## Workspace Role

Internal utility crate consumed by completion, diagnostics, and parser-facing components.

## License

MIT OR Apache-2.0.
