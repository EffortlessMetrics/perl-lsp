# File-Policy Rollout Burndown (non-rust)

> **Substrate (already built)**: `cargo xtask non-rust inventory` landed in #8512, with `policy/non-rust-allowlist.toml` as the authoritative ledger and generated `docs/policy/NON_RUST_INVENTORY.md` as the user-readable summary. Umbrella #8174 tracks the broader PR 3–11 ladder.
> **Connector gap**: `cargo xtask check-file-policy --mode advisory` (read-only CI surface that prints policy deltas without failing) plus `cargo xtask non-rust propose` (emits a draft TOML diff a reviewer can apply) — together they make the inventory user-trustworthy without forcing every contributor onto an all-or-nothing strict mode.
> **0.14.0 upside**: contributors can add or move non-rust files (workflows, policy docs, scripts, manifests) and immediately see what they tripped, with a precise proposal for how to register it. No more silent allowlist drift; no more "what is this file even for" review threads.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1 — advisory checker | #8566 | yes (filed by ladder agent) | — | `cargo xtask check-file-policy --mode advisory` |
| 2 — proposal generator | #8568 | yes (filed by ladder agent) | — | `cargo xtask non-rust propose` |
| 3 — per-entry tightening | #8574 (F-1), #8589 (gate wiring), #8592 (strict-mode promotion) | yes | — | `cargo xtask check-file-policy --mode strict` post-promotion |

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [ ] Receipt command in this doc reproduces the closeout proof.
- [ ] Status doc updated.
- [ ] Claim boundary recorded.

## Claim boundary

**This rail proves**: the non-rust file surface (workflows, scripts, policy docs, manifests, config) is mechanically enumerable, advisory-checkable, and proposal-driven. Contributors get tooling that explains what's in the allowlist and what's not.

**This rail does NOT prove**: that allowlist entries are *correct* — i.e., that every file genuinely belongs in the project, has a justifiable purpose, or matches the `covered_by` claim it asserts. Per-entry justification correctness is owned by the Phase 3 ladder (#8574 and successors). It also does NOT prove rust source files are policy-compliant — that surface is governed by clippy + the rust-1.95 rollout.

## Receipts

```bash
# Phase 1 receipt: advisory mode prints deltas without failing.
cargo xtask check-file-policy --mode advisory

# Phase 2 receipt: propose generates a TOML diff a reviewer can apply.
cargo xtask non-rust propose

# Phase 3 receipt (post-promotion): strict mode fails on undeclared non-rust files.
cargo xtask check-file-policy --mode strict

# Per-phase issue status.
gh issue view 8566
gh issue view 8568
gh issue view 8574
gh issue view 8589
gh issue view 8592
```

## Related

- Umbrella issue: #8174 (`policy(files): track file-policy rollout PRs 3–11`).
- Architecture / spec docs: `docs/FILE_POLICY.md`, `docs/POLICY_ALLOWLISTS.md`, `docs/policy/NON_RUST_INVENTORY.md`.
- Status doc: `docs/project/status/index.md`.
- Adjacent rails: `docs/development/CODECOV_EVIDENCE_RAIL.md` / `docs/ci/codecov-rollout.md` (Cov-6 registers `codecov.yml` under `policy/non-rust-allowlist.toml`), `docs/development/CI_UX_RAIL.md` (CI doctor will surface file-policy failures locally).

## Do not combine

- Do not combine with: Rust 1.95 lint cleanup, Codecov rollout, Perl-oracle work, dependency bumps.
- Do not bundle Phase 1 advisory mode with Phase 3 strict-mode promotion — they ship as separate PRs so reviewers can audit the noise level before tightening.
- Do not silently widen the allowlist to clear advisory output; the point of advisory mode is to surface drift, not to paper over it.

## Lane assignment

**factory-droid** owns this rail's PRs. It finalized #8512 after its P1 review and is already invested in the non-rust file policy lane. The CI-economics / non-rust file policy ladder agent (`ab3f35f479d265045`, running at file-time) is filing the Phase 1/2 entries; coordinate via search of `ci`, `policy`, and `non-rust-policy` label combinations before filing duplicates.
