# perl-refactoring

Refactoring and modernization utilities for Perl code, part of the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Features

- **Import Optimization** -- detect unused/duplicate imports, generate optimized `use` statements (`ImportOptimizer`)
- **Code Modernization** -- suggest modern Perl idioms (lexical filehandles, three-argument `open`, `say`, `strict`/`warnings`) via `PerlModernizer`
- **Unified Refactoring Engine** -- `RefactoringEngine` coordinates rename, modernize, and import-optimize operations with backup/rollback support
- **Workspace Refactoring** -- cross-file rename, extract module, move subroutine, inline variable (`WorkspaceRefactor`)
- **Workspace Rename** -- scope-aware, atomic symbol rename across an entire workspace with progress reporting (`WorkspaceRename`)

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `workspace_refactor` | yes | Workspace-wide refactoring via `WorkspaceIndex` |
| `modernize` | no | Code modernization transforms in `RefactoringEngine` |

## Usage

```rust
use perl_refactoring::import_optimizer::ImportOptimizer;

let optimizer = ImportOptimizer::new();
let analysis = optimizer.analyze_content("use Carp qw(croak); print 1;")?;
let optimized = optimizer.generate_optimized_imports(&analysis);
```

## License

Licensed under either of MIT or Apache-2.0 at your option.
