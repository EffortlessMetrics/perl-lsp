# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-lsp-completion` is a **Tier 2 LSP feature crate** (single-level internal dependencies) providing context-aware code completion for Perl.

**Purpose**: Generates ranked completion items from parsed ASTs, semantic symbols, and workspace indexes for the `textDocument/completion` LSP request.

**Version**: 0.9.1

## Commands

```bash
cargo build -p perl-lsp-completion       # Build
cargo test -p perl-lsp-completion        # Run tests
cargo clippy -p perl-lsp-completion      # Lint
cargo doc -p perl-lsp-completion --open  # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` -- AST types (`Node`, `SourceLocation`, `Parser`)
- `perl-semantic-analyzer` -- `SymbolExtractor`, `SymbolTable`, `SymbolKind`, `ScopeKind`
- `perl-workspace-index` -- `WorkspaceIndex` for cross-file symbol lookup
- `lsp-types` (optional, behind `lsp-compat` feature) -- LSP type compatibility
- `walkdir` -- secure filesystem traversal for file-path completion
- `serde`, `thiserror`, `url` -- serialization, errors, URI handling

### Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `lsp-compat` | yes | Enables `lsp-types` dependency for LSP wire-type conversion |

### Key Types (public)

| Type | Module | Role |
|------|--------|------|
| `CompletionProvider` | `completion` | Main entry point; wraps `SymbolTable` + optional `WorkspaceIndex` |
| `CompletionContext` | `completion::context` | Request-scoped context (position, trigger char, prefix, package scope) |
| `CompletionItem` | `completion::items` | Single completion suggestion with label, kind, insert text, sort/filter text |
| `CompletionItemKind` | `completion::items` | Enum: Variable, Function, Keyword, Module, File, Snippet, Constant, Property |

### Internal Modules

| Module | Purpose |
|--------|---------|
| `builtins` | 130+ Perl built-in function completions with signatures |
| `variables` | Scalar/array/hash completion from symbol table + special variables |
| `functions` | User-defined subroutine completion |
| `keywords` | Perl keyword completion with snippet expansion |
| `methods` | Method completion after `->`, with DBI type inference |
| `packages` | Package member completion after `::` via workspace index |
| `workspace` | Cross-file symbol completion from workspace index |
| `test_more` | Test::More/Test2::V0 function completions in test contexts |
| `file_path` | Secure file-path completion inside string literals |
| `sort` | Deduplication and deterministic sort of completion results |
| `context` | Context analysis (package detection, prefix extraction) |

### Provider Methods

```text
CompletionProvider::new(&ast)                                    -- local-only
CompletionProvider::new_with_index(&ast, Some(index))            -- with workspace
CompletionProvider::new_with_index_and_source(&ast, src, index)  -- full context

provider.get_completions(source, position)                       -- basic
provider.get_completions_with_path(source, pos, filepath)        -- path-aware
provider.get_completions_with_path_cancellable(src, pos, path, &cancel_fn) -- cancellable
```

## Usage

```rust
use perl_parser_core::Parser;
use perl_lsp_completion::CompletionProvider;

let source = "my $count = 42;\n$c";
let mut parser = Parser::new(source);
let ast = parser.parse()?;

let provider = CompletionProvider::new(&ast);
let completions = provider.get_completions(source, source.len());
// completions contains $count suggestion
```

## Important Notes

- Completions are context-sensitive: sigil-prefixed triggers dispatch to specific completion paths; `->` triggers method completion; `::` triggers package member completion.
- The `is_cancelled` callback is checked at multiple points during completion to support LSP cancellation.
- File-path completion runs only on non-wasm32 targets and implements defense-in-depth security (path traversal prevention, null byte rejection, Windows reserved name filtering, controlled traversal depth).
- Moo/Moose `has(...)` option-key completion is detected via a dedicated heuristic (`is_has_options_key_context`).
- DBI method inference uses variable naming conventions (`$dbh` -> `DBI::db`, `$sth` -> `DBI::st`) and assignment context analysis.
- Results are deduplicated by label and sorted deterministically (sort_text -> kind -> label).
