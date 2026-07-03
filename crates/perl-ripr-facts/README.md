# perl-ripr-facts

Batch exporter for the `ripr-perl-facts-v1` packet.

This crate produces the deterministic facts packet consumed by the
[`ripr`](https://github.com/EffortlessMetrics/ripr) swarm: test facts, oracle
facts, relations, dynamic-boundary facts, and typed verify-command candidates
extracted from a Perl workspace.

## Why a separate crate

The exporter deliberately lives **below** the LSP server runtime and **above**
the raw parser. It must not drag in the editor stack (`perl-lsp-rs`,
`perl-dap`, `lsp-types`, the JSON-RPC transport, the async provider runtime),
because RIPR fact production is a batch, deterministic projection of workspace
semantics — not an interactive editor feature.

Dependency contract (enforced by review): this crate must **not** depend on
`perllsp`, `perl-lsp-rs`, `perl-lsp-rs-core`, `perl-dap`, `lsp-types`, or the
LSP provider/transport runtime.

## Status

This is the **relocation slice**. The emitter body is still the conservative
string-scan implementation that previously lived in
`perl-lsp-rs::ripr_facts_emitter`; it was moved here behavior-preserving so the
architectural home is correct before the evidence layer is upgraded to
`perl-workspace` / `perl-semantic-facts`-backed facts.

The `perl-lsp` / `perllsp` binaries retain the `ripr-facts` subcommand as a
thin wrapper that calls [`run_ripr_facts`].
