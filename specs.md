# Specification: Import/Export Gap — Exporter Symbol Resolution

## Feature Description

This specification addresses the gap where perl-lsp does not analyze the content of `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS` arrays/hashes in modules that inherit from Exporter. Without this analysis, the LSP cannot resolve which symbols a module exports, causing go-to-definition and completion to fail for default (unqualified) imports.

## Feature Behavior

### 1. Export Symbol Extraction

When the semantic analyzer visits a file that uses Exporter, it extracts exported symbols:

- **Detection**: A module is considered an Exporter if the AST contains any of:
  - `use Exporter 'import';` statement
  - `use parent 'Exporter';` statement
  - `our @ISA = qw(Exporter);` assignment

- **Extraction**: For confirmed Exporter modules, parse:
  - `@EXPORT = qw(foo bar)` — default exports
  - `@EXPORT_OK = qw(baz qux)` — optional exports
  - `%EXPORT_TAGS = (tag1 => [qw(a b)], tag2 => [qw(c d)])` — tag-based exports

- **QW delimiter handling**: Support all Perl qw delimiters: `()`, `[]`, `{}`, `<>`, `//`, `||`

### 2. Export Table Storage

The workspace index maintains an export table:

- **Per-file (`FileIndex`)**: `exports`, `optional_exports`, `export_tags`
- **Per-workspace (`WorkspaceIndex`)**: `export_table: HashMap<String, HashSet<String>>` mapping `Module::Name` → set of exported bare symbol names
- **Early-exit**: Files that don't use Exporter are not analyzed for exports

### 3. Go-to-Definition Enhancement

When resolving a bare symbol (e.g., `func()`) that is not found in the current package:

1. Query the export table: "which module in scope exports this symbol?"
2. If exactly one module exports it, return the definition location from that module
3. If multiple modules export it, use import order to disambiguate (most recent `use` wins)
4. If no module exports it, fall back to existing behavior (undefined symbol)

### 4. Completion Enhancement

When providing completions in a file that uses a module:

- `use Module;` (no args): Include all `@EXPORT` symbols from that module
- `use Module qw(:tag);`: Include symbols from the specified export tag
- `use Module qw(foo bar);`: Existing behavior (only explicitly named symbols)

## Acceptance Criteria

### AC1: Export Extraction
- **Given** a file containing `use Exporter 'import'; our @EXPORT = qw(foo bar);`
- **When** the file is indexed
- **Then** the workspace index contains `Module::Name::foo` and `Module::Name::bar` in the export table

### AC2: Go-to-Definition for Default Exports
- **Given** module `My::Loader` that exports `load_data` via `@EXPORT`
- **And** a file containing `use My::Loader; load_data();`
- **When** the user triggers go-to-definition on `load_data`
- **Then** the LSP navigates to the `sub load_data` definition in `My/Loader.pm`

### AC3: Completion for Default Exports
- **Given** module `My::Utils` that exports `process` via `@EXPORT`
- **And** a file containing `use My::Utils;`
- **When** the user triggers completion after typing `proc`
- **Then** `process` appears in the completion list

### AC4: Export Tag Resolution
- **Given** module `My::Module` with `%EXPORT_TAGS = (ops => [qw(add subtract)])`
- **And** a file containing `use My::Module qw(:ops);`
- **When** the user triggers completion
- **Then** `add` and `subtract` are included in the completion list

### AC5: No False Positives for Non-Exporter Files
- **Given** a file that defines `@EXPORT = qw(foo)` but does NOT use Exporter
- **When** the file is indexed
- **Then** no symbols are added to the export table for that file

### AC6: Symbol Collision Resolution
- **Given** module A exports `helper` and module B exports `helper`
- **And** a file contains `use A; use B; helper();`
- **When** go-to-definition is triggered on `helper`
- **Then** the definition from B (most recently imported) is returned

## Non-Goals

- Runtime export modifications (`push @EXPORT, 'symbol'`)
- Symbolic references in export arrays
- External CPAN modules not in workspace
- `use base 'Exporter'` legacy pattern
- Rename refactoring for exported symbols

## Dependencies

- Parser must correctly parse `qw(...)` expressions in array/hash assignments (verified working)
- `find_symbol_key_definition_location` must be enhanced to query export table (not `symbol_at_cursor`)
- CompletionProvider already has `workspace_index: Option<Arc<WorkspaceIndex>>` access

## Test Coverage Requirements

1. Unit tests for `ExportSymbolExtractor` with various qw delimiters
2. Unit tests for Exporter inheritance detection (three patterns)
3. Integration tests for cross-module symbol resolution via Exporter
4. Completion tests for default export symbols
5. Edge case tests: circular dependencies, namespace collisions, multiple packages in one file
