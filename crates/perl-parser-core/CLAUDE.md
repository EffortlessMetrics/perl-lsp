# CLAUDE.md

## Crate Overview

- **Name**: `perl-parser-core`
- **Version**: 0.9.0
- **Tier**: 2 (aggregates Tier 1 leaf crates into the core parsing engine)
- **Purpose**: Recursive descent parser with IDE-friendly error recovery, AST construction, token stream utilities, and UTF-8/UTF-16 position mapping. Used by `perl-parser` and higher-level analysis/LSP crates.

## Commands

```bash
cargo build -p perl-parser-core            # Build
cargo test -p perl-parser-core             # Run tests
cargo clippy -p perl-parser-core           # Lint
cargo doc -p perl-parser-core --open       # View docs
```

## Architecture

### Dependencies (all workspace Tier 1 crates)

`perl-lexer`, `perl-token`, `perl-ast`, `perl-error`, `perl-position-tracking`, `perl-quote`, `perl-pragma`, `perl-edit`, `perl-builtins`, `perl-regex`, `perl-heredoc`, `perl-tokenizer`

### Key Types and Modules

| Type / Module | Location | Purpose |
|---------------|----------|---------|
| `Parser` | `engine/parser/mod.rs` | Main recursive descent parser with `parse()` and `parse_with_recovery()` |
| `ParserContext` | `engine/parser_context.rs` | Token-level context with budget-controlled error recovery |
| `RecoveryParser` | `engine/error/recovery_parser.rs` | Error-tolerant parser producing partial ASTs with error nodes |
| `Node`, `NodeKind`, `SourceLocation` | `engine/ast.rs` (re-exports `perl-ast`) | AST node types |
| `TokenStream`, `Token`, `TokenKind` | `tokens/token_stream.rs` (re-exports `perl-tokenizer`) | Buffered token stream with lookahead |
| `ParseError`, `ParseOutput`, `ParseResult` | `engine/error/mod.rs` (re-exports `perl-error`) | Error types and result wrappers |
| `PositionMapper`, `LineIndex` | `engine/position/mod.rs` (re-exports `perl-position-tracking`) | UTF-8/UTF-16 position conversion |
| `Trivia`, `TriviaPreservingParser` | `tokens/mod.rs` (re-exports `perl-tokenizer`) | Whitespace/comment preservation |
| `BudgetTracker`, `ParseBudget` | via `perl-error` | Resource limits for error recovery |

### Module Layout

- `lib.rs` -- public API surface, re-exports from submodules
- `engine/` -- parser logic: `parser/` (recursive descent + helpers via `include!`), `error/` (recovery), `ast.rs`, `parser_context.rs`, `position/`
- `tokens/` -- token stream and trivia facades over `perl-tokenizer`

### Parser Design

The parser in `engine/parser/mod.rs` uses `include!` macros to compose parsing logic from separate files: `helpers.rs`, `heredoc.rs`, `statements.rs`, `variables.rs`, `control_flow.rs`, `declarations.rs`, and `expressions/*.rs`. All included files are compiled as part of the `Parser` impl block.

Error recovery returns `Ok(ast)` with ERROR nodes for most failures; `Err` is reserved for catastrophic conditions (recursion limit). This enables IDE features on incomplete code.

## Usage Examples

```rust
use perl_parser_core::Parser;

// Basic parse
let mut parser = Parser::new("my $x = 42;");
let ast = parser.parse()?;

// Parse with recovery (preferred for LSP)
let mut parser = Parser::new("my $x = ;");
let output = parser.parse_with_recovery();
// output.ast always available; output.diagnostics contains errors
```

## Important Notes

- Prefer `perl-parser` for end-user usage; this crate is the internal engine
- `Parser` struct has a recursion depth limit of 128 to prevent stack overflow
- `ParserContext` uses `ParseBudget` to cap errors and nesting depth
- Changes to this crate affect all higher-level crates in the workspace
- Doctests are disabled (`doctest = false` in Cargo.toml)
