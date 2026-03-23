# Anatomy of a 120-Crate Rust Workspace

How the perl-lsp project organizes 116 workspace members, 189K lines of Rust,
and 111 published crates into a principled micro-crate architecture.

## Introduction

Most Rust workspaces contain a handful of crates. The perl-lsp project
contains 116 workspace members across 121 crate directories, organized into
seven dependency tiers and six crate families. Every crate ships to crates.io,
every crate has its own SemVer contract, and the entire workspace builds and
tests in under five minutes on commodity hardware.

This article documents how and why this architecture works, covering the
philosophy behind micro-crate decomposition, the tooling that holds it
together, and the trade-offs involved.

## The Micro-Crate Philosophy

The design principle is straightforward: **Single Responsibility Principle
(SRP) applied at the crate level**. Each crate owns one concept and exposes
a small, focused API. The heuristic for what qualifies as an SRP microcrate
is codified in the project tooling:

- 700 lines of code or fewer
- 3 or fewer Rust source files
- 8 or fewer direct dependencies

This is not a theoretical guideline. The project includes an automated
`cargo xtask srp-microcrates` command that scans every workspace member and
classifies crates into "SRP microcrates" (meeting the above criteria) and
"split candidates" (over 2,000 LOC, more than 20 dependencies, or more than
20 source files). The report drives a continuous extraction pipeline where
large crates gradually decompose into focused microcrates.

Why go this far? Three reasons:

1. **Incremental compilation**. When a developer modifies `perl-lsp-folding`
   (314 lines), only that crate and its reverse dependencies recompile. The
   parser, lexer, semantic analyzer, and other unrelated subsystems are
   untouched. With 120+ crates, the incremental recompilation unit is small.

2. **Independent SemVer**. Each crate has its own version contract. Consumers
   can depend on `perl-module-name` without pulling in `perl-lsp-providers`
   or the full parser. The publish allowlist in `Cargo.toml` tracks exactly
   111 crates that ship to crates.io.

3. **API surface control**. A crate with one `lib.rs` file and zero
   dependencies (like `perl-module-token-core` at 189 lines) has a small,
   auditable API. Breaking changes are visible in the public interface of
   a single file, not buried in a module deep inside a monolithic crate.

## Crate Family Taxonomy

The 116 workspace members fall into distinct families, each owning a domain:

### perl-module-\* (13 crates) -- Module Resolution

The module resolution pipeline is decomposed into 13 crates covering every
stage from tokenizing `use Foo::Bar qw(baz)` to resolving it on the
filesystem:

| Crate | Purpose |
|-------|---------|
| `perl-module-token-core` | Shared parser primitives for module tokens |
| `perl-module-token` | Module use/require token representation |
| `perl-module-token-parser` | Parser for module tokens from source text |
| `perl-module-name` | Validated Perl module name type |
| `perl-module-boundary` | Module boundary detection |
| `perl-module-import` | Import statement representation |
| `perl-module-import-match` | Import matching logic |
| `perl-module-path` | Module-to-filesystem path mapping |
| `perl-module-reference` | Cross-module reference tracking |
| `perl-module-rename` | Module rename refactoring |
| `perl-module-resolution` | Full module resolution pipeline |
| `perl-module-resolution-path` | Path-based resolution helpers |
| `perl-module-resolution-uri` | URI-based resolution helpers |

This decomposition means a tool that only needs to parse `use` statements
can depend on `perl-module-token` without pulling in filesystem resolution
or URI handling.

### perl-lsp-\* (41 crates) -- LSP Feature Providers

The largest family, with 41 crates covering every LSP capability:

- **Protocol and transport**: `perl-lsp-protocol`, `perl-lsp-transport`
- **Individual features**: `perl-lsp-completion`, `perl-lsp-folding`,
  `perl-lsp-rename`, `perl-lsp-navigation`, `perl-lsp-diagnostics`,
  `perl-lsp-semantic-tokens`, `perl-lsp-inlay-hints`, `perl-lsp-formatting`,
  `perl-lsp-code-actions`, `perl-lsp-document-links`,
  `perl-lsp-workspace-symbols`, `perl-lsp-on-type-formatting`
- **Shared utilities**: `perl-lsp-text-utils`, `perl-ast-utils`,
  `perl-lsp-input-validation`, `perl-lsp-symbol-query`,
  `perl-lsp-import-management`, `perl-lsp-critic-parser`
