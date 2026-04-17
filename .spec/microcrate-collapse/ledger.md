# Microcrate collapse migration ledger

**Tracking:** #4410
**ADR:** [docs/adr/0041-microcrate-collapse.md](../../docs/adr/0041-microcrate-collapse.md)
**Target:** 132 → 30 published crates
**Last updated:** 2026-04-16

Workboard for every workspace crate. Columns:

- **current crate** — crate name in current workspace
- **target owner** — crate it absorbs into (or itself if kept public)
- **final status** — `core-public` / `public-support` / `module` / `internal` / `tree-sitter`
- **wave** — migration order (0=prereq, 1-H=families, F=final)
- **risk** — Low / Med / High
- **notes** — gotchas, facade path, dev-dep re-route, etc.

Status legend:
- `core-public` — durable external product/primitive, stays published
- `public-support` — boring shared support crate kept published
- `module` — absorbs into owner crate as folder-module
- `internal` — `publish=false` (tooling/test infra)
- `tree-sitter` — tree-sitter ecosystem, kept published

---

## Prerequisites (waves 0-1)

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| (xtask publish-closure gate) | xtask | internal | 1 | Low | **MERGED** PR #4417 — adds `cargo xtask publish-closure` |
| (parser→LSP layering) | perl-parser | core-public | 0 | Low | PR #4418 in flight — removes 8 perl-lsp-* re-exports |

## Products (core-public, stay as-is)

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perllsp | perllsp | core-public | — | — | installer façade + binary |
| perl-lsp-rs | perl-lsp-rs | core-public | — | — | LSP server library + bin (dir: crates/perl-lsp) |
| perl-dap | perl-dap | core-public | H | — | DAP server; absorbs 11 perl-dap-* |
| perl-parser | perl-parser | core-public | 3-4 | — | parser facade; absorbs syntax + parser-adjacent |
| perl-lexer | perl-lexer | core-public | 3 | — | tokenizer; absorbs satellites |
| perl-token | perl-token | core-public | — | — | foundation primitive — stays published per ADR-0041 (see Amendment 4) |
| perl-semantic-analyzer | perl-semantic-analyzer | core-public | 5 | — | absorbs incremental/refactor; symbol crates go to perl-symbol (Wave B) |
| perl-symbol | perl-symbol (NEW) | core-public | B | — | absorbs 4 perl-symbol-*; own published crate (see Wave B) |
| perl-workspace-index | perl-workspace | core-public | 2 | — | renamed to perl-workspace during Wave 2; absorbs 6 perl-workspace-* |
| perl-module | perl-module (NEW name) | core-public | 1 (PILOT) | Med | absorbs 13 perl-module-*; facade not `perl-module-resolution` |
| perl-pragma | perl-pragma | core-public | 4 | Low | |
| perl-regex | perl-regex | core-public | 4 | Low | |
| perl-diagnostics-codes | perl-diagnostic-catalog | core-public | E | Low | renames; absorbs 2 more |
| tree-sitter-perl-c | tree-sitter-perl-c | tree-sitter | — | — | FFI binding |
| tree-sitter-perl-rs | tree-sitter-perl-rs | tree-sitter | — | — | ts-style facade |

## Public-support (kept published, small & stable)

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-parser-core | perl-parser-core | public-support | 4 | Low | engine internals — consider collapsing into perl-parser |
| perl-subprocess-runtime | perl-subprocess-runtime | public-support | G2 | Low | shared LSP+DAP |
| perl-lsp-perltidy | perl-lsp-perltidy | public-support | G3 | Low | formatter integration; plausibly reusable |
| perl-lsp-text-utils | perl-lsp-text-utils | public-support | G2 | Low | text edit helpers |
| perl-corpus | perl-corpus | public-support | — | Low | test corpus + generators for Perl parsers |
| perl-tdd-support | perl-tdd-support | public-support | — | Low | TDD helpers for Perl LSP ecosystem |
| perl-test-must | perl-test-must | public-support | — | Low | generic panic-on-failure helpers |
| perl-test-generators | perl-test-generators | public-support | — | Low | proptest strategies for Perl domain |

