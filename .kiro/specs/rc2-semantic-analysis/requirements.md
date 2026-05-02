# Requirements Document

## Introduction

RC2 Semantic Analysis builds the canonical semantic spine for perl-lsp. Today, each LSP provider (completion, diagnostics, hover, goto-definition, find-references, rename) maintains its own parallel semantic approximation — duplicating scope resolution, symbol lookup, and import inference. This feature replaces those provider-local approximations with a single, layered semantic substrate:

- **perl-semantic-facts**: neutral vocabulary of IDs, facts, import/export types, reference edges, and package graph types
- **perl-symbol**: exact AST projection producing `SymbolDecl`/`SymbolRef` and semantic fact adapters
- **perl-semantic-analyzer**: richer producer emitting scopes, imports, exports, framework/generated members, package/class/role graph, dynamic boundaries, and value-shape-lite
- **perl-workspace-index**: store + index + query (`FileFactShard`, semantic indexes, `SemanticQueries` facade, invalidation/caches)
- **LSP providers**: consumers that call the workspace semantic query facade instead of reimplementing semantic logic

The implementation follows a linear 13-phase path (~40 PRs) from stabilizing current substrate seams through provider migration to scorecards proving behavioral improvement.

### RC2 Implementation Priority

RC2 must first complete the semantic spine through visibility and provider cutover for navigation, references, completion, and undefined-symbol diagnostics. Rename, safe-delete, package graph, generated members, and value-shape-lite are RC2 goals only when implemented conservatively. Unsupported ambiguous or dynamic cases must block or return unavailable rather than guess. The implementation must not create new provider-local semantic logic. Every new semantic behavior should either emit facts, index facts, expose a query, or consume a query.

#### Priority Classification

**MUST for RC2:** canonical facts, adapters, FileFactShard, indexes, ImportSpec extraction, visible_symbols_at, SemanticQueries facade, goto-definition cutover, find-references cutover, completion import/visibility cutover, undefined-symbol diagnostic cutover, hover explanations, scorecards, shadow compare

**SHOULD for RC2 (conservative/partial):** rename_plan, safe_delete_plan, package graph, method_candidates, value-shape-lite, generated members, dynamic boundary classification — these should exist but can conservatively return blockers/unavailable

**MAY / post-RC2:** external @INC index, Perldoc integration, full CPAN metadata, persistent semantic database, broad framework-specific metamodel

#### Adjusted Implementation Order (front-loaded)

Fix SymbolRef adapter → ReferenceEdge → DefinitionCandidate reason fields → FileFactShard → typed reference index → scorecard counts → shadow compare → ImportSpec extraction → require/import → ImportExportIndex → visible_symbols_at → SemanticQueries facade

### Non-goals for RC2

- Full type inference
- General Perl symbolic evaluator
- Full Moose metamodel
- Runtime execution
- Persistent semantic database
- Complete CPAN metadata resolver

## Glossary

