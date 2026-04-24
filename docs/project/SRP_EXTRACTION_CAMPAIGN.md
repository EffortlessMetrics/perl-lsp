# The SRP Microcrate Extraction Campaign

*How perl-lsp went from 2 crates to 121 in seven months, driven almost
entirely by AI agents.*

---

## The Starting Point: A Monolithic Parser

The first tagged release, `v0.1.0-pest` (July 2022), contained exactly
**2 crates** -- both tree-sitter bindings.  By `v0.5.0` (August 2025) the
project had grown to **6 crates**: the parser, the lexer, a test harness,
two tree-sitter wrappers, and a benchmark harness.  The LSP server, DAP
adapter, semantic analyzer, workspace index, and refactoring engine all
lived inside `perl-parser` or were being built inside the nascent
`perl-lsp` binary crate.

At `v0.8.3` (August 23, 2025) the count was still just **8 crates**.
Everything changed between `v0.8.3` and `v0.9.1`.

---

## The Campaign Timeline

| Tag / Date | Crate Count | Delta | Notes |
|------------|:-----------:|:-----:|-------|
| `v0.1.0-pest` (Jul 2022) | 2 | -- | Tree-sitter bindings only |
| `v0.5.0` (Aug 3, 2025) | 6 | +4 | Parser, lexer, tests, benchmarks |
| `v0.8.3` (Aug 23, 2025) | 8 | +2 | Corpus crate, Pest parser archive |
| `v0.9.1` (Feb 20, 2026) | 53 | **+45** | Massive extraction campaign |
| `v0.10.0` (Feb 28, 2026) | 85 | **+32** | Second wave |
| HEAD (Mar 11, 2026) | **121** | **+36** | Third wave, still ongoing |

The most intense period was **March 5, 2026**, when 93 extraction-related
commits landed in a single day.  Between `v0.8.3` and the current HEAD,
1,729 commits were made -- roughly 507 of which (29%) mention extraction,
SRP, or microcrate work.

---

## The Extraction Philosophy: Why Micro-Crates?

The project documentation (`AGENTIC_DEV.md`) describes an "AI-native"
development model where high-throughput changes are verified by mechanical
gates rather than manual inspection.  The SRP microcrate campaign follows
directly from this philosophy:

1. **Smaller compilation units** -- a change to `perl-lsp-folding`
   (315 lines) does not trigger recompilation of `perl-lsp-providers`
   (2,100+ lines) or any of its other dependents.

2. **Independent publishability** -- each microcrate has its own
   `Cargo.toml` with full crates.io metadata, enabling selective
   publishing.  The workspace publish allowlist in `Cargo.toml` tracks
   130+ crates in topological dependency order.

3. **Agent-friendly boundaries** -- an AI agent asked to "extract the
   folding logic" can operate on one small, well-scoped crate without
   needing to understand the entire LSP server.

4. **Testability** -- each microcrate can be tested in isolation:
   `cargo test -p perl-lsp-folding` runs only folding tests.

5. **Tier enforcement** -- the workspace uses a 7-tier dependency
   hierarchy, annotated directly in `Cargo.toml`, preventing circular
   dependencies and making the build graph predictable.

---

## The Extraction Pattern

Every successful extraction follows a consistent four-step recipe.
Here is PR #1238 (`perl-lsp-folding`) as a worked example:

### Step 1: Create the New Crate

A new directory `crates/perl-lsp-folding/` is created with:
- `Cargo.toml` -- workspace edition/version/license, minimal dependencies
  (only `perl-lexer` and `perl-parser-core` in this case)
- `src/lib.rs` -- the extracted implementation, moved verbatim from the
  source crate
- `README.md` -- one-paragraph description

The `Cargo.toml` inherits workspace-level metadata:
```toml
[package]
name = "perl-lsp-folding"
version = "0.10.0"
edition.workspace = true
rust-version.workspace = true
# ... all metadata from workspace
```

### Step 2: Replace the Original with a Compatibility Shim

The original file (`perl-lsp-providers/src/ide/lsp_compat/folding.rs`)
is reduced to a re-export shim:

```rust
//! Folding range extraction compatibility shim.
//!
//! The implementation now lives in the `perl-lsp-folding` microcrate.

pub use perl_lsp_folding::{FoldingRange, FoldingRangeExtractor, FoldingRangeKind};
```

This preserves the public API so that existing consumers continue to
compile without changes.

### Step 3: Wire into the Workspace

