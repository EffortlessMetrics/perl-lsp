# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-refactoring` is a **Tier 3 mid-tier crate** providing refactoring and modernization utilities for Perl.

**Purpose**: Refactoring and modernization utilities for Perl — provides automated code transformations like rename, extract, and modernization fixes.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-refactoring          # Build this crate
cargo test -p perl-refactoring           # Run tests
cargo clippy -p perl-refactoring         # Lint
cargo doc -p perl-refactoring --open     # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST access
- `perl-workspace-index` - Cross-file operations

### Features

| Feature | Purpose |
|---------|---------|
| `workspace_refactor` | Workspace-wide refactoring (default) |
| `modernize` | Code modernization transforms |

### Main Modules

Located in `refactor/` subdirectory:

| Module | Purpose |
|--------|---------|
| `rename.rs` | Variable/subroutine renaming |
| `extract.rs` | Extract variable/subroutine |
| `inline.rs` | Inline variable/subroutine |
| `modernize.rs` | Modernization transforms |

### Refactoring Operations

| Operation | Description |
|-----------|-------------|
| **Rename** | Rename variable, subroutine, or package |
| **Extract Variable** | Extract expression to variable |
| **Extract Subroutine** | Extract code block to subroutine |
| **Inline Variable** | Inline variable usage |
| **Modernize** | Apply modern Perl idioms |

### Modernization Transforms

```perl
# Before → After

# Use say instead of print with newline
print "hello\n";  →  say "hello";

# Use // instead of defined-or
defined $x ? $x : $default  →  $x // $default

# Use state instead of persistent my
my $count; BEGIN { $count = 0 }  →  state $count = 0;
```

## Usage

```rust
use perl_refactoring::{Refactor, RenameOptions};

// Rename a symbol
let edits = Refactor::rename(
    workspace,
    "old_name",
    "new_name",
    RenameOptions::default(),
)?;

// Extract variable
let edits = Refactor::extract_variable(
    document,
    selection,
    "new_var_name",
)?;

// Modernize file
let edits = Refactor::modernize(document)?;
```

## Important Notes

- Refactoring produces `TextEdit` collections
- Workspace-wide operations use `perl-workspace-index`
- Modernization respects minimum Perl version
- All operations preserve semantics
