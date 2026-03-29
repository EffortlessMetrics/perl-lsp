# perl-dap-breakpoint

[![Crates.io](https://img.shields.io/crates/v/perl-dap-breakpoint.svg)](https://crates.io/crates/perl-dap-breakpoint)
[![Documentation](https://docs.rs/perl-dap-breakpoint/badge.svg)](https://docs.rs/perl-dap-breakpoint)

AST-based breakpoint validation for Perl DAP.

## When to use this crate

Use `perl-dap-breakpoint` when you need to decide whether a debugger breakpoint
can be placed on a Perl source line.

It solves three related problems:

- detect executable lines versus comments, blanks, and heredoc interiors
- suggest the nearest valid line when the requested line is invalid
- explain why a breakpoint was rejected or moved

## Quick example

```rust,ignore
use perl_dap_breakpoint::{AstBreakpointValidator, BreakpointValidator};

let validator = AstBreakpointValidator::new("# comment\nmy $x = 1;\n")?;
let result = validator.validate(0);
assert!(!result.verified);
```

## Features

- **AST-based validation** -- uses `perl-parser` to determine whether a line contains executable code
- **Line suggestion** -- finds the nearest valid line via `find_nearest_valid_line` with configurable search direction and max distance
- **Detailed rejection reasons** -- distinguishes blank lines, comment lines, heredoc interiors, and out-of-range lines (`ValidationReason`)

## Public API

| Item | Kind | Description |
|------|------|-------------|
| `BreakpointValidator` | trait | `validate`, `validate_with_column`, `is_executable_line` |
| `AstBreakpointValidator` | struct | Parses source with `perl-parser` and implements `BreakpointValidator` |
| `BreakpointValidation` | struct | Result with `verified`, `line`, `column`, `reason`, `message` fields |
| `ValidationReason` | enum | `BlankLine`, `CommentLine`, `HeredocInterior`, `LineOutOfRange`, `ParseError` |
| `BreakpointError` | enum | `ParseError(String)`, `LineOutOfRange(i64, usize)` |
| `find_nearest_valid_line` | fn | Searches forward, backward, or both for the nearest executable line |
| `suggestion::SearchDirection` | enum | `Forward`, `Backward`, `Both` |

## Workspace role

This is a support crate in the `perl-dap` family. It is useful on its own for
debugger-side validation, but it is primarily consumed by the runtime and test
layers that need breakpoint eligibility checks.

## License

MIT OR Apache-2.0
