# Implementation Checklist: #4541 — Wave Final PR B (LSP deferrals absorption)

**Scope:** Absorb 3 crates into `perl-lsp-rs-core`: `perl-feature-catalog`, `perl-lsp-config`, `perl-content-length-framing`. Reduce published count from 34 → 31. Delete G3 negative tests.

**Current HEAD:** `60d0711e0` (post-Wave 4-Completion merge)  
**Baseline:** 34 published crates (perl-feature-catalog, perl-lsp-config, perl-content-length-framing still standalone)

---

## Change order (compiles at each step)

### Step 1: Copy 3 platform functions from perl-dap into perl-lsp-rs-core
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-dap/src/platform/mod.rs` (source, lines 205–256)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/platform.rs` (NEW)
- **Change:** Extract `resolve_perl_path_with_toolchain()`, `detect_perlbrew_perl()`, `detect_plenv_perl()` from perl-dap::platform. Copy into new `perl-lsp-rs-core/src/platform.rs`. Keep comments and docstrings intact.
- **Details:**
  - Functions are ~60 LOC total, use only std library, no crate-specific deps
  - Exact signatures to copy:
    - `pub fn resolve_perl_path_with_toolchain() -> Result<PathBuf>`
    - `pub fn detect_perlbrew_perl() -> Option<PathBuf>`
    - `pub fn detect_plenv_perl() -> Option<PathBuf>`
  - Include all helper logic and imports they depend on (std::path, std::env, std::process::Command)
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 2: Expose platform module from perl-lsp-rs-core lib root
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/lib.rs`
- **Change:** Add `pub mod platform;` after existing module declarations
- **Details:** Add line roughly after the runtime/protocol/governance/etc declarations
- **Depends on:** Step 1
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 3: Repoint perl-lsp-config away from perl-dap
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-config/src/lib.rs` (line 8)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-config/Cargo.toml` (build-dependencies section)
- **Change:** 
  - In `lib.rs` line 8: change `use perl_dap::platform::resolve_perl_path_with_toolchain;` to `use perl_lsp_rs_core::platform::resolve_perl_path_with_toolchain;`
  - In `Cargo.toml`: replace `perl-dap = { workspace = true }` with `perl-lsp-rs-core = { workspace = true }`
- **Details:** This breaks the cycle: perl-lsp-config → perl-dap is replaced with perl-lsp-config → perl-lsp-rs-core
- **Depends on:** Step 2
- **Verify:** `cargo check -p perl-lsp-config`

### Step 4: Move perl-lsp-config content into perl-lsp-rs-core
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-config/src/lib.rs` (source)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/config.rs` (NEW)
- **Change:** Copy entire content of `perl-lsp-config/src/lib.rs` into new `perl-lsp-rs-core/src/config.rs`. Update the import of `resolve_perl_path_with_toolchain` to use `crate::platform::resolve_perl_path_with_toolchain` instead of external crate.
- **Details:**
  - Preserve all doc comments, re-exports, and public API surface
  - The only import change is line 8 from `use perl_dap::...` → `use crate::platform::...`
  - Keep the module structure (e.g., `pub mod native_build_hints` etc.)
- **Depends on:** Step 3
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 5: Expose config module from perl-lsp-rs-core lib root
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/lib.rs`
- **Change:** Add `pub mod config;` alongside the platform module from Step 2
- **Details:** Single line addition
- **Depends on:** Step 4
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 6: Repoint perl-lsp consumers to use perl-lsp-rs-core::config
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp/Cargo.toml` (line 42)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/language/misc.rs` (line 727)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs` (lines 96, 196, 233, 338, 941, 943)
- **Change:**
  - In `Cargo.toml`: Remove `perl-lsp-config = { workspace = true }` from dependencies
  - In both .rs files: Replace all `perl_lsp_config::` with `perl_lsp_rs_core::config::`
- **Details:**
  - Total: 1 line in Cargo.toml + 7 lines in misc.rs + 6 lines in module_resolution.rs that reference the old import
  - Global find-and-replace safe: grep confirms only these files import `perl_lsp_config`
- **Depends on:** Step 5
- **Verify:** `cargo check -p perl-lsp`

### Step 7: Copy feature-catalog content into perl-lsp-rs-core
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-feature-catalog/src/lib.rs` (source)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/feature_catalog.rs` (NEW)
- **Change:** Copy entire content of `perl-feature-catalog/src/lib.rs` into new `perl-lsp-rs-core/src/feature_catalog.rs`
- **Details:**
  - Preserve all doc comments, re-exports, pub use statements
  - No import changes needed (feature-catalog has no external crate deps except std)
  - This is a build-time-only utility (used in build.rs), so no public API compatibility concern
- **Depends on:** Step 5 (config must compile first to avoid dep ordering issues)
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 8: Expose feature_catalog module from perl-lsp-rs-core lib root
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/lib.rs`
- **Change:** Add `pub mod feature_catalog;`
- **Details:** Single line addition
- **Depends on:** Step 7
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 9: Update perl-lsp-rs-core build.rs to use local feature_catalog module
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/build.rs`
- **Change:** Change imports from `perl_feature_catalog::` to `crate::feature_catalog::`
- **Details:**
  - Grep shows build.rs likely has lines like `use perl_feature_catalog::...`
  - Replace with local module import
- **Depends on:** Step 8
- **Verify:** `cargo build -p perl-lsp-rs-core`

### Step 10: Remove perl-feature-catalog build-dep from perl-lsp-rs-core
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/Cargo.toml` (line 70)
- **Change:** Remove line `perl-feature-catalog.workspace = true` from `[build-dependencies]`
- **Details:** Verify line 70 contains this, or search for the exact line
- **Depends on:** Step 9
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 11: Update perl-dap to get feature-catalog from perl-lsp-rs-core
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-dap/Cargo.toml` (line 80, build-dependencies)
  - `/h/Code/Rust/perl-lsp/crates/perl-dap/build.rs` (codegen imports)
- **Change:**
  - **Option A (preferred per plan-review):** Replace `perl-feature-catalog = { workspace = true }` in [build-dependencies] with `perl-lsp-rs-core = { workspace = true }`. Update `build.rs` imports from `perl_feature_catalog::` to `perl_lsp_rs_core::feature_catalog::`
  - **Option B:** Inline ~100 LOC of feature-catalog codegen directly into perl-dap/build.rs. NOT preferred per plan-review.
- **Details:** Plan-review recommends Option A. perl-dap already depends on perl-lsp-rs-core at runtime (line 62), so adding it as build-dep is safe.
- **Depends on:** Step 10
- **Verify:** `cargo build -p perl-dap`

### Step 12: Move perl-content-length-framing content into perl-lsp-rs-core
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-content-length-framing/src/lib.rs` (source)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/transport/framing.rs` (already exists, replace re-export shim)
- **Change:** Replace the current re-export shim at lines 4–5 of framing.rs with full content from perl-content-length-framing/src/lib.rs
- **Details:**
  - Current framing.rs likely has stub like `pub use perl_content_length_framing::*;`
  - Replace with actual module content (frame struct, ContentLengthFramer, all logic)
  - No import changes needed (framing uses only std)
- **Depends on:** Step 11
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 13: Update perl-dap to use perl-lsp-rs-core::transport::framing
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-dap/src/debug_adapter/mod.rs` (line 42)
  - `/h/Code/Rust/perl-lsp/crates/perl-dap/src/tcp_attach.rs` (line 21)
  - `/h/Code/Rust/perl-lsp/crates/perl-dap/tests/dap_attach_e2e.rs` (line 6)
  - `/h/Code/Rust/perl-lsp/crates/perl-dap/tests/tcp_attach_tests.rs` (line 12)