Three locations in the root `Cargo.toml` must be updated:
1. **`members`** -- add `"crates/perl-lsp-folding"`
2. **`[workspace.dependencies]`** -- add the path + version entry
3. **`publish allow`** -- add to the topologically sorted allowlist

The parent crate's `Cargo.toml` gains a new workspace dependency:
```toml
perl-lsp-folding = { workspace = true }
```

### Step 4: Test

The typical test plan runs three commands:
```bash
cargo test -p perl-lsp-folding            # New crate's own tests
cargo test -p perl-lsp-providers          # Parent crate still passes
cargo test -p perl-lsp-rs --lib              # Integration tests pass
```

PR #1238 changed 10 files, added 379 lines, deleted 317 lines, and was
merged 47 minutes after opening.

---

## Batch Extractions

Not every extraction is a single crate.  The largest batch was PR #848
("extract feature governance into 9 microcrates"), which split the
monolithic `features/` module from `perl-lsp` into:

| Crate | Purpose | LOC |
|-------|---------|----:|
| `perl-feature-catalog` | Build-time `features.toml` parser | 519 |
| `perl-lsp-feature-ids` | Stable string constants | 150 |
| `perl-lsp-feature-flags` | Compile-time predicates | 494 |
| `perl-lsp-feature-contracts` | Catalog and BDD grid rows | 524 |
| `perl-lsp-feature-policy` | Runtime profile mapping | 171 |
| `perl-lsp-feature-profile` | CLI token parsing | 67 |
| `perl-lsp-feature-grid` | BDD-grid JSON payload | 218 |
| `perl-lsp-feature-governance` | Facade re-exporting above | 118 |
| `perl-lsp-launcher` | Typed CLI launch config | 330 |

This single PR changed 59 files and added 3,592 lines.

---

## Crate Family Structure

The 121 crates organize into families by naming convention:

| Family Prefix | Count | Examples |
|---------------|:-----:|---------|
| `perl-lsp-*` | 35+ | providers, navigation, completion, folding |
| `perl-lsp-feature-*` | 7 | ids, flags, contracts, policy, governance |
| `perl-module-*` | 13 | name, path, token, resolution, import |
| `perl-dap-*` | 8 | breakpoint, eval, security, shell, stack |
| `perl-ts-*` | 5 | heredoc-parser, logos-lexer, advanced-parsers |
| `perl-workspace-*` | 5 | index, discovery, ignore, folder |
| `perl-symbol-*` | 4 | types, cursor, table, index |
| Core / standalone | ~44 | lexer, parser, error, quote, regex, keywords |

---

## Tooling: xtask SRP Detection

PR #933 introduced an automated tool for identifying extraction candidates.
The `xtask` subcommand `srp-microcrates` runs `cargo metadata --no-deps`
and computes per-crate metrics (LOC, Rust file count, dependency counts).

It classifies crates into two buckets using explicit heuristics:

- **SRP microcrate** (already extracted): <=700 LOC, <=3 Rust source
  files, <=8 direct dependencies
- **Split candidate** (should be extracted): >2,000 LOC, >20 direct
  dependencies, or >20 Rust source files

```bash
cargo run -p xtask -- srp-microcrates
# Writes docs/SRP_MICROCRATES.md
```

The generated report (`docs/SRP_MICROCRATES.md`) serves as a living
inventory and hitlist for future extractions.

---

## The Rejection Rate: What Didn't Work

The extraction campaign has a strikingly high failure rate:

| Metric | Count |
|--------|:-----:|
| Extraction PRs merged | 48 |
| Extraction PRs rejected | 81 |
| Total attempted | 129 |
| **Rejection rate** | **63%** |

### Why So Many Rejections?

**Duplicate attempts at the same extraction.** Several extraction targets
were attempted by agents multiple times before one succeeded:

