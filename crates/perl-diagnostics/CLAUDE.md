# CLAUDE.md (perl-diagnostics)

## Role

Unified diagnostic codes, payload types, and LSP metadata catalog for Perl
LSP. Consolidates three formerly-separate crates into one type-and-vocabulary
layer.

## Owns

- `codes` -- canonical `DiagnosticCode`, `DiagnosticCategory`,
  `DiagnosticSeverity`, `DiagnosticTag`: the single source of truth for
  these types.
- `types` -- `Diagnostic` / `RelatedInformation` payload structs; re-exports
  `DiagnosticSeverity` and `DiagnosticTag` from `codes` rather than
  redefining them, so there is exactly one canonical definition of each.
- `catalog` -- LSP-facing metadata builders keyed by diagnostic code.
- `api` -- re-exports the full public surface at the crate root.

## Does not own

Does not run any diagnostics itself -- no parser, no analysis pass. This is
purely the vocabulary/type layer that emitting providers consume.

## Neighbors

- Upstream: `serde` (optional feature), `serde_json`. No internal
  perl-lsp-crate dependencies.
- Downstream: `perl-lsp-rs-core` (diagnostics provider and others that
  construct `Diagnostic` values), `perl-lsp-rs`.

## Read first

- `src/lib.rs` -- module map and the `codes`/`types` type-unification note.
- `src/codes.rs` -- the canonical code list; read before adding a new
  diagnostic code anywhere in the workspace.

## Focused validation

`cargo test -p perl-diagnostics`. `tests/codes_diagnostic_code_completeness.rs`
and `tests/catalog_coverage.rs` guard that every code has matching catalog
metadata. `tests/type_unification.rs` guards the `codes`/`types` re-export
contract described above.

## Review hotspots

Adding a new `DiagnosticCode` requires updating `codes.rs` plus its catalog
entry -- the completeness tests will fail CI if either is missed. Treat that
failure as an actionable checklist (what's missing), not test noise to
suppress.

## Claim boundary

Describes the type/catalog surface as authored. Does not assert which
providers currently emit which codes at runtime -- that lives in the
emitting crates (e.g. `perl-lsp-rs-core::providers::diagnostics`).
