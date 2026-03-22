# Feature Governance Architecture

This document explains how the Perl LSP manages its LSP feature catalog through a
layered crate architecture called "feature governance." It covers why the system
exists, how the crates fit together, how features flow from declaration to runtime
capability, and how to add new features.

> Architectural note: the build-time `features.toml` → generated Rust contract pipeline is now formalized in [ADR-0040](../adr/0040-generated-feature-catalog-contracts.md).

## Why Feature Governance Exists

The Perl LSP advertises 80+ features spanning LSP text document operations,
workspace services, window notifications, notebook support, debug adapter
protocol (DAP) capabilities, and protocol lifecycle methods. Each feature
carries metadata: its LSP spec version, functional area, maturity level,
whether it is advertised to clients, associated test files, and whether it
counts toward compliance metrics.

Managing this catalog by hand -- scattered across server initialization code,
test assertions, documentation, and CI gates -- would inevitably lead to drift.
A feature could be implemented but never advertised, or advertised but lacking
tests, or tracked for compliance in one place but not another.

Feature governance solves this with two core principles:

1. **Single source of truth.** `features.toml` at the workspace root declares
   every feature once. Build scripts compile this into Rust constants at build
   time, so the runtime catalog is always derived from the same file.

2. **Separation of concerns across thin crates.** Each crate in the governance
   stack owns one responsibility -- identifiers, flags, profiles, policy,
   contracts, capability mapping, grid reporting, or CLI parsing. This avoids
   circular dependencies and allows the LSP binary, xtask tooling, and CI
   scripts to depend on exactly the layer they need.

## Architecture Overview

The data flow from declaration to runtime looks like this:

```
features.toml
    |
    v
perl-feature-catalog          (parse TOML, validate, render Rust modules)
    |
    v
perl-lsp-feature-contracts    (build.rs compiles catalog into const arrays;
    |                           defines BddFeatureRow, FeatureProfileKind)
    |
    +---> perl-lsp-feature-ids        (stable &str constants for each feature)
    |
    +---> perl-lsp-capability-map     (feature IDs <-> lsp_types::ServerCapabilities)
    |
    v
perl-lsp-feature-flags        (BuildFlags / AdvertisedFeatures structs;
    |                           production(), ga_lock(), all() presets)
    |
    v
perl-lsp-feature-profile      (parse CLI tokens into FeatureProfileKind)
    |
    v
perl-lsp-feature-policy       (FeatureProfile enum; resolves profile + runtime
    |                           tooling checks into BuildFlags)
    |
    v
perl-lsp-feature-grid         (JSON payload for BDD matrices, compliance %)
    |
    v
perl-lsp-feature-governance   (facade re-exporting all of the above)
    |
    v
perl-lsp / perl-lsp-launcher  (server binary consumes the facade)
```

### `features.toml` -- the Single Source of Truth

Every feature is declared as a `[[feature]]` entry with these fields:

| Field | Purpose |
|-------|---------|
| `id` | Canonical identifier (e.g. `lsp.completion`, `dap.core`) |
| `spec` | LSP/DAP spec version where the feature was introduced |
| `area` | Functional grouping: `text_document`, `workspace`, `window`, `notebook`, `debug`, `protocol` |
| `maturity` | Lifecycle stage: `planned`, `experimental`, `preview`, `ga`, `production` |
| `advertised` | Whether the server announces this capability to clients |
| `counts_in_coverage` | Whether this feature is included in compliance math |
| `tests` | Paths to test files exercising the feature |
| `description` | Human-readable summary |

The `[meta]` section records the catalog version, target LSP spec version,
and the computed compliance percentage.

## Crate Responsibilities

