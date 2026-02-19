# CLAUDE.md

This file provides guidance to Claude Code when working with this directory.

## Directory Overview

`crates/tree-sitter-perl/` is a **non-Rust data directory** containing tree-sitter
highlight test fixtures for Perl. It has no `Cargo.toml` and is not a Rust crate.

**Purpose**: Provide annotated Perl source files that the xtask highlight runner
uses to validate tree-sitter highlight queries.

**publish**: N/A (not a crate)

## Contents

```
test/highlight/
  complex_constructs.pm   # Regex, heredocs, array slicing, special variables
  comprehensive.pm        # Scalars, arrays, hashes, control structures, subroutines
  debug.pm                # Minimal variable-declaration fixture
  simple.pm               # Arithmetic and simple assignment
  working_examples.pm     # Variables, numbers, use-statements, hashes
```

Each `.pm` file contains Perl source lines interleaved with comment annotations
that declare expected tree-sitter capture scopes:

```perl
my $scalar = "hello world";
# <- keyword
#    ^ punctuation.special
#     ^ variable
#            ^ operator
#              ^ string
```

## How the fixtures are consumed

The **xtask highlight task** (`xtask/src/tasks/highlight.rs`) is the only consumer:

1. It walks `crates/tree-sitter-perl/test/highlight/` for `.pm` files.
2. Each file is parsed into test cases (source + expected scopes).
3. Source is parsed with `tree-sitter-perl` and the `queries/highlights.scm` query.
4. Actual captures are compared against the annotated expectations.

### Running the tests

```bash
# From the workspace root
cd xtask && cargo run highlight

# With an explicit path
cd xtask && cargo run highlight -- --path ../crates/tree-sitter-perl/test/highlight
```

## Workspace integration

- **Not a workspace member** -- excluded from the Cargo workspace.
- **`.gitattributes`** marks the directory `linguist-vendored`.
- **`.trivyignore`** excludes `tree-sitter-perl/test/corpus/**` from security scans.
- The workspace `Cargo.toml` alias `tree-sitter-perl` resolves to `crates/tree-sitter-perl-rs`
  (the Rust bindings crate), not this directory.

## Adding new highlight tests

1. Create a new `.pm` file under `test/highlight/`.
2. Write Perl source lines followed by comment annotations using `# <- scope`
   (column 1) or `# ^ scope` (caret at the target column).
3. Run `cd xtask && cargo run highlight` to verify.

## Important notes

- This directory is **data only** -- no Rust source, no `Cargo.toml`, no binaries.
- Do not confuse with `crates/tree-sitter-perl-rs/` (the Rust crate) or
  `tree-sitter-perl/` (the vendored upstream C grammar at the repo root).
