# UX Closeouts — Queue Page

This page is the canonical queue of user-visible UX lanes agents can pick up.
Each row points at the next issue or PR for the lane, the receipt that
demonstrates progress, and the status doc that holds the authoritative record.

> **Rule**: Indexes provide candidates. Context determines authority.
>
> This page is a candidate list for agents to triage; it is **not** a contract.
> Authority remains with the linked status docs and receipt PRs. If this page
> and a status doc disagree, the status doc wins.

## Lanes

| Lane | User problem | Current state | Next issue / PR | Receipt | Status doc |
|---|---|---|---|---|---|
| **@INC** | Module resolution divergence between LSP consumers (completion vs PL701 vs goto vs hover); no-`use lib` consumers leaking | Closeouts landed (#8540 final no-lib workspace-index strictness; #8544 workspace-symbol filter through `EffectiveIncContext`) | [#8573](https://github.com/EffortlessMetrics/perl-lsp/issues/8573) (docs refresh) | [#8553](https://github.com/EffortlessMetrics/perl-lsp/pull/8553) classification table | [`status/module_resolution.md`](status/module_resolution.md) |
| **Completion latency** | Repeat prefix scans on namespace completion cause perceptible lag on large workspaces | Prefix-directed scan landed (#8498); runtime-owned TTL cache pending | [#8514](https://github.com/EffortlessMetrics/perl-lsp/issues/8514) | pending | [`status/lsp.md`](status/lsp.md) |
| **Literal require/import** | `require "Foo/Bar.pm"` and string-interp `import` forms don't resolve consistently | Umbrella open; no spec yet | [#4280](https://github.com/EffortlessMetrics/perl-lsp/issues/4280) (spec PR to follow) | pending | [`status/module_resolution.md`](status/module_resolution.md) |
| **DAP modules** | DAP module resolution and breakpoint UX gaps when modules live outside workspace | DAP scout in progress | TBD | pending | [`status/dap.md`](status/dap.md) |
| **Install UX** | First-run install instructions, MSRV mismatch, post-install probe failures | Install docs roadmap open | TBD | pending | [`status/release.md`](status/release.md) |
| **CI UX** | CI failure summaries are noisy; agents and humans both burn tokens parsing them | CI economics agent in flight | TBD | pending | [`status/ci_hardening.md`](status/ci_hardening.md) |
| **Local CI parity** | `just pr-fast` ≠ remote CI; agents claim "green locally" then CI fails | Partial — gate tiers documented, drift remains | TBD | pending | [`status/ci_hardening.md`](status/ci_hardening.md) |
| **File policy** | Non-Rust files (docs, fixtures, scripts) bypass quality gates | Non-Rust file policy agent in flight | TBD | pending | [`status/quality.md`](status/quality.md) |
| **Rust 1.95 quality** | Strong-clippy lints, MSRV alignment, post-1.95 dead-code cleanup | Rust 1.95 rollout agents in flight | TBD | pending | [`status/quality.md`](status/quality.md) |

## How to use this page

- **Triaging**: pick a lane with a populated "Next issue / PR" cell and a stale
  receipt. The linked issue is your spec entry point.
- **Closing a row**: when a lane closes out, update the row in a follow-up PR
  with the receipt PR and a link to the status-doc section that records it.
- **Adding a lane**: open a PR that adds a row here *and* a section to the
  relevant status doc. Lanes without a status doc anchor don't belong here.

## Cross-references

- [`CURRENT_STATUS.md`](CURRENT_STATUS.md) — top-level project status (stable stub)
- [`ROADMAP.md`](ROADMAP.md) — multi-quarter direction
- [`status/index.md`](status/index.md) — subsystem status fan-out