## Wave 1 PILOT — perl-module-* → perl-module

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-module-name | perl-module | module | 1 | Low | api.rs facade, pub via crate::name |
| perl-module-path | perl-module | module | 1 | Low | |
| perl-module-token | perl-module | module | 1 | Low | |
| perl-module-token-core | perl-module | module | 1 | Low | |
| perl-module-token-parser | perl-module | module | 1 | Low | |
| perl-module-boundary | perl-module | module | 1 | Low | |
| perl-module-import | perl-module | module | 1 | Low | |
| perl-module-import-match | perl-module | module | 1 | Low | |
| perl-module-reference | perl-module | module | 1 | Low | |
| perl-module-rename | perl-module | module | 1 | Low | |
| perl-module-resolution | perl-module | module | 1 | Low | internal resolution/ folder |
| perl-module-resolution-path | perl-module | module | 1 | Low | |
| perl-module-resolution-uri | perl-module | module | 1 | Low | |

## Wave 2 — perl-workspace-* → perl-workspace

Note: `perl-workspace-index` is **renamed to `perl-workspace`** during this wave. The absorbed
scope (enumeration: discovery, folder, ignore; observability: index-monitoring, index-slo,
index-state-machine) is broader than "indexing." The rename happens as part of Wave 2 execution,
not in this ledger PR.

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-workspace-folder | perl-workspace | module | 2 | Low | enumeration satellite |
| perl-workspace-ignore | perl-workspace | module | 2 | Low | enumeration satellite |
| perl-workspace-discovery | perl-workspace | module | 2 | Low | enumeration satellite |
| perl-workspace-index-monitoring | perl-workspace | module | 2 | Low | observability satellite |
| perl-workspace-index-state-machine | perl-workspace | module | 2 | Low | observability satellite |
| perl-workspace-index-slo | perl-workspace | module | 2 | Low | observability satellite |

## Wave 3 — lexer satellites → perl-lexer

`perl-token` is **NOT** absorbed. It remains a separately published foundation primitive per
ADR-0041 ("Foundation primitives (5): perl-lexer, perl-token, perl-line-index, perl-uri, perl-pod").
Wave 3 collapses only the 4 satellites below into `perl-lexer`. See Amendment 4.

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-tokenizer | perl-lexer | module | 3 | Low | |
| perl-keywords | perl-lexer | module | 3 | Low | |
| perl-builtins | perl-lexer | module | 3 | Low | |
| perl-builtins-phf | perl-lexer | module | 3 | Low | PHF lookup tables |

## Wave 4 — parser/AST satellites → perl-parser

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-ast | perl-parser | module | 4 | Med | expose via parser facade |
| perl-ast-v2 | perl-parser | module | 4 | Med | |
| perl-ast-utils | perl-parser | module | 4 | Low | |
| perl-quote | perl-parser | module | 4 | Low | |
| perl-heredoc | perl-parser | module | 4 | Low | |
| perl-heredoc-anti-patterns | perl-parser | module | 4 | Low | |
| perl-error | perl-parser | module | 4 | Low | |
| perl-incremental-parsing | perl-parser | module | 4 | Med | feature-gated |
| perl-refactoring | perl-parser | module | 4 | Med | refactor engine |
| perl-dead-code | perl-parser | module | 4 | Low | |
| perl-feature-catalog | perl-parser | module | 4 | Low | codegen build-dep; inline into owner build.rs |
| perl-line-index | perl-parser | module | 4 | Low | |
| perl-position-tracking | perl-parser | module | 4 | Low | |
| perl-pod | perl-parser | module | 4 | Low | |
| perl-qualified-name | perl-parser | module | 4 | Low | |
| perl-source-file | perl-parser | module | 4 | Low | |
| perl-percentile | perl-parser | module | 4 | Low | numeric utility |
| perl-text-line | perl-parser | module | 4 | Low | |
| perl-edit | perl-parser | module | 4 | Low | |
| perl-uri | perl-parser | module | 4 | Low | |
| perl-uri-classify | perl-parser | module | 4 | Low | fold into perl-uri |
| perl-path-normalize | perl-parser | module | 4 | Low | |
| perl-path-security | perl-parser | module | 4 | Med | security primitive |

## Wave 5 — semantic shards → perl-semantic-analyzer

*(No symbol crates — see Wave B below. The 4 perl-symbol-* crates are NOT absorbed into
perl-semantic-analyzer; they absorb into the standalone `perl-symbol` published crate instead.)*

