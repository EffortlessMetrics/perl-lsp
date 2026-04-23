# ADR-0036: Marker-Framed Debugger Queries with Poison-Safe Shared State

- **Status**: Accepted
- **Date**: 2026-03-18
- **Related**: [ADR-0011](0011-dap-bridge-mode-architecture.md), [ADR-0019](0019-security-first-dap.md), [ADR-0028](0028-safe-eval-timeout.md), [ADR-0031](0031-async-runtime-concurrent-dispatch.md)

## Context

The native DAP implementation has to communicate with a Perl debugger process whose stdout is a mixed stream of:

- command results,
- debugger prompts,
- asynchronous events,
- warnings or exception text,
- and user program output.

The codebase already reflects two important architectural decisions for dealing with that reality:

1. **Query framing with degraded fallback**: debugger commands that need structured results are usually wrapped in unique begin/end markers (`DAP_BEGIN_<id>` / `DAP_END_<id>`), but some handlers still fall back to the shared recent-output buffer when framed capture is empty or unavailable.
2. **Poison-safe shared state**: the adapter stores session state, sequence counters, breakpoint stores, and recent output in `Arc<Mutex<...>>` values and explicitly recovers from poisoned mutexes instead of crashing the adapter.

These decisions are visible in `crates/perl-dap/src/debug_adapter/mod.rs`, `output.rs`, and the DAP tests, but were not previously written down as an explicit ADR. That leaves future contributors to rediscover why the adapter is built around marker framing and recovery-oriented locking instead of assuming stdout order is naturally request-scoped or that mutex poisoning should terminate the process.

## Decision Drivers

- Extract command-specific results from a noisy shared stdout stream.
- Avoid coupling higher-level parsing to exact stream timing.
- Keep the adapter responsive when debugger output is incomplete or delayed.
- Preserve service continuity after non-fatal internal panics that poison mutexes.

## Decision

We adopt the following architecture for native DAP debugger interaction.

### 1. Frame debugger queries with synthetic markers

Whenever the adapter needs to extract structured results from the debugger (for example, `%INC` inspection, variable evaluation, or expression results), it sends:

1. a unique begin marker,
2. one or more debugger commands,
3. a matching end marker.

The adapter then prefers the normalized output between those markers.

### 2. Normalize output before parsing

Captured output is normalized by removing ANSI escape sequences, trimming debugger prompt prefixes, and discarding empty lines before higher-level parsing occurs. Parsing logic therefore operates on a deterministic, debugger-noise-reduced view of stdout.

### 3. Maintain a bounded recent-output buffer and fallback path

The adapter keeps a bounded recent-output history and polls that buffer when waiting for a framed query to complete. Some handlers also fall back to that shared buffer when framed capture is empty or unavailable. This avoids tying parsers directly to raw stream timing while still preserving a degraded path for commands that cannot recover a framed slice.

### 4. Treat shared adapter state as recoverable, not fatal

Shared DAP state remains stored behind `Arc<Mutex<...>>`. Lock acquisition uses poison recovery (`into_inner()`) with warning emission instead of panicking or shutting down the adapter.

This policy applies to state such as:

- message sequence numbers,
- active debug session handles,
- attached process metadata,
- recent output history,
- breakpoint and exception settings,
- and request-derived caches.

### 5. Prefer graceful degradation over adapter termination

If framed output cannot be captured before the timeout budget expires, the adapter returns an empty or degraded result for that query rather than crashing. In some paths that degraded result comes from parsing the shared recent-output buffer instead of an isolated framed slice. Likewise, a poisoned mutex becomes a warning and recovery path, not a fatal reliability event.

## Alternatives Considered

### Parse the debugger's raw stdout stream without synthetic markers

Rejected. Prompt text, asynchronous output, and user program output make raw stream parsing too ambiguous for request-scoped queries such as evaluation and `%INC` inspection.

### Treat poisoned mutexes as fatal adapter errors

Rejected. The adapter is a long-lived editor service. Preserving availability and returning degraded results is preferable to terminating the whole debug session after one panic path.

### Block indefinitely until a framed response appears

Rejected. The surrounding DAP architecture already prefers bounded waits and graceful degradation over unbounded hangs.

## Consequences

### Positive

- **Deterministic parsing where framing succeeds**: command results can often be separated from surrounding debugger chatter and program output.
- **Lower protocol coupling**: parsers operate on normalized slices instead of full raw stdout transcripts.
- **Operational resilience**: a panic while holding one mutex does not automatically take down all subsequent DAP requests.
- **Concurrency compatibility**: shared state management aligns with the rest of the adapter's thread-safe request handling.
- **Security/robustness alignment**: bounded waiting and explicit framing fit the repository's broader timeout and degradation strategy.

### Negative

- **Protocol intrusion**: framing depends on injecting extra `p "..."` commands into the debugger stream.
- **Heuristic parsing remains**: marker framing improves isolation, but output parsing still depends on debugger textual conventions and some commands still fall back to the shared recent-output buffer.
- **Poison recovery can mask bugs**: recovering a poisoned mutex keeps the adapter alive, but it can preserve partially-updated state after an internal panic.

### Neutral / Follow-up

- A future refactor may replace some mutex-protected state with more precise ownership or async-native synchronization.
- A future native debugger transport could supersede textual marker framing if the backend exposes structured responses directly.

## Implementation Notes

This ADR documents behavior already implemented in the codebase:

- `lock_or_recover` defines poison-safe locking for adapter state.
- `send_framed_debugger_commands` and `capture_framed_debugger_output` implement unique marker framing.
- `normalize_debugger_output_line` strips prompts and ANSI escapes before parsing.
- `%INC` querying prefers the framed-output path.
- Evaluation, variable inspection, and stack-trace handling prefer framed output but still contain shared-buffer fallbacks when framed capture is empty or unavailable.
- Existing tests already verify isolated framed capture behavior and Content-Length transport framing.
- This ADR records the existing implementation; it does not introduce a new DAP transport.
