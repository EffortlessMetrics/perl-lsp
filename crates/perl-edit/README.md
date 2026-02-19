# perl-edit

Edit tracking primitives for incremental Perl parsing.

## Scope

- Represents text edits with byte and line/column coordinates.
- Computes positional shifts caused by edits.
- Applies edits to positions/ranges for incremental parser and LSP updates.

## Public Surface

- `Edit` for a single text change.
- `EditSet` for ordered multi-edit application.

## Workspace Role

Internal utility crate used by incremental parsing and document update pipelines.

## License

MIT OR Apache-2.0.
