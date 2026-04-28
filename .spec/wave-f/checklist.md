# Wave F Implementation Checklist

## Overview

Absorb 8 `perl-lsp-feature-*` and `perl-lsp-capability-map` crates into new `perl-lsp-rs-core` implementation crate. Create `perl-lsp-rs` as thin UX facade re-exporting from `-core`. This mirrors Wave D's `perl-parser`/`perl-parser-core` split (Amendment 6, PR #4492).

**Published count:** 81 → 74 (8 removed, 1 added `perl-lsp-rs-core`, net −7).

**Total tasks:** 9 steps + 9 verify commands.

---

## Implementation Steps

### Step 1: Scaffold `perl-lsp-rs-core` crate directory

**File:** Create `crates/perl-lsp-rs-core/` directory structure.

**What to create:**

- Directory: `crates/perl-lsp-rs-core/`
- Directory: `crates/perl-lsp-rs-core/src/`
- Directory: `crates/perl-lsp-rs-core/tests/`
- File: `crates/perl-lsp-rs-core/Cargo.toml` (new)
- File: `crates/perl-lsp-rs-core/src/lib.rs` (new)
- File: `crates/perl-lsp-rs-core/src/features/mod.rs` (new)
- File: `crates/perl-lsp-rs-core/src/capability_map.rs` (new, initially empty)
- File: `crates/perl-lsp-rs-core/build.rs` (copied from `crates/perl-lsp-feature-contracts/`)
- File: `crates/perl-lsp-rs-core/features_sot.toml` (copied from `crates/perl-lsp-feature-contracts/`)

**Cargo.toml structure** (follow `crates/perl-parser-core/Cargo.toml` as template exactly):

```toml
[package]
name = "perl-lsp-rs-core"
description = "Implementation core for perl-lsp-rs (Wave F: feature flags, contracts, capability mapping)"
documentation = "https://docs.rs/perl-lsp-rs-core"
# Copy workspace-default fields from any workspace member

[lib]
doctest = false

[dependencies]
lsp-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[build-dependencies]
perl-feature-catalog = { workspace = true }

[features]
lsp-ga-lock = []

[dev-dependencies]
perl-tdd-support = { workspace = true }
serde_json = { workspace = true }

[lints]
workspace = true
```

**src/lib.rs** (initial):

```rust
pub mod capability_map;
pub mod features;
```

**src/features/mod.rs** (initial):

```rust
pub mod contracts;
pub mod flags;
pub mod grid;
pub mod ids;
pub mod policy;
pub mod profile;
pub mod profile_cli;
```

**Dependencies:** Must exist before step 2. Verify with:

```bash
cargo check -p perl-lsp-rs-core
```

Expect: build succeeds (empty modules are valid).

---

### Step 2: Move source code — 8 crates to modules

**Mapping:** Copy each source crate's `src/lib.rs` to target module path, then rewrite imports.

| Source crate | Source path | Target module | Target file |
|---|---|---|---|
| perl-lsp-feature-ids | `crates/perl-lsp-feature-ids/src/lib.rs` | `features::ids` | `src/features/ids.rs` |
| perl-lsp-feature-contracts | `crates/perl-lsp-feature-contracts/src/lib.rs` | `features::contracts` | `src/features/contracts.rs` |
| perl-lsp-feature-flags | `crates/perl-lsp-feature-flags/src/lib.rs` | `features::flags` | `src/features/flags.rs` |
| perl-lsp-feature-profile | `crates/perl-lsp-feature-profile/src/lib.rs` | `features::profile` | `src/features/profile.rs` |
| perl-lsp-feature-profile-cli | `crates/perl-lsp-feature-profile-cli/src/lib.rs` | `features::profile_cli` | `src/features/profile_cli.rs` |
| perl-lsp-feature-policy | `crates/perl-lsp-feature-policy/src/lib.rs` | `features::policy` | `src/features/policy.rs` |
| perl-lsp-feature-grid | `crates/perl-lsp-feature-grid/src/lib.rs` | `features::grid` | `src/features/grid.rs` |
| perl-lsp-capability-map | `crates/perl-lsp-capability-map/src/lib.rs` | `capability_map` | `src/capability_map.rs` |

**Import rewrites** (the 8 crates form a dependency chain; rewrite all cross-crate imports):

