# ADR-0017: Cross-Module Export Symbol Resolution

## Status
Proposed

## Context

GitHub issue #3416 describes a gap in perl-lsp's cross-module navigation: the LSP parses `@EXPORT` and `@EXPORT_OK` declarations to suppress unused-variable warnings but does **not** extract symbol names from those arrays, build a workspace-wide export symbol table, or resolve consumer-side `use Module;` imports to exported subroutine definitions. The result is that Ctrl+Click on a bareword subroutine call (e.g., `foo()`) in a consumer file does not jump to the exporter module's definition.

### Prior Artifacts Summary

**Verification agent** identified that export extraction already exists in `ClassModelBuilder` (class_model.rs:369–374) via `collect_symbol_names()`. The `ClassModel` struct has `exports: Vec<String>` and `export_ok: Vec<String>` fields with working tests (lines 2027–2060). The gap is that `ClassModel` is a completely separate extraction system from `SymbolTable` — both walk the same AST but share no data, and `ClassModel::exports` never reaches the workspace index.

**Plan reviewer** identified two critical corrections:
1. Phase 1 of the initial plan duplicates existing extraction — the task should bridge existing `ClassModel` data, not build extraction from scratch.
2. Phase 3 targets `DeclarationProvider` in `declaration.rs`, but `DeclarationProvider` has no `WorkspaceIndex` access. The actual go-to-definition entry point is `navigation.rs:handle_definition()` (line 887). For bareword `foo()` in `use MyModule; foo();`, `symbol_key.pkg` is the consumer's package, so workspace lookup never searches the exporter. Export resolution belongs in `navigation.rs` as a fallback step after local resolution fails.
3. `use Module ()` (explicit empty import) edge case is missing — export resolution must not trigger here.

**Maintainer vision agent** confirmed the issue is on the v0.12.4 roadmap and endorsed the three-phase direction with modifications.

## Decision

We will implement cross-module export symbol resolution in three phases, correcting the initial plan's two errors:

### Phase 1: Bridge `ClassModel` Exports into Workspace Index (not new extraction)

Instead of building new export extraction in `SymbolExtractor`, we will bridge the existing `ClassModelBuilder` export data into the workspace index:

1. Extend `ClassModelBuilder` to emit export data alongside its AST walk, so that when `our @EXPORT = qw(foo bar)` is encountered, the symbol names `foo` and `bar` are captured in a new `ExportInfo` structure attached to the `FileIndex`.
2. Alternatively (if architectural constraints prefer), add export extraction to `IndexVisitor` / `SymbolExtractor` — since the extraction logic is identical (`collect_symbol_names()` on the `ArrayLiteral` initializer) — and store results in `FileIndex::exports`.
3. Add `ExportEntry` struct: `{ module: String, symbol: String, location: Location, kind: ExportKind }` where `ExportKind ∈ { Explicit, Ok, Tag }`.
4. Add `ExportTable` to `WorkspaceIndex` aggregating `ExportEntry` records from all indexed files.

**Key insight**: The extraction logic in `ClassModelBuilder::collect_symbol_names()` (handling `String`, `Identifier`, and `ArrayLiteral` nodes from `qw()`) already exists and is tested. The implementation should reuse it, not duplicate it.

### Phase 2: Workspace Export Index (`perl-workspace-index`)

1. Add `ExportEntry` and `ExportKind` types in `perl-workspace-index`.
2. Add `FileIndex::exports: HashMap<String, ExportEntry>` keyed by symbol name.
3. Wire extraction into `WorkspaceIndex::index_file()` via the existing `IndexVisitor` (or via `ClassModelBuilder` output if that's the chosen bridge path).
4. Add `WorkspaceIndex::find_export(module, symbol) -> Option<Location>` method.
5. Add `WorkspaceIndex::get_exports_for_module(module) -> Vec<&ExportEntry>` method.
6. Handle `%EXPORT_TAGS` in addition to `@EXPORT`/`@EXPORT_OK` — extract tag names and their member symbols, storing entries for each member.

### Phase 3: Consumer Resolution in Navigation Layer (`perl-lsp`)

1. In `navigation.rs:handle_definition()`, after local resolution fails for bareword function calls, add a fallback step that:
   - Collects all `use Module;` statements in the current file (excluding `use Module ()` which means "import nothing").
   - For each module, calls `workspace_index.find_export(module, symbol_name)`.
   - If exactly one match is found, returns its location as a `LocationLink`.
   - If multiple modules export the same symbol, prefer local definitions or return an ambiguity indicator (out of scope for initial implementation — just skip export resolution in that case).
2. This is implemented in `handle_definition` (the layer that has `WorkspaceIndex` access), not in `DeclarationProvider` (which does not have `WorkspaceIndex` access).

## Consequences

### Benefits
- **Fixes the reported issue**: Ctrl+Click on bareword `foo()` in a consumer file that `use`s an exporter will jump to the exporter's definition.
- **Reuses existing extraction**: Leverages `ClassModelBuilder`'s `collect_symbol_names()` — no new parsing logic needed.
- **Incremental architecture**: Each phase is independently testable and does not break existing functionality.
- **Thread-safe**: `ExportTable` uses `parking_lot::RwLock` consistent with `WorkspaceIndex` model.

### Tradeoffs / Risks
- **Two extraction systems remain**: Even after bridging, `ClassModel` and `SymbolTable` remain partially separate. The bridge may need ongoing maintenance if one system changes.
- **`use Module ()` complexity**: Must correctly identify the empty-parens form to avoid false-positive resolution. Perl parsing of `use Module ()` vs `use Module qw(...)` requires correct AST handling — this must be verified with tests.
- **Multiple symbol matches**: When two modules export the same symbol, the fallback will not resolve (degrades gracefully).
- **Dynamic `import()` not supported**: Runtime `Exporter->export_items()` cannot be statically resolved — documented as a known limitation.
- **`Sub::Exporter` not supported**: Complex export graphs from `Sub::Exporter` are out of scope.

## Alternatives Considered

### Alternative 1: Extract directly in `SymbolExtractor` (initial plan Phase 1)
The initial plan proposed adding export extraction to `SymbolExtractor::visit_node()` via the `VariableListDeclaration` handler. **Rejected because**: `ClassModelBuilder` already implements this extraction using `collect_symbol_names()` with tests. Duplicating this logic creates two maintenance burdens. Instead, we bridge the existing extraction.

### Alternative 2: Implement export resolution in `DeclarationProvider` (initial plan Phase 3)
The initial plan targeted `DeclarationProvider::find_declaration()` in `declaration.rs`. **Rejected because**: `DeclarationProvider` has no access to `WorkspaceIndex` — it operates at the file level. The go-to-definition entry point with workspace context is `navigation.rs:handle_definition()`, which has the `WorkspaceIndex` available and is the correct layer for the fallback resolution step.

### Alternative 3: Unified `ClassModel` + `SymbolTable` extraction
Merge `ClassModelBuilder` and `SymbolExtractor` into a single AST walk that produces both class model and symbol table data simultaneously. **Rejected because**: This is a large refactoring that would touch many files and risk destabilizing existing working functionality. The bridge approach achieves the goal with targeted changes.
