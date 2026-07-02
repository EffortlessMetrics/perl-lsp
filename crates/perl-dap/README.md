# perl-dap

Use this crate when you need the native Debug Adapter Protocol server for Perl.

`perl-dap` is the runtime layer of the debugger stack. It speaks DAP over stdio
or TCP, dispatches requests, validates breakpoints, and renders values for
DAP-capable editors and tools.

## Boundaries

- `perl-dap-platform` finds the Perl executable, normalizes paths, and builds
  launch environment maps.
- `perl-dap-shell` formats shell-safe launch arguments and environment values.
- `perl-dap-types` carries shared frame, source, and variable models.
- `perl-dap-value` models debugger values for rendering.
- `perl-dap-breakpoint` validates whether a source line can accept a breakpoint.

## Key pieces

- `DapServer`, `DapConfig`, and `DapMode` wire the server and its launch mode.
- `DebugAdapter` handles native request routing and protocol state.
- `TcpAttachConfig` and `BreakpointStore` support socket attach and breakpoint tracking.
- Parser-backed breakpoint validation rejects non-executable source locations where context is available.

## Run modes

### Native launch (stdio DAP server)

```bash
perl-dap --stdio
```

### TCP attach endpoint (native DAP over socket)

```bash
perl-dap --socket --port 13603
```

## Runtime dependencies

Native launch and TCP attach use the built-in Rust runtime plus a local Perl
installation. The Rust parser-backed runtime (`perl-parser`, `perl-parser-core`,
`perl-lexer`, and the `perl-dap-*` support crates) is compiled into the shipped
`perl-dap` binary; users do not install `perl-parser` or any other internal crate
separately.

The release artifacts ship the native Rust binaries (`perllsp` and `perl-dap`).
External Perl debugger backends are not bundled into the native product surface.

## Benchmarks

```bash
# Full benchmark suite (config/platform + live session groups)
cargo bench -p perl-dap --bench dap_benchmarks

# Filter to live-session groups (stable names for diffing)
cargo bench -p perl-dap --bench dap_benchmarks -- dap_live_launch
cargo bench -p perl-dap --bench dap_benchmarks -- dap_live_attach
cargo bench -p perl-dap --bench dap_benchmarks -- dap_live_session
```

Live-session benchmark function names:

- `launch_cold`
- `launch_warm`
- `attach_loopback`
- `set_breakpoints_100`
- `step_continue_p95`
- `stack_trace_live`
- `variables_root`
- `variables_child_page`
- `evaluate_safe_blocked`
- `evaluate_live_simple`