## Wave B — perl-symbol-* → perl-symbol (NEW published crate)

`perl-symbol` is its own small published crate (see ADR-0041 "Symbol model (1): perl-symbol").
Absorbing the 4 satellites into `perl-semantic-analyzer` would invert the dependency layering:
`perl-workspace-index` and `perl-lsp` both consume symbol types directly and cannot depend on the
full semantic analyzer just to get them. `perl-symbol` stays as a separate published crate.

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-symbol-types | perl-symbol | module | B | Low | shared by perl-workspace-index, perl-semantic-analyzer, perl-lsp |
| perl-symbol-cursor | perl-symbol | module | B | Low | consumed directly by perl-lsp-rs |
| perl-symbol-index | perl-symbol | module | B | Med | |
| perl-symbol-surface | perl-symbol | module | B | Low | |

## Wave E — diagnostic catalog (NEW crate)

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-diagnostics-codes | perl-diagnostic-catalog | core-public | E | Low | renamed to `perl-diagnostic-catalog`; this row IS the new published crate |
| perl-lsp-diagnostic-catalog | perl-diagnostic-catalog | module | E | Low | retired name; content absorbs |
| perl-lsp-diagnostic-types | perl-diagnostic-catalog | module | E | Low | |

## Wave F — perl-lsp-feature-* → perl-lsp-rs::features

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-lsp-feature-ids | perl-lsp-rs | module | F | Low | |
| perl-lsp-feature-contracts | perl-lsp-rs | module | F | Low | |
| perl-lsp-feature-flags | perl-lsp-rs | module | F | Low | |
| perl-lsp-feature-profile | perl-lsp-rs | module | F | Low | |
| perl-lsp-feature-profile-cli | perl-lsp-rs | module | F | Low | preserve [[bin]] as subcommand |
| perl-lsp-feature-policy | perl-lsp-rs | module | F | Low | |
| perl-lsp-feature-grid | perl-lsp-rs | module | F | Low | |
| perl-lsp-capability-map | perl-lsp-rs | module | F | Low | |

## Wave G1 — LSP providers → perl-lsp-rs::providers

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-lsp-providers | perl-lsp-rs | module | G1 | Med | aggregation → module glue |
| perl-lsp-navigation | perl-lsp-rs | module | G1 | Med | |
| perl-lsp-completion | perl-lsp-rs | module | G1 | Med | snapshot heavy |
| perl-lsp-completion-item | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-file-completion | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-inline-completion | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-ai-provider | perl-lsp-rs | module | G1 | Med | gated by feature flags |
| perl-lsp-code-actions | perl-lsp-rs | module | G1 | Med | |
| perl-lsp-code-lens | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-document-highlight | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-document-links | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-folding | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-selection-range | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-semantic-tokens | perl-lsp-rs | module | G1 | Med | snapshot heavy |
| perl-lsp-inlay-hints | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-rename | perl-lsp-rs | module | G1 | Med | |
| perl-lsp-type-hierarchy | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-workspace-symbols | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-symbol-query | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-formatting | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-formatting-types | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-on-type-formatting | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-color-provider | perl-lsp-rs | module | G1 | Low | |
| perl-lsp-diagnostics | perl-lsp-rs | module | G1 | Med | |
| perl-lsp-import-management | perl-lsp-rs | module | G1 | Low | |

## Wave G2 — LSP runtime infra → perl-lsp-rs::runtime

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-lsp-protocol | perl-lsp-rs | module | G2 | Med | OR keep public as wire contract — orchestrator decision |
| perl-lsp-transport | perl-lsp-rs | module | G2 | Low | |
| perl-lsp-cancellation | perl-lsp-rs | module | G2 | Low | |
| perl-lsp-limits | perl-lsp-rs | module | G2 | Low | |
| perl-lsp-launcher | perl-lsp-rs | module | G2 | Low | lsp-ga-lock feature |
| perl-lsp-config | perl-lsp-rs | module | G2 | Low | |
| perl-lsp-uri | perl-lsp-rs | module | G2 | Low | |
| perl-lsp-input-validation | perl-lsp-rs | module | G2 | Low | |
| perl-content-length-framing | perl-lsp-rs | module | G2 | Low | duplicate into perl-dap or keep public (~150 LOC, shared) |