- **Change:** Replace all `use perl_content_length_framing::` with `use perl_lsp_rs_core::transport::framing::`
- **Details:**
  - Imports like `use perl_content_length_framing::{ContentLengthFramer, frame};` become `use perl_lsp_rs_core::transport::framing::{ContentLengthFramer, frame};`
  - 4 files total
- **Depends on:** Step 12
- **Verify:** `cargo check -p perl-dap`

### Step 14: Update perl-lsp tests to use perl-lsp-rs-core::transport::framing
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp/tests/support/lsp_harness.rs` (line 15)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp/tests/support/message_framing.rs` (line 10)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp/tests/lsp_content_length_framing_integration.rs` (line 4)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp/tests/lsp_streaming_completion_tests.rs` (line 497)
- **Change:** Replace all `use perl_content_length_framing::` with `use perl_lsp_rs_core::transport::framing::`
- **Details:**
  - 4 files total
- **Depends on:** Step 12
- **Verify:** `cargo check -p perl-lsp`

### Step 15: Remove perl-content-length-framing from perl-lsp dependencies
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp/Cargo.toml` (line 127)
- **Change:** Remove `perl-content-length-framing = { workspace = true }`
- **Details:** Single line removal
- **Depends on:** Step 14
- **Verify:** `cargo check -p perl-lsp`

### Step 16: Remove perl-content-length-framing from perl-dap dependencies
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-dap/Cargo.toml` (line 60)
- **Change:** Remove `perl-content-length-framing = { workspace = true }`
- **Details:** Single line removal
- **Depends on:** Step 13
- **Verify:** `cargo check -p perl-dap`