| Crate | Path | Role |
|-------|------|------|
| `perl-feature-catalog` | `crates/perl-feature-catalog/` | Parses `features.toml` into a `Catalog` struct. Validates uniqueness and field constraints. Renders Rust source modules for both LSP and DAP feature arrays. Used as a build dependency. |
| `perl-lsp-feature-ids` | `crates/perl-lsp-feature-ids/` | Defines `pub const` string constants for every feature identifier (`LSP_COMPLETION`, `LSP_HOVER`, `DAP_CORE`, etc.). Zero dependencies beyond `std`. Prevents typo-based identifier drift. |
| `perl-lsp-capability-map` | `crates/perl-lsp-capability-map/` | Bidirectional translation between feature ID strings and `lsp_types::ServerCapabilities`. `feature_ids_from_caps()` extracts IDs from a capabilities struct; `caps_from_feature_ids()` builds capabilities from IDs. |
| `perl-lsp-feature-flags` | `crates/perl-lsp-feature-flags/` | Defines `BuildFlags` (per-feature booleans for compile-time selection) and `AdvertisedFeatures` (runtime projection). Provides preset constructors: `production()`, `ga_lock()`, `all()`. Converts flags to feature ID vectors. |
| `perl-lsp-feature-contracts` | `crates/perl-lsp-feature-contracts/` | Runs a `build.rs` that compiles `features.toml` into `feature_contracts.rs` constants via `perl-feature-catalog`. Defines `FeatureProfileKind` (GaLock, Production, All) and `BddFeatureRow` for reporting. Exposes `all_features()`, `has_feature()`, `compliance_percent()`. |
| `perl-lsp-feature-profile` | `crates/perl-lsp-feature-profile/` | Parses raw CLI profile tokens (`"ga-lock"`, `"prod"`, `"all"`, `"auto"`) into `FeatureProfileKind`. Handles normalization (trimming, case folding, underscore-to-hyphen). |
| `perl-lsp-feature-policy` | `crates/perl-lsp-feature-policy/` | Defines `FeatureProfile` and resolves it into `BuildFlags`. Handles runtime environment effects (e.g. enabling formatting when `perltidy` is available). Provides `catalog_advertised_feature_ids()` which intersects profile flags with the catalog. |
| `perl-lsp-feature-grid` | `crates/perl-lsp-feature-grid/` | Assembles the BDD feature grid JSON payload. Computes per-profile compliance percentages. Consumed by `xtask`, CI reporting, and the server's feature catalog endpoint. |
| `perl-lsp-feature-profile-cli` | `crates/perl-lsp-feature-profile-cli/` | Parses `--feature-profile` CLI arguments. Returns structured `UnsupportedFeatureProfileError` with the supported token list for user diagnostics. |
| `perl-lsp-feature-governance` | `crates/perl-lsp-feature-governance/` | Facade crate that re-exports the public API surface from all governance sub-crates. The LSP server binary and launcher depend on this single crate rather than each sub-crate individually. |

## Feature Profiles

Profiles control which capabilities the server advertises. Three canonical
profiles are defined:

| Profile | Constructor | Description |
|---------|------------|-------------|
| `ga-lock` | `BuildFlags::ga_lock()` | Conservative set. Excludes `inline_values`. Includes formatting. Intended for stable API guarantees. |
| `production` | `BuildFlags::production()` | Standard runtime set. Excludes formatting (enabled dynamically when `perltidy` is detected). All other GA features enabled. |
| `all` | `BuildFlags::all()` | Every in-tree feature enabled. Used for testing, snapshots, and CI matrices. |

Profile selection happens through:

1. **Compile-time default:** The `lsp-ga-lock` Cargo feature propagates through
   the crate chain. When enabled, `FeatureProfileKind::current()` returns
   `GaLock`; otherwise it returns `Production`.

2. **CLI override:** The `--feature-profile` flag accepts tokens like `ga`,
   `ga-lock`, `ga_lock`, `prod`, `production`, `all`, or `auto` (which falls
   back to the compiled default).

3. **Runtime adaptation:** `FeatureProfile::runtime_flags()` checks for external
   tool availability. For example, formatting is enabled only when `perltidy` is
   installed, regardless of the base profile.

## Contracts and Compliance

The contracts crate acts as the bridge between the TOML catalog and the Rust
type system. At build time, its `build.rs` invokes `perl-feature-catalog` to:

1. Locate `features.toml` (checking `FEATURES_TOML_OVERRIDE` env var, then the
   workspace root, then a vendored `features_sot.toml` fallback).
2. Parse and validate the catalog (no empty IDs, no duplicates).
3. Render a generated Rust module with:
   - `ALL_FEATURES: &[Feature]` -- every feature row as a const array.
   - `ADVERTISED_LSP_FEATURES: &[&str]` -- IDs for GA/production features with
     `advertised = true`.
   - `has_feature()`, `compliance_percent()`, `advertised_features()` functions.

This generated module is included via `include!(concat!(env!("OUT_DIR"), "/feature_contracts.rs"))`,
giving downstream crates compile-time access to the complete feature catalog
without runtime TOML parsing.

Compliance is computed as:

```
compliance % = advertised_trackable_features / trackable_features * 100
```

Where "trackable" means `maturity != planned` and `counts_in_coverage == true`.
Features like protocol lifecycle methods (`lsp.initialize`, `lsp.shutdown`) and
window notifications set `counts_in_coverage = false` because they are
infrastructure, not user-facing language features.

## Feature Lifecycle

A feature progresses through maturity stages:

### `planned`

