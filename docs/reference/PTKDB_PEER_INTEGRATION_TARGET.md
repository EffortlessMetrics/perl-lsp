# ptkdb Peer Integration Target

What a future `Devel::ptkdb` patch needs to implement to cooperate live with
`perl-lsp`'s DAP server. The point of the [peer protocol](EXTERNAL_DEBUGGER_PEER_PROTOCOL.md)
is that this list is **small** — ptkdb stays ptkdb, and `perl-dap` owns all DAP
complexity.

> "We are not asking ptkdb to become our tool. We built a small, tested seam so
> ptkdb can remain ptkdb, and perl-lsp can expose it cleanly to editors."

## Minimum upstream ptkdb PR

The host side of the live-peer wiring landed in [#3404](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3404)
(mirror-mode launch wiring, tracked under [#3322](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3322)):
`perl-dap` can bind a loopback listener, mint a session token, and expose the
env-var rendezvous contract below to whatever process it launches. **This
section is the small, copy-paste-able ask for the upstream ptkdb-side PR** that
completes the other half — nothing here requires #3404 to have merged to be
true (the wire shapes are stable), but the env contract only becomes reachable
once it does.

A [reference implementation](../../fixtures/debug-peer/perl/minimal_ptkdb_peer.pl)
of exactly this checklist exists at `fixtures/debug-peer/perl/minimal_ptkdb_peer.pl`
— copy it as a starting point; it is a teaching fixture, not production ptkdb
code.

### Checklist

- [ ] **Detect `PERL_DAP_PEER`** at startup; if unset (or `PERL_DAP_PEER_MODE`
      is not `mirror`), do nothing and behave exactly as ptkdb does today.
- [ ] **Open a TCP socket** to the `HOST:PORT` in `PERL_DAP_PEER`.
- [ ] **Send `peer/hello`** (include `token` from `PERL_DAP_PEER_TOKEN` when
      it is set — the host rejects the handshake without a matching token).
- [ ] **Emit `debugger/output`** whenever the debuggee prints.
- [ ] **Emit `debugger/stopped`** on every stop (breakpoint, step, entry, ...).
- [ ] **Exit cleanly on disconnect** — a closed socket (clean EOF) from the
      host means the session is over; do not treat it as an error.
- [ ] **Ignore peer mode unless `mirror`** — `cooperative`/`dapControlled` are
      future work; a v1 patch only needs to speak mirror.
- [ ] **No DAP implementation required** — ptkdb never needs to know what DAP,
      `launch.json`, or an LSP client even are.

### Env-var contract

| Variable               | Set by  | Meaning                                                        |
| ----------------------- | ------- | --------------------------------------------------------------- |
| `PERL_DAP_PEER`         | host    | `HOST:PORT` the peer connects back to (loopback only)            |
| `PERL_DAP_PEER_TOKEN`   | host    | Per-session shared secret; optional, but if the host minted one, `peer/hello` **must** echo it back exactly or the handshake is rejected |
| `PERL_DAP_PEER_MODE`    | host    | Control mode for this session; `mirror` is the only mode a v1 ptkdb patch needs to handle |

(Source: `ENV_PEER_ADDR` / `ENV_PEER_TOKEN` / `ENV_PEER_MODE` in
`crates/perl-dap/src/backend/peer_launch.rs`, landed in #3404.)

### Exact payload shapes

Framing is `Content-Length`-headed JSON, byte-identical to the LSP/DAP base
protocol (`Content-Length: <N>\r\n\r\n<N bytes of JSON>`, no `Content-Type`
header, no trailing newline). Golden fixtures for all of these live in
[`fixtures/debug-peer/`](../../fixtures/debug-peer/).

**`peer/hello` request** (peer → host; `token` is omitted entirely, not sent
as `null`, when the host minted no token — see
`crates/perl-dap/src/peer_protocol/payloads.rs::HelloArgs`):

```json
{
  "type": "request",
  "seq": 1,
  "command": "peer/hello",
  "arguments": {
    "peer": "Devel::ptkdb",
    "peerVersion": "1.1091",
    "protocolVersion": "perl-debug-peer-v1",
    "token": "deadbeefcafef00ddeadbeefcafef00d",
    "capabilities": {}
  }
}
```

`capabilities` may be `{}` — every field defaults to `false`, which is a
complete and honest report for a peer that only emits `output`/`stopped`.

**`peer/hello` response** (host → peer; reject the session if `success` is
`false` and read `message` for why — bad token, unsupported
`protocolVersion`, or a replayed handshake):

```json
{
  "type": "response",
  "seq": 1,
  "requestSeq": 1,
  "success": true,
  "command": "peer/hello",
  "body": {
    "protocolVersion": "perl-debug-peer-v1",
    "sessionId": "perl-dap-peer-127.0.0.1:49321",
    "capabilities": { "wantsBreakpoints": true, "wantsStack": true,
                       "wantsVariables": true, "wantsOutput": true,
                       "wantsSourceFacts": true }
  }
}
```

**`debugger/output` event** (peer → host,
`OutputEventBody` in `payloads.rs`):

```json
{
  "type": "event",
  "seq": 2,
  "event": "debugger/output",
  "body": { "category": "stdout", "output": "some debuggee output\n" }
}
```

**`debugger/stopped` event** (peer → host, `StoppedEventBody`; `source`,
`line`, `column` are optional but should be sent whenever known):

```json
{
  "type": "event",
  "seq": 3,
  "event": "debugger/stopped",
  "body": {
    "reason": "breakpoint",
    "threadId": 1,
    "source": { "path": "/work/script.pl" },
    "line": 42,
    "column": 1
  }
}
```

### What ptkdb does NOT need to implement (v1)

- `debugger/continue` / `debugger/next` / `debugger/stepIn` / `debugger/stepOut` / `debugger/pause`
- `debugger/setBreakpoints` / `debugger/setFunctionBreakpoints`
- `debugger/stackTrace` / `debugger/scopes` / `debugger/variables`
- `debugger/evaluate`

All of the above are deferred to a v2/v3 (`cooperative`/`dapControlled`
control modes) once mirror-mode round-trips are proven live. A v1 patch that
only ever sends `peer/hello`, `debugger/output`, and `debugger/stopped` is a
complete, useful integration on its own: ptkdb stays ptkdb, and `perl-dap`
handles everything DAP.

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
