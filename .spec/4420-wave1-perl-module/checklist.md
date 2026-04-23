# Wave 1 PILOT Implementation Checklist — perl-module-* → perl-module

**Issue:** #4420
**Branch:** `impl/wave1-perl-module-4420`
**Scope:** Absorb 13 perl-module-* crates into single published perl-module facade

---

## Phase 0: Skeleton & Registration

### Step 0a: Create crates/perl-module/ directory and Cargo.toml

**File:** `/h/Code/Rust/perl-lsp/crates/perl-module/Cargo.toml` (CREATE)

```toml
[package]
name = "perl-module"
version = "0.14.0"
publish = true
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/onur-ozkan/perl-lsp"
description = "Perl module resolution, import analysis, and refactoring — unified facade"

[dependencies]
url = { workspace = true }
perl-path-security = { workspace = true }
perl-workspace-folder = { workspace = true }
perl-text-line = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }
perl-tdd-support = { workspace = true }

[[test]]
name = "name"
path = "tests/name_*.rs"

[[test]]
name = "path"
path = "tests/path_*.rs"

[[test]]
name = "token_core"
path = "tests/token_core_*.rs"

[[test]]
name = "boundary"
path = "tests/boundary_*.rs"

[[test]]
name = "token"
path = "tests/token_*.rs"

[[test]]
name = "import"
path = "tests/import_*.rs"

[[test]]
name = "token_parser"
path = "tests/token_parser_*.rs"

[[test]]
name = "reference"
path = "tests/reference_*.rs"

[[test]]
name = "import_match"
path = "tests/import_match_*.rs"

[[test]]
name = "rename"
path = "tests/rename_*.rs"

[[test]]
name = "resolution"
path = "tests/resolution_*.rs"
```

**After step 0a:** `cargo build -p perl-module --lib` should fail (no src/lib.rs yet), not error on Cargo.toml syntax.

**Verify command:** `cargo metadata -p perl-module --format-version 1 | jq '.packages[0].name'`
Expected: `"perl-module"`

---

### Step 0b: Add to workspace.members + workspace.dependencies

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml`

**Change 1: Add to [workspace] members (line 89)**

Find section starting at line 89 with `"crates/perl-module-boundary"`. Replace:
```toml
    "crates/perl-module-boundary",
    "crates/perl-module-token-core",
    ...
    "crates/perl-module-resolution-uri",
```

with (insert new crate before the 13 retiring ones):
```toml
    "crates/perl-module",
    "crates/perl-module-boundary",
    "crates/perl-module-token-core",
    ...
    "crates/perl-module-resolution-uri",
```

**Change 2: Add to [workspace.dependencies] (after line 355)**

Insert before the old perl-module-* entries (after `perl-symbol-index`):
```toml
perl-module = { path = "crates/perl-module", version = "0.14.0" }
```

Note: Keep all 13 old entries for now — they'll be removed in Phase 2.

**After step 0b:** `cargo metadata` should list both new perl-module and old 13 crates.

**Verify command:** `rtk cargo metadata --no-deps | jq -r '.packages[] | select(.name == "perl-module") | .id'`
Expected: non-empty ID string

---

### Step 0c: Update publish allowlist

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml` (lines 218-235)

**Change:** Replace the 7 existing perl-module-* entries with single entry:

Find (line 218 area):
```toml
    # Tier 3 — Module resolution chain
    "perl-module-name",
    "perl-module-path",
    "perl-module-token-core",
    "perl-module-boundary",
    "perl-module-token",
    "perl-module-import",
```

Replace with:
```toml
    # Tier 3 — Module resolution chain (unified facade; absorbs perl-module-* in Wave 1)
    "perl-module",
```

Keep the 7 old entries as-is (will be removed in final cleanup phase). This temporarily lists both; the build will pass.

**After step 0c:** `cargo xtask publish-closure` should list perl-module as publishable.

**Verify command:** `cargo xtask publish-closure 2>&1 | grep "perl-module"`
Expected: exactly one line with `perl-module`

---

## Phase 1: Copy Source Files by Dependency Layer

Follow DAG order: leaves first (name, token_core), then L2 (path), then L3-L6 up to resolution.

### Step 1a: Create src/lib.rs skeleton

**File:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/lib.rs` (CREATE)

```rust
// LAYER 1 (leaves)
pub mod name;
pub mod token_core;

// LAYER 2
pub mod path;

// LAYER 3
pub mod import;

// LAYER 4
pub mod boundary;

// LAYER 5
pub mod reference;
pub mod token;

// LAYER 6
pub mod token_parser;

