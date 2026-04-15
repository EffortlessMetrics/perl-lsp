# Publishing After the Microcrate Collapse

This document describes the simplified publishing pipeline that will be in place after the
microcrate collapse lands (tracked by [issue #4410](https://github.com/EffortlessMetrics/perl-lsp/issues/4410)).
For the current 132-crate pipeline, see [docs/PUBLISHING.md](../PUBLISHING.md).

See [ADR-0041](../adr/0041-microcrate-collapse.md) for the architectural rationale.

## 31 published crates in topological order

The post-collapse published set, grouped by category and listed in approximate dependency order
(leaves first, products last):

### Foundation primitives (5)

| Crate | Role |
|-------|------|
| `perl-token` | Token type definitions — leaf, no perl-lsp deps |
| `perl-line-index` | UTF-16 line/column index for LSP position math |
| `perl-uri` | Guaranteed-valid URI wrapper (ADR-0037) |
| `perl-pod` | POD documentation parser and renderer |
| `perl-lexer` | Context-aware tokenizer; depends on perl-token |

### Wire protocols (2)

| Crate | Role |
|-------|------|
| `perl-content-length-framing` | Content-Length framing for LSP/DAP transports |
| `perl-lsp-protocol` | LSP request/response/notification type definitions |

### Diagnostic surface (1)

| Crate | Role |
|-------|------|
| `perl-diagnostic-catalog` | NEW — absorbs perl-lsp-diagnostics, perl-lsp-anti-patterns, and related crates; unified diagnostic definition + severity surface |

### Alternate parser (1)

| Crate | Role |
|-------|------|
| `perl-parser-pest` | Pest-grammar based parser (v2 legacy, kept for benchmarking) |

### Tree-sitter bindings (2)

| Crate | Role |
|-------|------|
| `tree-sitter-perl-c` | C-based tree-sitter grammar (conventional binding) |
| `tree-sitter-perl-rs` | v3 parser facade with tree-sitter ergonomics |

### Symbol model (1)

| Crate | Role |
|-------|------|
| `perl-symbol` | Unified symbol type; absorbs perl-symbol-kind, perl-symbol-table, perl-symbol-visibility, perl-symbol-index |

### Semantic kernels (3)

| Crate | Role |
|-------|------|
| `perl-module-resolution` | Module name → file path resolution; absorbs 13 perl-module-* crates |
| `perl-workspace-index` | Workspace symbol index and discovery; absorbs 7 perl-workspace-* crates |
| `perl-semantic-analyzer` | Scope analysis, type inference, variable resolution |

### Tool integrations (1)

| Crate | Role |
|-------|------|
| `perl-lsp-perltidy` | perltidy formatting integration |

### Test and corpus ecosystem (4)

| Crate | Role |
|-------|------|
| `perl-test-must` | `must`/`must_some` test assertion helpers |
| `perl-tdd-support` | TDD support utilities for tests |
| `perl-test-generators` | Property-based and generative test helpers |
| `perl-corpus` | CPAN corpus fixtures and test data |

### Standalone tooling kernels (6)

| Crate | Role |
|-------|------|
| `perl-feature-catalog` | Feature governance catalog (ADR-0040) |
| `perl-incremental-parsing` | Incremental parse update infrastructure (ADR-0010) |
| `perl-refactoring` | Refactoring engine (rename, extract) |
| `perl-dead-code` | Dead code analysis |
| `perl-heredoc-anti-patterns` | Heredoc anti-pattern detection |
| `perl-path-security` | Path traversal and security guards (ADR-0019) |

### Products (4)

| Crate | Role |
|-------|------|
| `perl-parser` | Main recursive-descent parser (v3) |
| `perl-dap` | Debug Adapter Protocol server binary |
| `perl-lsp-rs` | LSP server library |
| `perllsp` | LSP server binary (the thing editors install) |

## Publish workflow simplification

Before the collapse, the publish pipeline required:

- **Tarjan SCC for dev-dep cycles** — `scripts/publish-topo.py` identified strongly-connected
  components in the dev-dependency graph and published them in SCC-merged order. With 31 crates,
  the dep graph is shallow and acyclic; this code becomes dead.
- **Rate-limit retries** — publishing 132 crates sequentially hit crates.io rate limits.
  31 crates publishes in under a minute; rate limits are not a concern.
- **Partial-publish recovery** — a failed publish at crate #80 of 132 required careful
  resume logic. With 31 crates, a full re-run is cheap if anything goes wrong.
- **Allowlist drift detection** — the allowlist in `[workspace.metadata.publish].allow`
  had to be manually maintained across 132 entries; new crates were silently excluded.
  31 entries is auditable by hand.

After the collapse, the workflow is:

1. Bump versions on the handful of crates that changed.
2. Run `cargo xtask publish-closure` to verify the closure is clean.
3. Run `cargo xtask layer-check` to verify no layering violations.
4. Publish in topological order — no SCCs, no rate-limit handling needed.

## xtask gates

Three new xtask commands guard the post-collapse invariants. These are being added in
PR #1 of the collapse series (tracked by [issue #4412](https://github.com/EffortlessMetrics/perl-lsp/issues/4412)):

| Command | What it checks |
|---------|----------------|
| `cargo xtask publish-closure` | Verifies no `publish = false` crate appears in the runtime/build dependency closure of any published crate |
| `cargo xtask layer-check` | Enforces dependency direction rules from `xtask/layer-rules.toml` at the import level inside crates |
| `cargo xtask published-crate-count` | Ratchet — fails if the published crate count exceeds the target ceiling (~31), preventing count creep |

## Comparison: before vs after

| Metric | Before (132 crates) | After (~31 crates) |
|--------|--------------------|--------------------|
| Publish run time | Hours | Minutes |
| Topo sort complexity | Non-trivial (SCCs) | Trivial (DAG, no cycles) |
| Allowlist entries | 132 | ~31 |
| Rate-limit handling | Required | Not needed |
| Per-release version bumps | Workspace-wide cascade | Handful of crates |
| docs.rs pages | 132 | ~31 |
| crates.io search results | 132 | ~31 |
| Semver contracts | 132 | ~31 |

## Reference

- [ADR-0041](../adr/0041-microcrate-collapse.md) — architectural rationale
- [Tracking issue #4410](https://github.com/EffortlessMetrics/perl-lsp/issues/4410) — collapse wave progress
- [docs/PUBLISHING.md](../PUBLISHING.md) — current 132-crate pipeline (superseded post-collapse)
- [docs/MIGRATION_v0.14.md](../MIGRATION_v0.14.md) — migration guide for downstream users
