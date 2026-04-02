# perl-refactoring

Refactoring and code-modernization utilities for Perl source.

Use this crate when you have parsed source plus analysis context and need to
rewrite code, not just inspect it.

## Where it fits

`perl-refactoring` sits above the parser and workspace index. It consumes AST
and symbol information, then performs import cleanup, modernization, rename,
and workspace-wide transformations.

## Key entry points

- `import_optimizer::ImportOptimizer`
- `modernize::PerlModernizer`
- `refactoring::RefactoringEngine`
- `workspace_refactor::WorkspaceRefactor`
- `workspace_rename::WorkspaceRename`

## Cargo features

- `workspace_refactor` enables workspace-wide refactoring via `WorkspaceIndex`
- `modernize` enables modernization transforms in `RefactoringEngine`

## Example

```rust
use perl_refactoring::import_optimizer::ImportOptimizer;

let optimizer = ImportOptimizer::new();
let analysis = optimizer.analyze_content("use Carp qw(croak); print 1;")?;
let _optimized = optimizer.generate_optimized_imports(&analysis);
```

## Typical use

Use `perl-refactoring` when you need to migrate code, remove unused imports, or
perform cross-file symbol edits. If you only need symbol analysis or workspace
lookup, use the lower-level crates directly.
