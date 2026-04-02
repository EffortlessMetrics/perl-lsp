# perl-lsp-launcher

Command-line launch parsing and startup reporting for `perl-lsp`.

Use this crate when you need to turn CLI flags into a launch plan, startup
banner, or feature-profile summary without pulling the server runtime into the
caller.

## Where it fits

This is the boundary between shell entry points and the long-lived LSP server.
It owns startup configuration, transport selection, logging setup, and
human-readable launch output.

## Key entry points

- `TransportArgs` and `LspArgs` - typed CLI input
- `TransportMode`, `LaunchAction`, `LaunchConfig`, `LaunchPlan`
- `parse_args(args)` - convert raw argv into a launch plan
- `help_text()` and `shell_completion(shell)` - CLI help and completion text
- `startup_banner(...)` and `log_server_startup(...)` - startup output helpers

## Example

```rust
use perl_lsp_launcher::{TransportMode, parse_args};

let plan = parse_args(["perl-lsp", "--stdio"])?;
assert_eq!(plan.config.transport.mode(), TransportMode::Stdio);
```

## Typical use

Use `perl-lsp-launcher` when you are wiring the binary entry point, generating
shell completions, or testing startup behavior. If you only need the long-lived
server logic, use `perl-lsp` instead.