### Step 17: Remove perl-content-length-framing from perl-lsp-rs-core workspace deps
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/Cargo.toml`
- **Change:** Remove `perl-content-length-framing.workspace = true` from [workspace-members] or [dependencies] section if it exists
- **Details:** Verify this line exists (not all crates track all workspace members in Cargo.toml, but confirm)
- **Depends on:** Step 16
- **Verify:** `cargo check -p perl-lsp-rs-core`

### Step 18: Mark old crates as publish = false
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-feature-catalog/Cargo.toml`
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-config/Cargo.toml`
  - `/h/Code/Rust/perl-lsp/crates/perl-content-length-framing/Cargo.toml`
- **Change:** Add `publish = false` to `[package]` section in each Cargo.toml (if not already present)
- **Details:** This prevents accidental re-publishing of these crates
- **Depends on:** Steps 6, 11, 16
- **Verify:** `cargo check --all`

### Step 19: Remove absorbed crates from root Cargo.toml allowlist
- **File:** `/h/Code/Rust/perl-lsp/Cargo.toml` (line 177, `[workspace.metadata.publish.allow]`)
- **Change:** Remove three lines:
  - `"perl-feature-catalog"`
  - `"perl-lsp-config"`
  - `"perl-content-length-framing"`
- **Details:** Root Cargo.toml has explicit allowlist of crates to publish. Delete these 3 entries.
- **Depends on:** Step 18
- **Verify:** `cargo xtask published-crate-count` (should output `OK (31 crates, baseline 34)` before baseline update)

### Step 20: Update published-crate-baseline.txt
- **File:** `/h/Code/Rust/perl-lsp/xtask/published-crate-baseline.txt`
- **Change:** Change `34` to `31`
- **Details:** Single number in file
- **Depends on:** Step 19
- **Verify:** `cargo xtask published-crate-count` (should pass: `OK (31 crates, baseline 31)`)

### Step 21: Update G3 assertion tests
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/tests/g3_published_count.rs`
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/tests/g3_publish_baseline_enforcement.rs`
- **Change:** Update test assertions from `== 34` to `== 31` (or whatever the current lines assert)
- **Details:**
  - Grep shows these files have test names like `g3_published_count_is_37` or `g3_baseline_file_has_37` — these were previously fixed to 34 after Wave 4-Completion, now need to be 31
  - Both files likely have 2 assertions each
- **Depends on:** Step 20
- **Verify:** `cargo test -p perl-lsp-rs-core --lib`

### Step 22: Delete G3 negative tests that are now superseded
- **Files:**
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/tests/g3_config_stays_standalone.rs` (DELETE)
  - `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/tests/g3_content_length_framing_stays.rs` (DELETE)
