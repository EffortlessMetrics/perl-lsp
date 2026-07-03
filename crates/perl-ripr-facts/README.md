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

The **deterministic packet-fingerprint slice** (#3293 PR 7), on top of the
parser-backed `direct_owner_call` relations (PR 6), diff-owned `changes[]`
(PR 5), tests/oracles (PR 4), and files/owners (PR 3). `files[]`/`owners[]`,
`tests[]`/`oracles[]`, and `changes[]` come from parsing, not string scans;
`relations[]` classify `direct_owner_call` from parsed call nodes; and the packet
now carries a deterministic `packet_fingerprint` (an `fnv64:` content hash of the
assembled packet, reproducible for identical inputs) instead of `null`:

- `files[]` — repo-relative path, role, a deterministic FNV-1a `digest`, and the
  declared package names, for each `.pm` / `.pl` / `.psgi` / `.t` file.
- `owners[]` — one fact per `package` / `class` / `role` / `sub` / `method`
  declaration, carrying the parser's source range.
- `tests[]` — one fact per `.t` file, with the framework detected from parsed
  `use` statements (Test::More, Test2::V0/V1/Suite, Test::Exception,
  Test::Fatal — never `content.contains`) and a real full-file range.
- `oracles[]` — one fact per recognized assertion **call node** (`is`, `ok`,
  `cmp_ok`, `throws_ok`, `exception`, …), each with the call's real source range,
  a schema `kind`/`strength`, and the call's source text as `expression`. No
  string-scan counting, no placeholder `1:1` ranges.
- `changes[]` — one fact per contiguous added-line hunk of a **caller-supplied**
  unified diff (`RiprFactsRequest.diff`), attributed to the smallest enclosing
  `owners[]` fact by line containment. A hunk outside every owner, or in a file
  not parsed under `root`, is recorded as a limitation rather than force-attributed
  to a placeholder. Only the three syntactically-detectable `behavior_hint` values
  (`predicate_boundary`/`return_value`/`exception_path`) are inferred; everything
  else is `"unknown"`. No git is run — the diff is opaque text.
- `provenance[]` — `syntax` (files/owners), `test_discovery` (framework/import),
  and `oracle_extraction` (assertions) entries the facts reference by id.
- `limitations[]` — unparseable files, recognized-framework-but-no-oracle files
  (wrapped/aliased/dynamic helpers), the narrower schema representation, and the
  diff-side notes (`no-diff-supplied`, `diff-file-not-found`, `unattributable-change`,
  `diff-provenance-unverified`, range/behavior-hint precision) are all surfaced,
  never silently dropped.

This uses the clean leaf crates `perl-parser-core` (parse + `LineIndex`
byte→line/column) and `perl-symbol` (`extract_symbol_decls` /
`extract_symbol_refs`) — not `perl-workspace` (which pulls `lsp-types`).
Relations (including a heuristic `direct_owner_call`) and dynamic boundaries
remain from earlier conservative slices. The `ripr-facts` CLI does not yet
supply a diff (so `perllsp ripr-facts … --fact-classes changes` yields an empty
`changes[]` + a `no-diff-supplied` limitation); the managed-producer diff source,
the parser-backed/semantic relations that will replace the string-heuristic
`direct_owner_call`, and the packet fingerprint land in later slices.

The `perl-lsp` / `perllsp` binaries retain the `ripr-facts` subcommand as a
thin wrapper that calls [`run_ripr_facts`].
