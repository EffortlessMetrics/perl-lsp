# Specs — Cross-Module Export Symbol Resolution

## Feature Description

When a Perl file uses a module that exports subroutines (via `use Module;`), go-to-definition on a bareword subroutine call (e.g., `foo()`) in the consumer file should navigate to the exporter module's subroutine definition, not fail or navigate to a local stub.

This requires:
1. Extracting exported symbol names from `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS` in exporter modules
2. Building a workspace-wide export table mapping `(module, symbol) → location`
3. Resolving bareword subroutine calls in consumer files against the export table when local resolution fails

## Non-Goals

- Dynamic `import()` calls (e.g., `$module->import(@symbols)` at runtime) — static analysis cannot resolve these
- `Sub::Exporter` complex export graphs — only standard `Exporter` is in scope
- Resolving imported variables (`our @EXPORT = ...`) as variable definitions — only subroutine (bareword) resolution is implemented
- Ambiguity resolution when multiple modules export the same symbol — degrades gracefully by not resolving

## Dependencies

- `perl-semantic-analyzer` — contains `SymbolExtractor`, `ClassModelBuilder`, `collect_symbol_names()`
- `perl-workspace-index` — `WorkspaceIndex`, `FileIndex`, `SymbolKey`
- `perl-lsp` navigation layer — `navigation.rs:handle_definition()`
- No parser changes needed — `VariableListDeclaration` with `ArrayLiteral` initializer already parses correctly

## Acceptance Criteria

### AC1: Export Extraction
Given a file `MyModule.pm` containing `our @EXPORT = qw(foo bar);` and `sub foo { ... }`, the workspace index must contain export entries for `foo` and `bar` associated with module `MyModule`.

**Test**: Parse `MyModule.pm` and verify `workspace_index.find_export("MyModule", "foo")` returns `Some(location)` where `location` points to the `sub foo` definition.

### AC2: Cross-Module Go-to-Definition
Given two files:
- `MyModule.pm`: `our @EXPORT = qw(foo); sub foo { ... }`
- `Consumer.pm`: `use MyModule; foo();`

When the cursor is on `foo` in `Consumer.pm` and go-to-definition is invoked, the LSP returns a `LocationLink` pointing to `sub foo` in `MyModule.pm`.

**Test**: Open `Consumer.pm`, invoke go-to-definition on `foo`, verify response contains location in `MyModule.pm`.

### AC3: `use Module ()` Does Not Trigger Export Resolution
Given:
- `MyModule.pm`: `our @EXPORT = qw(foo);`
- `Consumer.pm`: `use MyModule (); foo();`

When the cursor is on `foo` in `Consumer.pm` and go-to-definition is invoked, export resolution must NOT fire (because `()` means "import nothing").

**Test**: Same setup as AC2 but with `use MyModule ()`. Verify no export-based location is returned.

### AC4: `%EXPORT_TAGS` Support
Given a file containing:
```perl
our %EXPORT_TAGS = (
    all => [qw(foo bar baz)],
);
our @EXPORT_OK = qw(foo bar);
```
The export table must contain entries for `foo` and `bar` (members of `:all` tag and also in `@EXPORT_OK`).

**Test**: Parse the file, verify `workspace_index.get_exports_for_module("CurrentModule")` returns entries for `foo` and `bar` with appropriate `ExportKind`.

## Implementation Notes

### Architecture: Bridge, Not New Extraction
`ClassModelBuilder` (class_model.rs) already extracts `@EXPORT` and `@EXPORT_OK` via `collect_symbol_names()`. The implementation should reuse this helper. If bridging `ClassModel::exports` into the workspace index is too invasive, export extraction may be added to `IndexVisitor` — but `collect_symbol_names()` must not be duplicated.

### Key Code Locations
- `perl-semantic-analyzer/src/analysis/class_model.rs:369–374` — existing export extraction
- `perl-semantic-analyzer/src/analysis/class_model.rs:1155` — `collect_symbol_names()` helper
- `perl-workspace-index/src/workspace/workspace_index.rs` — `WorkspaceIndex::index_file()`
- `perl-lsp/src/runtime/language/navigation.rs:887` — `handle_definition()` (correct layer for Phase 3)
- `perl-semantic-analyzer/src/analysis/declaration.rs:1404–1409` — gap acknowledgment comment

### Thread Safety
`ExportTable` must use `parking_lot::RwLock` consistent with `WorkspaceIndex` locking model.

### Performance
Export extraction is O(exported_symbols) per file. Most modules have < 50 exported symbols. Incremental indexing impact must remain < 1ms per file.
