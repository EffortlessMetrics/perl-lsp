# perl-pragma

Pragma state tracking for Perl source analysis.

## Scope

- Tracks `use` and `no` pragmas over source order.
- Computes effective pragma state (strict/warnings and category toggles).
- Supports scope-aware pragma analysis for parser and diagnostics flows.

## Public Surface

- `PragmaState`.
- `PragmaTracker`.

## Workspace Role

Internal utility crate used by parser and semantic/diagnostic analyses.

## License

MIT OR Apache-2.0.
