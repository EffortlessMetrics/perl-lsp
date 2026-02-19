# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

- **Name**: `perl-lsp-inlay-hints`
- **Version**: 0.9.0 (workspace)
- **Tier**: LSP feature crate (depends on Tier 4 `perl-semantic-analyzer`)
- **Purpose**: Generates LSP inlay hints for Perl -- parameter name hints on built-in function calls and lightweight type hints on literals.

## Commands

```bash
cargo build -p perl-lsp-inlay-hints        # Build
cargo test -p perl-lsp-inlay-hints         # Run tests
cargo clippy -p perl-lsp-inlay-hints       # Lint
cargo doc -p perl-lsp-inlay-hints --open   # View docs
```

## Architecture

### Dependencies

| Crate | Used For |
|-------|----------|
| `perl-parser-core` | AST types (`Node`, `NodeKind`) |
| `perl-position-tracking` | `WirePosition`, `WireRange` |
| `perl-semantic-analyzer` | `get_node_children` for AST traversal |
| `serde_json` | Intermediate JSON hint representation |
| `lsp-types` | LSP protocol types (declared dep, not directly used in current source) |

### Key Types and Functions

- **`InlayHintsProvider`** -- unit struct, the main entry point. Methods: `generate_hints()`, `parameter_hints()`, `trivial_type_hints()`. Implements `Default`.
- **`InlayHint`** -- output struct with `position`, `label`, `kind`, `padding_left`, `padding_right`.
- **`InlayHintKind`** -- enum: `Type = 1`, `Parameter = 2`.
- **`parameter_hints()`** -- free function; walks AST for `FunctionCall` nodes, matches 14 built-in names (`open`, `split`, `substr`, `push`, `map`, `grep`, `sort`, `join`, `sprintf`, `printf`, `index`, `rindex`, `splice`, `pack`/`unpack`) and emits labelled parameter hints as `serde_json::Value`.
- **`trivial_type_hints()`** -- free function; walks AST for literal nodes (`Number`, `String`, `HashLiteral`, `ArrayLiteral`, `Regex`, anonymous `Subroutine`) and emits type labels (`Num`, `Str`, `Hash`, `Array`, `Regex`, `CodeRef`).
- **`walk_ast()`** -- private recursive visitor using `get_node_children`.
- **`pos_in_range()`** -- private range-filtering helper.

### Module Layout

- `src/lib.rs` -- re-exports public API from `inlay_hints` module.
- `src/inlay_hints.rs` -- all logic: provider, hint generation, AST walking.

## Usage

```rust
use perl_lsp_inlay_hints::InlayHintsProvider;

let provider = InlayHintsProvider::new();
let hints = provider.generate_hints(&ast, &to_pos16, Some(range));
for hint in &hints {
    // hint.label, hint.position, hint.kind, hint.padding_left, hint.padding_right
}
```

The free functions `parameter_hints()` and `trivial_type_hints()` return `Vec<serde_json::Value>` for lower-level access.

## Important Notes

- Hints are filtered to an optional `Range` so only the visible editor region is computed.
- The provider methods convert JSON intermediate values into typed `InlayHint` structs.
- The parameter hint table is a hardcoded match on 14 common Perl builtins -- adding a new function requires extending the match in `parameter_hints()`.
- No test directory exists; tests live upstream in `perl-lsp`.
- `lsp-types` is declared as a dependency but not directly imported in current source.
