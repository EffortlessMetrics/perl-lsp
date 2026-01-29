# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-breakpoint` is a **Tier 7 DAP feature module** providing breakpoint validation for the Perl debugger.

**Purpose**: Breakpoint validation for Perl DAP — validates and normalizes breakpoint locations against Perl source.

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

- `perl-parser` - Source parsing
- `ropey` - Text rope handling
- `thiserror` - Error definitions

### Key Types

| Type | Purpose |
|------|---------|
| `BreakpointValidator` | Validates breakpoint locations |
| `ValidatedBreakpoint` | Normalized breakpoint |
| `BreakpointError` | Validation errors |

### Breakpoint Types

| Type | Description |
|------|-------------|
| Line | Break at line number |
| Conditional | Break when condition true |
| Function | Break at function entry |
| Logpoint | Log message without stopping |

## Usage

```rust
use perl_dap_breakpoint::{BreakpointValidator, Breakpoint};

let validator = BreakpointValidator::new(source);

// Validate a line breakpoint
let bp = Breakpoint::line(10);
match validator.validate(bp) {
    Ok(validated) => {
        // Line might be adjusted to valid location
        println!("Breakpoint at line {}", validated.line);
    },
    Err(e) => {
        println!("Invalid breakpoint: {}", e);
    }
}
```

### Line Adjustment

Breakpoints on non-executable lines are adjusted:

```perl
# Line 1: comment - not executable
my $x = 1;  # Line 2: executable
            # Line 3: blank - not executable
print $x;   # Line 4: executable
```

```rust
// Breakpoint on line 1 → adjusted to line 2
// Breakpoint on line 3 → adjusted to line 4
```

## Important Notes

- Validates against actual Perl syntax
- Adjusts to nearest executable line
- Handles POD, comments, blank lines
- Used by `perl-dap` for breakpoint requests
