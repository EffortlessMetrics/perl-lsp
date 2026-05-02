# Implementation Plan: RC2 Semantic Analysis

## Overview

Replace per-provider semantic approximations with a single, layered semantic substrate spanning five existing crates. Implementation follows a linear 11-phase path (~40 PRs) from stabilizing current substrate seams through provider migration to scorecards proving behavioral improvement.

**Key crates:** perl-semantic-facts, perl-symbol, perl-semantic-analyzer, perl-workspace-index, perl-lsp-rs-core

**Constraints (Req 25):** Build from current master. Do not create new crates. Do not move parser-specific code into perl-semantic-facts. Add tests that fail before the change. Keep generated output deterministic. Use existing crate seams. New workspace semantic code goes in `src/semantic/` module tree (Req 24).

**Verification note:** The workspace index package is named `perl-workspace` even though its directory is `crates/perl-workspace-index/`. In PowerShell, invoke `cargo-safe` with the call operator (`& ./scripts/cargo-safe ...`) and do not pipe the script invocation directly. Capture first when filtering output:

```powershell
$output = & ./scripts/cargo-safe test -p perl-workspace --profile agent --locked -- prop_rename_plan_occurrence_classification 2>&1
$output | Select-String "test result"
```

## Tasks

- [x] 1. Phase 0 — Stabilize seams (MUST)
  - [x] 1.1 Fix SymbolRef adapter canonical path
    - Verify `symbol_refs_to_semantic_facts` in `crates/perl-symbol/src/surface/facts.rs` is the single canonical SymbolRef→OccurrenceFact adapter
    - Remove or consolidate any duplicate adapter modules or duplicate public type names for the SymbolRef→OccurrenceFact conversion
    - Add a compile-time or test assertion that no second adapter path exists
    - _Crates: perl-symbol_
    - _Requirements: 21.1, 21.2, 21.3_
    - _Verify: `./scripts/cargo-safe test -p perl-symbol --profile agent --locked`_

  - [x] 1.2 Write property test for adapter determinism (Property 4)
    - **Property 4: Adapter Determinism** — For any set of SymbolDecls and a FileId, running `symbol_decls_to_semantic_facts` twice with the same inputs produces identical output. Same for `symbol_refs_to_semantic_facts`.
    - Generate random SymbolDecl/SymbolRef lists with `proptest`, run adapter twice, assert equality
    - _Crates: perl-symbol_
    - _Requirements: 3.2, 3.3_
    - _Verify: `./scripts/cargo-safe test -p perl-symbol --profile agent --locked`_

  - [x] 1.3 Write property test for fact-to-anchor referential integrity (Property 2)
    - **Property 2: Fact-to-Anchor Referential Integrity** — Every emitted EntityFact has an `anchor_id` referencing an AnchorFact in the same result set. Every emitted OccurrenceFact has an `anchor_id` referencing an AnchorFact in the same result set.
    - Generate random SymbolDecl/SymbolRef lists, verify all anchor_id references resolve
    - _Crates: perl-symbol_
    - _Requirements: 2.2, 2.3_
    - _Verify: `./scripts/cargo-safe test -p perl-symbol --profile agent --locked`_

