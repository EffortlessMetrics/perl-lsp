# ADR-2025: Export Symbol Table for Exporter-Based Module Resolution

## Status
Proposed

## Context

The perl-lsp codebase has a documented gap in cross-module symbol resolution for modules using Perl's Exporter framework. When `Module->import()` is called with no arguments (the default export case), the declaration resolver at `declaration.rs:1404-1409` explicitly refuses to claim symbol ownership:

```rust
// `Module->import()` default import set is module-specific and may
// come from `@EXPORT` in another file.  We do not currently have
// a workspace export table in this lookup path, so stay
// conservative and do not claim symbol ownership here.
return false;
```

This conservative behavior causes:
- No go-to-definition for exported symbols from modules using `use Module;`
- No completion suggestions for default-exported functions
- Cross-module navigation breaks when modules export via Exporter

The system recognizes `@EXPORT`, `@EXPORT_OK`, `%EXPORT_TAGS` as special variables (per `is_special_variable` at `scope_analyzer.rs:1917-1918`) but never parses their content.

## Decision

We implement an **export symbol table** in the workspace index, with export table queries integrated into `find_symbol_key_definition_location` (which already has workspace index access) rather than threading the workspace index through `symbol_at_cursor`.

### Architecture

The solution has four phases:

**Phase 1: Export Symbol Extraction**
- Create `ExportSymbolExtractor` in `perl-semantic-analyzer/src/analysis/export_analyzer.rs`
- Detect Exporter inheritance via three AST patterns:
  1. `use Exporter 'import'` (Use node)
  2. `use parent 'Exporter'` (Use node with parent module)
  3. `our @ISA = qw(Exporter)` (VariableDeclaration with array literal)
- Parse `@EXPORT = qw(...)` and `@EXPORT_OK = qw(...)` array assignments
- Parse `%EXPORT_TAGS = (...)` hash assignments
- Return `ExportInfo { default_export, optional_export, export_tags }`

**Phase 2: Workspace Index Extension**
- Extend `FileIndex` with:
  - `exports: HashSet<String>` — symbols exported via `@EXPORT`
  - `optional_exports: HashSet<String>` — symbols exported via `@EXPORT_OK`
  - `export_tags: HashMap<String, Vec<String>>` — tag → symbols mapping
- Extend `WorkspaceIndex` with:
  - `export_table: HashMap<String, HashSet<String>>` — module → exported symbols
  - `is_exported(module, symbol) -> bool` — O(1) lookup
  - `get_export_tags(module, tag) -> Option<Vec<String>>`
- Early-exit: skip export extraction for non-Exporter files (no false positives)

**Phase 3: Declaration Resolution Update (REFRAMED)**
- Enhance `find_symbol_key_definition_location` (NOT `symbol_at_cursor`):
  - When local symbol resolution fails for a bare symbol
  - AND the symbol's package matches `current_pkg`
  - Query the export table: "which module in scope exports this symbol?"
  - If found, return the definition location from that module
- Rationale: `find_symbol_key_definition_location` already has workspace index access; `symbol_at_cursor` has too many call sites across multiple crates

**Phase 4: Completion Enhancement**
- Extend `CompletionProvider` to include default exports when `use Module` has no args
- Resolve `:tag` imports via `export_tags` lookup
- Completion already has `workspace_index: Option<Arc<WorkspaceIndex>>` — no new context threading needed

### Symbol Collision Resolution

When multiple modules in the workspace export the same symbol name:
1. **Import order**: Use the most recent `use Module;` statement (tracked via `ImportMap` entry presence as a proxy for order)
2. **Fallback**: Shortest module name wins (deterministic tiebreaker)

This avoids non-deterministic "first match" behavior from the existing dual-indexing strategy.

## Consequences

### Benefits
- Go-to-definition works for `use MyModule; func()` → finds `sub func` in `MyModule.pm`
- Completion suggests default-exported functions when using a module
- Export tag resolution (`:tag` imports)
- Foundation for future `use base 'Exporter'` support
- Follows existing dual-indexing architecture pattern

### Tradeoffs / Risks
1. **Memory overhead**: `exports`, `optional_exports`, `export_tags` per `FileIndex` increases memory footprint
2. **Incremental update atomicity**: Export table must update atomically with symbol table
3. **False positives possible**: Export extraction runs on any file with `@EXPORT` unless Exporter inheritance is confirmed first (mitigated by early-exit check)
4. **Static analysis only**: Runtime `push @EXPORT, ...` modifications cannot be handled (documented limitation)

### Out of Scope
- Runtime export modifications (`push @EXPORT, ...`)
- Symbolic references in export arrays
- External CPAN modules (not in workspace)
- `use base 'Exporter'` legacy pattern
- Rename refactoring of exported symbols

## Alternatives Considered

### Alternative 1: Pass WorkspaceIndex to `symbol_at_cursor`
- **Rejected**: `symbol_at_cursor` is called from `navigation.rs:1312`, `misc.rs:851`, `references.rs:64`, and multiple test files. Changing its signature requires updating all call sites and creating workspace-index-aware context in places that don't have it. Blast radius is too high.

### Alternative 2: Query Export Table in `find_import_source`
- **Rejected**: The plan review identified that `find_import_source` returns `false` for no-args `Module->import()` *before* the package is known. The export table query needs to happen *after* `symbol_at_cursor` returns the wrong package, which is exactly when `find_symbol_key_definition_location` is called.

### Alternative 3: Name-Based Exporter Detection Only
- **Rejected**: Using `is_special_variable("@EXPORT")` alone would cause false positives — any module with `@EXPORT` would be analyzed regardless of whether it inherits from Exporter. The three-pattern detection (Use node, parent, @ISA) is required.

## References
- Issue: [#3409 - Import/Export Gap: Exporter 'import' pattern not analyzed for symbol resolution](https://github.com/EffortlessMetrics/perl-lsp/issues/3409)
- Conservative behavior documented at `declaration.rs:1404-1409`
- Dual-indexing strategy: PR #122
- v0.12.4 semantic framework coverage: #3077, PR #3098
- Completion ImportMap gap documented at `completion.rs:127-133`
