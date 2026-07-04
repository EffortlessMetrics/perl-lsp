# Perl Debugger Peer Protocol — golden message fixtures

Canonical example messages for `perl-debug-peer-v1` (see
[`docs/reference/EXTERNAL_DEBUGGER_PEER_PROTOCOL.md`](../../docs/reference/EXTERNAL_DEBUGGER_PEER_PROTOCOL.md)).

Each file is one JSON message body (unframed; on the wire it is prefixed with a
`Content-Length` header). They double as documentation and as drift guards: the
`perl-dap` test `tests/peer_protocol_fixtures.rs` deserializes each into the
`perl_dap::peer_protocol` types, so a breaking change to the wire types fails a
test.

| File                            | Message                                   |
| ------------------------------- | ----------------------------------------- |
| `hello_request.json`            | peer → host `peer/hello`                   |
| `hello_response.json`           | host → peer `peer/hello` response          |
| `stopped_event.json`            | peer → host `debugger/stopped`             |
| `set_breakpoints_request.json`  | host → peer `debugger/setBreakpoints`      |
| `set_breakpoints_response.json` | peer → host `debugger/setBreakpoints` resp |
| `perl/minimal_ptkdb_peer.pl`    | copy-paste-able reference peer (core Perl only) — see [`docs/reference/PTKDB_PEER_INTEGRATION_TARGET.md`](../../docs/reference/PTKDB_PEER_INTEGRATION_TARGET.md#minimum-upstream-ptkdb-pr) |

Note: `hello_request.json` predates the optional `token` field added on
`peer/hello` in [#3404](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3404)
(`PERL_DAP_PEER_TOKEN` handshake auth) — a host that minted a session token
requires `arguments.token` to match. `minimal_ptkdb_peer.pl` sends it when set.