- **Change:** Remove both test files entirely
- **Details:** These tests explicitly asserted that config and framing remained standalone, published crates — they are now absorbed and must be deleted to prevent test failures
- **Depends on:** Step 21
- **Verify:** `cargo test -p perl-lsp-rs-core --lib`

### Step 23: Add new Wave Final absorption test suite
- **File:** `/h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/tests/wave_final_absorption_tests.rs` (NEW)
- **Change:** Create new test file with the following test functions (stubs provided by plan-review):
  ```rust
  #[test] fn config_accessible_via_rs_core()
  #[test] fn framing_accessible_via_rs_core_transport()
  #[test] fn feature_catalog_accessible_via_rs_core()
  #[test] fn platform_resolver_accessible_via_rs_core()
  #[test] fn wave_final_crates_have_publish_false()
  #[test] fn published_count_is_31_after_wave_final()
  ```
  Each test should verify the module is accessible and read Cargo.toml files to confirm publish = false
- **Details:** Tests are defined by red-TDD builder, not spec-planner. Spec-planner just notes the file should exist.
- **Depends on:** Step 22
- **Verify:** `cargo test -p perl-lsp-rs-core`

### Step 24: Final compilation and format check
- **Verify:**
  ```bash
  cargo check --all
  cargo test -p perl-lsp-rs-core --lib
  cargo test -p perl-dap --lib
  cargo test -p perl-lsp --lib
  cargo xtask fmt
  cargo clippy -p perl-lsp-rs-core -p perl-dap -p perl-lsp -- -D warnings
  cargo xtask layer-check  # Verify no new cycles introduced
  ```

---

## Callers and consumers

**perl-lsp-config::***
- Called from: `crates/perl-lsp/src/runtime/language/misc.rs` (1 call), `crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs` (6 calls), `crates/perl-lsp/Cargo.toml` (1 dep)

**perl-content-length-framing::{ContentLengthFramer, frame}**
- Called from:
  - `crates/perl-dap/src/debug_adapter/mod.rs` (1 import)
  - `crates/perl-dap/src/tcp_attach.rs` (1 import)
  - `crates/perl-dap/tests/dap_attach_e2e.rs` (1 import)
  - `crates/perl-dap/tests/tcp_attach_tests.rs` (1 import)
  - `crates/perl-lsp/tests/support/lsp_harness.rs` (1 import)
  - `crates/perl-lsp/tests/support/message_framing.rs` (1 import)
  - `crates/perl-lsp/tests/lsp_content_length_framing_integration.rs` (1 import)
  - `crates/perl-lsp/tests/lsp_streaming_completion_tests.rs` (1 import)

**perl_feature_catalog::{build-dep}**
- Called from:
  - `crates/perl-dap/build.rs` (codegen)
  - `crates/perl-lsp-rs-core/build.rs` (codegen)

**perl-dap::platform::{resolve_perl_path_with_toolchain, detect_perlbrew_perl, detect_plenv_perl}**
- Called from: `crates/perl-lsp-config/src/lib.rs` (line 8, `resolve_perl_path_with_toolchain` only)

---

## Scope boundary