// LAYER 7
pub mod import_match;

// LAYER 8
pub mod rename;

// LAYER 9
pub mod resolution;

pub mod api;
pub use api::*;
```

**After step 1a:** No compilation yet (modules don't exist).

---

### Step 1b: Copy perl-module-name source to crates/perl-module/src/name/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-name/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/name/mod.rs` (CREATE)

1. Copy entire content from `crates/perl-module-name/src/lib.rs`
2. Paste into `crates/perl-module/src/name/mod.rs`
3. Remove `#![...]` crate-level attributes (keep module-internal logic)
4. Change all `pub ` at module root to `pub(crate) ` (except items in the module)

**Note:** name is a leaf — no internal deps. Keep all items as-is.

**After step 1b:** `cargo build -p perl-module --lib 2>&1 | grep -A 5 "error\[E"` should show errors about missing modules (path, etc.). Expected error count: ~6 (one per missing dep).

**Verify command:** `cargo build -p perl-module --lib 2>&1 | grep "unresolved" | wc -l`

---

### Step 1c: Copy perl-module-token-core source to crates/perl-module/src/token_core/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-token-core/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/token_core/mod.rs` (CREATE)

Same process as 1b.

**After step 1c:** Still errors about missing modules (path, import, etc.).

---

### Step 1d: Copy perl-module-path source to crates/perl-module/src/path/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-path/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/path/mod.rs` (CREATE)

Note: path depends on name. Update imports:
- Change `use perl_module_name::` to `use crate::name::`

**After step 1d:** `cargo build -p perl-module --lib 2>&1 | grep "unresolved"` count should drop by 1 (path now resolves).

---

### Step 1e: Copy perl-module-import source to crates/perl-module/src/import/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-import/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/import/mod.rs` (CREATE)

Update imports:
- `use perl_module_path::` → `use crate::path::`

**After step 1e:** Incremental progress on error count.

---

### Step 1f: Copy perl-module-boundary source to crates/perl-module/src/boundary/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-boundary/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/boundary/mod.rs` (CREATE)

Update imports:
- `use perl_module_token_core::` → `use crate::token_core::`
- `use perl_module_import::` → `use crate::import::`
- `use perl_module_name::` → `use crate::name::`

**After step 1f:** Further progress.

---

### Step 1g: Copy perl-module-reference source to crates/perl-module/src/reference/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-reference/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/reference/mod.rs` (CREATE)

Update imports:
- `use perl_module_name::` → `use crate::name::`
- `use perl_module_token_parser::` → `use crate::token_parser::`
- `use perl_module_import::` → `use crate::import::`
- `use perl_module_path::` → `use crate::path::`

Note: token_parser doesn't exist yet; this will fail. Continue anyway; we fix in next steps.

**After step 1g:** Expected unresolved: token_parser (not yet copied).

---

### Step 1h: Copy perl-module-token source to crates/perl-module/src/token/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-token/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/token/mod.rs` (CREATE)

Update imports:
- `use perl_module_boundary::` → `use crate::boundary::`
- `use perl_module_name::` → `use crate::name::`
- `use perl_module_path::` → `use crate::path::`
- `use perl_module_token_core::` → `use crate::token_core::`

**After step 1h:** Further resolution.

---

### Step 1i: Copy perl-module-token-parser source to crates/perl-module/src/token_parser/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-token-parser/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/token_parser/mod.rs` (CREATE)

Update imports:
- `use perl_module_token_core::` → `use crate::token_core::`
- `use perl_module_reference::` → `use crate::reference::`

**After step 1i:** reference should now resolve.

---

### Step 1j: Copy perl-module-import-match source to crates/perl-module/src/import_match/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-import-match/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/import_match/mod.rs` (CREATE)

Update imports:
- `use perl_module_boundary::` → `use crate::boundary::`
- `use perl_module_import::` → `use crate::import::`
- `use perl_module_path::` → `use crate::path::`
- `use perl_module_token::` → `use crate::token::`

**After step 1j:** More progress.

---

### Step 1k: Copy perl-module-rename source to crates/perl-module/src/rename/