- **Fact_Schema**: The set of strongly-typed ID newtypes and serializable fact records defined in `perl-semantic-facts` (`AnchorFact`, `EntityFact`, `OccurrenceFact`, `EdgeFact`, `DiagnosticFact`, `ExportSet`, `ImportSpec`, `VisibleSymbol`)
- **Anchor**: A source-location binding (`AnchorFact`) that ties a semantic fact to a file, byte span, scope, provenance, and confidence
- **Entity**: A stable declaration or generated member (`EntityFact`) with a deterministic ID, kind, canonical name, and provenance
- **Occurrence**: A reference or use site (`OccurrenceFact`) classified by kind (definition, read, write, call, import, export, dynamic boundary)
- **Edge**: A typed relationship (`EdgeFact`) between two entities, optionally mediated by an occurrence
- **Reference_Edge**: An occurrence-based edge (not entity-to-entity) linking a reference site to zero, one, or many target candidates (`target_candidates: Vec<EntityId>`) through an `OccurrenceFact`. A low-confidence ambiguous reference is not the same as "not found" — multiple candidates support rename and safe-delete safety analysis
- **Definition_Candidate**: A ranked entry in a candidate list for definition resolution, scored by `DefinitionRank` and carrying a structured `DefinitionRankReason` (ExactQualifiedName, SamePackage, ExplicitImport { module }, DefaultExport { module }, WorkspaceSymbol, HeuristicNameMatch)
- **FileFactShard**: Per-file write-through semantic fact storage holding anchors, entities, occurrences, edges, and per-category content hashes for incremental invalidation
- **Semantic_Index**: A set of cross-file lookup structures (definitions_by_qualified, definitions_by_bare, references_by_name, references_by_entity, imports_by_file, exports_by_module, visible_symbols_cache, package_graph, value_shapes)
- **SemanticQueries**: The workspace query facade trait exposing `symbol_at`, `definitions`, `references`, `visible_symbols_at`, `method_candidates`, `rename_plan`, `safe_delete_plan`
- **Visible_Symbol**: A symbol visible at a query point with source attribution (`VisibleSymbol` with `VisibleSymbolSource` classification) and optional origin metadata (source_module, source_import_anchor_id, source_export_anchor_id) enabling hover explanations and rename safety
- **Provenance**: The origin classification of a fact (ExactAst, DesugaredAst, SemanticAnalyzer, FrameworkSynthesis, ImportExportInference, PragmaInference, NameHeuristic, SearchFallback, DynamicBoundary)
- **Confidence**: The certainty level of a fact (High, Medium, Low)
- **Shadow_Compare**: A mechanism that runs both old-path and new-path query implementations side-by-side, producing deterministic receipts with verdicts (Same, Improved, Regression, Ambiguous, Unavailable)
- **Scorecard**: An aggregated set of shadow-compare receipts and fixture results proving behavioral improvement across dimensions
- **Package_Graph**: A directed graph of package/class/role nodes connected by inheritance, role-composition, and dependency edges
- **Value_Shape**: A lightweight type approximation (Unknown, Scalar, ArrayRef, HashRef, CodeRef, PackageName { package }, Object { package, confidence }) used for method candidate filtering. Blessed is not a separate top-level shape
- **Rename_Plan**: A conservative plan for rename operations that enumerates affected occurrences and blockers (ambiguous references, dynamic boundaries, cross-module exports) and classifies import and export occurrences separately from normal references
- **Safe_Delete_Plan**: A conservative plan for safe-delete operations that enumerates blockers (exported symbols, imported symbols, remaining references, generated-member entities without generator-specific edit plans)
- **Dynamic_Boundary**: A Perl construct where static analysis cannot determine the target (string eval, symbolic derefs, AUTOLOAD dispatch, runtime require)
- **Provider**: An LSP feature implementation (completion, diagnostics, hover, goto-definition, find-references, rename, safe-delete) that consumes the semantic query facade
- **Workspace_Store**: The `WorkspaceIndex` component that owns `FileFactShard` instances and maintains cross-file semantic indexes

## Requirements

### Requirement 1: Canonical Fact Vocabulary

**User Story:** As a semantic layer developer, I want all semantic producers to emit facts in one shared vocabulary, so that consumers do not need to understand multiple incompatible representations.

#### Acceptance Criteria

1. THE Fact_Schema SHALL define typed ID newtypes for FileId, ScopeId, EntityId, AnchorId, OccurrenceId, EdgeId, and DiagnosticId
2. THE Fact_Schema SHALL define fact record types for AnchorFact, EntityFact, OccurrenceFact, EdgeFact, and DiagnosticFact
3. THE Fact_Schema SHALL define import/export types for ExportSet, ExportTag, ImportSpec, ImportKind, ImportSymbols, VisibleSymbol, and VisibleSymbolSource
4. THE Fact_Schema SHALL define Provenance and Confidence enums covering all producer categories (ExactAst, DesugaredAst, SemanticAnalyzer, FrameworkSynthesis, ImportExportInference, PragmaInference, NameHeuristic, SearchFallback, DynamicBoundary)
5. THE Fact_Schema SHALL define EntityKind and OccurrenceKind enums covering all declaration and reference classifications used by RC2 producers
6. THE Fact_Schema SHALL define EdgeKind enum covering all relationship types (Defines, References, Reads, Writes, Calls, ImportsModule, ImportsSymbol, ExportsSymbol, ExportsGroup, Inherits, ComposesRole, MemberOf, GeneratedFrom, AliasOf, DependsOn, DynamicBoundary)
7. FOR ALL fact record types, parsing then serializing then parsing through JSON SHALL produce an equivalent object (round-trip property)

### Requirement 2: Stable Source Anchoring

**User Story:** As a provider developer, I want every source-derived fact to carry a stable source anchor, so that I can map semantic results back to precise file locations.

#### Acceptance Criteria

