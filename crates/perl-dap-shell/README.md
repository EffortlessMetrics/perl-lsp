# perl-dap-shell

Shell-facing helpers for launching Perl debugging sessions.

This crate is the quoting-and-env layer. It takes launch intent and turns it
into command arguments and environment variables that are safe to hand to a
shell or launcher.

## Boundaries

- `perl-dap-platform` finds Perl and normalizes paths.
- `perl-dap-shell` formats shell arguments and launch environment values.
- `perl-dap` consumes both when starting a debug session.

## Key API

- `format_command_args`
- `setup_environment`

## Example

```rust
use perl_dap_shell::{format_command_args, setup_environment};
use std::path::PathBuf;

let formatted = format_command_args(&["arg with space".to_string()]);
let env = setup_environment(&[PathBuf::from("/workspace/lib")]);

assert_eq!(formatted.len(), 1);
assert!(formatted[0].contains("arg with space"));
assert_eq!(env.get("PERL5LIB").map(String::as_str), Some("/workspace/lib"));
```
