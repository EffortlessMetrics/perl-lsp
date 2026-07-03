# Perl Debugger Peer Protocol — v1

**Status:** implemented host-side in `perl-dap` (`crate::peer_protocol` + `crate::backend::external_peer`).
**Version string:** `perl-debug-peer-v1`
**Audience:** authors of an external Perl debugger engine/frontend (`Devel::ptkdb` first) that wants to cooperate with `perl-lsp`'s DAP server without implementing DAP.

> This is the wire contract. For *why* it exists and how the host is layered, see
> [EXTERNAL_DEBUGGER_PEER_DECISIONS.md](EXTERNAL_DEBUGGER_PEER_DECISIONS.md). For a
> ptkdb-specific implementation checklist, see
> [PTKDB_PEER_INTEGRATION_TARGET.md](PTKDB_PEER_INTEGRATION_TARGET.md).

## 1. Transport & framing

- TCP, `127.0.0.1` by default.
- Each message is a JSON object framed with an LSP/DAP-style header:

  ```
  Content-Length: <N>\r\n
  \r\n
  <N bytes of UTF-8 JSON>
  ```

  This is the **same** framing DAP uses, so an existing DAP-style codec can be
  reused verbatim. The host implements it with
  `perl_lsp_rs_core::transport::{frame, ContentLengthFramer}`.

- Either side may `Listen` or `Connect`; in the ptkdb MVP the host (`perl-dap`)
  listens and the peer connects. Once the socket is up, **the peer sends
  `peer/hello` first**.

## 2. Message envelope

Three message shapes, tagged by `type` (identical convention to DAP):

```jsonc
// request
{ "type": "request",  "seq": 1, "command": "debugger/continue", "arguments": { ... } }
// response
{ "type": "response", "seq": 2, "requestSeq": 1, "success": true, "command": "debugger/continue", "body": { ... } }
// event
{ "type": "event",    "seq": 3, "event": "debugger/stopped", "body": { ... } }
```

- `seq` is a per-sender monotonic counter. Each side keeps its own.
- A response echoes the request's `seq` in `requestSeq` and the `command`.
- On failure, a response sets `"success": false` and SHOULD set `"message"`.

## 3. Handshake

### `peer/hello` (peer → host, request)

```json
{
  "type": "request",
  "seq": 1,
  "command": "peer/hello",
  "arguments": {
    "peer": "Devel::ptkdb",
    "peerVersion": "1.1091",
    "protocolVersion": "perl-debug-peer-v1",
    "capabilities": {
      "canContinue": true,
      "canStep": true,
      "canPause": true,
      "canEvaluate": true,
      "canSetBreakpoints": true,
      "canSetFunctionBreakpoints": true,
      "canConditionBreakpoints": true,
      "canListStack": true,
      "canListVariables": true,
      "canReportSubroutines": true,
      "canReportBreakableLines": true,
      "controlMode": "mirror"
    }
  }
}
```

Every capability field defaults to `false`/`mirror` if omitted, so a minimal
peer that only reports stops may send `"capabilities": {}`.