1. THE AnchorFact SHALL contain file_id, span_start_byte, span_end_byte, scope_id, provenance, and confidence fields
2. WHEN a semantic producer emits an EntityFact, THE producer SHALL associate the EntityFact with an AnchorFact via the anchor_id field
3. WHEN a semantic producer emits an OccurrenceFact, THE producer SHALL associate the OccurrenceFact with an AnchorFact via the anchor_id field
4. THE AnchorFact span bytes SHALL represent exact byte offsets into the source file content
5. WHEN the same source location is referenced by multiple facts, THE producers SHALL reuse the same AnchorId for that location

### Requirement 3: Stable Entity Identifiers

**User Story:** As a workspace index developer, I want every declaration and generated member to have a deterministic entity ID, so that cross-file references can be resolved without ambiguity.

#### Acceptance Criteria

1. THE EntityFact SHALL contain id, kind, canonical_name, anchor_id, scope_id, provenance, and confidence fields
2. THE EntityFact id SHALL be deterministic given the same namespace, canonical_name, and source span
3. WHEN the same source file is re-analyzed without content changes, THE producer SHALL emit EntityFacts with identical IDs
4. THE EntityKind enum SHALL distinguish Package, Class, Role, Subroutine, Method, Variable, Constant, Field, Label, Format, Module, GeneratedMember, ExternalSymbol, and Unknown
5. WHEN a generated member is synthesized (Moo/Moose accessor, constant), THE producer SHALL emit an EntityFact with kind GeneratedMember and provenance FrameworkSynthesis

### Requirement 4: Occurrence-Based References

**User Story:** As a find-references developer, I want references represented as typed occurrences rather than bare locations, so that I can distinguish reads from writes, calls from imports, and static from dynamic references.

#### Acceptance Criteria

1. THE OccurrenceFact SHALL contain id, kind, entity_id, anchor_id, scope_id, provenance, and confidence fields
2. THE OccurrenceKind enum SHALL distinguish Definition, Reference, Read, Write, Call, MethodCall, StaticMethodCall, Import, Export, Inheritance, RoleComposition, GeneratedUse, and DynamicBoundary
3. WHEN a symbol reference is extracted from the AST, THE SymbolRef adapter SHALL emit an OccurrenceFact with the appropriate OccurrenceKind
4. WHEN an occurrence can be resolved to one or more known entities, THE OccurrenceFact SHALL carry the resolved entity_id for the best candidate, and the associated Reference_Edge SHALL carry all target candidates in `target_candidates: Vec<EntityId>`
5. WHEN an occurrence cannot be resolved to a known entity, THE OccurrenceFact SHALL carry entity_id as None, the associated Reference_Edge SHALL carry an empty target_candidates list, and confidence SHALL be Low or Medium
6. THE Reference_Edge SHALL allow zero, one, or many target candidates (`target_candidates: Vec<EntityId>`), not `Option<EntityId>`, so that ambiguous references are distinguishable from unresolved references

### Requirement 5: Ranked Definition Candidates

**User Story:** As a goto-definition developer, I want definitions returned as ranked candidate lists rather than a single winner, so that ambiguous cases are surfaced to the user instead of silently picking the wrong target.

#### Acceptance Criteria

1. THE Workspace_Store SHALL maintain a definition index mapping qualified and bare names to ordered lists of Definition_Candidate entries
2. THE Definition_Candidate SHALL carry a DefinitionRank value from the set ExactQualified, SamePackage, ExplicitImport, DefaultExport, WorkspaceCandidate, Heuristic
3. THE Definition_Candidate SHALL carry a structured DefinitionRankReason enum value (ExactQualifiedName, SamePackage, ExplicitImport { module: String }, DefaultExport { module: String }, WorkspaceSymbol, HeuristicNameMatch) so that shadow compare receipts can explain ranking decisions
4. WHEN the SemanticQueries `definitions` method is called, THE facade SHALL return candidates sorted by DefinitionRank (ExactQualified first, Heuristic last)
5. WHEN multiple candidates share the same rank, THE facade SHALL sort candidates deterministically by file URI and source position
6. WHEN no candidates are found, THE facade SHALL return an empty candidate list rather than an error

### Requirement 6: First-Class Imports and Exports

**User Story:** As a completion developer, I want imports and exports represented as first-class semantic facts, so that symbol visibility can be computed from the fact graph rather than re-parsing use statements.

#### Acceptance Criteria