## Wave G3 — LSP governance/tooling → perl-lsp-rs

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-lsp-feature-governance | perl-lsp-rs | module | G3 | Low | |
| perl-lsp-tooling | perl-lsp-rs | module | G3 | Low | |
| perl-lsp-performance | perl-lsp-rs | module | G3 | Low | |
| perl-lsp-critic-parser | perl-lsp-rs | module | G3 | Low | perlcritic output parser |

## Wave H — perl-dap-* → perl-dap

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-dap-breakpoint | perl-dap | module | H | Low | |
| perl-dap-eval | perl-dap | module | H | Low | |
| perl-dap-config | perl-dap | module | H | Low | |
| perl-dap-platform | perl-dap | module | H | Med | cfg(unix)/cfg(windows) preserved |
| perl-dap-command-args | perl-dap | module | H | Low | preserve build.rs if present |
| perl-dap-shell | perl-dap | module | H | Low | |
| perl-dap-stack | perl-dap | module | H | Low | |
| perl-dap-types | perl-dap | module | H | Low | |
| perl-dap-value | perl-dap | module | H | Low | |
| perl-dap-security | perl-dap | module | H | Med | security primitive |
| perl-dap-variables | perl-dap | module | H | Low | |

## Standalone kernels (ADR-named public, deferred evaluation)

These were flagged in ADR-0041 as potential standalone published kernels. Re-evaluate per-wave whether to keep public or collapse:

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| perl-feature-catalog | TBD | TBD | 4 | Low | build-dep; likely collapse to perl-parser or perl-lsp-rs |
| perl-incremental-parsing | perl-parser | module | 4 | Low | decided: collapse |
| perl-refactoring | perl-parser | module | 4 | Low | decided: collapse |
| perl-dead-code | perl-parser | module | 4 | Low | decided: collapse |
| perl-heredoc-anti-patterns | perl-parser | module | 4 | Low | decided: collapse |
| perl-path-security | perl-parser | module | 4 | Low | decided: collapse |

## Internal only (publish=false)

| current crate | target owner | final status | wave | risk | notes |
|---|---|---|---:|---:|---|
| xtask | xtask | internal | — | — | tooling |
| perl-ci-hygiene | perl-ci-hygiene | internal | — | — | CI tooling |
| perl-lsp-ux-tests | perl-lsp-ux-tests | internal | — | — | UX regression harness |
| perl-parser-pest | perl-parser-pest | core-public | — | — | **keep public per user** — legacy alternate parser |
| perl-parser-bench | perl-parser-bench | internal | — | — | benchmark binary |

## Final PR

| action | wave | notes |
|---|---:|---|
| Shrink `[workspace.metadata.publish].allow` to exactly 30 | F | final PR |
| Update CLAUDE.md, docs/dependency-tiers.md | F | |
| Publish `docs/MIGRATION_v0.13.md` with full retired→new import table | F | |

---

## Progress tracker

| Wave | Status | PR | Merged |
|---|---|---|---|
| 0 (prereq: ADR + docs) | ✅ MERGED | #4413 | 2026-04-15 |
| 0 (prereq: publish-closure) | ✅ MERGED | #4417 | 2026-04-15 |
| 0 (prereq: parser layering) | 🔄 IN REVIEW | #4418 | — |
| 1 PILOT (perl-module-*) | ⏳ queued | — | — |
| 2 (perl-workspace-*) | ⏳ queued | — | — |
| 3 (lexer satellites) | ⏳ queued | — | — |
| 4 (parser satellites) | ⏳ queued | — | — |
| 5 (semantic shards, no symbol crates) | ⏳ queued | — | — |
| B (perl-symbol-* → perl-symbol NEW) | ⏳ queued | — | — |
| E (diagnostic catalog) | ⏳ queued | — | — |
| F (LSP features) | ⏳ queued | — | — |
| G1 (LSP providers) | ⏳ queued | — | — |
| G2 (LSP runtime) | ⏳ queued | — | — |
| G3 (LSP governance) | ⏳ queued | — | — |
| H (perl-dap-*) | ⏳ queued | — | — |
| FINAL (publish surface) | ⏳ queued | — | — |

**Current:** 131 workspace members, 131 publish allowlist entries (post #4413 + #4417 merge; -1 from #4387/#4388 counts?). Will recount after truth check.
