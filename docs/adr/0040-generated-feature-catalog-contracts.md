# ADR-0040: Generated Feature Catalog Contracts from `features.toml`

**Status**: Accepted
**Date**: 2026-03-18
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0016](0016-feature-governance.md), [FEATURE_GOVERNANCE.md](../project/FEATURE_GOVERNANCE.md), [`features.toml`](../../features.toml)

## Context

One of the stranger architectural patterns in this repository is that LSP feature
metadata is not maintained as hand-written Rust constants and it is not loaded at
runtime from TOML either. Instead, the workspace root `features.toml` file is treated
as the single source of truth, and the `perl-lsp-feature-contracts` crate compiles that
catalog into Rust source during the build.

The code path is visible today in three places:

1. `features.toml` defines the catalog rows and coverage metadata.
2. `crates/perl-lsp-feature-contracts/build.rs` locates and validates the catalog,
   then renders `feature_contracts.rs` into `OUT_DIR`.
3. `crates/perl-lsp-feature-contracts/src/lib.rs` exposes the generated module with
   `include!(concat!(env!("OUT_DIR"), "/feature_contracts.rs"))` so downstream crates
   see normal Rust constants and functions rather than parsed TOML.

This is an unusual design choice even inside a codebase that already favors microcrates.
Most contributors expect one of two patterns:

- hand-maintained enums/const arrays checked into git, or
- runtime config parsing with dynamic validation.

The project instead uses **build-time code generation for architectural contracts**.
That behavior is described in narrative docs, but before this ADR it was not captured as
an explicit architecture decision in the ADR index.

## Decision

**We will keep `features.toml` as the canonical feature catalog and compile it into Rust
contracts at build time via `build.rs` + generated source included from `OUT_DIR`.**

This means:

- feature declaration remains centralized in one human-edited TOML file;
- validation happens during build rather than after the binary starts;
- downstream crates consume generated Rust items instead of reparsing TOML;
- the build keeps a vendored fallback path so the contracts crate can still compile when
  the workspace-root catalog is unavailable.

## Decision Drivers

1. **Single-source-of-truth discipline**: feature IDs, maturity, advertisement policy,
   tests, and coverage metadata must not drift across code, CI, and docs.
2. **Compile-time availability**: feature contracts are used by runtime code, tooling,
   tests, and reporting. Those consumers benefit from plain const/static Rust items.
3. **No runtime TOML dependency for the server hot path**: the server should not need to
   parse catalog metadata during startup just to answer capability questions.
4. **Thin-crate governance architecture**: the feature-governance family is intentionally
   decomposed; code generation lets those crates depend on typed artifacts instead of a
   shared runtime parser.
5. **Deterministic validation**: duplicate IDs, malformed rows, and catalog mistakes should
   fail the build early.

## Considered Options

### Option 1: Hand-maintained Rust constants and enums

Maintain `ALL_FEATURES`, profile metadata, and helper functions directly in Rust source.

**Pros**
- Simplest build pipeline.
- Easy for IDE navigation.
- No generated files in `OUT_DIR`.

**Cons**
- High drift risk between docs, tests, capability advertisement, and reporting.
- Every metadata change requires touching multiple Rust surfaces.
- Harder for tooling to treat the catalog as data.

### Option 2: Runtime parsing of `features.toml`

Ship the TOML file and parse it during startup or when tooling requests the catalog.

**Pros**
- Keeps metadata in one data file.
- Avoids generated Rust source.
- Dynamic reload is theoretically possible.

**Cons**
- Pushes validation later, after build time.
- Adds startup/runtime failure modes for something that should be static.
- Forces all consumers to carry parsing/error-handling paths.
- Makes low-level crates depend on runtime parsing behavior.

### Option 3: Build-time generation from `features.toml` into Rust contracts

Parse once during build, emit generated Rust, and `include!` it from the contracts crate.

**Pros**
- Preserves a single source of truth.
- Fails fast during build if the catalog is invalid.
- Gives downstream crates zero-cost access to typed/static data.
- Keeps runtime logic simple and deterministic.
- Supports tooling, CI, and server code through the same generated contract surface.

**Cons**
- `build.rs` + `include!` is less obvious to new contributors.
- Generated code can feel indirect during debugging.
- Build environment needs a predictable catalog discovery strategy.

## Decision Outcome

We choose **Option 3**.

The generated-contract approach best matches the repo's broader architecture: explicit
boundaries, data-driven policy, and compile-time enforcement where possible.

## Consequences

### Positive

- **Catalog drift is reduced**: feature metadata is edited once in `features.toml` and reused
  everywhere.
- **Consumers stay lightweight**: runtime crates work with ordinary Rust items rather than a
  parsed document model.
- **Validation is early**: bad catalog rows fail builds and CI instead of failing at runtime.
- **Tooling alignment improves**: BDD grids, compliance metrics, CLI profile handling, and
  server capability advertisement all share the same generated source.

### Negative

- **Contributor surprise**: `perl-lsp-feature-contracts` looks tiny until readers realize the
  real catalog lives in generated code.
- **IDE discoverability is weaker**: some symbols originate from `OUT_DIR`, not checked-in
  sources.
- **Build coupling**: the contracts crate must locate the root catalog or fall back to its
  vendored snapshot.

### Mitigations

- Keep this ADR linked from the ADR index and governance docs.
- Document the generation path in `docs/project/FEATURE_GOVERNANCE.md`.
- Preserve clear comments around the `include!` boundary in the contracts crate.

## Source-Grounded Evidence

The current implementation uses the following source-backed pattern:

- `features.toml` declares feature rows and metadata.
- `perl-feature-catalog` acts as the parser/renderer used during build.
- `perl-lsp-feature-contracts/build.rs` resolves the catalog path, validates the file, and
  writes generated Rust into `OUT_DIR`.
- `perl-lsp-feature-contracts/src/lib.rs` exports the generated module from `catalog` and then
  layers helper APIs such as `all_features()`, `bdd_feature_rows()`, and compliance helpers on
  top of those generated items.

This ADR intentionally documents the pattern as it exists in the tree today rather than as a
hypothetical future design.

## When to Revisit

Review this ADR if any of the following become true:

1. Feature metadata needs live reload or user-editable runtime overrides.
2. Build-time generation becomes a measurable contributor to CI latency.
3. The contracts crate stops being the narrow bridge between TOML data and Rust consumers.
4. A checked-in generated file or proc-macro approach becomes simpler than the current
   `build.rs` + `include!` path.

## References

- [ADR-0016: Feature Governance Subsystem](0016-feature-governance.md)
- [Feature Governance Architecture](../project/FEATURE_GOVERNANCE.md)
- [`crates/perl-lsp-feature-contracts/src/lib.rs`](../../crates/perl-lsp-feature-contracts/src/lib.rs)
- [`crates/perl-lsp-feature-contracts/build.rs`](../../crates/perl-lsp-feature-contracts/build.rs)
- [`features.toml`](../../features.toml)
