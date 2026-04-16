# Context — Wave 1 PILOT Implementation

Issue: #4420 | Tracking: #4410 | ADR: [docs/adr/0041-microcrate-collapse.md](../../docs/adr/0041-microcrate-collapse.md)

---

## Overview

This is the **pilot wave** of ADR-0041 (microcrate collapse). It establishes the move pattern for all subsequent waves: absorb a family of single-purpose crates into a single published facade crate with internal folder-modules.

**Target:** 13 perl-module-* crates → 1 perl-module facade (version 0.14.0, published).

**Rationale:** Reduce workspace friction (cargo build parallelism, dependency resolution, maintenance) while preserving external API and internal boundaries.

---

## Key Design Decisions (from plan-reviewer)

### 1. Visibility Model: `pub(crate)` default + api.rs facade

**Decision:** All items in submodules default to `pub(crate)`. Only api.rs re-exports chosen items as `pub use`.

**Why:** Enforces internal boundaries; prevents accidental cross-module coupling; makes public contract explicit and reviewable.

**Alternative rejected:** Making all items `pub` in modules. Downside: no clear internal/external boundary; future waves harder to refactor; maintenance burden for consumers who relied on "internal" items.

**Implementation:** When copying module source (steps 1b–1k), change all `pub ` at module root to `pub(crate) `, preserving internal impl visibility.

---

### 2. Module Layout: Token nested in parent, not sibling folders

**Decision:** `src/resolution/` contains both mod.rs (resolution) and path.rs/uri.rs (submodules), not three sibling files.

**Why:** Mirrors the old crate structure (perl-module-resolution as owner, perl-module-resolution-path/uri as submodules). Reduces mental overhead when reading code.

**Alternative rejected:** Flat layout (src/resolution_path.rs, src/resolution_uri.rs, src/resolution.rs at top level). Downside: loses semantic grouping; harder to find related modules.

**Implementation:** See step 1l (create src/resolution/mod.rs with pub mod path; pub mod uri; includes path.rs and uri.rs in same folder).

---

### 3. Qualified-name absorption deferred to Wave 4

**Decision:** perl-qualified-name stays published; Wave 4 (parser satellites) will absorb it.

**Why:** perl-qualified-name is consumed by many crates outside the module family (workspace-index, etc.). Absorbing it into perl-module now would force those consumers to update. Wave 4 absorbs it into perl-parser (the root of all those dependency chains), reducing ripple.

**Alternative rejected:** Absorb into perl-module now. Downside: requires updating 5+ non-module consumers; scope creep; increased coordination.

**Implementation:** Leave perl-qualified-name as-is. Do not include in 13 crates being moved.

---

### 4. Monolithic merge (all 13 in one PR)

**Decision:** Absorb all 13 in a single PR, not phased over multiple PRs.

**Why:** Dependency DAG is tight; splitting PR-by-module means many will block on others' merges. One big PR is faster and simpler.

**Alternative rejected:** Phase by layer (leaves in PR 1, then L2, then L3, etc.). Downside: ~6 PRs needed; each depends on previous; slow iteration; high coordination cost.

**Implementation:** Checklist orders steps to be parallelizable where possible (all leaf modules can be copied independently before L2), but commits in one branch.

---

### 5. Major version bump: 0.12.4 → 0.14.0

**Decision:** Published `perl-module` is version 0.14.0 (not 0.13.0 or 0.12.5).

**Why:** Public API shape changed (items under new names: `perl_module::name::*` instead of `perl_module_name::*`). Old code expecting `perl_module_name` won't compile. Major/minor bump signals breaking change to external consumers.

**Alternative rejected:** Keep 0.12.4. Downside: semver lie; consumers won't realize they need code changes.

**Implementation:** Cargo.toml sets version = "0.13.0" (step 0a). MIGRATION_v0.13.md documents the change (builder responsibility, not spec-planner).

---

### 6. DAG-driven module ordering

**Decision:** Copy modules in topological order (leaves first: name, token_core; then L2 path; then L3 import; etc.).

**Why:** Enables incremental compilation checks. After copying path, we can verify `cargo build -p perl-module --lib` compiles (if imports are correct). Fails immediately if an import is wrong.

**Alternative rejected:** Copy all at once, fix imports after. Downside: harder to debug; many errors at once; don't know which step introduced each error.

**Implementation:** Steps 1b–1k follow DAG order. Each step has "After step X" note confirming compilation state.

---

## Dependencies & Ripple Impact

### Direct consumers (must update imports):

