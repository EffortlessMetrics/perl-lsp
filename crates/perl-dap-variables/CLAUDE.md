# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-variables` is a **Tier 7 DAP feature module** providing variable rendering for debugging.

**Purpose**: Variable rendering for Perl DAP — formats Perl variables for display in debugger UI.

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

- `serde`, `serde_json` - Serialization
- `regex` - Value parsing
- `once_cell` - Lazy patterns
- `thiserror` - Error definitions

### Key Types

| Type | Purpose |
|------|---------|
| `Variable` | Single variable representation |
| `VariableContainer` | Expandable container (hash, array, object) |
| `ValueRenderer` | Formats values for display |

### Variable Types

| Type | Display | Expandable |
|------|---------|------------|
| Scalar | `$x = "value"` | No |
| Array | `@arr = [3 items]` | Yes |
| Hash | `%hash = {2 keys}` | Yes |
| Reference | `$ref = \SCALAR(0x...)` | Yes |
| Object | `$obj = MyClass=HASH(...)` | Yes |
| Code | `$code = CODE(0x...)` | No |

### Variable Hierarchy

```
Locals
├── $scalar = "hello"
├── @array [3 items]
│   ├── [0] = 1
│   ├── [1] = 2
│   └── [2] = 3
└── %hash {2 keys}
    ├── {foo} = "bar"
    └── {baz} = "qux"
```

## Usage

```rust
use perl_dap_variables::{Variable, ValueRenderer};

let renderer = ValueRenderer::new();

// Render a scalar
let var = renderer.render("$x", "42")?;
assert_eq!(var.value, "42");
assert_eq!(var.type_, Some("SCALAR".to_string()));

// Render an array
let var = renderer.render("@arr", "ARRAY(0x123)")?;
assert!(var.variables_reference > 0);  // Expandable
```

### Lazy Loading

Large structures are lazily loaded:

```rust
// Initial request returns summary
// %hash = {1000 keys}

// Expand request returns first N items
// Request: variablesReference = 123
// Response: first 100 key-value pairs

// Pagination for remaining items
// Request: variablesReference = 123, start = 100
```

## Important Notes

- Handles deeply nested structures
- Circular reference detection
- Truncation for large values
- Object blessing displayed in type
