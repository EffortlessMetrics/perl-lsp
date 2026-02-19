# perl-dap-breakpoint

AST-based breakpoint validation for the Perl Debug Adapter Protocol (DAP).

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace.

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

## License

MIT OR Apache-2.0
