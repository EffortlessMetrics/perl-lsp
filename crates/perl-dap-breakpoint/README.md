# perl-dap-breakpoint

AST-based breakpoint validation for Perl DAP sessions.

Use this crate before you hand a breakpoint to the runtime. It answers two
questions: is this line executable, and if not, what nearby line should the
debugger suggest instead?

## Boundaries

- `perl-dap` uses this crate to validate breakpoints before sending them to the
  runtime.
- `perl-dap-platform` and `perl-dap-shell` prepare launch inputs; they do not
  validate source lines.
- This crate does not speak DAP itself. It only reasons about source text and
  parsed AST shape.

## Key API

- `AstBreakpointValidator`
- `BreakpointValidation`
- `BreakpointValidator`
- `ValidationReason`
- `find_nearest_valid_line`
- `SearchDirection`

## Example

```rust,ignore
use perl_dap_breakpoint::{AstBreakpointValidator, BreakpointValidator};

let source = "# comment\nmy $x = 1;\n";
let validator = AstBreakpointValidator::new(source)?;
let result = validator.validate(1);

assert!(!result.verified);
assert_eq!(result.reason, Some(perl_dap_breakpoint::ValidationReason::CommentLine));
```
