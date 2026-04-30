# perl-dap

Use this crate when you need a native Debug Adapter Protocol server for Perl,
not just debugger helper components.

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
- `DapDispatcher` and `DebugAdapter` handle request routing and protocol state.
- `BridgeAdapter` supports migration from `Perl::LanguageServer`.
- `TcpAttachConfig` and `BreakpointStore` support socket attach and breakpoint tracking.

## Run

```bash
perl-dap --stdio
perl-dap --socket --port 13603
perl-dap --bridge
```

## BridgeAdapter dependency

`--bridge` mode requires the CPAN module `Perl::LanguageServer`.
Install it with either:

```bash
cpan Perl::LanguageServer
cpanm Perl::LanguageServer
```

You can verify availability with:

```bash
perl -e "use Perl::LanguageServer::DebuggerInterface; print qq{OK\n};"
```

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