1. WHEN a `use Module qw(a b)` statement is analyzed, THE import extractor SHALL emit an ImportSpec with kind UseExplicitList and symbols Explicit(["a", "b"])
2. WHEN a `use Module ()` statement is analyzed, THE import extractor SHALL emit an ImportSpec with kind UseEmpty and symbols None
3. WHEN a `use Module ':tag'` statement is analyzed, THE import extractor SHALL emit an ImportSpec with kind UseTag and symbols Tags(["tag"])
4. WHEN a `require Module; Module->import(...)` pattern is analyzed, THE import extractor SHALL emit an ImportSpec with kind RequireThenImport
5. WHEN a module declares `@EXPORT`, `@EXPORT_OK`, or `%EXPORT_TAGS`, THE export extractor SHALL emit an ExportSet with the corresponding default_exports, optional_exports, and tags
6. THE Workspace_Store SHALL maintain an imports_by_file index mapping FileId to the list of ImportSpec entries for that file
7. THE Workspace_Store SHALL maintain an exports_by_module index mapping module name to the ExportSet for that module
8. THE ImportSpec SHALL carry file_id, anchor_id, and scope_id fields so that visibility, hover, diagnostics, and refactors can attribute imported symbols to source locations
9. THE ExportSet SHALL carry the exporting module or package name and, where available, the source anchor_id of the export declaration so that the fact is self-describing

### Requirement 7: Explicit Dynamic Perl Boundaries

**User Story:** As a diagnostics developer, I want dynamic Perl constructs explicitly marked in the fact graph, so that false-positive diagnostics are suppressed at dynamic boundaries.

#### Acceptance Criteria

1. WHEN a string eval, symbolic dereference, AUTOLOAD dispatch, or runtime require is encountered, THE producer SHALL emit an OccurrenceFact with kind DynamicBoundary
2. WHEN a dynamic boundary occurrence is emitted, THE producer SHALL set provenance to DynamicBoundary and confidence to Low
3. WHEN a VisibleSymbol originates from a dynamic source, THE producer SHALL set the source to DynamicUnknown
4. WHEN the diagnostics provider encounters a symbol reference within a dynamic boundary scope, THE diagnostics provider SHALL suppress undefined-symbol diagnostics for that reference
5. WHEN the rename provider encounters a reference that crosses a dynamic boundary, THE rename provider SHALL include a PlanBlocker in the Rename_Plan

### Requirement 8: Semantic Query Facade

**User Story:** As a provider developer, I want a single query facade for all semantic lookups, so that providers do not need to understand the internal index structure.

#### Acceptance Criteria

1. THE SemanticQueries trait SHALL expose a `symbol_at` method that returns the entity and occurrence at a given file position
2. THE SemanticQueries trait SHALL expose a `definitions` method that returns ranked Definition_Candidate lists for a given symbol
3. THE SemanticQueries trait SHALL expose a `references` method that returns typed OccurrenceFact lists for a given entity
4. THE SemanticQueries trait SHALL expose a `visible_symbols_at` method that returns VisibleSymbol lists for a given file position and scope, where each VisibleSymbol carries optional origin metadata (source_module, source_import_anchor_id, source_export_anchor_id)
5. THE SemanticQueries trait SHALL expose a `method_candidates` method that returns method Definition_Candidate lists for a given receiver type and method name
6. THE SemanticQueries trait SHALL expose a `rename_plan` method that returns a Rename_Plan with affected occurrences and blockers
7. THE SemanticQueries trait SHALL expose a `safe_delete_plan` method that returns a Safe_Delete_Plan with blockers

### Requirement 9: Provider Migration to Semantic Queries

**User Story:** As a user, I want all LSP providers to use the canonical semantic substrate, so that navigation, completion, diagnostics, hover, rename, and safe-delete produce consistent, correct results.

#### Acceptance Criteria

1. WHEN the goto-definition provider receives a request, THE provider SHALL call SemanticQueries `definitions` and return the top-ranked candidate locations
2. WHEN the find-references provider receives a request, THE provider SHALL call SemanticQueries `references` and return the occurrence locations
3. WHEN the completion provider receives a request at a given position, THE provider SHALL call SemanticQueries `visible_symbols_at` to determine which symbols are in scope
4. WHEN the diagnostics provider analyzes a file, THE provider SHALL call SemanticQueries to verify symbol definitions before emitting undefined-symbol diagnostics
5. WHEN the hover provider receives a request, THE provider SHALL call SemanticQueries `symbol_at` to retrieve entity information for the hovered symbol
6. WHEN the rename provider receives a request, THE provider SHALL call SemanticQueries `rename_plan` and apply edits only when the plan contains no blockers
7. WHEN the safe-delete provider receives a request, THE provider SHALL call SemanticQueries `safe_delete_plan` and block deletion when the plan contains blockers

### Requirement 10: Migration Compatibility

**User Story:** As a user, I want existing LSP behavior preserved during migration, so that the semantic substrate rollout does not introduce regressions.

