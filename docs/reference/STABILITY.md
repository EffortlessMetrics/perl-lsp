# API Stability and SemVer Policy

**MSRV:** 1.92 • **Edition:** 2024 • **Status:** Public alpha (`0.12.x` line)

This document is the project's published compatibility contract for crates released to crates.io.
It replaces informal wording like "we try" with explicit guarantees, review gates, and expected
version-bump behavior.

## Scope

This policy applies to every crate in the workspace publish allowlist at
`[workspace.metadata.publish.allow]` in the root `Cargo.toml`.

As of workspace version `0.12.4` (2026-04-23), the allowlist contains **31 published crates**.

Contract tiers:

1. **Facade crates (highest stability expectation):** `perl-lsp-rs`, `perllsp`, `perl-parser`,
   `perl-dap`, `perl-uri`.
2. **Published support crates (stable but faster-moving):** all other allowlisted crates.
3. **Non-allowlisted crates:** internal-only; no external SemVer contract until published.

## Compatibility Guarantees

### 1) Patch releases (`0.Y.Z`)

Patch releases MUST NOT intentionally introduce breaking API changes in any published crate.
Allowed patch changes:

- bug fixes
- performance improvements
- docs/metadata updates
- internal refactors that preserve public API and behavior

### 2) Minor releases (`0.Y.0`)

While pre-1.0, minor releases MAY include breaking changes, but only when all conditions hold:

- the break is intentional and documented in changelog/release notes
- migration guidance is provided for facade-crate breaks
- SemVer checks and API baseline checks are reviewed in CI

### 3) Future 1.0+ policy (intent)

At `1.0.0`, breaking changes will move to major releases, with explicit deprecation windows.

## Enforcement in CI

Public-API compatibility is enforced by tooling, not memory:

- `cargo-semver-checks` gates detect public API breaks against the release baseline.
- facade API baselines are checked and ratcheted intentionally (no silent drift).
- publish allowlist drift is validated by CI/`just` checks.

Primary commands:

```bash
just semver-check
just semver-check-all
just public-api-check
just publish-allowlist-check
```

## Version Bump Rules

Required bump level for published crates:

| Change type | Required bump (`0.x`) |
| --- | --- |
| Remove/rename public item | minor |
| Signature/type change (incompatible) | minor |
| Trait impl removal or tighter bounds | minor |
| Behavioral break in documented contract | minor |
| Additive API (new function/type/field under compatibility rules) | minor |
| Bug fix preserving API+contract | patch |
| Internal-only refactor | patch |
| Docs/metadata only | patch |

## Contract Notes for Facade Crates

Facade crates are the main downstream integration surface. For these crates:

- breaking changes should be rare and deliberate
- release notes must include a "Migration" section for every break
- API movement from satellite crates should preserve facade import paths when feasible
- if a break is unavoidable, document old path → new path examples

## Contributor Checklist for API Changes

Before merging a PR that touches public items in a published crate:

1. Run SemVer checks (`just semver-check` or package-specific check).
2. Run facade API checks (`just public-api-check`) when a facade crate is touched.
3. Confirm version bump matches this policy.
4. Add/refresh changelog notes and migration guidance for intentional breaks.

## Source of Truth

- Publish allowlist and workspace version: `Cargo.toml`
- SemVer workflow details: `docs/how-to/SEMVER_WORKFLOW.md`
- Release process: `docs/release/RUNBOOK.md`