- **Infrastructure**: `perl-lsp-cancellation`, `perl-lsp-limits`,
  `perl-lsp-uri`, `perl-lsp-config`, `perl-lsp-launcher`,
  `perl-lsp-performance`, `perl-lsp-diagnostic-catalog`,
  `perl-lsp-diagnostic-types`
- **Feature governance** (8 crates, see below)

### perl-lsp-feature-\* (8 crates) -- Feature Governance Subsystem

A subset of the LSP family forming a complete feature toggle and governance
system:

| Crate | Purpose |
|-------|---------|
| `perl-lsp-feature-ids` | Canonical feature identifiers |
| `perl-lsp-feature-flags` | Runtime feature flag evaluation |
| `perl-lsp-feature-contracts` | Feature capability contracts |
| `perl-lsp-feature-profile` | Feature profile definitions |
| `perl-lsp-feature-profile-cli` | CLI parsing for feature profiles |
| `perl-lsp-feature-policy` | Feature enablement policy engine |
| `perl-lsp-feature-grid` | Feature capability matrix |
| `perl-lsp-feature-governance` | Top-level governance orchestrator |

This subsystem enables clients to selectively enable or disable LSP features,
with the policy engine enforcing constraints across the capability matrix.

### perl-dap-\* (9 crates) -- Debug Adapter Protocol

The Debug Adapter Protocol implementation is split into nine focused crates:

| Crate | Purpose |
|-------|---------|
| `perl-dap-breakpoint` | Breakpoint management |
| `perl-dap-command-args` | Command argument formatting |
| `perl-dap-eval` | Expression evaluation |
| `perl-dap-platform` | Platform abstraction |
| `perl-dap-security` | Security policy enforcement |
| `perl-dap-shell` | Shell/environment helpers |
| `perl-dap-stack` | Stack frame handling |
| `perl-dap-value` | Value representation |
| `perl-dap-variables` | Variable inspection |

### perl-ts-\* (5 crates) -- Tree-sitter Integration

Five crates provide tree-sitter integration for advanced parsing scenarios:

| Crate | Purpose |
|-------|---------|
| `perl-ts-advanced-parsers` | Advanced parsing strategies |
| `perl-ts-heredoc-analysis` | Heredoc analysis via tree-sitter |
| `perl-ts-heredoc-parser` | Heredoc-specific parser |
| `perl-ts-logos-lexer` | Logos-based lexer integration |
| `perl-ts-partial-ast` | Partial AST construction |

### perl-workspace-\* (6 crates) -- Workspace Discovery and Indexing

| Crate | Purpose |
|-------|---------|
| `perl-workspace-discovery` | Workspace root detection |
| `perl-workspace-folder` | Workspace folder abstraction |
| `perl-workspace-ignore` | Ignore-rule processing |
| `perl-workspace-index` | Full workspace symbol indexing |
| `perl-workspace-index-slo` | Indexing SLO (performance contracts) |
| `perl-workspace-index-state-machine` | Indexing lifecycle state machine |

### Core Leaf Crates

The remaining crates are standalone leaf crates with no internal workspace
dependencies:

`perl-token`, `perl-ast`, `perl-quote`, `perl-pragma`, `perl-edit`,
`perl-builtins`, `perl-builtins-phf`, `perl-regex`, `perl-heredoc`,
`perl-error`, `perl-keywords`, `perl-position-tracking`, `perl-percentile`,
`perl-content-length-framing`, `perl-subprocess-runtime`, `perl-uri`,
`perl-uri-classify`, `perl-path-normalize`, `perl-path-security`,
`perl-text-line`, `perl-line-index`,
`perl-source-file`, `perl-qualified-name`, `perl-diagnostics-codes`,
`perl-symbol-types`, `perl-symbol-cursor`,
`perl-symbol-index`

These leaf crates are the foundation of the dependency graph, and their
stability is critical: any breaking change here cascades upward through every
tier.

## The 7-Tier Dependency Architecture

The workspace Cargo.toml organizes all 116 workspace dependencies into
explicit tiers, documented in comments:

```
# Tier 1: Leaf crates (no internal dependencies)
perl-token = { path = "crates/perl-token", version = "0.10.0" }
perl-quote = { path = "crates/perl-quote", version = "0.10.0" }
...

# Tier 2: Single-level dependencies
perl-parser-core = { path = "crates/perl-parser-core", version = "0.10.0" }
perl-lsp-transport = { path = "crates/perl-lsp-transport", version = "0.10.0" }
...

# Tier 3: Two-level dependencies
perl-workspace-index = { path = "crates/perl-workspace-index", version = "0.10.0" }
...

# Tier 4: Three-level dependencies
perl-semantic-analyzer = { path = "crates/perl-semantic-analyzer", version = "0.10.0" }
perl-lsp-providers = { path = "crates/perl-lsp-providers", version = "0.10.0" }
```