#### Acceptance Criteria

1. WHILE the semantic query path is being validated, THE providers SHALL maintain the existing query path as a fallback
2. WHEN a provider is migrated, THE migration SHALL use shadow-compare to run both old and new paths and produce deterministic receipts
3. WHEN a shadow-compare receipt shows a Regression verdict, THE provider SHALL use the old-path result
4. WHEN all shadow-compare receipts for a provider show Same or Improved verdicts across the fixture suite, THE provider SHALL cut over to the new-path result
5. THE existing public APIs of WorkspaceIndex (find_definition, find_references, count_usages, definition_candidates) SHALL remain available until the semantic query path has scorecard proof
6. BEFORE goto-definition cutover, THE Scorecard SHALL show regressions=0, ambiguous cases classified, and unavailable cases falling back to legacy
7. BEFORE find-references cutover, THE Scorecard SHALL show legacy count parity or better and definition exclusion correct
8. BEFORE completion cutover, THE Scorecard SHALL show explicit import fixtures pass, default export fixtures pass, empty import suppresses defaults, and tag export fixtures pass
9. BEFORE diagnostics cutover, THE Scorecard SHALL show imported-symbol false positives=0 and dynamic-boundary exact warnings=0
10. BEFORE rename/safe-delete cutover, THE Scorecard SHALL show unsafe edits=0, dynamic references blocked, ambiguous references blocked, and export/import references blocked or planned

### Requirement 11: Scorecard Proof

**User Story:** As a project maintainer, I want scorecards proving behavioral improvement, so that the migration can be validated objectively before removing old code paths.

#### Acceptance Criteria

1. THE Scorecard SHALL aggregate shadow-compare receipts across all migrated providers and fixture suites
2. THE Scorecard SHALL report per-query verdicts (Same, Improved, Regression, Ambiguous, Unavailable) with counts
3. WHEN the completion visibility fixture suite is run, THE Scorecard SHALL show all fixtures passing
4. WHEN the undefined-symbol false-positive fixture suite is run, THE Scorecard SHALL show all fixtures passing
5. WHEN the definition/reference shadow-compare suite is run, THE Scorecard SHALL show Same or Improved verdicts for all fixtures
6. THE Scorecard SHALL report rename unsafe-edit count as zero
7. THE Scorecard SHALL report query latency p95 within the target threshold
8. THE Scorecard SHALL support three modes: emit (always okay), --check (deterministic artifact freshness), and --gate (future hard gate, opt-in by CI lane). For RC2, --check SHALL be available but --gate SHALL only become blocking after stabilization

### Requirement 12: Perl-Specific Import Semantics

**User Story:** As a Perl developer, I want the semantic substrate to correctly model Perl's import mechanisms, so that completion and diagnostics respect Perl's actual symbol visibility rules.

#### Acceptance Criteria

1. WHEN `use Foo qw(a b)` is analyzed, THE visibility index SHALL make symbols `a` and `b` visible in the importing file with source ExplicitImport
2. WHEN `use Foo ()` is analyzed, THE visibility index SHALL make no symbols from Foo visible in the importing file (empty import suppresses defaults)
3. WHEN `use Foo` (bare, no import list) is analyzed, THE visibility index SHALL make Foo's default exports (`@EXPORT`) visible in the importing file with source DefaultExport
4. WHEN `use Foo ':tag'` is analyzed, THE visibility index SHALL make all symbols in the named tag visible in the importing file with source ExportTag
5. WHEN `require Foo; Foo->import(qw(x y))` is analyzed, THE visibility index SHALL make symbols `x` and `y` visible in the importing file
6. WHEN `use parent 'Base'` or `use base 'Base'` is analyzed, THE package graph SHALL record an inheritance edge from the current package to Base
7. WHEN `@ISA = ('Base')` or `push @ISA, 'Base'` is analyzed, THE package graph SHALL record an inheritance edge from the current package to Base

### Requirement 13: Framework-Generated Members

**User Story:** As a Perl developer using Moo/Moose, I want generated accessors and methods recognized by the semantic substrate, so that completion and goto-definition work for framework-synthesized symbols.

#### Acceptance Criteria

1. WHEN `has 'x'` is analyzed in a Moo/Moose class, THE producer SHALL emit an EntityFact with kind GeneratedMember for the accessor method `x`
2. WHEN `has 'x' => (is => 'rw')` is analyzed, THE producer SHALL emit EntityFacts for both the getter `x` and the setter `x`
3. WHEN `has 'x' => (is => 'ro')` is analyzed, THE producer SHALL emit an EntityFact for the getter `x` only
4. WHEN a generated member EntityFact is emitted, THE EntityFact SHALL have provenance FrameworkSynthesis and confidence Medium
5. WHEN the SemanticQueries `method_candidates` method is called for a Moo/Moose class, THE facade SHALL include generated accessor methods in the candidate list

