# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-variables` is a **DAP feature module** providing variable parsing and rendering for Perl debugging.

**Purpose**: Parses Perl debugger text output into structured `PerlValue` representations, then renders them into DAP-compatible `RenderedVariable` structs for display in VSCode and other DAP-compatible editors.

**Version**: 0.1.0

## Commands

```bash
cargo build -p perl-dap-variables        # Build this crate
cargo test -p perl-dap-variables         # Run tests
cargo clippy -p perl-dap-variables       # Lint
cargo doc -p perl-dap-variables --open   # View documentation
```

## Architecture

### Dependencies

- `serde`, `serde_json` -- DAP protocol serialization
- `regex` -- Compiled patterns for parsing debugger output
- `once_cell` -- Lazy-initialized regex patterns via `Lazy<Result<Regex, regex::Error>>`
- `thiserror` -- `VariableParseError` derivation

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `src/lib.rs` | `PerlValue` enum, re-exports public API |
| `parser` | `src/parser.rs` | `VariableParser` for debugger output parsing |
| `renderer` | `src/renderer.rs` | `VariableRenderer` trait, `PerlVariableRenderer`, `RenderedVariable` |

### Key Types

| Type | Purpose |
|------|---------|
| `PerlValue` | Enum: `Undef`, `Scalar`, `Number`, `Integer`, `Array`, `Hash`, `Reference`, `Object`, `Code`, `Glob`, `Regex`, `Tied`, `Truncated`, `Error` |
| `VariableParser` | Parses debugger text lines (e.g., `$x = 42`) into `(String, PerlValue)` pairs |
| `VariableParseError` | Error enum: `UnrecognizedFormat`, `MaxDepthExceeded`, `UnterminatedString`, `UnterminatedCollection`, `RegexError` |
| `VariableRenderer` | Trait with `render()`, `render_with_reference()`, `render_children()` |
| `PerlVariableRenderer` | Default renderer with configurable `max_string_length`, `max_array_preview`, `max_hash_preview` |
| `RenderedVariable` | DAP-compatible struct: `name`, `value`, `type_name`, `variables_reference`, `named_variables`, `indexed_variables`, `presentation_hint`, `memory_reference` |
| `VariablePresentationHint` | DAP presentation hints: `kind`, `attributes`, `visibility` |

### Parsing Flow

1. `VariableParser::parse_assignment("$x = 42")` splits name from value via regex
2. `parse_value()` recursively matches against compiled regex patterns (undef, integer, float, quoted string, ARRAY/HASH/CODE refs, blessed objects, globs)
3. Literal arrays `(1, 2, 3)` / `[1, 2, 3]` and hashes `{k => v}` are parsed with nesting-aware comma splitting
4. `max_depth` (default 50) guards against excessive recursion

### Rendering Flow

1. `PerlVariableRenderer::render(name, &value)` formats the value string and sets type/child counts
2. `render_with_reference()` assigns a `variables_reference` ID for expandable values
3. `render_children()` paginates child elements (array indices, hash keys, dereferenced values)

## Usage

```rust
use perl_dap_variables::{PerlValue, PerlVariableRenderer, VariableRenderer, VariableParser};

// Parse debugger output
let parser = VariableParser::new();
let (name, value) = parser.parse_assignment("$x = 42")?;
assert_eq!(name, "$x");
assert!(matches!(value, PerlValue::Integer(42)));

// Render for DAP
let renderer = PerlVariableRenderer::new();
let rendered = renderer.render("$greeting", &PerlValue::Scalar("hello".to_string()));
assert_eq!(rendered.name, "$greeting");
assert_eq!(rendered.value, "\"hello\"");
assert_eq!(rendered.type_name, Some("SCALAR".to_string()));

// Render expandable array with reference ID
let arr = PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);
let rendered = renderer.render_with_reference("@arr", &arr, 42);
assert_eq!(rendered.variables_reference, 42);
assert_eq!(rendered.indexed_variables, Some(2));

// Get children for expansion
let children = renderer.render_children(&arr, 0, 10);
assert_eq!(children[0].name, "[0]");
assert_eq!(children[1].name, "[1]");
```

## Important Notes

- Regex patterns are stored as `Lazy<Result<Regex, regex::Error>>` with accessor functions returning `Option<&Regex>`, treating compile failure as "no match" (graceful degradation per workspace policy)
- `PerlValue::is_expandable()` returns true for `Array`, `Hash`, `Reference`, `Object`, and `Tied`
- String truncation controlled by `PerlVariableRenderer::with_max_string_length()` (default 100 chars)
- `RenderedVariable` uses `#[serde(rename_all = "camelCase")]` for DAP protocol JSON compatibility
- Used by `perl-dap` for variable inspection and evaluate responses
