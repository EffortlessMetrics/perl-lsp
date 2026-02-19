# CLAUDE.md — perl-parser

## Crate Overview

- **Tier**: 6 (application/composition crate)
- **Version**: 0.9.1
- **Purpose**: Central hub crate that aggregates and re-exports the core parser, semantic analyzer, workspace indexer, refactoring engine, TDD support, and all LSP provider crates into a single public API surface. Also provides the `perl-parse` CLI binary.

## Commands

```bash
cargo build -p perl-parser                # Build library
cargo build -p perl-parser --release      # Build optimized
cargo test -p perl-parser                 # Run all tests
cargo test -p perl-parser -- test_name    # Run a single test
cargo clippy -p perl-parser              # Lint
cargo doc -p perl-parser --no-deps       # Generate docs
cargo bench -p perl-parser               # Parser benchmarks
```

## Architecture

### Dependencies (re-export sources)

| Internal crate | Re-exported via |
|----------------|-----------------|
| `perl-parser-core` | `engine`, `tokens`, `builtins`, `util`, `line_index` |
| `perl-lexer` | direct (tokenization) |
| `perl-semantic-analyzer` | `analysis` (scope, type inference, symbols, dead code) |
| `perl-workspace-index` | `workspace` (cross-file indexing, document store, rename) |
| `perl-refactoring` | `refactor` (import optimizer, modernize, refactoring engine, workspace refactor) |
| `perl-tdd-support` | `tdd` (test generator, test runner, TDD workflow) |
| `perl-incremental-parsing` | `incremental` (feature-gated behind `incremental`) |
| `perl-lsp-providers` | `ide` module |
| `perl-lsp-code-actions` | `code_actions` module |
| `perl-lsp-completion` | `completion` module |
| `perl-lsp-diagnostics` | `diagnostics` module |
| `perl-lsp-inlay-hints` | `inlay_hints` module |
| `perl-lsp-navigation` | `references`, `document_links`, `type_definition`, `type_hierarchy`, `workspace_symbols`, `implementation_provider` modules |
| `perl-lsp-rename` | `rename` module |
| `perl-lsp-semantic-tokens` | `semantic_tokens` module |
| `perl-lsp-tooling` | `tooling` (perltidy, perl_critic, performance) |

### Key types and modules

- `Parser` — main recursive-descent parser (from `perl-parser-core`)
- `Node`, `NodeKind`, `SourceLocation` — AST types
- `ParseError`, `ParseResult` — error types
- `SemanticAnalyzer`, `SemanticModel`, `HoverInfo` — semantic analysis
- `ScopeAnalyzer`, `TypeInferenceEngine`, `SymbolTable` — analysis primitives
- `TestGenerator`, `TddWorkflow` — TDD support
- `RefactoringEngine`, `ImportOptimizer` — refactoring
- `PositionMapper` — byte/UTF-16 offset conversions for LSP
- `TokenStream`, `Token`, `TokenKind` — token layer
- `RecoveryParser` — error-recovery wrapper

### Source layout

```
src/
  lib.rs          # Re-exports and public API surface
  engine.rs       # Re-export from perl-parser-core::engine
  tokens.rs       # Re-export from perl-parser-core::tokens
  analysis.rs     # Re-export from perl-semantic-analyzer
  workspace.rs    # Re-export from perl-workspace-index + perl-refactoring
  refactor.rs     # Re-export from perl-refactoring
  tdd.rs          # Re-export from perl-tdd-support
  builtins.rs     # Re-export from perl-parser-core::builtins
  tooling.rs      # Re-export from perl-lsp-tooling (perltidy, critic, perf)
  ide.rs          # Re-export from perl-lsp-providers
  incremental.rs  # Re-export from perl-incremental-parsing (feature-gated)
  bin/perl-parse.rs  # CLI binary (feature: cli)
```

### Feature flags

| Feature | Effect |
|---------|--------|
| `workspace` (default) | Enables cross-file indexing via `perl-workspace-index` |
| `lsp-compat` (default) | Pulls in `lsp-types` and LSP compatibility shims |
| `cli` | Builds the `perl-parse` binary |
| `incremental` | Enables incremental parsing module |
| `workspace_refactor` | Workspace-wide refactoring capabilities |
| `modernize` | Code modernization transformations |

## Usage examples

```rust
use perl_parser::Parser;

// Parse Perl source
let mut parser = Parser::new("sub greet { print 'hello'; }");
let ast = parser.parse().expect("parse failed");
println!("{}", ast.to_sexp());

// Check for errors without failing
let errors = parser.errors();

// Semantic analysis
use perl_parser::SemanticAnalyzer;
let analyzer = SemanticAnalyzer::new();
let model = analyzer.analyze(&ast);
```

## Important notes

- This crate is a **composition layer**; almost all logic lives in the upstream microcrates. Edits to parser behaviour belong in `perl-parser-core`, semantic logic in `perl-semantic-analyzer`, etc.
- `#![deny(unsafe_code)]` and `#![warn(missing_docs)]` are enforced.
- The LSP server runtime has moved to the `perl-lsp` crate; this crate keeps engine and provider re-exports.
- `doctest = false` in `[lib]` — doc examples are validated through dedicated test files, not rustdoc.
- WASM target excludes `walkdir`, `dead_code_detector`, `workspace_refactor`, and `error_classifier`.
