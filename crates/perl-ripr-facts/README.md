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
LSP provider/transport runtime. Parser-backed extraction uses the clean leaf
crates `perl-parser-core` and `perl-symbol`; `perl-workspace` is deliberately
avoided because it transitively pulls `lsp-types`.

## API

Two entry points:

- **`build_ripr_facts_packet(&RiprFactsRequest) -> Result<serde_json::Value,
  RiprFactsError>`** — the structured batch API. It validates the request,
  runs the emitter, and returns the assembled `ripr-perl-facts-v1` packet.
  It performs **no I/O**: no disk write, no stderr, no process-exit mapping.
- **`run_ripr_facts(schema, root, base, head, fact_classes, out) -> i32`** —
  the thin CLI wrapper the `perl-lsp` / `perllsp` `ripr-facts` subcommand
  calls. It forwards its args to `build_ripr_facts_packet`, then validates the
  output path, writes the packet to `out`, and maps the outcome to a process
  exit code (`0` success, `1` on any validation or write failure).

```rust
use perl_ripr_facts::{build_ripr_facts_packet, RiprFactsRequest};

let packet = build_ripr_facts_packet(&RiprFactsRequest {
    schema: "ripr-perl-facts-v1",
    root: "crates/perl-parser",
    base: None,
    head: Some("HEAD"),
    fact_classes: "tests,oracles,relations",
})?;
assert_eq!(packet["schema_version"], "ripr-perl-facts-v1");
```

The packet `build_ripr_facts_packet` returns is byte-identical to what the CLI
wrapper writes for the same inputs.

## Status

The **parser-backed files/owners slice** (#3293 PR 3). `files[]` and `owners[]`
are now populated by parsing every Perl source/test file under the root:

- `files[]` — repo-relative path, role, a deterministic FNV-1a `digest`, and the
  declared package names, for each `.pm` / `.pl` / `.psgi` / `.t` file.
- `owners[]` — one fact per `package` / `class` / `role` / `sub` / `method`
  declaration, carrying the parser's source range.

This uses the clean leaf crates `perl-parser-core` (parse + `LineIndex`
byte→line/column) and `perl-symbol` (`extract_symbol_decls`) — not
`perl-workspace`. Tests/oracles, relations, and dynamic boundaries remain from
the earlier conservative string-scan slices; diff-derived `changes[]` land in a
later slice.

The `perl-lsp` / `perllsp` binaries retain the `ripr-facts` subcommand as a
thin wrapper that calls [`run_ripr_facts`].
