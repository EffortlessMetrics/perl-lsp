# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-breakpoint` is a **DAP feature module** providing AST-based breakpoint validation for the Perl debugger.

**Purpose**: Validates breakpoint locations against parsed Perl source to ensure breakpoints land on executable lines, rejecting comments, blank lines, heredoc interiors, and out-of-range lines.

**Version**: 0.1.0

## Commands

```bash
cargo build -p perl-dap-breakpoint       # Build this crate
cargo test -p perl-dap-breakpoint        # Run tests
cargo clippy -p perl-dap-breakpoint      # Lint
cargo doc -p perl-dap-breakpoint --open  # View documentation
```

## Architecture

### Dependencies

- `perl-parser` -- Parses Perl source into an AST (`Parser`, `Node`, `NodeKind`)
- `ropey` -- Rope data structure for efficient line-to-byte position mapping
- `thiserror` -- Error type derivation for `BreakpointError`

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `src/lib.rs` | Re-exports public API, defines `BreakpointError` enum |
| `validator` | `src/validator.rs` | `BreakpointValidator` trait, `AstBreakpointValidator` struct, `BreakpointValidation`, `ValidationReason` |
| `suggestion` | `src/suggestion.rs` | `find_nearest_valid_line` function, `SearchDirection` enum |

### Key Types

| Type | Purpose |
|------|---------|
| `BreakpointValidator` | Trait with `validate(line)`, `validate_with_column(line, col)`, `is_executable_line(line)` |
| `AstBreakpointValidator` | Struct holding parsed AST, Rope, and source; implements `BreakpointValidator` |
| `BreakpointValidation` | Result struct: `verified`, `line`, `column`, `reason`, `message` |
| `ValidationReason` | Enum: `BlankLine`, `CommentLine`, `HeredocInterior`, `LineOutOfRange`, `ParseError` |
| `BreakpointError` | Error enum: `ParseError(String)`, `LineOutOfRange(i64, usize)` |
| `SearchDirection` | Enum: `Forward`, `Backward`, `Both` (in `suggestion` module, not re-exported from root) |

### Validation Logic

1. Parse source with `perl_parser::Parser` to get an AST `Node`
2. For a given 1-based line number, convert to byte range via `Rope`
3. Check heredoc interior first (AST `NodeKind::Heredoc` with `body_span`)
4. Check comment/blank: fast-path text checks, then AST-based check for executable nodes in range
5. If all checks pass, the line is executable and the breakpoint is verified

## Usage

```rust
use perl_dap_breakpoint::{AstBreakpointValidator, BreakpointValidator, find_nearest_valid_line};
use perl_dap_breakpoint::suggestion::SearchDirection;

let source = "# comment\nmy $x = 1;\n\nprint $x;\n";
let validator = AstBreakpointValidator::new(source)?;

// Line 1 is a comment -- rejected
let result = validator.validate(1);
assert!(!result.verified);

// Line 2 is executable -- verified
let result = validator.validate(2);
assert!(result.verified);

// Find nearest executable line from line 1
let nearest = find_nearest_valid_line(&validator, 1, SearchDirection::Forward, None);
assert_eq!(nearest, Some(2));
```

## Important Notes

- `SearchDirection` is public in `suggestion` module but not re-exported from the crate root; access via `perl_dap_breakpoint::suggestion::SearchDirection`
- All line numbers are 1-based (`i64`)
- Comments are detected by text inspection (lines starting with `#`), not AST nodes, because comments are stripped during lexing
- Heredoc interior detection uses `NodeKind::Heredoc { body_span }` from the AST
- Used by `perl-dap` for breakpoint request handling
