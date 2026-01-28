# perl-dap-breakpoint

AST-based breakpoint validation for Perl Debug Adapter Protocol (DAP).

## Features

- **AST-Based Validation**: Uses the Perl parser AST to validate breakpoint locations
- **Line Suggestion**: Suggests the nearest valid line when a breakpoint is on an invalid location
- **Validation Reasons**: Provides detailed reasons for why a breakpoint was rejected or adjusted

## Usage

```rust
use perl_dap_breakpoint::{BreakpointValidator, AstBreakpointValidator};

let source = "# comment\nmy $x = 1;\n";
let validator = AstBreakpointValidator::new(source)?;

// Validate a breakpoint on line 1 (which is a comment)
let result = validator.validate(1);
assert!(!result.verified);

// Validate a breakpoint on line 2 (which has code)
let result = validator.validate(2);
assert!(result.verified);
```

## Validation Types

The validator detects the following invalid breakpoint locations:

- **Blank Lines**: Lines containing only whitespace
- **Comment Lines**: Lines containing only comments
- **Heredoc Interior**: Lines inside heredoc content
- **Out of Range**: Line numbers beyond the file length

## Finding Nearest Valid Line

```rust
use perl_dap_breakpoint::{AstBreakpointValidator, find_nearest_valid_line};
use perl_dap_breakpoint::suggestion::SearchDirection;

let source = "# comment\n# comment\nmy $x = 1;\n";
let validator = AstBreakpointValidator::new(source)?;

// Find nearest valid line from line 1 (a comment)
let nearest = find_nearest_valid_line(&validator, 1, SearchDirection::Forward, None);
assert_eq!(nearest, Some(3)); // Line 3 has executable code
```

## License

MIT OR Apache-2.0
