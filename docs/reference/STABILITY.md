# API Stability and Version Policy

**MSRV:** 1.92 • **Edition:** 2024 • **Status:** Public alpha (`v0.12.x`)

This document is the current public-API stability contract for published artifacts in this
workspace. It is intentionally concrete so contributors can reason about "what is allowed"
before changing public types or behavior.

## Scope of the Contract

- **Published crate set:** 31 crates (the entries in
  `[workspace.metadata.publish.allow]` in the root `Cargo.toml`).
- **Current release line:** `v0.12.x` (workspace package version `0.12.4`).
- **Stricter API ratchet set:** `perl-lsp-rs`, `perl-parser`, `perl-uri`, `perl-dap`, and
  `perllsp` are guarded by both semver checks and a simplified public API baseline diff.

If this count or crate set changes, update this document in the same PR.

## Compatibility Rules (What We Promise)

### 1) Patch releases (`0.Y.Z`)

Patch releases are for bug fixes and hardening. They **must not intentionally introduce
breaking API changes** for published crates.

### 2) Minor releases (`0.Y.0`, still pre-1.0)

Because this is still `0.x`, minor releases may contain breaking changes, but they are
treated as exceptional and must include:

1. explicit release-note callouts,
2. migration guidance when user code is affected,
3. rationale for why the break is necessary now vs. delayed.

### 3) Additive evolution preference

For all published crates, prefer additive changes over replacements:

- add new APIs rather than mutating signatures,
- keep old names as deprecated shims when practical,
- mark public enums/structs `#[non_exhaustive]` where future growth is expected.

## Enforcement (How This Is Kept Honest)

### CI guardrails

- `cargo semver-checks` runs in CI for the current facade set.
- Public API baseline diff (`just public-api-check`) runs against checked-in simplified
  baseline files under `.ci/public-api-baselines/`.

### Review-time expectations

When a PR changes a published crate's public surface, include:

- the affected crate(s),
- whether change is additive or breaking,
- semver/public-api check results,
- release-note impact.

## Relationship to Other Stability Surfaces

- **LSP/DAP behavior stability:** see governance and capability docs in
  [`docs/project/FEATURE_GOVERNANCE.md`](../project/FEATURE_GOVERNANCE.md).
- **Release process + publish ordering:** see
  [`docs/release/RUNBOOK.md`](../release/RUNBOOK.md).
- **Current project posture and receipts:** see
  [`docs/project/CURRENT_STATUS.md`](../project/CURRENT_STATUS.md) and
  [`docs/project/ROADMAP.md`](../project/ROADMAP.md).

## Verification Commands

```bash
cargo test -p xtask public_api_ratchet_tests
just public-api-check
just semver-check
```