1. **perl-lsp** (5 modules) — modules_resolution, module_import, module_reference, module_rename, module_path
2. **perl-lsp-completion** (1) — module_import
3. **perl-lsp-document-links** (2) — module_path, module_import
4. **perl-lsp-workspace-symbols** (1) — module_path
5. **perl-dap** (1) — module_path
6. **perl-refactoring** (1) — module_path
7. **perl-text-line test** (2) — module_token, module_token_parser

**Total scope:** 7 crates + 1 test file, ~6 Cargo.toml updates + 7 source file updates. All in one PR.

### Indirect consumers (don't need changes):

- perl-workspace-index, perl-semantic-analyzer, perl-parser, etc. — they depend on above crates, not on perl-module-* directly. No import changes needed (dependency resolution handles it).

---

## Internal DAG

All 13 crates fit in 11 layers (see issue body). Topological order used in Phase 1:

```
Leaf (0 deps):         name, token_core
L1 (1 dep):             path (→ name)
L2 (1 dep):             import (→ path)
L3 (1 dep):             boundary (→ token_core, import, name)
L4 (2 deps):            reference (→ import, name, path)
L5 (4 deps):            token (→ boundary, name, path, token_core)
L6 (2 deps):            token_parser (→ reference, token_core)
L7 (4 deps):            import_match (→ boundary, import, path, token)
L8 (3 deps):            rename (→ import_match, path, token)
L9a (1 dep):            resolution_path (→ path)
L9b (1 dep):            resolution_uri (→ path)
L10 (2 deps):           resolution (→ resolution_path, resolution_uri)
```

No cycles detected. Ordering is strict.

---

## Risks & Mitigations

### Risk: Test failures after import migration

**Likelihood:** Medium (import paths differ; typos possible).

**Mitigation:** Checklist step 2b uses grep-and-replace in test files. Builder should spot-check ~5 test files to confirm imports match module structure.

**Signal:** `cargo test -p perl-module --lib` passes all 62 tests (step 5b).

---

### Risk: Missing public items in api.rs

**Likelihood:** Low (checklist requires copying all `pub use` from modules).

**Mitigation:** api.rs is generated by reading each module's public surface. Builder should run `cargo doc -p perl-module --no-deps --open` and verify facade shows expected items.

**Signal:** Consumers build without undefined-reference errors. `cargo build -p perl-lsp-rs --release` succeeds (step 5a).

---

### Risk: Visibility leakage (public items should be pub(crate))

**Likelihood:** Medium (easy to forget when copying).

**Mitigation:** Checklist explicitly says to change `pub ` → `pub(crate) `. Builder should spot-check ~3 modules to confirm.

**Signal:** No clippy warnings about pub items with few usages (would suggest over-exposed API).

---

### Risk: Workspace member count doesn't match expectation

**Likelihood:** Low (deterministic math: 135 - 13 + 1 = 123).

**Mitigation:** Step 4d deletes 13 directories; step 4a removes from members. Builder should re-run `cargo metadata` and verify count.

**Signal:** `cargo metadata --no-deps | jq '.workspace_members | length'` returns 123.

---

## References

- **Tracking issue:** #4410 — Overall microcrate collapse roadmap
- **ADR:** [docs/adr/0041-microcrate-collapse.md](../../docs/adr/0041-microcrate-collapse.md) — Architectural rationale and decision history
- **Ledger:** [.spec/microcrate-collapse/ledger.md](.spec/microcrate-collapse/ledger.md) — Wave-by-wave tracking and final publish surface
- **Prerequisite PR #4417:** Added `cargo xtask publish-closure` gate (needed for step 5c verification)
- **Prerequisite PR #4418:** Cleaned up parser layering (parser no longer re-exports LSP items, reducing ripple)

---

## Plan-Reviewer Decisions Locked

The following **cannot be changed during implementation** without re-planning:

1. All 13 crates absorbed into one (not phased)
2. Version bumped to 0.14.0
3. Visibility defaults to `pub(crate)` (api.rs re-exports only)
4. Qualified-name deferred to Wave 4
5. Module layout with nested resolution/

If builder discovers a showstopper (e.g., circular dependency, missing build script), call for plan-reviewer re-review (comment on issue with blocker). Do not adapt the plan solo.

---

## Acceptance Signature

Plan-reviewer: Locked for build (see `[workspace.metadata.publish]` commit hash and issue labels).

Red-TDD builder: Will write failing tests covering all 62 test files + api.rs exports + 6 consumer builds.

Builder: Will implement per checklist, verify at each breakpoint, commit atomically.

Reviewer: Will verify diff matches spec (no hidden changes, api.rs completeness, visibility enforcement).