**Files IN scope for this PR:**
- `crates/perl-lsp-rs-core/src/lib.rs`
- `crates/perl-lsp-rs-core/src/platform.rs` (NEW)
- `crates/perl-lsp-rs-core/src/config.rs` (NEW)
- `crates/perl-lsp-rs-core/src/feature_catalog.rs` (NEW)
- `crates/perl-lsp-rs-core/src/transport/framing.rs` (modify re-export shim)
- `crates/perl-lsp-rs-core/build.rs`
- `crates/perl-lsp-rs-core/Cargo.toml`
- `crates/perl-lsp-rs-core/tests/g3_published_count.rs`
- `crates/perl-lsp-rs-core/tests/g3_publish_baseline_enforcement.rs`
- `crates/perl-lsp-rs-core/tests/g3_config_stays_standalone.rs` (DELETE)
- `crates/perl-lsp-rs-core/tests/g3_content_length_framing_stays.rs` (DELETE)
- `crates/perl-lsp-rs-core/tests/wave_final_absorption_tests.rs` (NEW, red-TDD builder writes this)
- `crates/perl-dap/Cargo.toml`
- `crates/perl-dap/build.rs`
- `crates/perl-dap/src/debug_adapter/mod.rs`
- `crates/perl-dap/src/tcp_attach.rs`
- `crates/perl-dap/tests/dap_attach_e2e.rs`
- `crates/perl-dap/tests/tcp_attach_tests.rs`
- `crates/perl-lsp/Cargo.toml`
- `crates/perl-lsp/src/runtime/language/misc.rs`
- `crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs`
- `crates/perl-lsp/tests/support/lsp_harness.rs`
- `crates/perl-lsp/tests/support/message_framing.rs`
- `crates/perl-lsp/tests/lsp_content_length_framing_integration.rs`
- `crates/perl-lsp/tests/lsp_streaming_completion_tests.rs`
- `crates/perl-feature-catalog/Cargo.toml` (mark `publish = false`)
- `crates/perl-lsp-config/src/lib.rs` (modify import, then not deleted but becomes internal)
- `crates/perl-lsp-config/Cargo.toml` (modify deps, mark `publish = false`)
- `crates/perl-content-length-framing/Cargo.toml` (mark `publish = false`)
- Root `Cargo.toml` (line 177: remove 3 entries from allowlist)
- `xtask/published-crate-baseline.txt` (34 → 31)
- `docs/adr/0041-microcrate-collapse.md` (add Amendment 9, separate PR likely)

**Files OUT of scope:**
- Parser satellites (`perl-dead-code`, `perl-refactoring`, `perl-incremental-parsing`) — already absorbed in PR A
- All other crates
- MIGRATION_v0.13.md (documented as follow-up)

---

## Flags for builder

1. **Cycle-break order is critical:** Steps 1–3 must be done in order because Step 3 repoints perl-lsp-config away from perl-dap. Doing this before copying the functions will fail.

2. **Build-dep handling for perl-dap:** Step 11 offers two options. Plan-review **strongly recommends Option A** (add perl-lsp-rs-core as build-dep to perl-dap). If builder chooses Option B, they must inline ~100 LOC of codegen logic into perl-dap/build.rs, which increases scope and maintenance burden.

3. **G3 test updates (Step 21):** The three G3 assertion tests (`g3_published_count.rs`, `g3_publish_baseline_enforcement.rs`) currently assert `== 34`. After Step 20 updates the baseline to 31, these tests MUST be updated to assert `== 31`, or they will fail. This is load-bearing for Step 21.

4. **Old crate directories:** After this PR merges, the three crate directories (`perl-feature-catalog/`, `perl-lsp-config/`, `perl-content-length-framing/`) remain in the repo with `publish = false` in their Cargo.toml. They are NOT deleted. This is intentional per Wave Final design (crates are retired, not deleted from history). If future consolidation decides to delete them entirely, that is a separate archival decision.

5. **Amendment 9 documentation:** Plan-review recommends adding Amendment 9 to `docs/adr/0041-microcrate-collapse.md` documenting this PR. This can be done as a separate commit in the same PR or in a follow-up PR. The spec-planner has outlined the content; the builder should consider whether to include it.

6. **No new modules in perl-lsp-rs-core public API:** The three absorbed modules (platform, config, feature_catalog) are added to rs-core as `pub mod`, but they are **only** consumed by build.rs and internal runtime code. Avoid re-exporting them from the main lib surface unless external users have a real need.

7. **Verify layer-check after every 5 steps:** Run `cargo xtask layer-check` after Steps 10 and 16 to catch any new cycles early.
