# Implementation Checklist: Wave B Microcrate Collapse (perl-symbol-*)

**Issue:** #4428
**Branch:** `impl/4428-perl-symbol-wave-b`
**Target:** Create new published `perl-symbol` facade crate; absorb 4 satellites (`perl-symbol-types`, `perl-symbol-cursor`, `perl-symbol-index`, `perl-symbol-surface`); delete all 4 old directories.
**Test counts:** 7 test files total (6 migrated with prefix renames + 1 new `facade_api_completeness.rs`).
**Crate touch:** `perl-symbol` (NEW), 5 consumer crates (`perl-workspace-index`, `perl-semantic-analyzer`, `perl-lsp`, `perl-lsp-rename`, `perl-lsp-performance`), plus root `Cargo.toml`.
**Consumer source edits:** 5 files with known file:line locations (below).

---

## Preconditions

- [ ] Wave A (#4434) merged (commit `b6b8d1d7d` or newer). Verify: `git log --oneline origin/master | grep -i "Wave A"`
- [ ] Branch `impl/4428-perl-symbol-wave-b` is based on latest `origin/master` post-Wave-A
- [ ] No uncommitted changes in working tree: `git status`

---

## Phase 1: Create New Crate Skeleton

### Step 1.1: Create directory and Cargo.toml

**Location:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/Cargo.toml`

**Action:** Create new directory `crates/perl-symbol/` with `Cargo.toml`:

```toml
[package]
name = "perl-symbol"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
description = "Unified Perl symbol taxonomy, cursor extraction, indexing, and AST surface projection"
license = "MIT OR Apache-2.0"
repository.workspace = true
homepage.workspace = true
documentation = "https://docs.rs/perl-symbol"
readme = "README.md"
keywords = ["perl", "lsp", "symbols", "taxonomy", "language-server"]
categories = ["development-tools", "data-structures", "text-editors"]
publish = true
include = [
    "src/**",
    "Cargo.toml",
    "LICENSE*",
    "README.md",
    "CLAUDE.md",
]

[lib]
doctest = false

[dependencies]
perl-ast = { workspace = true }
serde = { workspace = true }

[dev-dependencies]
perl-ast = { workspace = true }
perl-tdd-support = { workspace = true }
serde_json = { workspace = true }

[package.metadata.docs.rs]
rustdoc-args = ["--cfg", "docsrs"]

[lints]
workspace = true
```

**Key points:**
- `edition.workspace = true` — currently workspace pins edition=2024 (Wave 1 gotcha)
- `publish = true` — must be explicit; new published crate
- `[lib] doctest = false` — all 4 satellites disable; preserve
- Direct deps: ONLY `perl-ast` + `serde` (per plan-review decision 5)
- `perl-tdd-support` needs `workspace = true` entry in root `[workspace.dependencies]` — verify present (used by perl-symbol-cursor today)

**Verify:** `cat crates/perl-symbol/Cargo.toml` (file exists and matches spec)

---

### Step 1.2: Create `src/lib.rs`

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/src/lib.rs`

**Content:**

```rust
//! Unified Perl symbol model: taxonomy, cursor extraction, search indexing, and AST projection.
//!
//! This crate consolidates the former `perl-symbol-types`, `perl-symbol-cursor`,
//! `perl-symbol-index`, and `perl-symbol-surface` microcrates into a single published
//! facade (see ADR-0041). Internal module boundaries are preserved; the public surface
//! is defined by `api.rs`.
//!
//! # Modules
//!
//! - [`types`] — Symbol taxonomy: `SymbolKind`, `VarKind`, and LSP mappings
//! - [`cursor`] — Cursor-based symbol extraction helpers
//! - [`index`] — Trie + inverted-index symbol search
//! - [`surface`] — Projection layer: derives symbol views from Perl AST
//!
//! # Quick start
//!
//! ```rust,ignore
//! use perl_symbol::{SymbolKind, VarKind};
//!
//! let var = SymbolKind::Variable(VarKind::Scalar);
//! assert_eq!(var.sigil(), Some("$"));
//! ```

pub mod types;
pub mod cursor;
pub mod index;
pub mod surface;

pub mod api;
pub use api::*;
```

**Verify:** `cargo check -p perl-symbol` (will fail until modules exist — expected)

---

### Step 1.3: Create `src/api.rs` (public contract)

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/src/api.rs`

**Content:**

```rust
//! Public API re-exports for `perl-symbol`.
//!
//! This module defines the full public surface. All items in submodules are
//! re-exported here explicitly (no wildcards) so the public contract is reviewable
//! at a glance.
//!
//! ## Conventions
//!
//! - `types::{SymbolKind, VarKind}` are also re-exported at the crate root for
//!   ergonomic consumer migration (semantic-analyzer and workspace-index had
//!   `pub use perl_symbol_types::{SymbolKind, VarKind};` as part of their own
//!   public API — a one-line rename keeps their downstream callers working).

// types — at crate root for ergonomics
pub use crate::types::{SymbolKind, VarKind};

// cursor — full public surface
pub use crate::cursor::{
    CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source,
    get_symbol_range_at_position, is_modchar, is_word_boundary, token_under_cursor,
};

// index — full public surface
pub use crate::index::SymbolIndex;

// surface — full public surface
pub use crate::surface::{SymbolDecl, extract_symbol_decls};
```

**Note on cursor API:** Double-check the actual exported items in `crates/perl-symbol-cursor/src/lib.rs` before finalizing this list. If cursor exports more or fewer items than listed, update api.rs to match. The issue body verified these 7 items are present; confirm during build.

**Verify:** `cargo check -p perl-symbol` (still fails — modules empty)

---

### Step 1.4: Create `CLAUDE.md` for new facade

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/CLAUDE.md`

**Content:** Combine relevant guidance from `perl-symbol-types/CLAUDE.md` and `perl-symbol-surface/CLAUDE.md`. Sections:

1. **Crate overview** — purpose of each of 4 modules
2. **Commands** — standard build/test/clippy/doc
3. **Architecture / Dependencies** — allowed: `perl-ast`, `serde`
4. **Architectural invariant (from surface/CLAUDE.md, verbatim):**
   > **NOT allowed** (for the `surface` module in particular, and for the crate as a whole): `perl-parser-core`, `lsp-types`, or any LSP provider crate.
5. **Key types per module** — `SymbolKind`/`VarKind` in `types`; `CursorSymbolKind` + helpers in `cursor`; `SymbolIndex` in `index`; `SymbolDecl`/`extract_symbol_decls` in `surface`
6. **Important notes** — doctests disabled; types derive Copy/Eq/Hash; changes to `SymbolKind` variants affect workspace-wide symbol reporting

**Verify:** `ls crates/perl-symbol/CLAUDE.md`

---

### Step 1.5: Create `README.md`

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/README.md`

**Action:** Short readme describing the crate, matching Wave 1 / Wave A pattern. 10-25 lines, describes the 4 modules, references ADR-0041 for history.

**Verify:** `ls crates/perl-symbol/README.md`

---

## Phase 2: Absorb Satellite Source into Modules

### Step 2.1: Create `src/types/mod.rs` from `perl-symbol-types/src/lib.rs`

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/src/types/mod.rs`

**Action:** Copy complete content of `crates/perl-symbol-types/src/lib.rs` into the new file.

**Edits during copy:**
- Remove crate-level `//!` module docstring block if it duplicates what `perl-symbol/src/lib.rs` says (preserve type-level docs)
- Update doc examples: `use perl_symbol_types::VarKind;` → `use perl_symbol::VarKind;` (lines ~35 and ~182)
- All items remain `pub` (the facade model preserves public surface within the types module; api.rs controls what's exposed externally)

**Verify:** `cargo check -p perl-symbol 2>&1 | grep -E "error|types" | head -10`

---

### Step 2.2: Create `src/cursor/mod.rs` from `perl-symbol-cursor/src/lib.rs`

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/src/cursor/mod.rs`

**Action:** Copy complete content of `crates/perl-symbol-cursor/src/lib.rs` into the new file.

**Edits during copy:**
- Remove crate-level `//!` module docstring if redundant with lib.rs
- No external crate imports to rewrite (cursor is standalone)

**Verify:** `cargo check -p perl-symbol 2>&1 | grep -E "error|cursor" | head -10`

---

### Step 2.3: Create `src/index/mod.rs` from `perl-symbol-index/src/lib.rs`

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/src/index/mod.rs`

**Action:** Copy complete content of `crates/perl-symbol-index/src/lib.rs` into the new file.

**Edits during copy:**
- Preserve the `#![deny(unsafe_code)]` and `#![warn(...)]` attributes — but remove the `#!` form (crate-level attrs) and convert to module-level `#![...]` INSIDE the module if they remain, OR drop them if the workspace lints cover them. Simplest: drop module-level crate attrs since `[lints] workspace = true` in the facade's Cargo.toml covers them.
- Remove crate-level `//!` module docstring if redundant

**Verify:** `cargo check -p perl-symbol 2>&1 | grep -E "error|index" | head -10`

---

### Step 2.4: Create `src/surface/mod.rs` + `src/surface/decl.rs` from `perl-symbol-surface/src/`

**Files:**
- `/h/Code/Rust/perl-lsp/crates/perl-symbol/src/surface/mod.rs` (from surface/src/lib.rs)
- `/h/Code/Rust/perl-lsp/crates/perl-symbol/src/surface/decl.rs` (from surface/src/decl.rs)

**Action for surface/mod.rs:** Copy `crates/perl-symbol-surface/src/lib.rs` contents. Update:
- `pub mod decl;` stays
- `pub use decl::{SymbolDecl, extract_symbol_decls};` stays
- Doc example line 21: `use perl_symbol_surface::extract_symbol_decls;` → `use perl_symbol::surface::extract_symbol_decls;`

**Action for surface/decl.rs:** Copy `crates/perl-symbol-surface/src/decl.rs` contents. Update:
- Line 14: `use perl_symbol_types::{SymbolKind, VarKind};` → `use crate::types::{SymbolKind, VarKind};`
- Any other `perl_symbol_types` references become `crate::types::`

**Verify:** `cargo check -p perl-symbol 2>&1 | grep -E "error|surface" | head -15`

---

### Step 2.5: Full-crate check after module population

**Command:**
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-symbol 2>&1 | tail -20
```

**Expected:** Clean compile or only minor warnings. If errors, resolve before proceeding.

---

## Phase 3: Update Workspace Root `Cargo.toml`

### Step 3.1: Register `perl-symbol` and remove 4 satellites from `[workspace.members]`

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml`

**Changes in `[workspace.members]` array:**

1. **Remove** `"crates/perl-symbol-surface",` (currently line 70)
2. **Remove** `"crates/perl-symbol-types",` (currently line 81)
3. **Remove** `"crates/perl-symbol-cursor",` (currently line 82)
4. **Remove** `"crates/perl-symbol-index",` (currently line 83)
5. **Add** `"crates/perl-symbol",` — insert in an appropriate tier location (suggest near the Tier 2+ section — around where symbol-types was, e.g., line 81)

**Warning:** `perl-symbol-surface` is SEPARATED (line 70) from the cluster at 81-83. Do NOT apply contiguous-block replacement.

**Verify:** `grep -n 'perl-symbol' Cargo.toml | head -20` — should show only `perl-symbol` entries, no `perl-symbol-*`.

---

### Step 3.2: Update `[workspace.dependencies]`

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml`

**Changes in `[workspace.dependencies]` block:**

1. **Remove** `perl-symbol-types = { path = "crates/perl-symbol-types", version = "0.12.4" }` (line 275)
2. **Remove** `perl-symbol-cursor = { path = "crates/perl-symbol-cursor", version = "0.12.4" }` (line 276)
3. **Remove** `perl-symbol-index = { path = "crates/perl-symbol-index", version = "0.12.4" }` (line 277)
4. **Remove** `perl-symbol-surface = { path = "crates/perl-symbol-surface", version = "0.12.4" ... }` (line 290 — standalone, NOT in the cluster at 275-277)
5. **Add** `perl-symbol = { path = "crates/perl-symbol", version = "0.12.4" }` — insert near the former cluster

**Warning:** Same as 3.1 — `perl-symbol-surface` is SEPARATED from the other three. Two separate removal points.

**Verify:** `grep -n 'perl-symbol' Cargo.toml` — no `perl-symbol-types`, no `-cursor`, no `-index`, no `-surface`; one `perl-symbol` entry.

---

### Step 3.3: Update `[workspace.metadata.publish].allow`

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml`

**Changes:**

1. **Remove** `"perl-symbol-types",` (line 158)
2. **Remove** `"perl-symbol-surface",` (line 159)
3. **Remove** `"perl-symbol-cursor",` (line 160)
4. **Remove** `"perl-symbol-index",` (line 161)
5. **Add** `"perl-symbol",` — insert at the same location (tier 2+)

**Net change:** -4 + 1 = -3 entries.

**Verify:**
```bash
cd /h/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); allow=d['metadata']['publish']['allow']; print('Count:',len(allow)); print('Has perl-symbol:', 'perl-symbol' in allow); print('Has satellites:', any(x in allow for x in ['perl-symbol-types','perl-symbol-cursor','perl-symbol-index','perl-symbol-surface']))"
```

---

### Step 3.4: Verify workspace parses

**Command:**
```bash
cd /h/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 2>&1 | head -3
```

**Expected:** JSON output (no parse errors). Any cargo error at this point means the Cargo.toml edits are inconsistent.

---

## Phase 4: Migrate Test Files

Copy 6 test files + create 1 new facade test. All go into `crates/perl-symbol/tests/`.

### Step 4.1: Migrate types tests (2 files)

**Source → Destination:**

- `crates/perl-symbol-types/tests/comprehensive_unit_tests.rs` → `crates/perl-symbol/tests/types_comprehensive_unit_tests.rs` (RENAMED to avoid collision with cursor)
- `crates/perl-symbol-types/tests/symbol_types_extended.rs` → `crates/perl-symbol/tests/types_extended.rs`

**For each file:**
1. Copy content to new destination
2. Update imports: `use perl_symbol_types::` → `use perl_symbol::` (tests import `SymbolKind`/`VarKind` directly; crate-root re-export makes this work)

**Verify:** `cargo test -p perl-symbol --test types_comprehensive_unit_tests 2>&1 | tail -5`

---

### Step 4.2: Migrate cursor tests (2 files)

**Source → Destination:**

- `crates/perl-symbol-cursor/tests/comprehensive_unit_tests.rs` → `crates/perl-symbol/tests/cursor_comprehensive_unit_tests.rs` (RENAMED)
- `crates/perl-symbol-cursor/tests/cursor_symbol_bdd.rs` → `crates/perl-symbol/tests/cursor_bdd.rs` (RENAMED to drop redundant "symbol_" prefix)

**For each file:**
1. Copy content
2. Update imports: `use perl_symbol_cursor::` → `use perl_symbol::cursor::`

**Verify:** `cargo test -p perl-symbol --test cursor_comprehensive_unit_tests 2>&1 | tail -5`

---

### Step 4.3: Migrate index tests (1 file)

**Source → Destination:**

- `crates/perl-symbol-index/tests/trie_and_fuzzy.rs` → `crates/perl-symbol/tests/index_trie_and_fuzzy.rs` (RENAMED with prefix for consistency)

**Edits:** `use perl_symbol_index::` → `use perl_symbol::index::` (or `use perl_symbol::SymbolIndex;` via crate root)

**Verify:** `cargo test -p perl-symbol --test index_trie_and_fuzzy 2>&1 | tail -5`

---

### Step 4.4: Migrate surface tests (1 file)

**Source → Destination:**

- `crates/perl-symbol-surface/tests/symbol_decl_tests.rs` → `crates/perl-symbol/tests/surface_decl.rs` (RENAMED)

**Edits:**
- `use perl_symbol_surface::{SymbolDecl, extract_symbol_decls};` → `use perl_symbol::surface::{SymbolDecl, extract_symbol_decls};` (or via crate root: `use perl_symbol::{SymbolDecl, extract_symbol_decls};`)
- `use perl_symbol_types::{SymbolKind, VarKind};` → `use perl_symbol::{SymbolKind, VarKind};`

**Verify:** `cargo test -p perl-symbol --test surface_decl 2>&1 | tail -5`

---

### Step 4.5: Create `facade_api_completeness.rs` (NEW — required per Wave 1 pattern)

**File:** `/h/Code/Rust/perl-lsp/crates/perl-symbol/tests/facade_api_completeness.rs`

**Content:**

```rust
//! Guards the public API surface of `perl-symbol`. If an item listed here becomes
//! inaccessible at the documented path, this test fails — catching accidental
//! API breakage during future refactoring.

use perl_symbol::{
    SymbolKind, VarKind,
    cursor::{
        CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source,
        get_symbol_range_at_position, is_modchar, is_word_boundary, token_under_cursor,
    },
    index::SymbolIndex,
    surface::{SymbolDecl, extract_symbol_decls},
};

#[test]
fn symbol_kind_and_var_kind_accessible_at_crate_root() {
    let _k = SymbolKind::Subroutine;
    let _v = VarKind::Scalar;
}

#[test]
fn cursor_surface_accessible() {
    let _k = CursorSymbolKind::Scalar;
    // Smoke-check each function exists at the path (no-op calls)
    let _ = extract_symbol_from_source(0, "");
    let _ = get_symbol_range_at_position(0, "");
    let _ = byte_offset_utf16("", 0);
    let _ = is_modchar('a');
    let _ = is_word_boundary("", 0);
    let _ = token_under_cursor("", 0);
}

#[test]
fn symbol_index_accessible() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("Foo::bar".to_string());
    assert!(!idx.search_prefix("Foo").is_empty());
}

#[test]
fn surface_decl_accessible() {
    // SymbolDecl and extract_symbol_decls must be callable at perl_symbol::surface
    // (and at crate root via re-export). Compilation is the test.
    let _: fn(&perl_ast::Node, Option<&str>) -> Vec<SymbolDecl> = extract_symbol_decls;
}
```

**Note:** Function signatures inside smoke calls must match the real types. If real signatures differ (e.g., `byte_offset_utf16` takes different params), adjust. The primary intent is to import each item at the documented path; compilation = pass.

**Verify:** `cargo test -p perl-symbol --test facade_api_completeness 2>&1 | tail -5`

---

### Step 4.6: Full test run for new crate

**Command:**
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-symbol 2>&1 | tail -15
```

**Expected:** All 7 test binaries pass.

---

## Phase 5: Update Consumer Crates

### Step 5.1: `perl-workspace-index`

**Cargo.toml file:** `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/Cargo.toml`

**Changes:** Line 31: `perl-symbol-types = { workspace = true }` → `perl-symbol = { workspace = true }`

**Source file:** `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/workspace/workspace_index.rs`

**Changes:**
- Line 1020 (comment): `// Re-export the unified symbol types from perl-symbol-types` → `// Re-export the unified symbol types from perl-symbol`
- Line 1022: `pub use perl_symbol_types::{SymbolKind, VarKind};` → `pub use perl_symbol::{SymbolKind, VarKind};`

**Source file:** `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/dual_indexing_tests.rs`

**Changes:**
- Line 442 (or any line): `perl_symbol_types::SymbolKind::Package` → `perl_symbol::SymbolKind::Package`

**Verify:** `cargo build -p perl-workspace-index 2>&1 | tail -10`

---

### Step 5.2: `perl-semantic-analyzer`

**Cargo.toml:** `/h/Code/Rust/perl-lsp/crates/perl-semantic-analyzer/Cargo.toml`

**Changes:** Line 28: `perl-symbol-types = { workspace = true }` → `perl-symbol = { workspace = true }`

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-semantic-analyzer/src/analysis/symbol.rs`

**Changes:**
- Line 35 (comment): `// Re-export the unified symbol types from perl-symbol-types` → `// Re-export the unified symbol types from perl-symbol`
- Line 37: `pub use perl_symbol_types::{SymbolKind, VarKind};` → `pub use perl_symbol::{SymbolKind, VarKind};`

**Verify:** `cargo build -p perl-semantic-analyzer 2>&1 | tail -10`

---

### Step 5.3: `perl-lsp`

**Cargo.toml:** `/h/Code/Rust/perl-lsp/crates/perl-lsp/Cargo.toml`

**Changes:** Line 82: `perl-symbol-cursor = { workspace = true }` → `perl-symbol = { workspace = true }`

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/util/mod.rs`

**Changes:**
- Line 17: `pub use perl_symbol_cursor::{byte_offset_utf16, is_modchar, is_word_boundary, token_under_cursor};` → `pub use perl_symbol::cursor::{byte_offset_utf16, is_modchar, is_word_boundary, token_under_cursor};`

**Verify:** `RUST_TEST_THREADS=2 cargo build -p perl-lsp-rs 2>&1 | tail -10`

---

### Step 5.4: `perl-lsp-rename`

**Cargo.toml:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rename/Cargo.toml`

**Changes:** Line 23: `perl-symbol-cursor = { workspace = true }` → `perl-symbol = { workspace = true }`

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rename/src/rename/resolve.rs`

**Changes:**
- Line 7: `use perl_symbol_cursor as cursor;` → `use perl_symbol::cursor as cursor;`
- Any references `cursor::CursorSymbolKind` remain unchanged (the local alias stays `cursor`).

**Verify:** `cargo build -p perl-lsp-rename 2>&1 | tail -10`

---

### Step 5.5: `perl-lsp-performance`

**Cargo.toml:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-performance/Cargo.toml`

**Changes:** Line 21: `perl-symbol-index = { workspace = true }` → `perl-symbol = { workspace = true }`

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-performance/src/lib.rs`

**Changes:**
- Line 14: `pub use perl_symbol_index::SymbolIndex;` → `pub use perl_symbol::SymbolIndex;` (via crate-root re-export) or `pub use perl_symbol::index::SymbolIndex;`

**Verify:** `cargo build -p perl-lsp-performance 2>&1 | tail -10`

---

### Step 5.6: Update comment-only reference in `perl-lsp-workspace-symbols`

**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-workspace-symbols/src/lib.rs`

**Changes:** Line 298: `// Symbol kind conversion is handled by perl_symbol_types::SymbolKind::to_lsp_kind()` → `// Symbol kind conversion is handled by perl_symbol::SymbolKind::to_lsp_kind()`

**No Cargo.toml change** (this crate doesn't currently depend on any perl-symbol-* crate).

**Verify:** `cargo build -p perl-lsp-workspace-symbols 2>&1 | tail -5`

---

## Phase 6: Delete Old Crate Directories

### Step 6.1: Delete 4 satellite directories

**Command:**
```bash
rm -rf /h/Code/Rust/perl-lsp/crates/perl-symbol-types \
       /h/Code/Rust/perl-lsp/crates/perl-symbol-cursor \
       /h/Code/Rust/perl-lsp/crates/perl-symbol-index \
       /h/Code/Rust/perl-lsp/crates/perl-symbol-surface
```

**Verify:**
```bash
ls -d /h/Code/Rust/perl-lsp/crates/perl-symbol* 2>&1
```
Expected: only `/h/Code/Rust/perl-lsp/crates/perl-symbol`.

---

## Phase 7: Hygiene / Hardcoded Strings

### Step 7.1: Check for hardcoded crate name references

**Command:**
```bash
cd /h/Code/Rust/perl-lsp && grep -rn 'perl-symbol-types\|perl-symbol-cursor\|perl-symbol-index\|perl-symbol-surface' crates/ --include='*.rs' --include='*.toml' --include='*.md' 2>&1 | grep -v '.spec/' | head -20
```

**Expected:** Zero hits in `crates/` after Phase 6. If any remain (e.g., in CI hygiene tooling or test snapshot string literals), update them. Wave A found 2 such hits (perl-ci-hygiene and perl-parser missing_docs_ac_tests.rs).

---

### Step 7.2: Update any found hardcoded strings

**Files to check if grep hits:**
- `crates/perl-ci-hygiene/src/main.rs` — if present, rename all 4 satellite names to `perl-symbol`
- `crates/perl-parser/tests/missing_docs_ac_tests.rs` — if present, same
- `docs/` markdown files — if present, update references (low priority; can be deferred)

**Verify:**
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-ci-hygiene 2>&1 | tail -5
```

---

## Phase 8: Final Verification

### Step 8.1: No old crate names in source

```bash
cd /h/Code/Rust/perl-lsp && grep -rn 'perl_symbol_types\|perl_symbol_cursor\|perl_symbol_index\|perl_symbol_surface' crates/ --include='*.rs' 2>&1 | head -20
```

**Expected:** Zero hits.

```bash
cd /h/Code/Rust/perl-lsp && grep -rn 'perl-symbol-types\|perl-symbol-cursor\|perl-symbol-index\|perl-symbol-surface' crates/ --include='*.toml' 2>&1 | head -20
```

**Expected:** Zero hits.

---

### Step 8.2: Workspace member count

```bash
cd /h/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print('Members:', len(d['workspace_members']))"
```

**Expected:** Previous count − 3 (4 removed, 1 added). Verify by comparing to pre-change count captured before Phase 1 (run `cargo metadata` once before starting and record).

---

### Step 8.3: Publish allowlist count

```bash
cd /h/Code/Rust/perl-lsp && cargo xtask publish-closure 2>&1 | tail -10
```

**Expected:**
- `perl-symbol` present in the closure
- None of `perl-symbol-types`, `perl-symbol-cursor`, `perl-symbol-index`, `perl-symbol-surface` present
- Allowlist count decreased by 3 (4 removed, 1 added)

---

### Step 8.4: Full test suite

```bash
cd /h/Code/Rust/perl-lsp && cargo test --workspace --lib 2>&1 | tail -30
```

**Expected:** All tests pass (existing failures excluded by `.ci/blockers.yaml` if any).

```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-symbol 2>&1 | tail -10
```

**Expected:** All 7 test binaries pass.

---

### Step 8.5: LSP test threading

```bash
cd /h/Code/Rust/perl-lsp && RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2 2>&1 | tail -20
```

**Expected:** perl-lsp-rs tests pass with enforced threading (per its CLAUDE.md).

---

### Step 8.6: Clippy and formatting

```bash
cd /h/Code/Rust/perl-lsp && cargo clippy --workspace --lib 2>&1 | tail -20
```

**Expected:** No new warnings.

```bash
cd /h/Code/Rust/perl-lsp && cargo xtask fmt 2>&1 | tail -5
```

**Expected:** Formatting clean.

---

### Step 8.7: All 5 consumers build

```bash
cd /h/Code/Rust/perl-lsp && \
  cargo build -p perl-workspace-index && \
  cargo build -p perl-semantic-analyzer && \
  cargo build -p perl-lsp-rs && \
  cargo build -p perl-lsp-rename && \
  cargo build -p perl-lsp-performance && \
  echo "All 5 consumer crates built successfully"
```

---

## Compilation Checkpoints

- **After Phase 1:** `cargo check -p perl-symbol` — may fail (empty modules); that's OK
- **After Phase 2:** `cargo check -p perl-symbol` — MUST succeed (modules populated, api.rs resolves)
- **After Phase 3:** `cargo metadata --no-deps` — MUST succeed (workspace parse clean)
- **After Phase 4:** `cargo test -p perl-symbol` — all 7 test binaries pass
- **After Phase 5:** Each consumer builds individually; `cargo build --workspace --lib` builds
- **After Phase 6:** `cargo build --workspace` — clean (no dangling directory)
- **After Phase 7:** `cargo build -p perl-ci-hygiene` — if touched
- **After Phase 8:** Full verification pass — all green

---

## Notes for Builder

1. **Pre-change baseline:** Before starting Phase 1, run `cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(len(json.load(sys.stdin)['workspace_members']))"` and `cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(len(json.load(sys.stdin)['metadata']['publish']['allow']))"`. Record both numbers. Expected delta: members -3, allowlist -3.

2. **Cargo.toml scattered edits:** `perl-symbol-surface` is separated from the cluster in both `[workspace.members]` (line 70 vs. 81-83) and `[workspace.dependencies]` (line 290 vs. 275-277). Search-and-edit, don't assume contiguous blocks.

3. **Edition 2024:** Workspace-inherited — verify `edition.workspace = true` in new Cargo.toml (Wave 1 gotcha: builder forgot and fell back to 2021).

4. **doctest = false:** Mandatory in `[lib]` section; satellites all disabled; preserve.

5. **Consumer re-export pattern:** `pub use perl_symbol::{SymbolKind, VarKind}` works because of crate-root re-export in api.rs. If that re-export fails, downstream consumers (perl-semantic-analyzer's public API) break.

6. **Test imports:** Most test files can use `use perl_symbol::{SymbolKind, VarKind}` at crate root. Module-qualified paths (`use perl_symbol::cursor::...`, `use perl_symbol::surface::...`) are also valid — pick one consistently.

7. **Commit strategy:** Consider one commit per phase for clarity; squash-merge into a single PR commit. Use conventional commit format: `refactor(symbol): collapse perl-symbol-* (4 crates) → perl-symbol facade (Wave B) (#4428)`.

8. **PR title suffix:** Must end with `(#4428)` for validate-title CI (MEMORY: `feedback_validate_title_issue_ref.md`).

9. **Branch is based on post-Wave-A master:** Rebase-before-PR only if newer Wave commits land on master before this PR opens.

---

## Change Order Summary

1. Create new crate `perl-symbol/` with Cargo.toml, lib.rs, api.rs, CLAUDE.md, README.md
2. Absorb 4 satellites into `src/{types,cursor,index,surface}/` — update internal imports (types → crate::types)
3. Update root Cargo.toml: remove 4 members, remove 4 deps, remove 4 allowlist entries, add 1 each
4. Migrate 6 test files + add `facade_api_completeness.rs` (7 total)
5. Update 5 consumer Cargo.toml files and their source imports (6 source files including comment-only update)
6. Delete 4 satellite directories
7. Scan/update any hardcoded crate name strings
8. Full verification: grep, metadata, publish-closure, test, clippy, fmt

**Estimated effort:** 3-5 hours. Bulk is mechanical; validation is thorough.
