# No-panic policy

`perl-lsp` treats panic-family calls as governed debt because the parser, LSP, DAP,
workspace index, and CI policy tools should return structured failures rather than
abort the process.

## Current rollout status

The Rust 1.95 / 0.14.0 rollout keeps the current no-panic state unchanged in the first
PR. The target state is exact counted no-new-debt enforcement, but that happens only
after the compatibility spike and MSRV/toolchain PRs have landed.

| Surface | Current | Target | Rollout step |
|---|---|---|---|
| Panic-family linting | Partly active through workspace Clippy lints and the lint ledger | Strict policy with no hidden test carveouts | Clippy and no-panic policy PRs |
| Allowlist identity | Missing or incomplete | `path + family + selector_kind + selector_callee + snippet + count` | Exact identity PR |
| Baseline mode | Not reset in docs-first PR | `no-new-debt` with generated baseline | Baseline PR only |
| Diagnostics | Existing reports only | Missing-baseline, stale-entry, delta, and blocking-mode explanations | Diagnostics PR |

## Rollout rule

Do not reset or regenerate a no-panic baseline outside the dedicated baseline PR. The
first implementation PR after the docs map is a Rust 1.95 compatibility spike, and it
must not change no-panic identity, allowlists, baselines, or policy mode.

## Target matching model

The target no-panic matcher consumes findings in this order:

1. exact allowlist count slots;
2. baseline count slots unless the policy mode is blocking;
3. remaining findings reported as new debt.

Allowlist entries should include a precise snippet and count. Coarse matching by file
or callee alone is not acceptable because it can cover unrelated panic-family calls in
the same file.