The tiers continue through Tier 5 (task runner and application crates), Tier
6 (higher-level module resolution), and Tier 7 (the top-level `perl-lsp`
binary).

### What the Tiers Buy You

**Publish ordering.** The publish allowlist in `[workspace.metadata.publish]`
lists all 111 published crates in topological dependency order. The CI publish
workflow (`publish-crates.yml`) computes a topological sort via Kahn's
algorithm and publishes crates sequentially, waiting for crates.io's sparse
index to confirm each crate before publishing its dependents.

**Compilation parallelism.** Cargo's dependency resolver can compile all Tier
1 crates in parallel because they have no internal dependencies. As each tier
completes, the next tier becomes unblocked. This creates a natural pipeline
where the widest tier (Tier 1, with the most crates) benefits most from
parallel compilation.

**Change impact analysis.** A change to a Tier 1 crate like `perl-token`
potentially affects every higher tier. A change to a Tier 4 crate like
`perl-semantic-analyzer` affects only Tier 5+ consumers. The tier level gives
an instant estimate of blast radius.

### Enforcement

Tier enforcement is structural rather than procedural. The Cargo dependency
resolver enforces the DAG constraint (no cycles), and the tiered comments in
`Cargo.toml` serve as documentation. The `cargo xtask srp-microcrates`
command audits crate metrics, and clippy with workspace-level lints
(`[workspace.lints.clippy]`) enforces coding standards uniformly:

```toml
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
```

Every crate inherits these lints via `[lints] workspace = true` in its own
`Cargo.toml`, ensuring uniform enforcement without per-crate configuration.

## The Extraction Pattern: Module to Crate Pipeline

The project has a well-established pattern for extracting modules into
dedicated microcrates. The git log shows over 25 extraction PRs in recent
history, each following a consistent workflow:

### The Pattern

**Step 1: Identify the candidate.** Run `cargo xtask srp-microcrates` to
generate the split-candidate report. Crates exceeding 2,000 LOC, 20
dependencies, or 20 source files are flagged.

**Step 2: Create the new crate.** A new `crates/perl-lsp-{feature}/`
directory with:
- A `Cargo.toml` using workspace inheritance for metadata
  (`version.workspace = true`, `edition.workspace = true`, etc.)
- A single `src/lib.rs` containing the extracted code
- Dependencies declared against workspace versions

A typical extracted crate's `Cargo.toml` is minimal:

```toml
[package]
name = "perl-lsp-folding"
version = "0.10.0"
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
description = "SRP microcrate for Perl LSP folding range extraction"

[dependencies]
perl-lexer = { workspace = true }
perl-parser-core = { workspace = true }

[lints]
workspace = true
```

**Step 3: Update the workspace.** Add the new crate to the `members` list
and `[workspace.dependencies]` section of the root `Cargo.toml`. Place it in
the correct tier.

**Step 4: Re-export for backward compatibility.** The original crate
re-exports the extracted functionality so existing consumers are not broken.
Dependent crates are then migrated to use the new microcrate directly.

**Step 5: Update the publish allowlist.** Add the crate to
`[workspace.metadata.publish.allow]` in the correct topological position.

### Real Example: perl-lsp-folding

PR #1238 extracted the folding range provider from `perl-lsp-providers` into
`perl-lsp-folding`. The resulting crate is 314 lines in a single file, with
two dependencies (`perl-lexer` and `perl-parser-core`). It contains a
`FoldingRangeExtractor` struct, a `FoldingRange` type, and a
`FoldingRangeKind` enum -- nothing more.

