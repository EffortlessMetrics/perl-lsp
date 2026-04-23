# API Stability and Version Policy

**MSRV:** 1.92 • **Edition:** 2024 • **Status:** Public alpha (`0.12.x`)

This document defines the current public-API stability contract for published crates in this workspace.

## Scope of This Contract

The public crate surface is the explicit publish allowlist at `[workspace.metadata.publish.allow]` in `Cargo.toml`. That allowlist is intentionally hand-maintained and topologically ordered for release safety. At the time of writing, it contains **31 published crates**.

This contract applies to:

- all crates in the publish allowlist
- public items exported directly by those crates
- public items re-exported by facade crates (re-exports are part of the API contract)

This contract does **not** guarantee stability for:

- non-allowlisted/internal crates
- items behind unstable/internal feature flags
- undocumented implementation details not reachable through public API

## Current Alpha Posture (What `0.x` Means Here)

The project is still pre-1.0. We use semantic versioning conventions with explicit alpha caveats:

- **Patch (`0.Y.Z`)**: no intentional public API breakage for allowlisted crates
- **Minor (`0.Y+1.0`)**: breaking changes are allowed, but must be deliberate and documented
- **Major (`1.0+`)**: not active yet; stronger guarantees are targeted for the `v0.15.0` stability-contract milestone

If we intentionally break public API in a minor release, the release notes must include migration guidance.

## Facade and Re-export Policy

Facade crates are treated as first-class API boundaries. If a facade re-exports a symbol, that re-export is considered part of the public contract.

Rules:

1. Moving a type/function between internal crates is non-breaking **only** if facade exports remain source-compatible.
2. Renaming/removing a re-export is treated as a public API change and must follow the versioning rules above.
3. Public enums/structs in facade-facing APIs should be `#[non_exhaustive]` where forward-compatibility is required.

## Required Compatibility Checks

Compatibility is enforced by committed baselines plus semver checks:

- `just public-api-check` compares facade/public baselines in `.ci/public-api-baselines/`
- `just semver-check` runs `cargo-semver-checks` on the published core package set
- `just release-check` includes semver validation in the pre-release gate

When an intentional API change occurs:

1. update the API baseline (`just public-api-update`)
2. document the rationale in the PR and release notes
3. use the correct semver bump level (minor for breaking changes during alpha)

## Distribution Surfaces

| Distribution | Format | Support level |
| --- | --- | --- |
| GitHub Releases | Tagged source and binary artifacts | Alpha |
| crates.io | Published crates from allowlist | Alpha |
| VS Code extension | Marketplace / Open VSX distribution | Alpha |
| Source builds | Git checkout + Cargo | Alpha |

Availability may vary by release; check release notes for exact artifacts.

## Toward the `v0.15.0` Stability Milestone

The `v0.15.0` milestone is where the contract tightens further around:

1. deprecation windows and migration policy
2. stricter compatibility automation coverage across all published crates
3. explicit platform/runtime support commitments
4. protocol-level compatibility guarantees

## Verification

```bash
just public-api-check
just semver-check
```

For current receipts and project posture, see:

- [../project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md)
- [../project/ROADMAP.md](../project/ROADMAP.md)
