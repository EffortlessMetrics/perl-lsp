# CLAUDE.md (perl-workspace-core)

## Role

The LSP-free, deterministic project-facts substrate for Perl. Sits below the
editor/LSP runtime and above the raw parser -- see
[PLSP-ADR-0006](../../docs/adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md)
and `docs/reference/NATIVE_STACK_POLICY.md`.

## Owns

- A typed `ProjectModel` with per-fact records for files, packages, and
  symbols, plus explicit `DynamicBoundary`s and `ModelLimitation`s.
- Deterministic, host-path-free identity (`FileId`, `PackageId`,
  `SymbolId`) and content `Digest`s.
- One internal range format (`SourceRange`): byte offsets + 0-based UTF-8
  line/column. UTF-16 LSP positions are produced only at the LSP boundary,
  never stored here.
- `Provenance` + `Confidence` + `EvidenceSource` on every fact.
- A `FactClasses` selector so a request only pays for the fact classes it
  asks for.
- Modules: `boundary`, `builder`, `dist`, `effects`, `error`, `export`,
  `fact_classes`, `file`, `id`, `import` (+ `import_walk`), `model`,
  `package`, `pod`, `provenance`, `range`, `relation`, `symbol`, `test`.
- `SCHEMA_VERSION` -- the fact-schema version this crate emits; bump on any
  breaking model change.

## Does not own

Must never depend on `perl-lsp-rs`, `perl-lsp-rs-core`, `perllsp`,
`perl-dap`, `lsp-types`, `tokio`, `tower-lsp`, or `perl-workspace`
(transitively pulls `lsp-types`) -- this is mechanically enforced by
`tests/dependency_contract.rs`. Also does not own UTF-16 position
conversion; that's a boundary concern for LSP-facing callers.

## Neighbors

- Upstream (leaf/facts-safe only, per PLSP-ADR-0006): `perl-parser-core`,
  `perl-symbol`, `perl-pragma`, `perl-pod`, `serde`, `serde_json`.
- Downstream: `perl-tree-sitter-compat` today. PLSP-ADR-0006 names the LSP
  server, DAP server, native critic/tidy, the RIPR exporter, and Kwalitee
  scoring as intended future consumers -- not all of them have wired this in
  yet.

## Read first

- `src/lib.rs` -- the full architecture doc comment (purpose, invariants,
  quick-start example).
- `docs/adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md`.
- `docs/reference/NATIVE_STACK_POLICY.md`.
- `tests/dependency_contract.rs`.

## Focused validation

`cargo test -p perl-workspace-core`. `tests/dependency_contract.rs` must
stay green on every dependency change -- it's the mechanical enforcement of
the "must never depend on" list above. `tests/dist_facts.rs`,
`tests/import_facts.rs`, and `tests/pod_relation_facts.rs` cover the
higher-level fact-extraction flows.

## Review hotspots

- Any new `Cargo.toml` dependency addition -- verify it doesn't violate the
  leaf-facts contract before merging.
- `SCHEMA_VERSION` -- must bump on breaking model changes; downstream
  consumers key off it.
- `range.rs` -- the byte-offset/UTF-8 invariant; UTF-16 conversion logic
  must never leak into this crate.

## Claim boundary

Reflects the crate's documented architecture and dependency contract as
authored. Does not assert that all ADR-0006-named future consumers have
actually integrated it -- only `perl-tree-sitter-compat` currently depends on
it in the workspace graph.