Other recent extractions include:
- `perl-lsp-import-management` (import statement handling, PR #1168)
- `perl-lsp-completion-item` (completion item types, PR #1241)
- `perl-lsp-workspace-symbols` (workspace symbol provider, PR #1237)
- `perl-line-index` (line/column indexing, PR #1234)
- `perl-workspace-ignore` (ignore-rule processing, PR #1204)
- `perl-dap-security` (DAP security helpers, PR #1194)

## Build Performance at Scale

### Profiles

The workspace uses four Cargo profiles tuned for different scenarios:

```toml
[profile.dev]
opt-level = 1                    # Slight optimization for dev builds

[profile.release]
opt-level = 3
lto = true                       # Full link-time optimization
codegen-units = 1                # Maximum optimization, slower compile
strip = "debuginfo"              # Smaller binaries

[profile.bench]
inherits = "release"
lto = true
codegen-units = 1

[profile.dist]
inherits = "release"
lto = "thin"                     # Faster LTO for distribution builds
```

The `dev` profile uses `opt-level = 1` rather than the default 0, trading a
small amount of compile time for noticeably better runtime performance during
development.

### Caching Strategy

The CI workflow (`.github/workflows/ci.yml`) uses `Swatinem/rust-cache@v2`
with `cache-all-crates: true` and a lockfile-keyed shared cache. Because
Cargo compiles each crate independently, the build cache is granular: a
change to `perl-lsp-folding` reuses cached artifacts for all 115 other
crates.

The `.cargo/config.toml` enables the sparse registry protocol
(`registries.crates-io.protocol = "sparse"`) for faster dependency
resolution.

### Target Directory

All crates share a single `target/` directory
(`target-dir = "target"` in `.cargo/config.toml`). This avoids redundant
compilation of shared external dependencies and keeps disk usage contained.

### CI Gate Performance

The tiered gate system is designed around build time budgets:

| Tier | Target | Purpose |
|------|--------|---------|
| PR-fast | < 3 min | Catch obvious issues: format, core clippy, core tests |
| Merge gate | < 8 min | Full workspace clippy, all tests, security audit, policy checks |
| Nightly | < 60 min | Mutation testing, fuzzing, benchmarks, cross-platform matrix |

The PR-fast tier only compiles and tests `perl-parser`, `perl-lexer`, and
`perl-parser-core`, keeping feedback under three minutes. The merge gate
expands to the full workspace but still targets under eight minutes thanks
to incremental compilation and caching.

## Workspace Tooling

### xtask

The `xtask/` crate is a Tier 5 task runner providing 20+ commands for
workspace management:

- **`cargo xtask srp-microcrates`** -- Generate the microcrate inventory
  and split-candidate report
- **`cargo xtask gates`** -- Run CI gates with JSON receipt generation,
  baseline comparison, and tier selection
- **`cargo xtask publish-crates`** -- Publish all crates in topological
  order, with dry-run support
- **`cargo xtask bump-version`** -- Coordinate version bumps across
  Cargo.toml files, package.json, and source code
- **`cargo xtask features verify`** -- Verify LSP features match the
  capability catalog in `features.toml`
- **`cargo xtask parser-corpus-sweep`** -- Sweep system Perl modules
  against the parser, tracking error rates against a baseline

### justfile

The justfile provides 40+ recipes organized into tiered CI gates, individual
check targets, and convenience wrappers. Key recipes:

- `just ci-gate` (alias for `just merge-gate`) -- The canonical pre-push
  check, running format, clippy, full workspace tests, LSP smoke tests,
  security audit, and policy enforcement
- `just pr-fast` -- The fast-feedback loop for active development
- `just dead-code` / `just dead-code-strict` -- Dead code detection across
  the entire workspace
- `just semver-check` -- SemVer compatibility checking for all published
  packages
- `just health` -- Aggregate codebase health metrics
- `just coverage` -- Code coverage with `cargo-llvm-cov`

### Nix

The project uses Nix flakes for reproducible development environments. The
`flake.nix` pins Rust 1.92.0 (the project's MSRV) and provides all CI tools:
`just`, `cargo-nextest`, `cargo-audit`, `gh`, `jq`, and Python with PyYAML.
The canonical development command is:

```bash
nix develop -c just ci-gate
```

This guarantees that every developer and CI runner uses identical tooling
versions.

### Gate Policy

The `.ci/gate-policy.yaml` file is the single source of truth for all CI
gates. It defines:

- 4 tiers (pr_fast, merge_gate, nightly, release) with time budgets
- 18 individual gates with timeout, retry, and enforcement settings
- Flake management policy with quarantine rules
- Escape hatches for emergency bypasses (documented, audited, rate-limited)
- Receipt-based tracking for gate execution history

## Publishing 111 Crates to crates.io

Publishing 111 crates in the correct order is non-trivial. The project
solves this with a three-phase CI workflow:

**Phase 1: Compute order.** The `publish-crates.yml` workflow runs
`cargo metadata` and computes a topological sort of all workspace members
using Kahn's algorithm. It filters the result through the explicit allowlist
in `[workspace.metadata.publish.allow]`, which is maintained in dependency
order in the workspace Cargo.toml.

**Phase 2: Sequential publish.** Crates are published one at a time in
topological order. After each publish, the workflow polls crates.io's sparse
index (up to 5 minutes) waiting for the new version to appear before
publishing the next crate. Failed publishes are retried up to 3 times with
30-second backoff. The `--no-verify` flag is used because workspace
dev-dependency cycles can cause verification failures before all crates are
indexed.

**Phase 3: Verify.** After all publishes complete, a verification step
confirms every crate at the correct version is visible on crates.io.

The entire publish pipeline has a 120-minute timeout, accounting for
crates.io index propagation delays across 111 crates.

All crates share a single version (`0.12.0` as of this writing), managed
through workspace inheritance. The `workspace.package.version` field in the
root Cargo.toml is the single source of truth, and `cargo xtask bump-version`
updates all references in Cargo.toml files, package.json, README, and source
code version strings.

## Workspace Exclusions and Special Builds

Four directories are excluded from the default workspace:

```toml
exclude = [
    "tree-sitter-perl",           # Legacy C parser
    "crates/tree-sitter-perl-c",  # Requires libclang-dev (bindgen)
    "fuzz",                       # cargo-fuzz specialized requirements
    "archive",                    # Archived legacy components
]
```

**tree-sitter-perl-c** requires `libclang-dev` for `bindgen` to generate
Rust bindings from C headers. Including it in the default workspace would
force every developer and CI run to install libclang, even if they never
touch the C parser. Excluding it keeps the default build dependency-free of
system C libraries.

**fuzz/** uses `cargo-fuzz`, which requires nightly Rust and a specialized
build configuration. The nightly CI tier runs bounded fuzzing via proptest
integration instead, keeping the default workspace on stable Rust.

**tree-sitter-perl** is a legacy C parser kept for benchmarking and parity
testing. The `v2_parity` gate uses `--features legacy` to enable it
selectively during merge checks.

**archive/** contains archived legacy components that are no longer under
active development.

## Trade-offs and Lessons Learned

### The Cost of Many Crates

**Cargo.toml maintenance.** Every extraction creates a new Cargo.toml that
must be kept in sync with workspace settings. Workspace inheritance
(`version.workspace = true`, `edition.workspace = true`) mitigates this but
does not eliminate it. The workspace dependencies section alone is 70+ lines.

**Publish complexity.** Publishing 111 crates requires topological ordering,
index propagation waits, and retry logic. The publish workflow is
substantally more complex than a single-crate publish.

**Cognitive overhead for newcomers.** A contributor looking for "where does
folding work?" must discover that the answer spans `perl-lsp-folding`
(the extraction), `perl-lsp-providers` (the integration point), and
`perl-lsp` (the server wiring). The CLAUDE.md file and crate family
taxonomy help, but the navigation cost is real.

### What Works Well

**Incremental compilation.** The primary motivation is validated daily. A
change to a leaf microcrate results in sub-second recompilation of that
crate. The dev profile's `opt-level = 1` provides reasonable runtime
performance without the compile-time cost of full optimization.

**Forced API clarity.** When a module must become a crate, its public
interface is scrutinized. Dependencies must be explicitly declared.
Internal implementation details cannot accidentally leak. The extraction
process itself improves code quality.

**Parallel CI.** Cargo compiles Tier 1 crates (30+ crates with zero internal
dependencies) fully in parallel. Higher tiers pipeline naturally. The PR-fast
gate completes in under two minutes by testing only core crates.

**Independent versioning.** Downstream consumers can pin to specific
microcrates without taking on transitive dependencies they do not need.

## When to Micro-Crate and When Not To

The project's experience suggests these guidelines:

**Extract when:**
- A module has zero or few dependencies on its parent crate
- Multiple crates independently need the same functionality
- The module has a clear, stable API boundary
- The code is under 700 LOC and can stand alone
- You want to publish the functionality independently

**Do not extract when:**
- The module is tightly coupled to its parent's internal state
- The extraction would create circular dependencies
- The module is unstable and changing rapidly (extract after stabilization)
- The overhead of a new Cargo.toml and publish entry outweighs the benefit

The SRP microcrate report (`cargo xtask srp-microcrates`) automates the
identification of extraction candidates, but human judgment determines
whether the extraction is worthwhile. Not every module that *can* be a
crate *should* be one.
