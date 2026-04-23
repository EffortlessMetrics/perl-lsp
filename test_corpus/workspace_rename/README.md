# Workspace Rename Test Corpus

Test corpus for workspace-wide rename refactoring feature (#433).

## Structure

```
workspace_rename/
├── basic/              # Simple single-file and multi-file renames
├── scoping/            # Package and lexical scope tests
├── edge_cases/         # Circular deps, unicode, special vars
└── performance/        # Large workspace benchmarks
```

## Test Categories

### Basic Tests
- Single file variable rename
- Multi-file subroutine rename
- Qualified vs bare references

### Scoping Tests
- Package-scoped symbols
- Lexical scope boundaries
- Symbol shadowing scenarios

### Edge Cases
- Circular module dependencies
- Unicode identifiers
- Special Perl variables ($_, @_, etc.)

### Performance Tests
- 100+ file workspaces
- Symbol with 1000+ references
- Parallel processing verification

## Test Naming Convention

Files use descriptive names:
- `single_file_var_rename.pl` - Single file variable rename
- `multi_file_qualified.pl` - Multi-file with qualified references
- `package_scope_conflict.pl` - Package scope conflict detection
- `circular_deps_a.pm` - Circular dependency scenario (file A)

## Expected Results

Each test directory contains an `expected/` subdirectory with files showing
the expected state after rename operation.

## Usage

Tests are consumed by:
- `cargo test -p perl-refactoring` - Integration tests
- `cargo bench -p perl-refactoring` - Performance benchmarks

## Maintenance

When adding new tests:
1. Create input file in appropriate category directory
2. Create expected output in `expected/` subdirectory
3. Add test case in `crates/perl-refactoring/tests/workspace_rename_tests.rs`
4. Tag test with `// AC:ACx` for acceptance criteria mapping
