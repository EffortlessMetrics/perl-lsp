# CLAUDE.md (perl-semantic-facts)

## Role

Neutral semantic-fact vocabulary for Perl analysis layers: strongly-typed
IDs and serializable fact records shared between parser-derived semantics,
semantic-analyzer synthesis, and workspace indexing. A single-file (`lib.rs`)
type-definition crate.

## Owns

- ID newtypes: `FileId`, `ScopeId`, `EntityId`, `AnchorId`, `OccurrenceId`,
  `EdgeId`, `DiagnosticId` (all `u64`-backed via the `id_newtype!` macro).
- Kind enums: `EntityKind` (Package, Class, Role, Subroutine, ...),
  `OccurrenceKind` (Definition, Reference, Call, Import, ...), `EdgeKind`
  (Defines, References, Inherits, ComposesRole, ...).
- Confidence/provenance vocabulary: `Provenance` (how a fact was derived --
  `ExactAst` through `LiteralRequireImport`), `Confidence` (High/Medium/Low).
- Fact records: `AnchorFact`, `EntityFact`, `OccurrenceFact`, `EdgeFact`,
  `DiagnosticFact`.
- Export/import modeling: `ExportSet`, `ExportTag`, `ImportSpec`,
  `ImportKind`, `ImportSymbols`, `UseLibFact`.
- Visibility resolution: `VisibleSymbol`, `VisibleSymbolContext`.

## Does not own

Explicitly, per the crate's own doc comment: does not parse Perl, does not
implement LSP providers, and does not own workspace storage backends. This
is a types-only vocabulary crate.

## Neighbors

- Upstream: `serde` (only dependency).
- Downstream: `perl-lsp-rs`, `perl-parser-core`, `perl-semantic-analyzer`,
  `perl-symbol`, `perl-workspace`.

## Read first

- `src/lib.rs` -- the entire crate; read the top doc comment first for the
  "does not" boundary, then the ID newtypes and kind enums before the fact
  record structs (records reference the IDs/kinds).

## Focused validation

`cargo test -p perl-semantic-facts`. `tests/prop_json_roundtrip.rs`
(property-based) guards serde round-tripping for every fact type --
required since these records cross process/cache boundaries as JSON.
`tests/bdd_semantic_facts.rs` covers behavioral scenarios.

## Review hotspots

- Any new fact-record field needs the round-trip property test to still
  pass; a field that can't round-trip through `serde_json` breaks every
  consumer that persists these facts.
- `Provenance` variants encode a specific derivation method (e.g.
  `LiteralRequireImport` is documented as more precise than `ExactAst` for
  a narrow case) -- read each variant's doc comment before adding a new one
  or assuming a general reliability ordering across all of them.

## Claim boundary

Describes the fact vocabulary as authored (types and their doc comments).
Does not assert which analysis stage actually populates which fact kind at
runtime -- that logic lives in the consuming crates listed above.