### Requirement 14: Package and Class Graph

**User Story:** As a method-resolution developer, I want a package/class/role graph in the semantic index, so that method candidates can be resolved through inheritance and role composition chains.

#### Acceptance Criteria

1. THE Workspace_Store SHALL maintain a package_graph index containing PackageNode entries for each known package, class, and role
2. THE package_graph SHALL record PackageEdge entries with kinds Inherits, ComposesRole, and DependsOn
3. WHEN the SemanticQueries `method_candidates` method is called, THE facade SHALL traverse the package_graph inheritance and role-composition edges to collect method candidates from ancestor packages
4. WHEN a circular inheritance chain is detected, THE traversal SHALL terminate and report the cycle rather than looping indefinitely
5. WHEN a package inherits from an unknown external package, THE package_graph SHALL record the edge with confidence Low

### Requirement 15: Value Shape Lite

**User Story:** As a completion developer, I want lightweight type approximations for variables, so that method completion can filter candidates by receiver shape.

#### Acceptance Criteria

1. THE Fact_Schema SHALL define a ValueShape enum with variants Unknown, Scalar, ArrayRef, HashRef, CodeRef, PackageName { package: String }, and Object { package: String, confidence: Confidence }
2. THE Workspace_Store SHALL maintain a value_shapes index mapping EntityId to ValueShape
3. WHEN a variable is assigned from a constructor call (`Foo->new`), THE producer SHALL record ValueShape::Object { package: "Foo", confidence } for that variable
4. WHEN a variable is assigned from `bless`, THE producer SHALL record ValueShape::Object { package, confidence: Low } for that variable with the blessed package name
5. WHEN a variable's shape cannot be determined, THE producer SHALL record ValueShape::Unknown

### Requirement 16: Conservative Rename

**User Story:** As a developer, I want rename to be safe by default, so that ambiguous or dynamic references are flagged rather than silently producing incorrect edits.

#### Acceptance Criteria

1. WHEN the rename provider calls SemanticQueries `rename_plan`, THE facade SHALL return a Rename_Plan containing all affected OccurrenceFacts and any PlanBlocker entries
2. WHEN a reference crosses a dynamic boundary (string eval, symbolic deref, AUTOLOAD), THE Rename_Plan SHALL include a PlanBlocker with reason DynamicBoundary
3. WHEN a symbol is exported and referenced from other modules, THE Rename_Plan SHALL include a PlanBlocker with reason CrossModuleExport
4. WHEN the Rename_Plan contains blockers, THE rename provider SHALL present the blockers to the user and require confirmation before applying edits
5. THE Scorecard SHALL verify that rename unsafe-edit count is zero across the fixture suite
6. THE Rename_Plan SHALL classify import and export occurrences separately from normal references; a rename of a symbol may involve definition edit, export list edit, import list edit, and call site edit as distinct occurrence categories
7. IF import or export occurrence classification is not yet implemented, THEN THE rename provider SHALL block the rename or warn the user rather than silently omitting import/export edits

### Requirement 17: Conservative Safe Delete

**User Story:** As a developer, I want safe-delete to block deletion of symbols that are still referenced, exported, or imported, so that I do not break dependent code.

#### Acceptance Criteria

1. WHEN the safe-delete provider calls SemanticQueries `safe_delete_plan`, THE facade SHALL return a Safe_Delete_Plan containing any PlanBlocker entries
2. WHEN a symbol has remaining references in the workspace, THE Safe_Delete_Plan SHALL include a PlanBlocker with reason ReferencesExist
3. WHEN a symbol is listed in an ExportSet, THE Safe_Delete_Plan SHALL include a PlanBlocker with reason ExportedSymbol
4. WHEN a symbol is imported by another file, THE Safe_Delete_Plan SHALL include a PlanBlocker with reason ImportedSymbol
5. WHEN the Safe_Delete_Plan contains blockers, THE safe-delete provider SHALL present the blockers to the user and block deletion
6. THE Rename_Plan SHALL block or warn on generated-member entities unless a generator-specific edit plan exists
7. THE Safe_Delete_Plan SHALL block or warn on generated-member entities unless a generator-specific delete plan exists

### Requirement 18: FileFactShard Incremental Invalidation

