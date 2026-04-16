# Specification: Rose::DB::Object IDE Support — work-cb980638

## Feature Description

Add LSP support for Rose::DB::Object, a popular Perl ORM. The feature provides:
- **Recognition**: Detection of classes inheriting from Rose::DB::Object via `use base qw(Rose::DB::Object)`
- **Completion**: Method completion for auto-generated column accessors (e.g., `id()`, `name()`, `email()`)
- **Navigation**: Go-to-definition from column accessor usage to the column definition in `meta->setup(...)`

## Behavior Specification

### Detection

When the semantic analyzer encounters a `use base qw(... Rose::DB::Object ...)` statement:
1. The package's `Framework` is set to `Framework::RoseDBObject`
2. If the file also contains `__PACKAGE__->meta->setup(...)` calls, column information is extracted

### meta->setup Extraction

The analyzer extracts column information from `__PACKAGE__->meta->setup(...)` calls:

```perl
__PACKAGE__->meta->setup(
    table => 'users',
    columns => [qw(id name email status)],
    primary_key_columns => ['id'],
);
```

Extracted information:
- `table` name (stored for reference)
- `columns` array values → synthesized accessor names
- `primary_key_columns` array values (marked as primary keys)

### Completion Behavior

When the user types `$obj->` where `$obj` is typed as a Rose::DB::Object subclass:
1. Standard method completions appear (inherited from Rose::DB::Object)
2. Column accessor completions appear: `id()`, `name()`, `email()`, `status()`
3. Each completion item shows documentation: "Column accessor (Rose::DB::Object)"
4. Synthesized methods are marked with `declaration = "meta->setup"` in the symbol table

### Navigation Behavior

| User action | Navigates to |
|-------------|--------------|
| Go-to-definition on `id()` (where `id` is a Rose::DB::Object column accessor) | The `meta->setup(...)` call that defines the `id` column |
| Go-to-definition on `meta->setup` identifier | The `__PACKAGE__->meta->setup(...)` method call |
| Go-to-definition on column name in `meta->setup(...)` | The column name in the `columns => [...]` array |

### Limitations (Initial Scope)

- Only `qw()` form of `columns => [qw(id name email)]` is extracted
- Variable references (`columns => $array`) are not resolved
- Custom accessor names (`accessor => 'custom_name'`) are not supported
- Relationships (`one_to_many`, `many_to_many`) are not extracted
- Rose::DB::Object::Manager patterns are not supported
- Cross-file schema resolution relies on workspace index

## Acceptance Criteria

### AC1: Framework Detection

**Given** a Perl file containing `package MyApp::User; use base qw(Rose::DB::Object);`
**When** the semantic analyzer processes the file
**Then** the package is classified as `Framework::RoseDBObject`

### AC2: Column Accessor Completion

**Given** a Rose::DB::Object subclass with `columns => [qw(id name email)]`
**When** the user types `$user->` and triggers completion
**Then** completion items include: `id()`, `name()`, `email()` with documentation "Column accessor (Rose::DB::Object)"

### AC3: Navigation to meta->setup

**Given** a Rose::DB::Object subclass with `columns => [qw(id name email)]`
**When** the user performs go-to-definition on `id()` (where `id` is a column accessor)
**Then** the cursor navigates to the `meta->setup(...)` call that defines the `id` column

### AC4: meta->setup Extraction

**Given** `__PACKAGE__->meta->setup(columns => [qw(id name email)])`
**When** the semantic analyzer processes the file
**Then** synthesized symbols are created for `id()`, `name()`, and `email()` with `declaration = "meta->setup"`

### AC5: Framework Enum Documentation

**When** `Framework::RoseDBObject` is added to the enum
**Then** the variant doc comment explicitly states it represents "runtime schema conformance" rather than a "method-declaration framework"

### AC6: Test Suite

**When** `cargo test -p perl-semantic-analyzer` is run
**Then** all existing tests pass, plus new tests for Rose::DB::Object detection and extraction

**When** `cargo test -p perl-lsp-completion` is run
**Then** all existing tests pass, plus new tests for column accessor completion

## Non-Goals

This specification does NOT include:
- Relationship navigation (one_to_many, many_to_many)
- Rose::DB::Object::Manager query pattern completion
- Type inference for query results
- SQL completion within `where` clauses
- Support for `accessor => 'custom_name'` overrides
- Resolution of variable references in `columns => $array`

## Dependencies

1. **perl-semantic-analyzer**: Framework enum, detection, extraction, symbol synthesis
2. **perl-lsp-completion**: Method completion inference
3. **perl-lsp-navigation**: Go-to-definition support
4. **perl-workspace-index**: Cross-file parent chain resolution (for completion)

## File Changes

| File | Change |
|------|--------|
| `crates/perl-semantic-analyzer/src/analysis/class_model.rs` | Add `RoseDBObject` variant, detection, extraction |
| `crates/perl-semantic-analyzer/src/analysis/symbol.rs` | Add framework flags, symbol synthesis |
| `crates/perl-lsp-completion/src/completion/methods.rs` | Add completion inference |
| `crates/perl-lsp-navigation/src/` | Add navigation support |
| `crates/perl-corpus/` | Add test fixtures |

## Test Corpus Example

```perl
# test_corpus/rose_db_object/basic_user.pl
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    table => 'users',
    columns => [qw(id name email status)],
    primary_key_columns => ['id'],
);

1;
```

Expected behaviors:
- Framework detected as RoseDBObject
- Symbols created for `id()`, `name()`, `email()`, `status()`
- Completion on `$user->` includes column accessors
- Go-to-definition on `id()` navigates to meta->setup call