- [x] 2. Phase 1 — Facts through workspace (MUST)
  - [x] 2.1 Add ReferenceEdge type to perl-semantic-facts
    - Add `ReferenceEdge` struct with `occurrence_id`, `anchor_id`, `file_id`, `symbol_key`, `target_candidates: Vec<EntityId>`, `kind`, `provenance`, `confidence` fields
    - Use `#[non_exhaustive]` per project convention
    - Derive `Serialize`, `Deserialize`, `Debug`, `Clone`, `PartialEq`, `Eq`
    - Add unit test for JSON round-trip
    - _Crates: perl-semantic-facts_
    - _Requirements: 4.4, 4.5, 4.6_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-facts --profile agent --locked`_

  - [x] 2.2 Add DefinitionRank, DefinitionRankReason, and DefinitionCandidate to perl-semantic-facts
    - Add `DefinitionRank` enum: ExactQualified, SamePackage, ExplicitImport, DefaultExport, WorkspaceCandidate, Heuristic
    - Add `DefinitionRankReason` enum: ExactQualifiedName, SamePackage, ExplicitImport { module }, DefaultExport { module }, WorkspaceSymbol, HeuristicNameMatch
    - Add `DefinitionCandidate` struct with `entity_id`, `anchor_id`, `canonical_name`, `display_name`, `package`, `kind`, `provenance`, `confidence`, `rank`, `rank_reason` fields
    - Use `#[non_exhaustive]` on all new types
    - Add unit tests for round-trip serialization
    - _Crates: perl-semantic-facts_
    - _Requirements: 5.2, 5.3_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-facts --profile agent --locked`_

  - [x] 2.3 Extend FileFactShard with canonical population path
    - Create `crates/perl-workspace-index/src/semantic/` module tree with `mod.rs` and `facts.rs`
    - Implement `build_canonical_fact_shard` that populates `FileFactShard` from `SymbolDeclSemanticFacts`, `SymbolRefSemanticFacts`, `ImportSpec` list, and dynamic boundary occurrences
    - Produce `FileFactShard` with real byte spans, `ExactAst` provenance, and per-category hashes computed from canonical fact vectors
    - Preserve ScopeId on facts where scope is known; carry scope_id as None when unknown (Req 26)
    - Wire into `WorkspaceIndex` alongside existing `build_fact_shard` (do not remove legacy path yet)
    - _Crates: perl-workspace-index_
    - _Requirements: 18.1, 18.6, 24.1, 24.2, 24.3, 26.1, 26.2, 26.3_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 2.4 Add typed reference index
    - Create `crates/perl-workspace-index/src/semantic/references.rs`
    - Implement `references_by_name: HashMap<String, Vec<ReferenceEdge>>` and `references_by_entity: HashMap<EntityId, Vec<ReferenceEdge>>` indexes
    - Populate from `FileFactShard` occurrences and edges during indexing
    - Provide incremental add/remove methods for file re-indexing
    - _Crates: perl-workspace-index_
    - _Requirements: 4.4, 4.6, 8.3_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 2.5 Write property test for JSON round-trip of all new fact types (Property 1)
    - **Property 1: Fact Record JSON Round-Trip** — For any valid ReferenceEdge, DefinitionCandidate, ValueShape, PackageEdge, RenamePlan, SafeDeletePlan, serializing to JSON then deserializing produces an equal object.
    - Use `proptest` to generate random instances of each new type
    - _Crates: perl-semantic-facts_
    - _Requirements: 1.7_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-facts --profile agent --locked`_

- [x] 3. Checkpoint — Ensure all tests pass
  - Ensure all tests pass for perl-semantic-facts, perl-symbol, and perl-workspace-index. Ask the user if questions arise.

- [x] 4. Phase 1 continued — Scorecard counts and shadow compare (MUST)
  - [x] 4.1 Add scorecard count infrastructure
    - Create `crates/perl-workspace-index/src/semantic/scorecard.rs`
    - Implement scorecard aggregation: per-query verdicts (Same, Improved, Regression, Ambiguous, Unavailable) with counts
    - Support three modes: emit (always okay), --check (deterministic artifact freshness), --gate (future hard gate, opt-in)
    - Add unit tests for aggregation logic
    - _Crates: perl-workspace-index_
    - _Requirements: 11.1, 11.2, 11.8_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 4.2 Extend shadow compare for semantic queries
    - Extend existing `crates/perl-workspace-index/src/semantic_shadow_compare.rs` to support new semantic query names (VisibleSymbols, MethodCandidates, etc.)
    - Add `ShadowQueryName` variants for all `SemanticQueries` methods
    - Ensure receipt JSON shape remains stable and deterministic
    - _Crates: perl-workspace-index_
    - _Requirements: 10.2, 10.3, 10.4_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 4.3 Write property test for shadow compare verdict determinism (Property 17)
    - **Property 17: Shadow Compare Verdict Determinism** — For any pair of old-path and new-path summaries, `classify_verdict` produces the same verdict when called with the same inputs.
    - Generate random `ShadowResultSummary` pairs with `proptest`
    - _Crates: perl-workspace-index_
    - _Requirements: 10.2_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_


