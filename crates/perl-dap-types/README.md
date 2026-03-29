# perl-dap-types

Shared session model types for the Perl Debug Adapter Protocol implementation.

## When to use this crate

Use `perl-dap-types` when you need the small shared Rust data model behind Perl
debug sessions without depending on the full `perl-dap` runtime.

It is useful for:

- tests that construct DAP stack frames and variables directly
- bridge or transport layers that serialize DAP session state
- tools that want stable Perl-specific debug value structs

## Quick example

```rust
use perl_dap_types::{Source, StackFrame};

let source = Source::new("/workspace/script.pl");
let frame = StackFrame::new(1, "main::run", source, 42).with_column(3);

assert_eq!(frame.line, 42);
assert_eq!(frame.column, 3);
```

## Public API

- `StackFrame`: stack-frame model with builder-style helpers
- `Source`: source-file descriptor for debugger responses
- `Variable`: debugger variable payload with DAP-friendly serialization

## License

MIT OR Apache-2.0
