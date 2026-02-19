# tree-sitter-perl (highlight test corpus)

This directory contains **tree-sitter-style highlight test fixtures** for the
[perl-lsp](https://github.com/EffortlessMetrics/perl-lsp) workspace.
It is **not** a Rust crate -- there is no `Cargo.toml`. It holds only Perl
source files annotated with expected highlight scopes.

## Contents

```
crates/tree-sitter-perl/
  test/
    highlight/
      complex_constructs.pm   # Regex, heredocs, array slicing, special variables
      comprehensive.pm        # Scalars, arrays, hashes, control structures, subroutines
      debug.pm                # Minimal variable declaration (debug fixture)
      simple.pm               # Arithmetic and simple assignment
      working_examples.pm     # Variables, numbers, use-statements, hashes
```

## Purpose

The highlight test files are consumed by the **xtask highlight runner**
(`xtask/src/tasks/highlight.rs`). Each `.pm` file pairs Perl source lines with
comment annotations that declare expected tree-sitter capture names:

```perl
my $scalar = "hello world";
# <- keyword
#    ^ punctuation.special
#     ^ variable
#            ^ operator
#              ^ string
```

The runner parses each file with `tree-sitter-perl`, executes the workspace's
`queries/highlights.scm` query, and verifies that every annotated scope appears
in the actual captures.

## Running the tests

```bash
# From the workspace root
cd xtask && cargo run highlight

# Or specify the path explicitly
cd xtask && cargo run highlight -- --path ../crates/tree-sitter-perl/test/highlight
```

## Workspace role

| Aspect | Detail |
|--------|--------|
| Workspace member | No -- excluded from `Cargo.toml` workspace members |
| Consumed by | `xtask` highlight task (`xtask/src/tasks/highlight.rs`) |
| Default highlight path | `crates/tree-sitter-perl/test/highlight` (fallback in xtask) |
| `.gitattributes` | Marked `linguist-vendored` |

## License

MIT OR Apache-2.0