- [x] 5. Phase 2 — Import extraction (MUST)
  - [x] 5.1 Add location fields to ImportSpec and ExportSet in perl-semantic-facts
    - Add `file_id: Option<FileId>`, `anchor_id: Option<AnchorId>`, `scope_id: Option<ScopeId>` to `ImportSpec`
    - Add `module_name: Option<String>`, `anchor_id: Option<AnchorId>` to `ExportSet`
    - Add `context: Option<VisibleSymbolContext>` to `VisibleSymbol`
    - Add `VisibleSymbolContext` struct with `source_module`, `source_import_anchor_id`, `source_export_anchor_id`
    - Update existing tests for new fields
    - _Crates: perl-semantic-facts_
    - _Requirements: 6.8, 6.9, 8.4_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-facts --profile agent --locked`_

  - [x] 5.2 Implement ImportSpec extractor for static `use` statements
    - Create `crates/perl-semantic-analyzer/src/analysis/import_extractor.rs`
    - Handle `use Module qw(a b)` → UseExplicitList, `use Module ()` → UseEmpty, `use Module ':tag'` → UseTag, `use Module` (bare) → Use/Default, `use constant { ... }` → UseConstant
    - Walk AST to extract ImportSpec entries with file_id and anchor_id
    - Add unit tests for each import pattern
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 6.1, 6.2, 6.3, 25.4_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

  - [x] 5.3 Implement ImportSpec extractor for `require`/`import` patterns
    - Extend import extractor to handle `require Module; Module->import(...)` → RequireThenImport
    - Handle `require $var` → DynamicRequire with ImportSymbols::Dynamic
    - Add unit tests for require patterns
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 6.4, 7.1_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

  - [x] 5.4 Implement ImportExportIndex in workspace
    - Create `crates/perl-workspace-index/src/semantic/imports.rs`
    - Implement `imports_by_file: HashMap<FileId, Vec<ImportSpec>>` index
    - Implement `exports_by_module: HashMap<String, ExportSet>` index
    - Populate from FileFactShard during indexing with incremental add/remove
    - _Crates: perl-workspace-index_
    - _Requirements: 6.6, 6.7_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 5.5 Implement export extractor for @EXPORT, @EXPORT_OK, %EXPORT_TAGS
    - Create or extend export analysis in perl-semantic-analyzer
    - Extract `ExportSet` with `default_exports`, `optional_exports`, and `tags` from Exporter-based modules
    - Carry `module_name` and `anchor_id` on the ExportSet
    - Add unit tests for export patterns
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 6.5, 6.9_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

  - [x] 5.6 Write property test for export set completeness (Property 8)
    - **Property 8: Export Set Completeness** — For any module with @EXPORT, @EXPORT_OK, or %EXPORT_TAGS, the ExportSet contains all declared symbols, sorted and deduplicated.
    - Generate random export array contents with `proptest`
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 6.5_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

- [x] 6. Checkpoint — Ensure all tests pass
  - Ensure all tests pass for perl-semantic-facts, perl-symbol, perl-semantic-analyzer, and perl-workspace-index. Ask the user if questions arise.

- [x] 7. Phase 3 — Semantic query facade (MUST)
  - [x] 7.1 Implement visible_symbols_at
    - Create `crates/perl-workspace-index/src/semantic/visibility.rs`
    - Implement `visible_symbols_at(file_id, byte_offset, scope_id)` returning `Vec<VisibleSymbol>`
    - Resolve visibility from: local lexical scope, local package scope, explicit imports, default exports, export tags, constants, generated members
    - Each VisibleSymbol carries `VisibleSymbolSource` classification and optional `VisibleSymbolContext` origin metadata
    - Handle `use Foo qw(a b)` → ExplicitImport, `use Foo ()` → suppress defaults, `use Foo` bare → DefaultExport, `use Foo ':tag'` → ExportTag
    - _Crates: perl-workspace-index_
    - _Requirements: 8.4, 12.1, 12.2, 12.3, 12.4, 12.5_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 7.2 Write property test for import visibility source attribution (Property 9)
    - **Property 9: Import Visibility Source Attribution** — `use Foo qw(a b)` makes a,b visible with ExplicitImport. Bare `use Foo` makes @EXPORT visible with DefaultExport. `use Foo ':tag'` makes tag symbols visible with ExportTag.
    - Generate random import statements and export sets with `proptest`
    - _Crates: perl-workspace-index_
    - _Requirements: 12.1, 12.3, 12.4_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 7.3 Implement SemanticQueries trait and WorkspaceSemanticQueries
    - Create `crates/perl-workspace-index/src/semantic/queries.rs`
    - Define `SemanticQueries` trait with methods: `symbol_at`, `definitions`, `references`, `visible_symbols_at`, `method_candidates`, `rename_plan`, `safe_delete_plan`
    - Define `QueryContext` struct with `file_id`, `scope_id`, `byte_offset`
    - Implement `WorkspaceSemanticQueries` struct that delegates to the semantic indexes
    - For `definitions`: return candidates sorted by DefinitionRank, then deterministically by URI and position; return empty list (not error) when no candidates found
    - For `symbol_at`: look up entity and occurrence at byte offset using anchor index
    - For `references`: look up typed OccurrenceFact lists by entity_id
    - Stub `method_candidates`, `rename_plan`, `safe_delete_plan` to return empty/conservative results
    - _Crates: perl-workspace-index_
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 5.4, 5.5, 5.6_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 7.4 Write property test for definition candidate sorting invariant (Property 7)
    - **Property 7: Definition Candidate Sorting Invariant** — Returned candidates are sorted by DefinitionRank (ExactQualified first, Heuristic last), and within same rank, sorted deterministically by file URI then source position.
    - Generate random DefinitionCandidate lists with `proptest`, sort, verify invariant
    - _Crates: perl-workspace-index_
    - _Requirements: 5.4, 5.5_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 7.5 Implement byte-span to LSP range mapping
    - Create or extend `crates/perl-workspace-index/src/semantic/facts.rs` with `byte_span_to_lsp_range` function
    - Use existing `perl_parser_core::line_index` to convert byte offsets to line/column
    - Convert columns to UTF-16 offsets for LSP protocol (handle multi-byte UTF-8 correctly)
    - AnchorFact stores byte offsets only; conversion happens at query time
    - _Crates: perl-workspace-index_
    - _Requirements: 27.1, 27.2, 27.3_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 7.6 Write property test for byte-span to LSP range determinism (Property 16)
    - **Property 16: Byte-Span to LSP Range Determinism and UTF-8 Correctness** — For any source with multi-byte UTF-8 and any valid byte span, the mapping produces a deterministic result with correct UTF-16 column offsets.
    - Generate source strings with multi-byte UTF-8 characters and valid byte spans with `proptest`
    - _Crates: perl-workspace-index_
    - _Requirements: 27.1, 27.3_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

- [x] 8. Checkpoint — Ensure all tests pass
  - Ensure all tests pass for all semantic crates. Ask the user if questions arise.

- [x] 9. Phase 4 — Provider migration: navigation (MUST)
  - [x] 9.1 Implement goto-definition shadow mode
    - In `perl-lsp-rs-core`, add shadow compare path for goto-definition
    - Run both legacy `find_definition` and new `SemanticQueries::definitions` side-by-side
    - Emit `SemanticShadowCompareReceipt` for each request
    - Return legacy result during shadow phase
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 10.1, 10.2, 22.1_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 9.2 Implement goto-definition cutover with fallback
    - Switch goto-definition to use `SemanticQueries::definitions` as primary path
    - Exact result → jump to definition; Ambiguous → show candidate list; Dynamic/Unavailable → fall back to legacy
    - Gate cutover on scorecard: regressions=0, ambiguous classified, unavailable falls back
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 9.1, 10.6, 22.3_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 9.3 Implement find-references shadow mode
    - Add shadow compare path for find-references
    - Run both legacy `find_references` and new `SemanticQueries::references` side-by-side
    - Emit receipts, return legacy result during shadow phase
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 10.1, 10.2, 22.1_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 9.4 Implement find-references cutover with fallback
    - Switch find-references to use `SemanticQueries::references` as primary path
    - Exact → return typed refs; Ambiguous → include grouped candidates; Dynamic/Unavailable → fall back to legacy
    - Gate cutover on scorecard: legacy count parity or better, definition exclusion correct
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 9.2, 10.7, 22.4_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

- [x] 10. Phase 5 — Provider migration: UX (MUST)
  - [x] 10.1 Implement completion shadow and cutover
    - Add shadow compare for completion visibility
    - Switch completion to use `SemanticQueries::visible_symbols_at` for symbol visibility
    - Exact → rank high; Ambiguous → rank lower; Dynamic/Unavailable → show low or omit
    - Gate on scorecard: explicit import fixtures pass, default export fixtures pass, empty import suppresses defaults, tag export fixtures pass
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 9.3, 10.8, 22.5_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 10.2 Implement diagnostics shadow and cutover
    - Add shadow compare for undefined-symbol diagnostics
    - Switch diagnostics to use `SemanticQueries` to verify symbol definitions before emitting undefined-symbol diagnostics
    - Exact → warn; Ambiguous → suppress or weak warning; Dynamic/Unavailable → suppress
    - Suppress undefined-symbol diagnostics for references within dynamic boundary scopes
    - Gate on scorecard: imported-symbol false positives=0, dynamic-boundary exact warnings=0
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 7.4, 9.4, 10.9, 22.6_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 10.3 Add dynamic boundary acceptance test fixtures
    - Create fixture suite for `eval $code`, `eval "sub $name { ... }"` patterns
    - Create fixture suite for `require $module`, `$module->import(qw(foo))` patterns
    - Create fixture suite for `*alias = \&target`, `${$name} = 1` symbolic dereference patterns
    - Create fixture suite for `sub AUTOLOAD { ... }` dispatch patterns
    - Verify diagnostics suppresses undefined-symbol, rename blocks, safe-delete blocks, hover explains dynamic boundary
    - _Crates: perl-lsp-rs-core, perl-semantic-analyzer_
    - _Requirements: 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 10.4 Write property test for dynamic boundary fact invariants (Property 10)
    - **Property 10: Dynamic Boundary Fact Invariants** — For any OccurrenceFact with kind DynamicBoundary, provenance is DynamicBoundary and confidence is Low.
    - Generate dynamic boundary occurrences with `proptest`
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 7.2_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

- [x] 11. Checkpoint — Ensure all tests pass
  - Ensure all tests pass for all crates. Ask the user if questions arise.

- [x] 12. Phase 6 — Package graph and generated members (SHOULD)
  - [x] 12.1 Implement package graph extractor
    - Create `crates/perl-semantic-analyzer/src/analysis/package_graph_extractor.rs`
    - Extract `PackageEdge` entries from `use parent 'Base'`, `use base 'Base'`, `@ISA = ('Base')`, `push @ISA, 'Base'`, `extends 'Base'` (Moo/Moose), `with 'Role'` (Moo/Moose)
    - Emit edges with appropriate `PackageEdgeKind` (Inherits, ComposesRole, DependsOn)
    - Record unknown external packages with confidence Low
    - Add unit tests for each inheritance/role pattern
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 12.6, 12.7, 14.1, 14.2, 14.5_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

  - [x] 12.2 Implement PackageGraphIndex in workspace with cycle detection
    - Create `crates/perl-workspace-index/src/semantic/package_graph.rs`
    - Maintain `PackageNode` entries for each known package, class, and role
    - Record `PackageEdge` entries with Inherits, ComposesRole, DependsOn kinds
    - Implement cycle detection: terminate traversal and report cycle rather than looping
    - Populate from FileFactShard during indexing
    - _Crates: perl-workspace-index_
    - _Requirements: 14.1, 14.2, 14.4_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 12.3 Write property test for package graph cycle termination (Property 12)
    - **Property 12: Package Graph Cycle Termination** — For any package graph with circular inheritance, method_candidates traversal terminates in finite time and reports the cycle.
    - Generate random graphs with cycles using `proptest`
    - _Crates: perl-workspace-index_
    - _Requirements: 14.4_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 12.4 Implement generated member extractor
    - Create `crates/perl-semantic-analyzer/src/analysis/generated_member_extractor.rs`
    - Extract `GeneratedMember` entries from Moo/Moose `has` declarations
    - `has 'x'` → accessor; `has 'x' => (is => 'rw')` → getter + setter; `has 'x' => (is => 'ro')` → getter only
    - Emit EntityFact with kind GeneratedMember, provenance FrameworkSynthesis, confidence Medium
    - Leverage existing `class_model::Attribute` extraction
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 13.1, 13.2, 13.3, 13.4_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

  - [x] 12.5 Write property test for generated member provenance invariant (Property 11)
    - **Property 11: Generated Member Provenance Invariant** — For any EntityFact with kind GeneratedMember, provenance is FrameworkSynthesis and confidence is Medium.
    - Generate has declarations with `proptest`
    - _Crates: perl-semantic-analyzer_
    - _Requirements: 13.4_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

  - [x] 12.6 Implement method_candidates in SemanticQueries
    - Extend `WorkspaceSemanticQueries` to implement `method_candidates(receiver_shape, method_name)`
    - Traverse package_graph inheritance and role-composition edges to collect method candidates from ancestor packages
    - Include generated accessor methods in candidate list for Moo/Moose classes
    - Return conservative empty results for unknown receiver shapes
    - _Crates: perl-workspace-index_
    - _Requirements: 8.5, 13.5, 14.3_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

- [ ] 13. Phase 7 — Value-shape-lite (SHOULD)
  - [x] 13.1 Add ValueShape types to perl-semantic-facts and implement inferrer
    - Add `ValueShape` enum (Unknown, Scalar, ArrayRef, HashRef, CodeRef, PackageName { package }, Object { package, confidence }) to perl-semantic-facts
    - Add `PackageNode`, `PackageEdge`, `PackageKind`, `PackageEdgeKind` types to perl-semantic-facts
    - Add `GeneratedMember`, `GeneratedMemberKind` types to perl-semantic-facts
    - Create `crates/perl-semantic-analyzer/src/analysis/value_shape_inferrer.rs`
    - Infer: `Foo->new(...)` → Object { Foo, High }; `bless $ref, 'Pkg'` → Object { Pkg, Low }; `$self` in method → Object { enclosing, Medium }; unknown → Unknown
    - _Crates: perl-semantic-facts, perl-semantic-analyzer_
    - _Requirements: 15.1, 15.3, 15.4, 15.5_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-facts --profile agent --locked && ./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked`_

  - [x] 13.2 Implement ValueShapeIndex in workspace
    - Create `crates/perl-workspace-index/src/semantic/value_shape.rs`
    - Maintain `value_shapes: HashMap<EntityId, ValueShape>` index
    - Populate from value shape inferrer output during indexing
    - Wire into `method_candidates` for receiver-shape filtering
    - _Crates: perl-workspace-index_
    - _Requirements: 15.2_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