- `capability_map.rs`: `use perl_lsp_feature_ids::*` → `use crate::features::ids::*`
- `flags.rs`: `use perl_lsp_feature_ids::*` → `use crate::features::ids::*`
- `contracts.rs`: `use perl_lsp_capability_map::*` → `use crate::capability_map::*`
- `profile.rs`: `use perl_lsp_feature_contracts::*` → `use crate::features::contracts::*`
- `profile_cli.rs`: `use perl_lsp_feature_policy::*` → `use crate::features::policy::*`; `use perl_lsp_feature_profile::*` → `use crate::features::profile::*`
- `policy.rs`: similar rewrites for contracts, profile, flags, ids
- `grid.rs`: similar rewrites for contracts, policy, ids
- `ids.rs`: no rewrites (leaf module, no intra-wave dependencies)

**Verify** (before step 3):

```bash
cargo check -p perl-lsp-rs-core
```

Expect: build succeeds, no unresolved imports.

---

### Step 3: Move tests — 8 crates to `crates/perl-lsp-rs-core/tests/`

**Test file naming:** `feature_<short>_<original>.rs` flat in `tests/` directory.

| Source test file | Target test file |
|---|---|
| `crates/perl-lsp-feature-ids/tests/comprehensive_unit_tests.rs` | `tests/feature_ids_comprehensive.rs` |
| `crates/perl-lsp-feature-contracts/tests/comprehensive_unit_tests.rs` | `tests/feature_contracts_comprehensive.rs` |
| `crates/perl-lsp-feature-contracts/tests/extended_unit_tests.rs` | `tests/feature_contracts_extended.rs` |
| `crates/perl-lsp-feature-flags/tests/comprehensive_unit_tests.rs` | `tests/feature_flags_comprehensive.rs` |
| `crates/perl-lsp-feature-flags/tests/extended_unit_tests.rs` | `tests/feature_flags_extended.rs` |
| `crates/perl-lsp-feature-profile/tests/comprehensive_unit_tests.rs` | `tests/feature_profile_comprehensive.rs` |
| `crates/perl-lsp-feature-policy/tests/comprehensive_unit_tests.rs` | `tests/feature_policy_comprehensive.rs` |
| `crates/perl-lsp-feature-policy/tests/extended_unit_tests.rs` | `tests/feature_policy_extended.rs` |
| `crates/perl-lsp-feature-grid/tests/comprehensive_unit_tests.rs` | `tests/feature_grid_comprehensive.rs` |
| `crates/perl-lsp-feature-grid/tests/extended_unit_tests.rs` | `tests/feature_grid_extended.rs` |
| `crates/perl-lsp-capability-map/tests/comprehensive_unit_tests.rs` | `tests/capability_map_comprehensive.rs` |

**Note:** `perl-lsp-feature-profile-cli` has no `tests/` directory — nothing to move.

**Import rewrites in each test file:**

Replace `use perl_lsp_feature_*::` with `use perl_lsp_rs_core::features::*::` paths. Preserve all `#[cfg(feature = "lsp-ga-lock")]` gates.

**Verify** (before step 4):

```bash
cargo test -p perl-lsp-rs-core
```

Expect: all tests pass, no unresolved imports.

---

### Step 4: Update workspace root `Cargo.toml`

**File:** `Cargo.toml` (workspace root)

**[workspace.members]**

Remove these 8 entries (find and delete lines):
- `"crates/perl-lsp-feature-ids"`
- `"crates/perl-lsp-feature-contracts"`
- `"crates/perl-lsp-feature-flags"`
- `"crates/perl-lsp-feature-profile"`
- `"crates/perl-lsp-feature-profile-cli"`
- `"crates/perl-lsp-feature-policy"`
- `"crates/perl-lsp-feature-grid"`
- `"crates/perl-lsp-capability-map"`

Add new entry (in alphabetical order with other perl-lsp entries):
- `"crates/perl-lsp-rs-core"`

**[workspace.dependencies]**

