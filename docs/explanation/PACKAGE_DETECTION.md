# Package Detection

## Overview

We resolve the current package at an offset by scanning the AST for the nearest `package NAME;` where `node.start <= offset`. We also honor reset to the default package via `no package;` (treat subsequent nodes as `main` until another `package` declaration appears). This context feeds definition, references, and rename.

## Implementation

The package detection system is implemented in `crates/perl-parser/src/lsp/workspace_index.rs` and provides context-aware resolution for:

- **Go to Definition**: Determines the correct package context for symbol resolution
- **Find References**: Scopes reference searches to the appropriate package
- **Rename Symbol**: Ensures refactoring respects package boundaries

## Package Scoping Rules

1. **Default Package**: Code before any `package` declaration belongs to `main`
2. **Package Declaration**: `package Foo;` sets the current package to `Foo`
3. **Package Reset**: `no package;` or `package;` resets to `main`
4. **Block Scoped Packages**: `package Foo { ... }` limits the package to the block
5. **Lexical Scoping**: Package changes respect lexical scope boundaries

## Examples

```perl
# Default package is 'main'
my $x = 1;  # main::x

package Foo;
my $y = 2;  # Foo::y

no package;
my $z = 3;  # main::z

package Bar {
    my $a = 4;  # Bar::a
}
my $b = 5;  # main::b (block exited)
```

## AST Traversal

The implementation uses an efficient AST traversal that:
1. Collects all package declarations in document order
2. Builds a sorted index by position
3. Uses binary search for O(log n) lookups
4. Caches results for repeated queries

## Integration Points

- **LSP Server**: Calls `package_of(&doc, offset)` for context
- **Symbol Resolution**: Uses package context for qualified names
- **Refactoring**: Respects package boundaries during rename operations