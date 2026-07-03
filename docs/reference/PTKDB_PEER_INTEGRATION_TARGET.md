# ptkdb Peer Integration Target

What a future `Devel::ptkdb` patch needs to implement to cooperate live with
`perl-lsp`'s DAP server. The point of the [peer protocol](EXTERNAL_DEBUGGER_PEER_PROTOCOL.md)
is that this list is **small** — ptkdb stays ptkdb, and `perl-dap` owns all DAP
complexity.

> "We are not asking ptkdb to become our tool. We built a small, tested seam so
> ptkdb can remain ptkdb, and perl-lsp can expose it cleanly to editors."

## Two ways to integrate

### A. Live peer (the target)

The rich path: ptkdb speaks the peer protocol over a socket.

1. **Detect the rendezvous env** on startup:
   ```
   PERL_DAP_PEER=127.0.0.1:NNNN
   PERL_DAP_PEER_TOKEN=...          # optional shared secret
   PERL_DAP_PEER_MODE=mirror        # mirror | cooperative | dapControlled
   ```
2. **Open a TCP socket** to that address.
3. **Send `peer/hello`** with the capabilities ptkdb actually has (see below).
4. **Emit events** as the debugger UI drives execution:
   - `debugger/stopped` on every stop (the most important one),
   - `debugger/output` when the target prints,
   - `debugger/terminated` on exit,
   - optionally `debugger/sourceFacts` (breakable lines, subs).
5. **Answer requests** when able: `debugger/stackTrace`, `scopes`, `variables`,
   `evaluate`.
6. **Accept control** (later / cooperative mode): `debugger/continue`, `next`,
   `stepIn`, `stepOut`, `setBreakpoints`.

Framing is `Content-Length`-headed JSON — the same as DAP, so any DAP codec
works. Message shapes are in the [protocol spec](EXTERNAL_DEBUGGER_PEER_PROTOCOL.md)
and the golden examples in [`fixtures/debug-peer/`](../../fixtures/debug-peer/).

Realistic v1 capability report for ptkdb (verified against the ptkdb POD):

```json
{
  "canContinue": true, "canStep": true, "canPause": true, "canEvaluate": true,
  "canSetBreakpoints": true, "canSetFunctionBreakpoints": true,
  "canConditionBreakpoints": true, "canListStack": true,
  "canListVariables": true, "canReportSubroutines": true,
  "canReportBreakableLines": true, "controlMode": "mirror"
}
```

ptkdb documents conditional breakpoints, sub breakpoints, and expression
evaluation — but **not** DAP-style logpoints, hit conditions, or data
breakpoints, so those stay off and the host will not advertise them.

`mirror` mode is the friendliest first step: ptkdb's Tk UI stays authoritative
and just *reports* state; the editor mirrors it. No fight over who owns stepping.

### B. `.ptkdbrc` bootstrap (works today, no ptkdb change)

Before any ptkdb patch exists, `perl-dap` can already drive ptkdb by generating
a `.ptkdbrc` startup file. This is a compatibility bootstrap, not the live
bridge. The generated file uses ptkdb's **documented** startup API (verified
against [metacpan.org/pod/Devel::ptkdb](https://metacpan.org/pod/Devel::ptkdb)
and the [ptkdb.pm source](https://metacpan.org/dist/Devel-ptkdb/source/ptkdb.pm)):

| Purpose                     | ptkdb `.ptkdbrc` call                         |
| --------------------------- | --------------------------------------------- |
| Line breakpoint             | `brkpt($file, @lines)`                        |
| Conditional breakpoint      | `condbrkpt($file, $line, $expr, ...)`         |
| Break on named sub          | `brkonsub(@names)`                            |
| Break on sub regex          | `brkonsub_regex(@regexes)`                    |
| Watch / expression list     | `add_exprs(@expressions)`                     |
| Suppress initial stop       | `$DB::no_stop_at_start = 1;`                  |

`.ptkdbrc` is `eval`'d as Perl (from `~/` or the invocation directory) before the
first stop; the functions are called as bare package subs. `perl-dap` emits all
interpolated data as single-quoted, escaped Perl literals so a crafted path or
condition cannot inject code, and wraps each registration in `eval { ... }` so an
unsupported call degrades gracefully.

Invocation: `perl -d:ptkdb script.pl`, with `PTKDB_DISPLAY` for remote/headless X.

The renderer is `perl_dap::ptkdb_bootstrap` and consumes a
[`DebugSessionPacket`](#session-packet).

## Session packet

`perl-dap` can hand ptkdb a frozen, versioned description of the session —
program, breakpoints, function breakpoints, watch expressions, and per-source
facts (breakable lines, subroutines) — as `perl-lsp-debug-session-v1` JSON
(`perl_dap::model::DebugSessionPacket`, built via
`perl_dap::session_plan::DebugSessionPlanBuilder`). ptkdb can consume this via
`.ptkdbrc` today, the peer protocol tomorrow, or a future ptkdb import command.

## What ptkdb does *not* need to do

- Implement DAP.
- Understand editors, `launch.json`, or LSP.
- Support every DAP feature — capability negotiation is honest and per-peer.