Remove these 8 entries (find and delete lines starting with each crate name):
- `perl-lsp-feature-ids = { path = "crates/perl-lsp-feature-ids", version = "0.12.4" }`
- `perl-lsp-feature-contracts = { path = "crates/perl-lsp-feature-contracts", version = "0.12.4" }`
- `perl-lsp-feature-flags = { path = "crates/perl-lsp-feature-flags", version = "0.12.4" }`
- `perl-lsp-feature-profile = { path = "crates/perl-lsp-feature-profile", version = "0.12.4" }`
- `perl-lsp-feature-profile-cli = { path = "crates/perl-lsp-feature-profile-cli", version = "0.12.4" }`
- `perl-lsp-feature-policy = { path = "crates/perl-lsp-feature-policy", version = "0.12.4" }`
- `perl-lsp-feature-grid = { path = "crates/perl-lsp-feature-grid", version = "0.12.4" }`
- `perl-lsp-capability-map = { path = "crates/perl-lsp-capability-map", version = "0.12.4" }`

Add new entry (alphabetically in perl-lsp section):
- `perl-lsp-rs-core = { path = "crates/perl-lsp-rs-core", version = "0.12.4" }`

**[workspace.metadata.publish].allow**

Find the array containing the 8 crate names in the Tier 5 LSP governance block. Remove all 8 entries:
- `"perl-lsp-feature-ids"`
- `"perl-lsp-capability-map"`
- `"perl-lsp-feature-contracts"`
- `"perl-lsp-feature-flags"`
- `"perl-lsp-feature-profile"`
- `"perl-lsp-feature-profile-cli"`
- `"perl-lsp-feature-policy"`
- `"perl-lsp-feature-grid"`

Add new entry to the allow array:
- `"perl-lsp-rs-core"`

**Verify** (before step 5):

```bash
cargo metadata --no-deps | grep -c "perl-lsp-rs-core"
```

Expect: output is `2` (crate appears in members and dependencies sections).

---

### Step 5: Update consumer `Cargo.toml` files (3 crates)

#### 5a. `crates/perl-lsp/Cargo.toml` (LSP server facade)

**[dependencies]** section: Remove these 8 lines (find and delete):

- `perl-lsp-capability-map = { workspace = true }`
- `perl-lsp-feature-flags = { workspace = true }`
- `perl-lsp-feature-policy = { workspace = true }`
- `perl-lsp-feature-contracts = { workspace = true }`
- `perl-lsp-feature-grid = { workspace = true }`
- `perl-lsp-feature-profile = { workspace = true }`
- `perl-lsp-feature-profile-cli = { workspace = true }`

(Note: `perl-lsp-feature-ids` is NOT listed as a direct dependency — verify with `grep` before looking.)

Add this new line (in alphabetical order):

- `perl-lsp-rs-core = { workspace = true }`

**[features]** section: Find any lines containing `perl-lsp-feature-contracts/lsp-ga-lock` or similar feature gates, and rewrite them to `perl-lsp-rs-core/lsp-ga-lock`.

Example rewrite:
- `"lsp-ga-lock" = ["perl-lsp-feature-contracts/lsp-ga-lock"]` becomes `"lsp-ga-lock" = ["perl-lsp-rs-core/lsp-ga-lock"]`

**Verify** (before 5b):

```bash
cargo check -p perl-lsp-rs
```

Expect: no unresolved dependency errors.

#### 5b. `crates/perl-lsp-protocol/Cargo.toml` (protocol wrapper)

**[dependencies]** section: Remove these 2 lines:

- `perl-lsp-feature-flags = { workspace = true }`
- `perl-lsp-feature-contracts = { workspace = true }`

Add this new line:

- `perl-lsp-rs-core = { workspace = true }`

**[features]** section: Rewrite any `perl-lsp-feature-contracts/lsp-ga-lock` references to `perl-lsp-rs-core/lsp-ga-lock`.

**Verify** (before 5c):

```bash
cargo check -p perl-lsp-protocol
```

Expect: no unresolved dependency errors.

#### 5c. `crates/perl-lsp-feature-governance/Cargo.toml` (Wave G3, stays published)