| Extraction Target | Rejected Attempts | Eventually Merged? |
|-------------------|:-----------------:|:------------------:|
| Document links | 8 | Yes (#1164) |
| Diagnostics | 6 | Yes (#960, #1180) |
| Skip/ignore rules | 5 | Yes (#1204) |
| Security validation | 4 | Yes (#1194) |
| Completion items | 4 | Yes (#1241) |
| Limits config | 4 | Yes (#934) |

The pattern: an agent would attempt an extraction, fail `ci-gate` due to
a missing re-export, a broken dependency, or a clippy violation, get
rejected, and a new agent instance would retry with a slightly different
approach.  This is the expected behavior in an AI-native workflow -- agents
are cheap, human review is expensive, and the mechanical gate is the
quality bar.

### Common Failure Modes

1. **Missing re-exports** -- the compatibility shim didn't re-export all
   public symbols, breaking downstream consumers.
2. **Dependency tier violations** -- the new crate depended on a
   higher-tier crate, violating the build graph.
3. **Duplicate work** -- two agents working in parallel attempted the same
   extraction, and the second PR conflicted with the first.
4. **Pre-existing clippy warnings** -- the extraction surfaced warnings
   that already existed but were now visible in the new crate's stricter
   lint scope.

---

## Extremes: The Smallest and Largest Crates

### The 8-Line Crate

`perl-module-resolution` (`crates/perl-module-resolution/src/lib.rs`) is
the smallest crate at **8 lines of Rust**.  Its `lib.rs` is a pure
re-export facade:

```rust
pub use perl_module_resolution_path;
pub use perl_module_resolution_uri;
// ...
```

It exists solely to provide a single dependency for consumers that need
both path-based and URI-based module resolution.

### The 3,123-Line Crate

`perl-lexer` (`crates/perl-lexer/src/lib.rs`) is the largest single
`lib.rs` at **3,123 lines**.  The full lexer crate totals around 3,200
LOC across its source tree.  This is a natural boundary -- a context-aware
tokenizer for Perl is inherently complex and does not decompose easily.

### Total Source by Crate (Top 10)

| Crate | Total LOC | Role |
|-------|----------:|------|
| `tree-sitter-perl-rs` | 18,950 | Generated tree-sitter bindings |
| `perl-lsp` | 15,852 | LSP server binary |
| `perl-dap` | 11,855 | DAP server |
| `perl-corpus` | 9,388 | Test corpus |
| `perl-semantic-analyzer` | 8,126 | Semantic analysis |
| `perl-refactoring` | 7,467 | Refactoring engine |
| `perl-incremental-parsing` | 5,984 | Incremental parse |
| `perl-workspace-index` | 5,776 | Workspace indexing |
| `perl-ts-heredoc-parser` | 4,719 | Heredoc parsing |
| `perl-ts-advanced-parsers` | 4,196 | Tree-sitter advanced |

The smallest crates (under 100 LOC total):

| Crate | Total LOC | Purpose |
|-------|----------:|---------|
| `perl-module-resolution` | 8 | Re-export facade |
| `perl-lsp-feature-governance` | 32 | Re-export facade |
| `perl-module-token-parser` | 36 | Token parsing helpers |
| `perl-line-index` | 44 | Line/offset indexing |
| `perl-dap-command-args` | 47 | DAP arg formatting |
| `perl-lsp-uri` | 49 | URI parsing helpers |

---

## Dependency Tier Management

The workspace `Cargo.toml` enforces a 7-tier dependency hierarchy:

| Tier | Purpose | Example Crates |
|------|---------|---------------|
| 1 | Leaf crates (no workspace deps) | `perl-token`, `perl-keywords`, `perl-builtins` |
| 1b | Depend only on Tier 1 | `perl-ast`, `perl-lexer`, `perl-heredoc` |
| 1c | Depend on Tier 1b | `perl-error`, `perl-pragma`, `perl-tokenizer` |
| 2 | Core infrastructure | `perl-parser-core`, `perl-symbol-types` |
| 3 | Analysis and indexing | `perl-workspace-index`, `perl-lsp-diagnostics` |
| 4 | LSP providers | `perl-lsp-navigation`, `perl-lsp-providers` |
| 5 | Application and DAP | `perl-dap`, `perl-dead-code`, `perl-parser` |
| 6 | Module resolution chain | `perl-module-resolution`, `perl-workspace-discovery` |
| 7 | Top-level application | `perl-lsp` |

New extractions must be placed in the correct tier.  A common rejection
cause was a new crate placed in Tier 2 that depended on a Tier 3 crate.

The publish allowlist in `Cargo.toml` is maintained in topological order,
so `cargo publish` can proceed sequentially without dependency failures.

---

## The Agent Dimension

Almost every extraction PR was created by OpenAI Codex agents -- identifiable
by the "[Codex Task](https://chatgpt.com/codex/tasks/...)" footer in their
PR descriptions.  The repository has **152 PRs** with Codex Task links out
of ~1,270 total PRs.

The extraction campaign's high rejection rate (63%) is characteristic of
the project's AI-native workflow.  The agents work in parallel, sometimes
race on the same target, and rely on `ci-gate` rather than upfront
coordination.  The project treats rejected PRs not as wasted effort but as
a natural cost of machine-speed iteration -- each rejected attempt
narrows the search space for the next agent.

Key dates in the campaign:

| Date | Event |
|------|-------|
| Feb 19, 2026 | PR #838: tree-sitter microcrate extraction begins |
| Feb 22, 2026 | PR #848: 9 feature governance crates in one batch |
| Feb 28, 2026 | PR #933: xtask SRP reporting tool ships |
| Feb 28, 2026 | v0.10.0 released at 85 crates |
| Mar 2, 2026 | 7 extraction PRs merged in one day |
| Mar 5, 2026 | **Peak day**: 93 extraction commits, 22 PRs merged |
| Mar 11, 2026 | Campaign continues; 121 crates reached |

---

## Was It Worth It? Trade-offs and Results

### Benefits Realized

- **Incremental compilation** -- touching a 50-line microcrate only
  recompiles that crate and its direct dependents, not the 16K-line LSP
  server.
- **Independent publishing** -- 120+ crates can be published to crates.io
  in dependency order.
- **Clear ownership** -- each crate has a `README.md` and `CLAUDE.md`
  describing its single responsibility.
- **Parallel agent work** -- agents can extract, test, and PR a microcrate
  without conflicting with agents working on other crates.

### Costs Incurred

- **Workspace complexity** -- the root `Cargo.toml` is 400+ lines of
  member and dependency declarations.  Adding a crate requires edits in
  three places (members, workspace deps, publish allowlist).
- **High rejection rate** -- 63% of extraction PRs were rejected, meaning
  significant compute was spent on failed attempts.
- **Facade crates** -- several crates exist purely as re-export facades
  (e.g., `perl-module-resolution` at 8 lines, `perl-lsp-feature-governance`
  at 32 lines), adding organizational overhead without new functionality.
- **Version lockstep** -- all 121 crates share the same version
  (`0.10.0`), so a version bump requires updating every `Cargo.toml`.
  The `xtask bump-version` command automates this.

### The Verdict

The extraction campaign is an empirical answer to "how far can SRP go in a
Rust workspace?"  At 121 crates, the project is well past the point where
manual dependency management is feasible -- but the tooling (`xtask
srp-microcrates`, the tiered publish allowlist, workspace dependency
inheritance) makes it manageable.

The key insight is that in an AI-native codebase, the cost of having too
many crates is lower than the cost of having too few.  Agents work better
with small, well-scoped units.  The 63% rejection rate on extractions is
not a bug -- it is the expected cost of running cheap agents against a
strict mechanical gate.

---

## Guidance: When to Extract and When to Stop

### Extract When

- A module has a **single clear responsibility** that is testable in
  isolation (folding, URI parsing, diagnostic types).
- The module is **reused by multiple consumers** (diagnostic types used
  by both diagnostics and code-actions crates).
- The module has **few dependencies** and can live at a low tier.
- An `xtask srp-microcrates` report flags the parent crate as a split
  candidate (>2,000 LOC or >20 dependencies).

### Do Not Extract When

- The module is **tightly coupled** to its parent's internals (the lexer's
  3,123-line `lib.rs` is a single state machine -- splitting it would
  create artificial boundaries).
- The result would be a **facade-only crate** with no logic of its own
  (the 8-line `perl-module-resolution` is arguably already past this line).
- The extraction would **create a circular dependency** or force a tier
  violation.
- The **compile-time savings** are negligible (extracting a 30-line helper
  into its own crate adds Cargo overhead that may exceed the build time
  saved).

### The Current Split Candidates

As of March 2026, the `xtask srp-microcrates` report flags these crates
as candidates for further extraction:

| Crate | LOC | Why |
|-------|----:|-----|
| `perl-lsp` | 15,852 | Main binary, naturally large |
| `perl-dap` | 11,855 | DAP server, similar |
| `perl-semantic-analyzer` | 8,126 | Could split by analysis pass |
| `perl-refactoring` | 7,467 | Could split by refactoring type |
| `perl-workspace-index` | 5,776 | Partially split already |
| `perl-incremental-parsing` | 5,984 | Complex, may resist splitting |

Whether these should be split depends on whether agents or humans will be
working on them next -- and whether the compile-time savings justify the
workspace overhead.