**User Story:** As a workspace index developer, I want per-category content hashes on FileFactShard, so that unchanged fact categories are not reprocessed during incremental re-indexing.

#### Acceptance Criteria

1. THE FileFactShard SHALL contain per-category hash fields for anchors, entities, occurrences, and edges
2. WHEN a file is re-indexed, THE Workspace_Store SHALL compare per-category hashes between the old and new FileFactShard
3. WHEN a category hash is unchanged, THE Workspace_Store SHALL skip re-indexing that category's cross-file indexes
4. WHEN a category hash has changed, THE Workspace_Store SHALL remove the old category entries and insert the new ones
5. THE whole-file content_hash SHALL be compared first; WHEN the content_hash is unchanged, THE Workspace_Store SHALL skip all per-category comparisons
6. THE FileFactShard in perl-workspace-index MAY contain Url fields. Any fact type in perl-semantic-facts SHALL use FileId and plain serializable fields only; the perl-semantic-facts crate SHALL NOT depend on URL, LSP, or workspace-specific types

### Requirement 19: Query Performance

**User Story:** As a user, I want semantic queries to respond within interactive latency bounds, so that the LSP experience remains responsive.

#### Acceptance Criteria

1. THE SemanticQueries `symbol_at` method SHALL respond within 5ms at p95 for a workspace of 1000 files
2. THE SemanticQueries `definitions` method SHALL respond within 10ms at p95 for a workspace of 1000 files
3. THE SemanticQueries `references` method SHALL respond within 20ms at p95 for a workspace of 1000 files
4. THE SemanticQueries `visible_symbols_at` method SHALL respond within 15ms at p95 for a workspace of 1000 files
5. THE Scorecard SHALL report query latency p95 measurements and flag any that exceed the target thresholds

### Requirement 20: Build and Test Gates

**User Story:** As a CI maintainer, I want all semantic crates to pass build gates, so that the semantic substrate does not introduce build regressions.

#### Acceptance Criteria

1. THE build gate SHALL pass for perl-semantic-facts, perl-symbol, perl-semantic-analyzer, perl-workspace-index, and perl-lsp-rs-core
2. THE test gate SHALL verify that semantic fact counts are greater than zero for all fact categories (anchors, entities, occurrences, edges) when analyzing non-trivial Perl source
3. THE clippy gate SHALL pass with no warnings for all semantic crates
4. WHEN a new fact type or query method is added, THE developer SHALL add corresponding unit tests verifying round-trip serialization and deterministic output

### Requirement 21: Canonical SymbolRef Adapter Path

**User Story:** As a semantic layer developer, I want exactly one canonical adapter path from SymbolRef to OccurrenceFact, so that duplicate adapter modules do not cause confusion or inconsistent fact emission.

#### Acceptance Criteria

1. THE perl-symbol surface SHALL expose exactly one canonical SymbolRef to OccurrenceFact adapter path
2. THE perl-symbol surface SHALL NOT contain duplicate adapter modules or duplicate public type names for the SymbolRef to OccurrenceFact conversion
3. WHEN a new adapter path is proposed, THE developer SHALL verify that no existing adapter path already covers the same conversion and remove the duplicate

### Requirement 22: Provider Migration Fallback Policy

**User Story:** As a provider developer, I want explicit fallback behavior for every provider cutover, so that users do not experience degraded results when the semantic path is unavailable or ambiguous.

#### Acceptance Criteria

1. WHILE a provider migration is in progress, THE provider SHALL run in shadow mode before cutover
2. WHEN the semantic query returns unavailable, low-confidence, or dynamic-boundary-only results after cutover, THE provider SHALL fall back to the legacy path or return a conservative result
3. WHEN the goto-definition provider receives an exact result, THE provider SHALL jump to the definition; WHEN the result is ambiguous, THE provider SHALL show candidates; WHEN the result is dynamic or unavailable, THE provider SHALL fall back to the legacy path
4. WHEN the find-references provider receives an exact result, THE provider SHALL return typed references; WHEN the result is ambiguous, THE provider SHALL include grouped candidates; WHEN the result is dynamic or unavailable, THE provider SHALL fall back to the legacy path
5. WHEN the completion provider receives an exact result, THE provider SHALL rank the symbol high; WHEN the result is ambiguous, THE provider SHALL show the symbol lower; WHEN the result is dynamic or unavailable, THE provider SHALL show the symbol low or omit
6. WHEN the diagnostics provider receives an exact result, THE provider SHALL warn; WHEN the result is ambiguous, THE provider SHALL suppress or emit a weak warning; WHEN the result is dynamic or unavailable, THE provider SHALL suppress the diagnostic
7. WHEN the hover provider receives an exact result, THE provider SHALL explain the symbol; WHEN the result is ambiguous, THE provider SHALL explain the ambiguity; WHEN the result is dynamic or unavailable, THE provider SHALL explain the dynamic boundary
8. WHEN the rename provider receives an exact result, THE provider SHALL allow the rename; WHEN the result is ambiguous or dynamic or unavailable, THE provider SHALL block the rename
9. WHEN the safe-delete provider receives an exact result with no references, THE provider SHALL allow deletion; WHEN the result is ambiguous or dynamic or unavailable, THE provider SHALL block deletion