- [x] 14. Phase 8 — Hover (MUST)
  - [x] 14.1 Implement hover provider using SemanticQueries
    - Switch hover provider to call `SemanticQueries::symbol_at` for entity information
    - Use `visible_symbols_at` context to explain symbol origin (source_module, import/export anchors)
    - Exact → explain symbol; Ambiguous → explain ambiguity; Dynamic/Unavailable → explain dynamic boundary
    - Include VisibleSymbolContext origin metadata in hover explanations
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 9.5, 22.7_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 14.2 Add hover explanation tests for import origins and dynamic boundaries
    - Test hover on explicitly imported symbol shows source module
    - Test hover on default-exported symbol shows DefaultExport source
    - Test hover on dynamic boundary symbol explains the boundary
    - Test hover on ambiguous symbol explains ambiguity
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 9.5, 22.7, 23.8_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

- [x] 15. Phase 9 — Rename and safe-delete (SHOULD, conservative)
  - [x] 15.1 Add RenamePlan, SafeDeletePlan, PlanBlocker types to perl-semantic-facts
    - Add `RenamePlan`, `SafeDeletePlan`, `PlanBlocker`, `PlanBlockerReason`, `PlanWarning`, `PlannedEdit`, `PlannedEditCategory` types
    - Use `#[non_exhaustive]` on all new types
    - Add unit tests for round-trip serialization
    - _Crates: perl-semantic-facts_
    - _Requirements: 16.1, 16.6, 17.1_
    - _Verify: `./scripts/cargo-safe test -p perl-semantic-facts --profile agent --locked`_

  - [x] 15.2 Implement rename_plan in SemanticQueries
    - Implement `rename_plan(entity_id, new_name)` returning `RenamePlan` with affected occurrences and blockers
    - Add PlanBlocker with DynamicBoundary reason when references cross dynamic boundaries
    - Add PlanBlocker with CrossModuleExport reason when symbol is exported and referenced from other modules
    - Classify occurrences: Definition, Reference, ImportList, ExportList as distinct PlannedEditCategory values
    - If import/export classification not yet implemented, block rename or warn rather than silently omitting
    - Block on generated-member entities unless generator-specific edit plan exists
    - _Crates: perl-workspace-index_
    - _Requirements: 16.1, 16.2, 16.3, 16.6, 16.7, 17.6_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 15.3 Implement safe_delete_plan in SemanticQueries
    - Implement `safe_delete_plan(entity_id)` returning `SafeDeletePlan` with blockers
    - Add PlanBlocker with ReferencesExist when symbol has remaining references
    - Add PlanBlocker with ExportedSymbol when symbol is in an ExportSet
    - Add PlanBlocker with ImportedSymbol when symbol is imported by another file
    - Block on generated-member entities unless generator-specific delete plan exists
    - _Crates: perl-workspace-index_
    - _Requirements: 17.1, 17.2, 17.3, 17.4, 17.7_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 15.4 Wire rename and safe-delete providers to SemanticQueries
    - Switch rename provider to call `SemanticQueries::rename_plan`; apply edits only when plan has no blockers; present blockers to user when present
    - Switch safe-delete provider to call `SemanticQueries::safe_delete_plan`; block deletion when plan has blockers
    - Ambiguous/Dynamic/Unavailable → block
    - _Crates: perl-lsp-rs-core_
    - _Requirements: 9.6, 9.7, 16.4, 17.5, 22.8, 22.9_
    - _Verify: `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 15.5 Write property test for rename plan safety (Property 13)
    - **Property 13: Rename Plan Safety — Dynamic and Export Blockers** — Any rename plan where target has dynamic boundary references contains DynamicBoundary blocker. Any rename plan where target is exported and cross-module referenced contains CrossModuleExport blocker.
    - Generate scenarios with dynamic/export refs using `proptest`
    - _Crates: perl-workspace-index_
    - _Requirements: 16.2, 16.3_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 15.6 Write property test for rename plan occurrence classification (Property 14)
    - **Property 14: Rename Plan Occurrence Classification** — Import occurrences classified as ImportList, export as ExportList, definition as Definition, reference as Reference. No occurrence left unclassified.
    - Generate rename scenarios with import/export using `proptest`
    - _Crates: perl-workspace-index_
    - _Requirements: 16.6_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

- [x] 16. Checkpoint — Ensure all tests pass
  - Ensure all tests pass for all crates. Ask the user if questions arise.

- [x] 17. Phase 10 — Invalidation and performance (MUST for invalidation, SHOULD for latency)
  - [x] 17.1 Implement per-category incremental invalidation
    - Extend `FileFactShard` replacement logic in `WorkspaceIndex` to compare per-category hashes (anchors_hash, entities_hash, occurrences_hash, edges_hash)
    - When content_hash unchanged → skip all per-category comparisons
    - When a category hash unchanged → skip re-indexing that category's cross-file indexes
    - When a category hash changed → remove old entries, insert new ones for that category
    - Add unit tests verifying skip behavior and correct replacement
    - _Crates: perl-workspace-index_
    - _Requirements: 18.1, 18.2, 18.3, 18.4, 18.5_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 17.2 Write property test for incremental invalidation correctness (Property 15)
    - **Property 15: Incremental Invalidation Correctness** — Unchanged content_hash → no index modification. Unchanged category hash → skip that category. Changed category hash → remove old, insert new.
    - Generate FileFactShards with varying hashes using `proptest`
    - _Crates: perl-workspace-index_
    - _Requirements: 18.3, 18.4, 18.5_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 17.3 Add query latency benchmarks and scorecard latency reporting
    - Add benchmarks for `symbol_at` (target: 5ms p95), `definitions` (10ms p95), `references` (20ms p95), `visible_symbols_at` (15ms p95) on a 1000-file workspace
    - Wire latency measurements into scorecard reporting
    - Flag any measurements exceeding target thresholds
    - _Crates: perl-workspace-index_
    - _Requirements: 19.1, 19.2, 19.3, 19.4, 19.5, 11.7_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

- [x] 18. Phase 11 — Scorecards and final validation (MUST)
  - [x] 18.1 Implement per-provider scorecard gate fixtures
    - Create fixture suites for goto-definition scorecard (Req 10.6): regressions=0, ambiguous classified, unavailable falls back
    - Create fixture suites for find-references scorecard (Req 10.7): legacy count parity or better, definition exclusion correct
    - Create fixture suites for completion scorecard (Req 10.8): explicit import pass, default export pass, empty import suppresses, tag export pass
    - Create fixture suites for diagnostics scorecard (Req 10.9): imported-symbol false positives=0, dynamic-boundary exact warnings=0
    - Create fixture suites for rename/safe-delete scorecard (Req 10.10): unsafe edits=0, dynamic blocked, ambiguous blocked, export/import blocked or planned
    - _Crates: perl-workspace-index, perl-lsp-rs-core_
    - _Requirements: 10.6, 10.7, 10.8, 10.9, 10.10, 11.3, 11.4, 11.5, 11.6_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked && ./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked`_

  - [x] 18.2 Implement scorecard aggregation and reporting
    - Aggregate shadow-compare receipts across all migrated providers and fixture suites
    - Report per-query verdicts with counts (Same, Improved, Regression, Ambiguous, Unavailable)
    - Support emit mode (always okay) and --check mode (deterministic artifact freshness)
    - Verify rename unsafe-edit count is zero
    - _Crates: perl-workspace-index_
    - _Requirements: 11.1, 11.2, 11.6, 11.8_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 18.3 Verify existing public APIs remain available
    - Confirm `WorkspaceIndex` public APIs (`find_definition`, `find_references`, `count_usages`, `definition_candidates`) remain available and functional
    - These APIs must remain until the semantic query path has scorecard proof
    - Add regression tests if not already covered
    - _Crates: perl-workspace-index_
    - _Requirements: 10.5_
    - _Verify: `./scripts/cargo-safe test -p perl-workspace --profile agent --locked`_

  - [x] 18.4 Verify build and test gates pass for all semantic crates
    - Run build gate for perl-semantic-facts, perl-symbol, perl-semantic-analyzer, perl-workspace-index, perl-lsp-rs-core
    - Verify semantic fact counts > 0 for all categories when analyzing non-trivial Perl source
    - Run clippy with no warnings for all semantic crates
    - _Crates: all semantic crates_
    - _Requirements: 20.1, 20.2, 20.3, 20.4_
    - _Verify: `./scripts/cargo-safe clippy -p perl-semantic-facts -p perl-symbol -p perl-semantic-analyzer -p perl-workspace -p perl-lsp-rs-core --profile agent --locked -- -D warnings -A missing_docs`_

- [x] 19. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass across all crates. Run `just agent-pr-fast` for final validation. Ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation between phases
- Property tests validate universal correctness properties from the design document
- Unit tests validate specific examples and edge cases
- **MUST** tasks are required for RC2 (Phases 0–5, 8, 11)
- **SHOULD** tasks are conservative/partial for RC2 (Phases 6, 7, 9, 10)
- All new workspace semantic code goes in `src/semantic/` module tree per Req 24
- Do not create new crates; use existing crate seams per Req 25
- Scope identity (ScopeId) should be preserved on facts where known per Req 26