The feature is declared in `features.toml` with `maturity = "planned"` and
`advertised = false`. It does not count toward compliance. This is the starting
point for tracking work items.

### `experimental`

An initial implementation exists. The feature may be included in the `all`
profile for testing but is not advertised in production profiles. It counts
toward trackable metrics but not advertised metrics.

### `preview`

The implementation is functional and under active testing. It may be advertised
in the `all` profile. Feedback from early adopters can still drive API changes.

### `ga` (General Availability)

The feature is stable, tested, and advertised to clients. It appears in all
profiles (including `ga-lock`). Breaking changes require a major version bump.
This is the target maturity for most LSP capabilities.

### `production`

Equivalent to `ga` for compliance and advertising purposes. Used when a feature
has been running in production for an extended period with no issues.

## How to Add a New LSP Feature

### Step 1: Declare in `features.toml`

Add a `[[feature]]` entry:

```toml
[[feature]]
id = "lsp.new_feature"
spec = "LSP 3.18"
area = "text_document"
maturity = "preview"
advertised = false
tests = ["tests/lsp_new_feature_tests.rs"]
description = "Description of the new feature"
```

### Step 2: Add a feature ID constant

In `crates/perl-lsp-feature-ids/src/lib.rs`, add:

```rust
/// New feature identifier.
pub const LSP_NEW_FEATURE: &str = "lsp.new_feature";
```

### Step 3: Register in BuildFlags

In `crates/perl-lsp-feature-flags/src/lib.rs`:

1. Add a `pub new_feature: bool` field to `BuildFlags`.
2. Add a corresponding field to `AdvertisedFeatures` if it will be client-visible.
3. Wire the flag in `to_advertised_features()` and `to_feature_ids()`.
4. Set it to `true` in the appropriate profile constructors (`all()` first,
   then `production()` and `ga_lock()` when it reaches GA).

### Step 4: Register in capability map

In `crates/perl-lsp-capability-map/src/lib.rs`:

1. Add a match arm in `caps_from_feature_ids()` to set the relevant
   `ServerCapabilities` field.
2. Add a check in `feature_ids_from_caps()` to extract the ID from capabilities.

### Step 5: Implement the LSP provider

Create or update the provider crate (e.g. `crates/perl-lsp-new-feature/`) and
register it in the server's request routing.

### Step 6: Write tests

Add test files referenced in the `tests` array in `features.toml`. The BDD grid
will track whether the feature has associated test coverage.

### Step 7: Promote maturity

When the feature is stable, update `features.toml`:

```toml
maturity = "ga"
advertised = true
```

Then enable it in the `production()` and `ga_lock()` `BuildFlags` constructors.
The compliance percentage will automatically update on the next build.

## Dependency Graph

The governance crates form a strict layered DAG:

```
Tier 1 (leaf, no internal deps):
  perl-lsp-feature-ids

Tier 2 (single internal dep):
  perl-lsp-capability-map        -> perl-lsp-feature-ids
  perl-feature-catalog           (standalone, build-time only)

Tier 3:
  perl-lsp-feature-contracts     -> perl-lsp-feature-ids
                                    perl-lsp-capability-map
                                    perl-feature-catalog (build dep)

  perl-lsp-feature-flags         -> perl-lsp-feature-ids

  perl-lsp-feature-profile       -> perl-lsp-feature-contracts

Tier 4:
  perl-lsp-feature-policy        -> perl-lsp-feature-contracts
                                    perl-lsp-feature-profile
                                    perl-lsp-feature-flags

  perl-lsp-feature-profile-cli   -> perl-lsp-feature-policy
                                    perl-lsp-feature-profile

Tier 5:
  perl-lsp-feature-grid          -> perl-lsp-feature-contracts
                                    perl-lsp-feature-policy

Tier 6 (facade):
  perl-lsp-feature-governance    -> perl-lsp-feature-contracts
                                    perl-lsp-feature-grid
                                    perl-lsp-feature-policy
                                    perl-lsp-feature-profile
                                    perl-lsp-feature-profile-cli
```

This layering means that leaf crates like `perl-lsp-feature-ids` compile
quickly and can be depended on by any crate without pulling in `lsp-types`,
`serde`, or other heavier dependencies. Only the crates that need those
dependencies pay the compile-time cost.

## Related Documentation

- [features.toml](../../features.toml) -- the canonical feature catalog
- [CURRENT_STATUS.md](CURRENT_STATUS.md) -- computed compliance metrics
- [STABILITY.md](../reference/STABILITY.md) -- API stability policy
- [LSP_IMPLEMENTATION_GUIDE.md](../reference/LSP_IMPLEMENTATION_GUIDE.md) -- server architecture