**Source:** `/h/Code/Rust/perl-lsp/crates/perl-module-rename/src/lib.rs`
**Target:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/rename/mod.rs` (CREATE)

Update imports:
- `use perl_module_import_match::` → `use crate::import_match::`
- `use perl_module_path::` → `use crate::path::`
- `use perl_module_token::` → `use crate::token::`

**After step 1k:** Getting close.

---

### Step 1l: Create crates/perl-module/src/resolution/ subfolder and copy modules

**Action:** Create `/h/Code/Rust/perl-lsp/crates/perl-module/src/resolution/mod.rs` (CREATE)

**Content:** Copy from `/h/Code/Rust/perl-lsp/crates/perl-module-resolution/src/lib.rs` with import updates.

**Action:** Create `/h/Code/Rust/perl-lsp/crates/perl-module/src/resolution/path.rs` (CREATE)

**Content:** Copy from `/h/Code/Rust/perl-lsp/crates/perl-module-resolution-path/src/lib.rs` with import updates.

**Action:** Create `/h/Code/Rust/perl-lsp/crates/perl-module/src/resolution/uri.rs` (CREATE)

**Content:** Copy from `/h/Code/Rust/perl-lsp/crates/perl-module-resolution-uri/src/lib.rs` with import updates.

In resolution/mod.rs, add:
```rust
pub mod path;
pub mod uri;

pub use path::*;
pub use uri::*;
```

Update resolution/mod.rs imports:
- `use perl_module_resolution_path::` → `use crate::resolution::path::`
- `use perl_module_resolution_uri::` → `use crate::resolution::uri::`

Update resolution/path.rs imports:
- `use perl_module_path::` → `use crate::path::`

Update resolution/uri.rs imports:
- `use perl_module_path::` → `use crate::path::`

**After step 1l:** `cargo build -p perl-module --lib 2>&1 | grep -c "error"` should be 0 (all internal deps resolved).

**Verify command:** `cargo build -p perl-module --lib`
Expected: Success.

---

## Phase 2: Create API Facade

### Step 2a: Create src/api.rs

**File:** `/h/Code/Rust/perl-lsp/crates/perl-module/src/api.rs` (CREATE)

```rust
//! Public API facade for perl-module.
//!
//! All items are re-exported from internal modules via this facade.
//! Consumers should import from `perl_module` only, not from submodules.