### `peer/hello` response (host → peer)

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
    "capabilities": {
      "wantsBreakpoints": true,
      "wantsStack": true,
      "wantsVariables": true,
      "wantsOutput": true,
      "wantsSourceFacts": true
    }
  }
}
```

The host records the peer's capabilities and **advertises to the editor only the
intersection** of what it compiled in and what the peer offered — a `mirror`-mode
peer that cannot step never causes the editor to show a step button that fails.

## 4. Capability → DAP negotiation

| Peer capability            | Enables host DAP capability            |
| -------------------------- | -------------------------------------- |
| `canSetBreakpoints`        | source breakpoints                     |
| `+ canConditionBreakpoints`| `supportsConditionalBreakpoints`       |
| `canSetFunctionBreakpoints`| `supportsFunctionBreakpoints`          |
| `canEvaluate`              | `supportsEvaluateForHovers`, evaluate  |
| `canListVariables`         | variables + scopes                     |
| `canListStack`             | stackTrace                             |
| `canStep`                  | continue / next / stepIn / stepOut     |
| `canPause`                 | pause (async interrupt — separate from `canStep`) |

A v1 peer never negotiates logpoints, hit-conditions, data breakpoints, or
set-variable; those stay off.

## 5. Events (peer → host)

| Event                          | Body                                                                 |
| ------------------------------ | -------------------------------------------------------------------- |
| `debugger/initialized`         | *(none)* — peer ready for configuration                              |
| `debugger/stopped`             | `{ reason, threadId, source?, line?, column? }`                      |
| `debugger/continued`           | `{ threadId }`                                                       |
| `debugger/output`              | `{ category: stdout\|stderr\|console, output }`                      |
| `debugger/terminated`          | `{ exitCode? }`                                                      |
| `debugger/sourceFacts`         | `{ source, breakableLines: [..], subroutines: [{name,source,startLine,endLine}] }` |
| `debugger/breakpointsChanged`  | `{ breakpoints: [{ id, verified, line, column?, message? }] }`      |

`reason` is one of `entry`, `step`, `breakpoint`, `functionBreakpoint`,
`dataBreakpoint`, `exception`, `pause`; unknown values are preserved.

Example `debugger/stopped`:

```json
{ "type": "event", "seq": 10, "event": "debugger/stopped",
  "body": { "reason": "breakpoint", "threadId": 1,
            "source": { "path": "/work/script.pl" }, "line": 42, "column": 1 } }
```

## 6. Requests (host → peer)

| Command                            | Arguments                                            | Response body                              |
| ---------------------------------- | ---------------------------------------------------- | ------------------------------------------ |
| `debugger/setBreakpoints`          | `{ source, breakpoints: [{line, column?, condition?, hitCondition?, logMessage?}] }` | `{ breakpoints: [{id, verified, line, column?, message?}] }` — same order |
| `debugger/setFunctionBreakpoints`  | `{ names: [".."] }`                                  | `{ breakpoints: [..] }`                    |
| `debugger/continue`                | `{ threadId }`                                       | *(any / empty)*                            |
| `debugger/next` / `stepIn` / `stepOut` | `{ threadId }`                                   | *(any / empty)*                            |
| `debugger/pause`                   | `{ threadId }`                                       | *(any / empty)*                            |
| `debugger/stackTrace`              | `{ threadId, startFrame?, levels? }`                 | `{ stackFrames: [{id,name,source,line,column}] }` |
| `debugger/scopes`                  | `{ frameId }`                                        | `{ scopes: [{name, variablesReference, expensive}] }` |
| `debugger/variables`               | `{ variablesReference }`                             | `{ variables: [{name, value, typeName?, variablesReference, indexedVariables?, namedVariables?}] }` |
| `debugger/evaluate`                | `{ expression, frameId?, context? }`                 | `{ result, typeName?, variablesReference }` |
| `debugger/disconnect`              | *(none)*                                             | *(any / empty)*                            |

A peer that lacks a capability may reply `success: false` with a `message`; the
host also refuses to *send* a command the peer did not advertise.

## 7. Shutdown

Either side may send `peer/goodbye` (request); the other replies and closes. A
dropped connection is treated as termination: in-flight host requests fail with a
transport/`NotConnected` error rather than hanging.

## 8. Golden examples

Canonical framed-message examples live in
[`fixtures/debug-peer/`](../../fixtures/debug-peer/) and are asserted by the
`perl-dap` conformance tests (`tests/external_peer_conformance.rs`).

## 9. Control modes

| Mode           | Who owns stepping | Status            |
| -------------- | ----------------- | ----------------- |
| `mirror`       | the peer (ptkdb)  | fully implemented |
| `cooperative`  | both              | modeled, future   |
| `dapControlled`| the editor        | modeled, future   |

`mirror` is the friendliest first integration: ptkdb's own UI stays
authoritative, and the editor receives mirrored state.
