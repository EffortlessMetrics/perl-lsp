# perl-dap-types

Shared DAP session model types for Perl debugging.

This crate is the transport-friendly data layer behind the debugger stack. It
keeps stack frames, sources, and variables in one small package so `perl-dap`
and tests can share the same shapes without pulling in the runtime.

## Boundaries

- Use this crate for shared DAP payloads and fixtures.
- Use `perl-dap` for the wire protocol and server behavior.
- Use `perl-dap-value` when you need the underlying Perl value model, not the
  DAP transport shape.
- Use `perl-dap-platform` and `perl-dap-shell` for launch-time environment and
  command-line preparation.

## Key types

- `StackFrame`
- `Source`
- `Variable`

## Example

```rust
use perl_dap_types::{Source, StackFrame, Variable};

let source = Source::new("/workspace/script.pl");
let frame = StackFrame::new(1, "main::run", source, 42).with_column(3);
let variable = Variable {
    name: "$x".to_string(),
    value: "42".to_string(),
    type_: Some("SCALAR".to_string()),
    variables_reference: 0,
    named_variables: None,
    indexed_variables: None,
};

assert_eq!(frame.line, 42);
assert_eq!(variable.value, "42");
```
