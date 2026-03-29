# perl-dap

Native Debug Adapter Protocol server for Perl.

`perl-dap` is the runtime layer of the debugger stack. It speaks DAP over stdio
or TCP, dispatches requests, validates breakpoints, and renders values. Use it
when you want to debug Perl from a DAP-capable editor or tool.

## Boundaries

- `perl-dap-platform` finds the Perl executable, normalizes paths, and builds
  launch environment maps.
- `perl-dap-shell` formats shell-safe launch arguments and environment values.
- `perl-dap-types` carries shared frame, source, and variable models.
- `perl-dap-value` models debugger values for rendering.
- `perl-dap-breakpoint` validates whether a source line can accept a breakpoint.

## Key pieces

- `DapServer`, `DapConfig`, and `DapMode` wire the server and its launch mode.
- `DapDispatcher` and `DebugAdapter` handle request routing and protocol state.
- `BridgeAdapter` supports migration from `Perl::LanguageServer`.
- `TcpAttachConfig` and `BreakpointStore` support socket attach and breakpoint tracking.

## Run

```bash
perl-dap --stdio
perl-dap --socket --port 13603
perl-dap --bridge
```