### Requirement 23: Dynamic Boundary Acceptance Test Fixtures

**User Story:** As a test developer, I want concrete dynamic boundary fixtures, so that the semantic substrate's behavior at dynamic boundaries is verified against known Perl patterns.

#### Acceptance Criteria

1. THE fixture suite SHALL include a test for `eval $code` and `eval "sub $name { ... }"` patterns
2. THE fixture suite SHALL include a test for `require $module` and `$module->import(qw(foo))` patterns
3. THE fixture suite SHALL include a test for `*alias = \&target` and `${$name} = 1` symbolic dereference patterns
4. THE fixture suite SHALL include a test for `sub AUTOLOAD { ... }` dispatch patterns
5. FOR ALL dynamic boundary fixtures, THE diagnostics provider SHALL NOT emit exact undefined-symbol diagnostics
6. FOR ALL dynamic boundary fixtures, THE rename provider SHALL block or warn
7. FOR ALL dynamic boundary fixtures, THE safe-delete provider SHALL block or warn
8. FOR ALL dynamic boundary fixtures, THE hover provider SHALL explain the dynamic boundary
9. FOR ALL dynamic boundary fixtures, THE completion provider MAY degrade gracefully

### Requirement 24: Workspace Semantic Module Layout

**User Story:** As a workspace developer, I want semantic code organized under a dedicated module tree, so that workspace_index.rs does not accumulate semantic logic directly.

#### Acceptance Criteria

1. THE perl-workspace-index semantic code SHALL live under a dedicated semantic module tree (e.g., src/semantic/)
2. THE workspace_index.rs file SHALL contain only thin call-through methods for semantic operations, not direct semantic logic
3. WHEN new semantic functionality is added to perl-workspace-index, THE developer SHALL place the implementation in the semantic module tree

### Requirement 25: Kiro Implementation Constraints

**User Story:** As an implementation agent, I want explicit constraints on how the semantic substrate is built, so that PRs are compatible with the existing codebase and do not introduce architectural drift.

#### Acceptance Criteria

1. THE implementation SHALL build from current master and SHALL NOT assume sibling PRs have landed
2. THE implementation SHALL use existing crate seams and SHALL NOT create a new semantic crate
3. THE implementation SHALL NOT move parser-specific code into perl-semantic-facts
4. THE implementation SHALL NOT make providers parse imports locally; import parsing SHALL be centralized in the semantic layer
5. THE implementation SHALL add tests that fail before the change and pass after the change
6. THE implementation SHALL keep generated output deterministic
7. THE implementation SHALL run targeted tests and report any unavailable commands

### Requirement 26: Scope Identity

**User Story:** As a semantic producer developer, I want scope identity preserved on facts, so that lexical scoping and visibility queries can distinguish same-named symbols in different scopes.

#### Acceptance Criteria

1. THE semantic producers SHALL emit or preserve ScopeId for lexical declarations, imports, and occurrences where scope is known
2. WHEN scope is unknown, THE fact SHALL carry scope_id as None and confidence SHALL NOT be High unless the fact is globally scoped
3. THE ScopeId SHALL be deterministic for the same source location across re-analysis of unchanged content

### Requirement 27: Byte-Span to LSP Range Mapping

**User Story:** As a provider developer, I want a deterministic byte-span to LSP range mapping, so that AnchorFact byte offsets can be converted to LSP positions without forcing AnchorFact to store UTF-16 columns.

#### Acceptance Criteria

1. THE Workspace_Store SHALL provide a deterministic byte-span to LSP range mapping for every AnchorFact used by providers
2. THE AnchorFact SHALL NOT store UTF-16 column offsets; byte offsets SHALL be the canonical representation
3. THE mapping SHALL handle multi-byte UTF-8 characters correctly, producing valid UTF-16 offsets for the LSP protocol
