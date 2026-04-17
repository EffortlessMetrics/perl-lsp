# Context: Wave B Microcrate Collapse (perl-symbol-* 4 satellites → perl-symbol NEW)

Issue: #4428 | Tracking: #4410 | ADR: [docs/adr/0041-microcrate-collapse.md](../../docs/adr/0041-microcrate-collapse.md) | Pilot: #4422 | Wave A precedent: #4434

---

## Overview

Wave B of ADR-0041 (microcrate collapse). Absorbs 4 `perl-symbol-*` satellite crates into a **new published facade crate** `perl-symbol`, following the pattern established by Wave 1 (#4422 perl-module-*) and Wave A (#4434 perl-workspace-*).

**Target:** 4 crates → 1 new `perl-symbol` facade crate (version 0.12.4 at collapse; will rev to 0.14.0 when published publicly per ADR-0041).

**Rationale:** `perl-symbol` must remain a standalone published crate (not absorbed into `perl-semantic-analyzer`) because `perl-workspace-index` and `perl-lsp` consume symbol types directly and cannot depend on the full semantic analyzer just to get them. See ledger amendment 3 (merged via #4431).

---

## Decision Log

### 1. Crate name: `perl-symbol` (NEW published crate)

- **Decision:** Create `crates/perl-symbol/` from scratch as a brand-new published crate. Unlike Wave A (which renamed an existing directory), Wave B creates a greenfield facade directory and deletes all 4 satellite directories.
- **Rationale:** None of the 4 satellites is a natural "owner" — all are peers with distinct responsibilities. Creating a new directory avoids arbitrary choice and mirrors ledger row 43 ("perl-symbol (NEW)").
- **Precedent:** Wave E (#4435 perl-diagnostics) also created a new crate.

### 2. Layout: flat module folders (`types/`, `cursor/`, `index/`, `surface/`)

- **Decision:** 4 new folders inside `crates/perl-symbol/src/` — one per absorbed crate; `surface/` contains both `mod.rs` and `decl.rs` (preserving surface's 2-file structure).
- **Rationale:** Proven pattern from Wave 1 and Wave A. Flat is simpler than nested, and each satellite becomes an internal module with a short path.
- **Alternative rejected:** Nested grouping (e.g., `src/projection/{surface,types}/`) — over-engineered for 4 peer modules.

### 3. `api.rs`: explicit re-exports only, NO wildcards

- **Decision:** `src/api.rs` uses only explicit named `pub use` — no `pub use crate::types::*;`.
- **Types at crate root:** `SymbolKind`, `VarKind` must be re-exported at `perl_symbol::{SymbolKind, VarKind}` for ergonomic consumer migration (architecture-reviewer recommendation in plan-review).
- **Pattern:** `lib.rs` does `pub use api::*;` — so api.rs is the single public contract definition point.
- **Rationale:** Explicit re-exports document the public surface; wildcards allow silent API expansion and type-name clashes.
- **Alternative rejected:** Wildcard re-exports — fragile; plan-review explicitly rejected.

### 4. CLAUDE.md invariant preservation

- **Decision:** Create `crates/perl-symbol/CLAUDE.md` that preserves `perl-symbol-surface/CLAUDE.md`'s "NOT allowed" invariant verbatim: the new facade (and specifically its `surface/` module) must not depend on `perl-parser-core`, `lsp-types`, or any LSP provider crate.
- **Why:** perl-symbol-surface's architectural invariant is the critical constraint that keeps the surface module a clean projection layer. Losing this note in the collapse would let future drift break the invariant.
- **Implementation:** Top-level crate CLAUDE.md lists allowed deps (`perl-ast`, `serde`) and explicitly lists the NOT-allowed set.

### 5. Cargo deps for new facade

- **Direct dependencies:** `perl-ast` (for surface), `serde` (for types derive). **NOT** `perl-position-tracking` (transitive via perl-ast) and **NOT** `perl-symbol-types` (becomes an internal module).
- **Dev dependencies:** `serde_json` (types tests), `perl-tdd-support` (cursor tests), `perl-ast` (surface tests).
- **`[lib] doctest = false`** — all 4 satellites disable doctests; preserve.
- **`edition = "2024"`** — project standard (Wave 1 learning: builder forgot this).
- **`publish = true`** — new crate must be publishable; added to `[workspace.metadata.publish].allow`.

### 6. Five consumer crate renames

Consumers must migrate from one of the 4 satellite crate names to `perl-symbol`:

| Consumer crate | Old dep | New dep | Import changes |
|---|---|---|---|
| `perl-workspace-index` | `perl-symbol-types` | `perl-symbol` | `perl_symbol_types::` → `perl_symbol::types::` (or crate-root `perl_symbol::`) |
| `perl-semantic-analyzer` | `perl-symbol-types` | `perl-symbol` | same |
| `perl-lsp` | `perl-symbol-cursor` | `perl-symbol` | `perl_symbol_cursor::` → `perl_symbol::cursor::` |
| `perl-lsp-rename` | `perl-symbol-cursor` | `perl-symbol` | `use perl_symbol_cursor as cursor` → `use perl_symbol::cursor as cursor` |
| `perl-lsp-performance` | `perl-symbol-index` | `perl-symbol` | `perl_symbol_index::` → `perl_symbol::index::` |

**Note:** Internal re-exports at `perl-workspace-index/src/workspace/workspace_index.rs:1022` and `perl-semantic-analyzer/src/analysis/symbol.rs:37` are the two places that currently `pub use perl_symbol_types::{SymbolKind, VarKind}`. Both become `pub use perl_symbol::{SymbolKind, VarKind}` thanks to crate-root re-export (decision 3).

### 7. Test file prefix scheme (resolve collisions)

Two `comprehensive_unit_tests.rs` files collide (`perl-symbol-types/tests/` and `perl-symbol-cursor/tests/`). Applying the Wave 1 prefix pattern:

| Source | Destination in `perl-symbol/tests/` |
|---|---|
| `perl-symbol-types/tests/comprehensive_unit_tests.rs` | `types_comprehensive_unit_tests.rs` |
| `perl-symbol-types/tests/symbol_types_extended.rs` | `types_extended.rs` |
| `perl-symbol-cursor/tests/comprehensive_unit_tests.rs` | `cursor_comprehensive_unit_tests.rs` |
| `perl-symbol-cursor/tests/cursor_symbol_bdd.rs` | `cursor_bdd.rs` |
| `perl-symbol-index/tests/trie_and_fuzzy.rs` | `index_trie_and_fuzzy.rs` |
| `perl-symbol-surface/tests/symbol_decl_tests.rs` | `surface_decl.rs` |

Plus new: `facade_api_completeness.rs` guarding the re-export surface (Wave 1 pattern, mandatory).

### 8. Wave A merge gate

- **Hard constraint:** Wave A (#4434) must be merged before Wave B opens a PR.
- **Reason:** Both edit root `Cargo.toml` (`[workspace.members]`, `[workspace.dependencies]`, `[workspace.metadata.publish].allow`). Per MEMORY entry `feedback_multi_pr_cargo_toml_race.md`, two PRs touching same TOML section break `cargo metadata` at merge time.
- **Status:** Wave A (#4434) merged as commit `b6b8d1d7d`. Constraint satisfied; this branch is based on origin/master post-Wave-A.

### 9. Edition and workspace inheritance

- **Decision:** New `perl-symbol/Cargo.toml` inherits `version.workspace = true`, `edition.workspace = true`, `rust-version.workspace = true`, `authors.workspace = true`, `license.workspace = true`, `repository.workspace = true`, `homepage.workspace = true`.
- **Sets explicitly:** `name`, `description`, `documentation`, `readme`, `keywords`, `categories`, `include`.
- **Include pattern:** `src/**`, `Cargo.toml`, `LICENSE*`, `README.md`, `CLAUDE.md` (Wave 1 pattern; surface's include set was the richest of the 4).

---

## Alternatives Considered

1. **Absorb into `perl-semantic-analyzer`** — rejected; inverts dependency layering (perl-workspace-index and perl-lsp would need the full semantic analyzer just to get SymbolKind). ADR amendment 3 locks this.
2. **Pick one satellite as new owner (e.g., perl-symbol-types)** — rejected; arbitrary choice and misrepresents the scope (index + surface are much larger than types).
3. **Nested folders** (e.g., `src/projection/surface/`, `src/projection/types/`) — rejected; over-engineered for 4 peer modules.
4. **Wildcard re-exports in `api.rs`** — rejected; creates public API ambiguity and silent API expansion.
5. **Keep `perl-symbol-types` published as-is + absorb other 3** — rejected; partial collapse doesn't deliver the consolidation value the ADR targets.

---

## Edge Cases & Mitigations

### Edge case: Consumer re-exports `SymbolKind`/`VarKind` via `pub use`

- **Risk:** Two consumer crates (`perl-workspace-index` and `perl-semantic-analyzer`) do `pub use perl_symbol_types::{SymbolKind, VarKind}` as part of their own public API.
- **Mitigation:** Because we re-export `SymbolKind`/`VarKind` at crate root of `perl-symbol` (decision 3), the consumer change is one-line: `pub use perl_symbol::{SymbolKind, VarKind}`. Downstream consumers of `perl-semantic-analyzer` and `perl-workspace-index` see no API change.
- **Likelihood:** Hit directly; trivially mitigated.

### Edge case: `perl-symbol-surface` is separated from the cluster in Cargo.toml

- **Risk:** `[workspace.members]` has `perl-symbol-surface` at line 70 (isolated) while `perl-symbol-{types,cursor,index}` cluster at lines 81-83. `[workspace.dependencies]` similar: `perl-symbol-types`/`cursor`/`index` at lines 275-277, `perl-symbol-surface` at line 290.
- **Mitigation:** Checklist explicitly flags BOTH locations in BOTH sections. Builder must not assume contiguous blocks.
- **Likelihood:** Medium (easy to miss the isolated entry).

### Edge case: Doctests in source doc comments reference old crate names

- **Risk:** `perl-symbol-types/src/lib.rs:35,182` have doc examples like `use perl_symbol_types::VarKind;` and `perl-symbol-surface/src/lib.rs:21` has `use perl_symbol_surface::extract_symbol_decls;`.
- **Mitigation:** Update to `use perl_symbol::VarKind;` / `use perl_symbol::surface::extract_symbol_decls;` when copying into new modules. Since `doctest = false`, these are for documentation only but should still be correct.
- **Likelihood:** High for doc freshness; low for build breakage.

### Edge case: `perl-lsp-workspace-symbols` has a comment referencing `perl_symbol_types`

- **Risk:** `crates/perl-lsp-workspace-symbols/src/lib.rs:298` contains a comment `// Symbol kind conversion is handled by perl_symbol_types::SymbolKind::to_lsp_kind()`.
- **Mitigation:** Update comment to reference `perl_symbol::SymbolKind::to_lsp_kind()`. Comment-only, no functional impact, no Cargo.toml change for this crate.
- **Likelihood:** Low impact; included for cleanliness.

### Edge case: `test_corpus` or other tooling references old crate names

- **Risk:** Hardcoded string `"perl-symbol-*"` in CI hygiene / documentation / test snapshots (Wave A analogue: perl-ci-hygiene line 4505, perl-parser missing_docs_ac_tests.rs lines 607-608).
- **Mitigation:** Pre-implementation grep `grep -rn 'perl-symbol-types\|perl-symbol-cursor\|perl-symbol-index\|perl-symbol-surface' crates/ docs/ --include='*.rs' --include='*.toml' --include='*.md'` — verify any hits.
- **Likelihood:** Medium; Wave A had 2 such hits.

### Edge case: LICENSE files in satellite crates

- **Risk:** `perl-symbol-types` has `LICENSE-APACHE` and `LICENSE-MIT` committed. Deleting the directory removes them.
- **Mitigation:** Workspace root has LICENSE files; per Cargo convention `include = ["LICENSE*"]` in new `perl-symbol/Cargo.toml` will reference root symlinks / copies if needed. Align with other published crates' LICENSE handling; do NOT leave dangling LICENSE files.
- **Note:** Wave A and Wave 1 did not separately re-add LICENSE per-crate; this wave follows suit. If publish-closure complains, add LICENSE copies to `crates/perl-symbol/`.
- **Likelihood:** Low; verify with `cargo package --list -p perl-symbol` during final verification.

### Edge case: `perl-symbol-types` `CLAUDE.md` content merge

- **Risk:** Both `perl-symbol-types/CLAUDE.md` and `perl-symbol-surface/CLAUDE.md` exist with different content. Cursor and index have no CLAUDE.md.
- **Mitigation:** Single `crates/perl-symbol/CLAUDE.md` that combines (not concatenates) relevant guidance — one section per module. Preserve the "NOT allowed" invariant from surface verbatim.
- **Likelihood:** Certain; explicit step in checklist.

---

## Verification Strategy

1. **Build verification** (builder responsibility after each phase):
   - `cargo check -p perl-symbol` after module creation and `lib.rs`/`api.rs` setup
   - `cargo build -p perl-symbol` after Cargo.toml finalized
   - `cargo build -p <consumer>` for each of 5 consumers
   - `cargo build --workspace` before phase 10

2. **Test verification:**
   - `cargo test -p perl-symbol` — all 7 test binaries (6 migrated + 1 facade_api_completeness)
   - `cargo test --workspace --lib` — no regressions from consumer migration
   - RUST_TEST_THREADS=2 for perl-lsp-rs tests per its CLAUDE.md constraint

3. **Cargo verification:**
   - `cargo metadata --no-deps --format-version 1 | jq '.workspace_members | length'` — member count decreases by 3 (4 removed + 1 added)
   - `cargo xtask publish-closure` — no `perl-symbol-*` entries; `perl-symbol` present
   - `cargo clippy --workspace --lib` — no new warnings
   - `cargo xtask fmt` — formatting clean

4. **Grep verification:**
   - `grep -rn 'perl_symbol_types\|perl_symbol_cursor\|perl_symbol_index\|perl_symbol_surface' crates/ --include='*.rs'` — ZERO hits (except under `crates/perl-symbol/` internally, which should have none either since they use sibling module paths)
   - `grep -rn 'perl-symbol-types\|perl-symbol-cursor\|perl-symbol-index\|perl-symbol-surface' crates/ --include='*.toml'` — ZERO hits
   - No dangling directories: `ls crates/ | grep perl-symbol` shows only `perl-symbol`

---

## Key Risk Flags

- **High:** Double Cargo.toml edit sites — `perl-symbol-surface` isolated from the other three in both `[workspace.members]` and `[workspace.dependencies]`; builder must not apply contiguous-range replacement.
- **High:** Two `pub use perl_symbol_types::{SymbolKind, VarKind}` re-export sites in consumers — missing either silently breaks downstream.
- **Medium:** Test file collisions (2 × `comprehensive_unit_tests.rs`) require prefix renames during copy.
- **Medium:** CLAUDE.md invariant from surface must survive — regression would let future drift break architectural layering.
- **Low:** Test renames (per prefix scheme) are mechanical but numerous (7 files).
- **Low:** Windows MAX_PATH risk — working in main checkout (not deep worktree); safe.

---

## References

- **ADR-0041:** `docs/adr/0041-microcrate-collapse.md` — microcrate collapse rationale and design
- **Ledger:** `.spec/microcrate-collapse/ledger.md` (Wave B row, lines 142-154) — amendment 3 confirms standalone `perl-symbol` published crate
- **Wave 1 pilot:** PR #4422 (perl-module-* collapse) — established flat folder-module + api.rs pattern
- **Wave A precedent:** PR #4434 (perl-workspace-* collapse, merged as `b6b8d1d7d`) — most recent; same multi-section Cargo.toml pattern
- **Wave E precedent:** PR #4435 (perl-diagnostics NEW crate) — created a new published crate from multiple satellites, closest structural analog
- **Tracking issue:** #4410 (microcrate collapse tracking)
- **Related memory entries:**
  - `feedback_wave1_collapse_gotchas.md` — edition=2024, test collisions, red-TDD brittleness
  - `feedback_multi_pr_cargo_toml_race.md` — why Wave A merge gate is hard
  - `feedback_no_loc_caps.md` — organize by coherence not line count