**Critical note:** This crate is NOT absorbed in Wave F (it's Wave G3 scope). However, its dependencies must be updated because it depends on 5 of the 8 Wave F crates.

**[dependencies]** section: Remove these 5 lines:

- `perl-lsp-feature-contracts = { workspace = true }`
- `perl-lsp-feature-grid = { workspace = true }`
- `perl-lsp-feature-policy = { workspace = true }`
- `perl-lsp-feature-profile = { workspace = true }`
- `perl-lsp-feature-profile-cli = { workspace = true }`

Add this new line:

- `perl-lsp-rs-core = { workspace = true }`

**[features]** section: Rewrite feature forwarding to use `perl-lsp-rs-core`:

- Any `perl-lsp-feature-contracts/lsp-ga-lock` becomes `perl-lsp-rs-core/lsp-ga-lock`
- Any other feature references should follow the same pattern

**Verify** (before step 6):

```bash
cargo check -p perl-lsp-feature-governance
```

Expect: no unresolved dependency errors.

---

### Step 6: Update `use` statements in source files (2 crates)

#### 6a. `crates/perl-lsp-protocol/src/capabilities.rs`

**Two confirmed import sites** (find and rewrite these exact lines):

Old:
```rust
pub use perl_lsp_feature_flags::{AdvertisedFeatures, BuildFlags};
use perl_lsp_feature_contracts::feature_ids_from_caps;
```

New:
```rust
pub use perl_lsp_rs_core::features::flags::{AdvertisedFeatures, BuildFlags};
use perl_lsp_rs_core::features::contracts::feature_ids_from_caps;
```

**Verify** (before 6b):

```bash
cargo check -p perl-lsp-protocol
```

Expect: no unresolved imports.

#### 6b. `crates/perl-lsp-feature-governance/src/` (all files)

**Find and rewrite all imports** referencing the absorbed crates:

- `use perl_lsp_feature_*::` becomes `use perl_lsp_rs_core::features::<module>::`
- `use perl_lsp_capability_map::` becomes `use perl_lsp_rs_core::capability_map::`

**Audit tip:** Use this command to find all occurrences:

```bash
grep -rn "perl_lsp_feature_ids\|perl_lsp_feature_contracts\|perl_lsp_feature_flags\|perl_lsp_capability_map\|perl_lsp_feature_profile\|perl_lsp_feature_policy\|perl_lsp_feature_grid\|perl_lsp_feature_profile_cli" crates/perl-lsp-feature-governance/src/
```

Should return only the lines you just rewrote (zero after rewrite).

#### 6c. `crates/perl-lsp/src/` (LSP server binary)

**Search for any direct use sites** (expect zero):

```bash
grep -rn "perl_lsp_feature_ids\|perl_lsp_feature_contracts\|perl_lsp_feature_flags\|perl_lsp_capability_map\|perl_lsp_feature_profile\|perl_lsp_feature_policy\|perl_lsp_feature_grid\|perl_lsp_feature_profile_cli" crates/perl-lsp/src/
```

Expect: no matches (all references go through `perl-lsp-feature-governance`).

If any matches appear, rewrite them to `perl_lsp_rs_core::features::*` paths.

**Verify** (before step 7):

```bash
cargo check -p perl-lsp-rs
```

Expect: no unresolved imports.

---

### Step 7: Add facade re-exports to `perl-lsp-rs`

**File:** `crates/perl-lsp/src/lib.rs`

**Add these two lines at the end of the file** (or after existing Wave D re-exports):

```rust
// Wave F re-exports
pub use perl_lsp_rs_core::capability_map;
pub use perl_lsp_rs_core::features;
```

This makes the absorbed code available as a public API from the `perl-lsp-rs` facade.

**Verify** (before step 8):

```bash
cargo check -p perl-lsp-rs
cargo doc -p perl-lsp-rs --no-deps
```

Expect: no errors, documentation builds cleanly.

---

### Step 8: Delete the 8 absorbed crate directories

**Execute these removals** (one per line, or use a single `rm -rf` command):

```bash
rm -rf crates/perl-lsp-feature-ids/
rm -rf crates/perl-lsp-feature-contracts/
rm -rf crates/perl-lsp-feature-flags/
rm -rf crates/perl-lsp-feature-profile/
rm -rf crates/perl-lsp-feature-profile-cli/
rm -rf crates/perl-lsp-feature-policy/
rm -rf crates/perl-lsp-feature-grid/
rm -rf crates/perl-lsp-capability-map/
```

**Verify** (before step 9):

```bash
ls -la crates/ | grep "perl-lsp-feature\|perl-lsp-capability-map"
```

Expect: no output (directories are gone).

---

### Step 9: Update project metadata

#### 9a. `xtask/published-crate-baseline.txt`

**Change the single line** from:

```
81
```

to:

```
74
```

#### 9b. `.spec/microcrate-collapse/ledger.md`

**Find the Wave F section** (lines ~174–185, search for "Wave F").

For each of the 8 crates, mark the status as complete by changing their rows from `Pending` or `To spec` to `✓ Completed (Wave F)`.

**Example row update:**

Old:
```
| perl-lsp-feature-ids | ...scope... | perl-lsp-rs-core | module | To spec |
```

New:
```
| perl-lsp-feature-ids | ...scope... | perl-lsp-rs-core | module | ✓ Completed (Wave F) |
```

**Verify** (before final tests):

```bash
grep "Completed (Wave F)" .spec/microcrate-collapse/ledger.md | wc -l
```

Expect: output is `8`.

---

## Verification Commands

Run these **in order** after step 9 completes:

### V1. Workspace builds clean

```bash
cargo check --workspace
```

**Expect:** No errors, all dependencies resolve.

---

### V2. New core crate tests pass

```bash
cargo test -p perl-lsp-rs-core
```

**Expect:** All 11 test files pass (feature_ids_comprehensive + 10 others).

---

### V3. LSP server tests pass

```bash
cargo test -p perl-lsp-rs
```

**Expect:** All LSP server tests pass.

---

### V4. Protocol crate tests pass

```bash
cargo test -p perl-lsp-protocol
```

**Expect:** All protocol tests pass.

---

### V5. Governance crate tests pass

```bash
cargo test -p perl-lsp-feature-governance
```

**Expect:** All governance tests pass (dependencies re-wired successfully).

---

### V6. Layer-check gate passes

```bash
cargo xtask layer-check
```

**Expect:** No layering violations, `perl-lsp-rs-core` sits below `perl-lsp-rs` and `perl-lsp-protocol`.

---

### V7. Published baseline correct

```bash
grep -x "74" xtask/published-crate-baseline.txt
```

**Expect:** Command outputs `74` (the file matches).

---

### V8. Formatting clean

```bash
cargo xtask fmt
```

**Expect:** No changes needed (or auto-formats and passes).

---

### V9. Clippy clean on new crate

```bash
cargo clippy -p perl-lsp-rs-core -p perl-lsp-protocol -p perl-lsp-rs --all-targets
```

**Expect:** No warnings or errors.

---

## Build Order Dependencies

1. **Step 1** must complete before step 2 (crate structure required).
2. **Step 2** must complete before step 3 (source code needed for tests).
3. **Step 3** must complete before steps 4–5 (imports may reference moved code).
4. **Steps 4–5** must complete before step 6 (dependencies locked down).
5. **Step 6** must complete before step 7 (source files must compile).
6. **Step 7** must complete before step 8 (facade re-exports added before deleting old crates).
7. **Step 8** must complete before step 9 (old directories gone before updating metadata).
8. **Step 9** must complete before verification (baseline and ledger updated).

**Total compilation points:** Each step has a `cargo check` or `cargo test` gate.

---

## Notes

- **Binary target:** `perl-lsp-feature-profile-cli` has NO `[[bin]]` section on master (library-only). Do NOT create one in Wave F. The spec line 182 in ledger.md is outdated — drop it.
- **Three-consumer pattern:** `perl-lsp`, `perl-lsp-protocol`, and `perl-lsp-feature-governance` all get 8 deps replaced with ONE: `perl-lsp-rs-core`.
- **perl-lsp-feature-ids non-dependency:** `perl-lsp-feature-ids` is never directly listed in `perl-lsp/Cargo.toml` — it was transitive. After consolidation, it's part of `-core` and pulls in transitively.
- **Feature forwarding:** All 5 crates that use `lsp-ga-lock` must forward it through `perl-lsp-rs-core/lsp-ga-lock`.

---

## Commits

Commit all changes as a single PR-ready commit (red-tdd will add tests; builder will fill in implementation).

**Commit message style:**

```
refactor(lsp-features): collapse perl-lsp-feature-* (8 crates) -> perl-lsp-rs-core::features (Wave F) (#4489)
```

---

## PR Title Format (for later)

```
refactor(lsp-features): collapse perl-lsp-feature-* (8 crates) → perl-lsp-rs-core::features (Wave F) (#4489)
```

(Note: Builder will add this when creating the PR; spec-planner just creates the branch.)
