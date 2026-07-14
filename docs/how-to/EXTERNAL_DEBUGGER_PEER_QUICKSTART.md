# External Debugger Peer — Quickstart

`perl-dap` can act as a **host** for an external Perl debugger engine (e.g.
`Devel::ptkdb`) instead of driving `perl -d` itself. Your editor keeps speaking
DAP; `perl-dap` translates DAP ↔ the small **Perl Debugger Peer Protocol** and
lets the external engine own the actual debug session.

This page is the first-mile guide. For the wire format see
[EXTERNAL_DEBUGGER_PEER_PROTOCOL.md](../reference/EXTERNAL_DEBUGGER_PEER_PROTOCOL.md);
for the design rationale and what was deliberately deferred see
[EXTERNAL_DEBUGGER_PEER_DECISIONS.md](../reference/EXTERNAL_DEBUGGER_PEER_DECISIONS.md);
to make a *new* engine speak the protocol see
[DEBUGGER_BACKEND_AUTHORS.md](DEBUGGER_BACKEND_AUTHORS.md).

> The default, always-available debugger is the native `perl -d` adapter — see
> the [DAP User Guide](../tutorials/DAP_USER_GUIDE.md). You only need this page if
> you want to cooperate with an external engine.

## What you can do today

| Surface | Status | Command |
|---------|--------|---------|
| `.ptkdbrc` bootstrap | ✅ shippable | `perl-dap --ptkdb-bootstrap-rc PROGRAM` |
| Session plan (JSON) | ✅ shippable | `perl-dap --debug-session-plan PROGRAM` |
| DAP ↔ peer bridge | ✅ against a peer that speaks the protocol | `perl-dap --external-peer HOST:PORT` |
| Live stock `Devel::ptkdb` | ⏳ pending the ptkdb-side patch | — |

The bridge is proven end-to-end against an in-repo fake ptkdb peer. Pointing it
at a **stock** `Devel::ptkdb` needs a thin ptkdb-side patch that teaches ptkdb to
speak the peer protocol — tracked in
[PTKDB_PEER_INTEGRATION_TARGET.md](../reference/PTKDB_PEER_INTEGRATION_TARGET.md).
Until that lands, `--debug-session-plan` (§2) is the genuinely useful standalone
tool; the `.ptkdbrc` bootstrap (§1) is a minimal, extend-it-yourself starting point.

## 1. Generate a `.ptkdbrc` bootstrap (no bridge needed)

Emit a `Devel::ptkdb` bootstrap file for a program and run it under ptkdb the
ordinary way:

```bash
perl-dap --ptkdb-bootstrap-rc path/to/script.pl > .ptkdbrc
perl -d:ptkdb path/to/script.pl
```

`ptkdb` reads `.ptkdbrc` from the current directory before the first stop. The
generated file is plain Perl (safely escaped). It registers any line/subroutine
breakpoints and watch expressions carried in the session plan, each wrapped in
`eval { ... }` so an unsupported ptkdb call degrades gracefully. Built from the
bare `--ptkdb-bootstrap-rc PROGRAM` (which takes no breakpoint flags yet), it is
a **minimal valid** `.ptkdbrc` — extend it with your own ptkdb breakpoints, or
use `--debug-session-plan` (below) to see the breakable lines and subroutines
`perl-lsp` found and pick where to break.

## 2. Inspect the session plan (JSON)

See exactly what `perl-lsp` derives for a program — breakable lines, subroutines,
include paths — as a stable `perl-lsp-debug-session-v1` document:

```bash
perl-dap --debug-session-plan path/to/script.pl
```

```json
{
  "schema": "perl-lsp-debug-session-v1",
  "program": "path/to/script.pl",
  "breakpoints": [],
  "source_facts": {
    "path/to/script.pl": {
      "breakable_line_candidates": [2, 3, 5, 6, 7, 8, 10, 11],
      "subroutines": [{ "name": "greet", "start_line": 5, "end_line": 8 }]
    }
  }
}
```

This is handy for scripting, for feeding another tool, or for verifying which
lines `perl-lsp` considers breakable before you set a breakpoint.

## 3. Bridge an editor to a running peer

If you have a debugger engine listening on a socket and speaking the peer
protocol, bridge your editor to it. `perl-dap` connects **out** to the peer and
speaks DAP to the editor.

**stdio (the usual case — the editor spawns `perl-dap`):**

```bash
perl-dap --external-peer 127.0.0.1:13604
```

**socket (the editor connects to `perl-dap` on a TCP port):**

```bash
perl-dap --socket --port 13603 --external-peer 127.0.0.1:13604
```

Only capabilities the peer actually advertises in its handshake are surfaced to
the editor (mirror-mode honesty): `perl-dap` never claims a control command the
peer didn't offer. A malformed frame or a peer that stops responding fails fast
rather than hanging.

### From VS Code

Add a launch config that selects the external backend. The extension passes it
through to `perl-dap --external-peer`:

```jsonc
{
  "type": "perl",
  "request": "attach",
  "name": "Perl: External Debugger Peer (ptkdb)",
  "debuggerBackend": "external",
  "externalDebugger": {
    "kind": "ptkdb",
    "mode": "connect",
    "control": "mirror",
    "host": "127.0.0.1",
    "port": 13604
  }
}
```

Or the shorthand `"externalPeer": "127.0.0.1:13604"` on the config. Pick
**"External Debugger Peer (ptkdb)"** from *Perl: Create Debug Configuration* to
scaffold it. Only `mode: "connect"` with a concrete port is wired today;
`listen`/`launchPeer` and `port: 0` fall back to the native adapter rather than
failing.

## Troubleshooting

| Symptom | Cause / fix |
|---------|-------------|
| `failed to connect to debugger peer` | No peer is listening at `HOST:PORT`. Start the engine first, then launch the bridge. |
| `no editor connected within …` | Socket mode: the editor never connected on `--port`. Check the port matches the editor's config. |
| Selecting the ptkdb config runs the native adapter | `mode` is `listen`/`launchPeer`, or `port` is `0` — these aren't wired yet. Use `mode: "connect"` with a real port. |
| Control buttons (step/pause) are greyed out | The peer didn't advertise those capabilities — mirror-mode honesty. Drive them from the peer's own UI. |

## See also

- [DAP User Guide](../tutorials/DAP_USER_GUIDE.md) — the default native debugger
- [EXTERNAL_DEBUGGER_PEER_PROTOCOL.md](../reference/EXTERNAL_DEBUGGER_PEER_PROTOCOL.md) — the wire protocol
- [EXTERNAL_DEBUGGER_PEER_DECISIONS.md](../reference/EXTERNAL_DEBUGGER_PEER_DECISIONS.md) — design decisions and deferrals
- [DEBUGGER_BACKEND_AUTHORS.md](DEBUGGER_BACKEND_AUTHORS.md) — implement a new backend
- [PTKDB_PEER_INTEGRATION_TARGET.md](../reference/PTKDB_PEER_INTEGRATION_TARGET.md) — the ptkdb-side integration checklist
