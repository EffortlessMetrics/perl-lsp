# Policy allowlists

Policy allowlists are receipts for intentionally retained risk. They should be
narrow, owned, dated, reviewable, and enforced by repository tooling before they
become blocking gates.

## Rust 1.95 rollout targets

The Rust 1.95 / 0.14.0 rollout maps these allowlist families without activating
them in the documentation PR:

| Family | Current state | Target |
|---|---|---|
| Clippy lint debt | `policy/clippy-lints.toml` and `policy/clippy-debt.toml` govern active/tracked/planned lints. | Activate Rust 1.93/1.95 floors in dedicated PRs and record retained exceptions with narrow receipts. |
| No-panic | Exact counted allowlist/baseline is missing or incomplete. | Match by path, family, selector kind, selector callee, snippet, and count; enforce no-new-debt after baseline. |
| Non-Rust files | Missing/incomplete file ledger. | Add blocking non-Rust allowlist with required owner, reason, surface, classification, coverage, and review dates. |
| Companion file risks | Not yet represented as separate ledgers. | Add generated, executable, dependency, workflow, process, and network allowlists. |
| ripr | Advisory suppressions exist through `policy/ripr-suppressions.toml` when configured. | Keep advisory while improving routing and artifact consistency. |

## Required allowlist posture

New allowlists should avoid broad anonymous globs. Each retained exception needs
an owner, reason, created date, review date, and evidence of the gate or test
that covers it. Broad globs need an explicit broad-glob reason.

See [Rust 1.95 / 0.14.0 rollout map](ci/perl-lsp-rust-1.95-rollout.md) for the
implementation order. Documentation-only rollout mapping must not create a new
baseline or silently absorb new debt.