pub use crate::name::{ModuleName, QualifiedName};
pub use crate::path::ModulePath;
pub use crate::import::Import;
pub use crate::import_match::ImportMatch;
pub use crate::boundary::BoundaryRule;
pub use crate::reference::ModuleReference;
pub use crate::rename::RenamePlan;
pub use crate::resolution::{Resolver, Resolution};
```

Note: This is a template. Adjust the exported items to match the actual public API of each module (run clippy and check which items are actually public).

**After step 2a:** `cargo build -p perl-module --lib` should still succeed.

---

### Step 2b: Copy test files

**Action:** Copy all test files from 13 crates/perl-module-*/tests/ directories to crates/perl-module/tests/:

```bash
for dir in crates/perl-module-name crates/perl-module-path ... crates/perl-module-resolution-uri; do
  cp "$dir"/tests/*.rs crates/perl-module/tests/
done
```

Update all imports in test files from `use perl_module_X::` to `use perl_module::X::`.

**After step 2b:** `cargo test -p perl-module --lib` should compile (test files processed). May have test failures if test dependencies or test setup differ.

**Verify command:** `cargo test -p perl-module --lib --no-run 2>&1 | tail -5`
Expected: "Finished" or test binary paths, not errors.

---

## Phase 3: Update External Consumers

### Step 3a: Update perl-lsp Cargo.toml

**File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp/Cargo.toml`

Replace:
```toml
perl-module-resolution = { workspace = true }
perl-module-import = { workspace = true }
perl-module-reference = { workspace = true }
perl-module-rename = { workspace = true }
perl-module-path = { workspace = true }
```

with:
```toml
perl-module = { workspace = true }
```

**After step 3a:** Cargo.toml syntax valid, not yet compiled.

---

### Step 3b: Update perl-lsp source files

**Files affected:** (from issue body)
- `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs`
- `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/language/misc.rs`
- `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/util/mod.rs`
- `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/workspace.rs`

For each file, use global find-replace:
- `use perl_module_resolution::` → `use perl_module::resolution::`
- `use perl_module_import::` → `use perl_module::import::`
- `use perl_module_reference::` → `use perl_module::reference::`
- `use perl_module_rename::` → `use perl_module::rename::`
- `use perl_module_path::` → `use perl_module::path::`

**After step 3b:** `cargo build -p perl-lsp --lib` should compile (no more missing imports).

**Verify command:** `cargo build -p perl-lsp --lib 2>&1 | grep -c "error\[E"`
Expected: 0

---

### Step 3c: Update perl-lsp-completion

**File:** `/h/Code/Rust/perl-lsp-completion/Cargo.toml`

Replace:
```toml
perl-module-import = { workspace = true }
```

with:
```toml
perl-module = { workspace = true }
```

**File:** `/h/Code/Rust/perl-lsp-completion/src/completion.rs`

Replace:
- `use perl_module_import::` → `use perl_module::import::`

**After step 3c:** `cargo build -p perl-lsp-completion --lib` compiles.

---

### Step 3d: Update perl-lsp-document-links

**File:** `/h/Code/Rust/perl-lsp-document-links/Cargo.toml`

Replace:
```toml
perl-module-path = { workspace = true }
perl-module-import = { workspace = true }
```

with:
```toml
perl-module = { workspace = true }
```

**File:** `/h/Code/Rust/perl-lsp-document-links/src/lib.rs`

Replace:
- `use perl_module_path::` → `use perl_module::path::`
- `use perl_module_import::` → `use perl_module::import::`

---

### Step 3e: Update perl-lsp-workspace-symbols

**File:** `/h/Code/Rust/perl-lsp-workspace-symbols/Cargo.toml`

Replace:
```toml
perl-module-path = { workspace = true }
```

with:
```toml
perl-module = { workspace = true }
```

**File:** `/h/Code/Rust/perl-lsp-workspace-symbols/src/lib.rs`

Replace:
- `use perl_module_path::` → `use perl_module::path::`

---

### Step 3f: Update perl-dap

**File:** `/h/Code/Rust/perl-dap/Cargo.toml`

Replace:
```toml
perl-module-path = { workspace = true }
```

with:
```toml
perl-module = { workspace = true }
```

**File:** `/h/Code/Rust/perl-dap/src/debug_adapter/mod.rs`

Replace:
- `use perl_module_path::` → `use perl_module::path::`

---

### Step 3g: Update perl-refactoring

**File:** `/h/Code/Rust/perl-refactoring/Cargo.toml`

Replace:
```toml
perl-module-path = { workspace = true }
```

with:
```toml
perl-module = { workspace = true }
```

**File:** `/h/Code/Rust/perl-refactoring/src/refactor/workspace_refactor.rs`

Replace:
- `use perl_module_path::` → `use perl_module::path::`

---

### Step 3h: Update perl-text-line test

**File:** `/h/Code/Rust/perl-text-line/tests/text_line_integration.rs`

Replace:
- `use perl_module_token::` → `use perl_module::token::`
- `use perl_module_token_parser::` → `use perl_module::token_parser::`

**After step 3h:** All 6 Cargo.toml files + 1 test file updated.

**Verify command:** `cargo build -p perl-lsp-rs --release 2>&1 | grep -c "error"`
Expected: 0

---

## Phase 4: Retire Old Crates

### Step 4a: Remove from workspace.members

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml` (lines 89-102)

Delete the 13 lines:
```toml
    "crates/perl-module-boundary",
    "crates/perl-module-token-core",
    "crates/perl-module-token",
    "crates/perl-module-token-parser",
    "crates/perl-module-name",
    "crates/perl-module-reference",
    "crates/perl-module-import",
    "crates/perl-module-import-match",
    "crates/perl-module-path",
    "crates/perl-module-rename",
    "crates/perl-module-resolution",
    "crates/perl-module-resolution-path",
    "crates/perl-module-resolution-uri",
```

**After step 4a:** Cargo.toml still valid; workspace now has 135 - 13 + 1 = 123 members.

---

### Step 4b: Remove from workspace.dependencies

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml` (lines 340-353)

Delete the 13 old workspace.dependencies entries:
```toml
perl-module-boundary = ...
perl-module-token-core = ...
perl-module-token = ...
perl-module-token-parser = ...
perl-module-name = ...
perl-module-reference = ...
perl-module-import = ...
perl-module-import-match = ...
perl-module-path = ...
perl-module-rename = ...
perl-module-resolution = ...
perl-module-resolution-path = ...
perl-module-resolution-uri = ...
```

Keep the new `perl-module` entry.

**After step 4b:** `cargo metadata` now lists only perl-module (not the 13 old ones).

---

### Step 4c: Remove from publish allowlist

**File:** `/h/Code/Rust/perl-lsp/Cargo.toml` (around line 218)

Replace:
```toml
    # Tier 3 — Module resolution chain (unified facade; absorbs perl-module-* in Wave 1)
    "perl-module",
    "perl-module-name",
    "perl-module-path",
    "perl-module-token-core",
    "perl-module-boundary",
    "perl-module-token",
    "perl-module-import",
    "perl-module-token-parser",
    "perl-module-reference",
    "perl-module-import-match",
    "perl-module-rename",
    "perl-module-resolution-path",
    "perl-module-resolution-uri",
    "perl-module-resolution",
```

with:
```toml
    # Tier 3 — Module resolution chain (unified facade; absorbs perl-module-* in Wave 1)
    "perl-module",
```

**After step 4c:** Publish allowlist has 13 fewer entries, 1 new entry (net: -12).

**Verify command:** `cargo xtask publish-closure 2>&1 | grep -c "perl-module-"`
Expected: 0

---

### Step 4d: Delete 13 crate directories

**Action:** Delete:
- `/h/Code/Rust/perl-lsp/crates/perl-module-boundary/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-token-core/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-token/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-token-parser/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-name/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-reference/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-import/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-import-match/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-path/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-rename/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-resolution/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-resolution-path/`
- `/h/Code/Rust/perl-lsp/crates/perl-module-resolution-uri/`

**After step 4d:** Workspace directory structure cleaned.

**Verify command:** `find /h/Code/Rust/perl-lsp/crates -maxdepth 1 -name "perl-module-*" -type d | wc -l`
Expected: 0

---

## Phase 5: Verification & Testing

### Step 5a: Full workspace build

**Command:** `cargo build -p perl-lsp-rs --release`

**Expected:** Compilation succeeds with no errors or warnings related to module imports.

**Verify command:** Same as command; exit code 0.

---

### Step 5b: Test suite

**Command:** `cargo test -p perl-module --lib`

**Expected:** All 62 tests pass (62 test files copied and updated).

**Verify command:** `cargo test -p perl-module --lib 2>&1 | tail -3 | grep -E "^test result"`
Expected: `ok. [0-9]+ passed`

---

### Step 5c: Publish closure verification

**Command:** `cargo xtask publish-closure`

**Expected:** perl-module is listed exactly once; zero perl-module-* crates listed.

**Verify command:** `cargo xtask publish-closure 2>&1 | grep "perl-module" | wc -l`
Expected: 1

---

### Step 5d: Workspace member count

**Expected:** 135 - 13 + 1 = 123 members

**Verify command:** `cargo metadata --no-deps | jq '.workspace_members | length'`
Expected: 123

---

### Step 5e: pr-fast gate

**Command:** `just pr-fast` (or `cargo xtask fmt && cargo clippy --workspace`)

**Expected:** All formatting and linting passes.

---

## Completion Checklist

- [ ] Step 0a: Cargo.toml created
- [ ] Step 0b: Added to workspace.members and workspace.dependencies
- [ ] Step 0c: Added to publish allowlist
- [ ] Step 1a: src/lib.rs skeleton created
- [ ] Steps 1b–1k: All 11 module sources copied with import updates
- [ ] Step 1l: resolution/ subfolder with path.rs and uri.rs created
- [ ] Step 2a: src/api.rs facade created
- [ ] Step 2b: Test files copied and import paths updated
- [ ] Step 3a: perl-lsp Cargo.toml updated
- [ ] Step 3b: perl-lsp source files updated (4 files)
- [ ] Step 3c: perl-lsp-completion updated
- [ ] Step 3d: perl-lsp-document-links updated
- [ ] Step 3e: perl-lsp-workspace-symbols updated
- [ ] Step 3f: perl-dap updated
- [ ] Step 3g: perl-refactoring updated
- [ ] Step 3h: perl-text-line test updated
- [ ] Step 4a: Removed from workspace.members
- [ ] Step 4b: Removed from workspace.dependencies
- [ ] Step 4c: Removed from publish allowlist
- [ ] Step 4d: Deleted 13 crate directories
- [ ] Step 5a: cargo build -p perl-lsp-rs --release succeeds
- [ ] Step 5b: cargo test -p perl-module passes all 62 tests
- [ ] Step 5c: cargo xtask publish-closure shows perl-module only
- [ ] Step 5d: Workspace has exactly 123 members
- [ ] Step 5e: just pr-fast passes

---

## Ordering Rationale

1. **Skeleton first (Phase 0):** Register new crate in workspace before copying source.
2. **Leaves first (Phase 1):** Copy modules in dependency order to allow incremental compilation checks.
3. **Facade after modules (Phase 2):** api.rs needs all modules present.
4. **Update consumers after facade works (Phase 3):** Only update when source module is ready.
5. **Retire old crates last (Phase 4):** Never delete while consumers still reference them.
6. **Verify at the end (Phase 5):** Full build and test after all changes.

---

## Compilation Breakpoints

Each step compiles at point of completion (marked in "After step" notes). If a step doesn't compile, investigate the specific step before proceeding.

Example: If step 1d fails, check that path module imports use `crate::name::` not `perl_module_name::`.
