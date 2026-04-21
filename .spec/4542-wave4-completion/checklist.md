# Wave 4-Completion — Implementation Checklist

Implementation order: 3 sequential absorptions, then baseline update, then tests. Each absorption is a push-ready commit.

## Step 1: Absorb perl-dead-code → perl-parser::dead_code

### 1a. Create module directory and copy content
- **File**: CREATE `crates/perl-parser/src/dead_code/mod.rs`
- **Action**: Copy entire content from `crates/perl-dead-code/src/lib.rs` (all code, doc comments, tests)
- **Keep**: All module doc comments and test code from original
- **Verify**: `cargo check -p perl-parser` compiles (dead_code module inline)

### 1b. Update perl-parser/src/lib.rs lib-level exports
- **File**: `crates/perl-parser/src/lib.rs`
- **Line**: 459 (currently: `pub use perl_dead_code as dead_code_detector;`)
- **Change**: Replace with:
  ```rust
  pub mod dead_code;
  pub use dead_code as dead_code_detector;
  ```
- **Verify**: `cargo check -p perl-parser` compiles, re-export alias works

### 1c. Remove perl-dead-code dependency from perl-parser
- **File**: `crates/perl-parser/Cargo.toml`
- **Line**: 30 (currently: `perl-dead-code = { workspace = true }`)
- **Action**: Delete line 30
- **Verify**: `cargo check -p perl-parser` compiles (no missing dependency)

### 1d. Mark perl-dead-code crate as unpublished
- **File**: `crates/perl-dead-code/Cargo.toml`
- **Action**: Add field `publish = false` (in [package] section, near version/authors)
- **Verify**: Manual inspection confirms field present

### 1e. Remove perl-dead-code from publish allowlist
- **File**: `Cargo.toml` (root)
- **Line**: 169 (currently: `"perl-dead-code"` in `[workspace.metadata.publish.allow]`)
- **Action**: Delete line 169 (just the entry, keep bracket structure intact)
- **Verify**: Remaining allowlist has no `perl-dead-code` entry

### 1f. Commit and push
- **Commit message**: `refactor(parser): absorb perl-dead-code → perl-parser::dead_code (#4542)` — D1 locked decision, replaces re-export alias with real module, marks crate unpublished
- **Push**: `git push -u origin impl/4542-wave4-completion`
- **Verify** (all together):
  ```bash
  cargo check --workspace
  cargo test -p perl-parser --lib
  cargo clippy -p perl-parser -- -D warnings
  ```

---

## Step 2: Absorb perl-refactoring → perl-parser::refactor

### 2a. Create module directory and copy content
- **File**: CREATE `crates/perl-parser/src/refactor/` directory
- **Action**: Copy entire `crates/perl-refactoring/src/refactor/` directory (mod.rs and all submodules)
- **Keep**: All module doc comments, tests, re-exports from submodules
- **Verify**: `cargo check -p perl-parser` compiles (refactor module inline with all submodules)

### 2b. Update perl-parser/src/lib.rs lib-level exports
- **File**: `crates/perl-parser/src/lib.rs`
- **Lines**: 462-468 (all existing `pub use refactor::*` statements)
- **Action**: Change current `refactor.rs` re-export pattern from a separate file to inline module.
  Replace file `crates/perl-parser/src/refactor.rs` with inline `pub mod refactor;` in lib.rs OR keep refactor.rs but change to:
  ```rust
  //! Refactoring module (absorbed from perl-refactoring)
  pub mod import_optimizer;
  pub mod modernize;
  // ... etc, pub use statements from submodules
  ```
  **Simpler approach**: Delete refactor.rs file, add `pub mod refactor;` to lib.rs, keep refactor/ submodule directory as-is.
- **Verify**: `cargo check -p perl-parser` compiles, all refactor re-exports still work

### 2c. Remove perl-refactoring feature forwarding from perl-parser
- **File**: `crates/perl-parser/Cargo.toml`
- **Lines**: 83-84 (currently in [features] section, perl-refactoring forwarding)
- **Action**: Find and delete perl-refactoring feature forwarding:
  ```toml
  workspace_refactor = ["perl-refactoring/workspace_refactor"]
  modernize = ["perl-refactoring/modernize"]
  ```
  **Replace with**: Re-enable these features as always-on or optional within perl-parser (absorbed code), **OR** keep them feature-gated if submodules use the same pattern. **Decision**: Keep as feature-gated, features point to no external crate now.
- **Verify**: Feature definitions compile correctly

### 2d. Remove perl-refactoring dependency
- **File**: `crates/perl-parser/Cargo.toml`
- **Line**: 32 (currently: `perl-refactoring = { workspace = true }`)
- **Action**: Delete the line
- **Verify**: `cargo check -p perl-parser` compiles

### 2e. Mark perl-refactoring crate as unpublished
- **File**: `crates/perl-refactoring/Cargo.toml`
- **Action**: Add field `publish = false`
- **Verify**: Manual inspection

### 2f. Remove perl-refactoring from publish allowlist
- **File**: `Cargo.toml` (root)
- **Line**: 166 (currently: `"perl-refactoring"` in `[workspace.metadata.publish.allow]`)
- **Action**: Delete the line
- **Verify**: No `perl-refactoring` entry remains

### 2g. Commit and push
- **Commit message**: `refactor(parser): absorb perl-refactoring → perl-parser::refactor (#4542)` — D2 locked decision, replaces re-export shim with real module, marks crate unpublished
- **Push**: `git push`
- **Verify**:
  ```bash
  cargo check --workspace
  cargo test -p perl-parser --lib
  cargo test -p perl-refactoring --lib
  cargo clippy -p perl-parser -p perl-refactoring -- -D warnings
  ```

---

## Step 3: Absorb perl-incremental-parsing → perl-parser::incremental

### 3a. Create module directory and copy content
- **File**: CREATE `crates/perl-parser/src/incremental/` directory
- **Action**: Copy entire `crates/perl-incremental-parsing/src/incremental/` directory (mod.rs and all submodules)
- **Keep**: All module doc comments, tests, feature gating from original
- **Verify**: `cargo check -p perl-parser --features incremental` compiles (incremental module inline)

### 3b. Update perl-parser/src/lib.rs lib-level exports
- **File**: `crates/perl-parser/src/lib.rs`
- **Lines**: 478-502 (all `#[cfg(feature = "incremental")]` pub use incremental::* statements)
- **Action**: Similar to refactor: change `crates/perl-parser/src/incremental.rs` from re-export shim to inline module via `pub mod incremental;`
  Keep all #[cfg(feature = "incremental")] gating on the re-exports.
- **Verify**: `cargo check -p perl-parser --features incremental` compiles

### 3c. Update perl-lsp imports in text_sync.rs (critical consumer in different crate)
- **File**: `crates/perl-lsp/src/runtime/text_sync.rs`
- **Lines**: 213, 233, 472, 474, 674, 719, 729 (all 6+ `perl_incremental_parsing::incremental::*` imports)
- **Action**: Change each import from:
  - `perl_incremental_parsing::incremental::X` → `perl_parser::incremental::X`
  - Example line 213: `use perl_incremental_parsing::incremental::incremental_document::IncrementalDocument;` → `use perl_parser::incremental::incremental_document::IncrementalDocument;`
- **Verify**: `cargo check -p perl-lsp --features incremental` compiles, all imports resolve

### 3d. Remove perl-incremental-parsing from perl-lsp dependencies
- **File**: `crates/perl-lsp/Cargo.toml`
- **Line**: 34 (currently: `perl-incremental-parsing = { workspace = true, optional = true }`)
- **Action**: Delete the line
- **Verify**: Manual inspection

### 3e. Update perl-lsp incremental feature
- **File**: `crates/perl-lsp/Cargo.toml`
- **Line**: 95 (currently in [features]: `incremental = ["perl-parser/incremental", "perl-incremental-parsing"]`)
- **Action**: Change to:
  ```toml
  incremental = ["perl-parser/incremental"]
  ```
  (remove `"perl-incremental-parsing"` from the array)
- **Verify**: Feature definition syntax is valid

### 3f. Remove perl-incremental-parsing dependency from perl-parser
- **File**: `crates/perl-parser/Cargo.toml`
- **Line**: 33 (currently: `perl-incremental-parsing = { workspace = true, optional = true }`)
- **Action**: Delete the line
- **Verify**: Manual inspection

### 3g. Remove perl-incremental-parsing feature dep from perl-parser features
- **File**: `crates/perl-parser/Cargo.toml`
- **Line**: 64 (currently in [features]: `incremental = ["anyhow", "perl-incremental-parsing"]`)
- **Action**: Change to:
  ```toml
  incremental = ["anyhow"]
  ```
  (remove `"perl-incremental-parsing"` from the array; keep "anyhow")
- **Verify**: Feature definition syntax is valid

### 3h. Mark perl-incremental-parsing crate as unpublished
- **File**: `crates/perl-incremental-parsing/Cargo.toml`
- **Action**: Add field `publish = false`
- **Verify**: Manual inspection

### 3i. Remove perl-incremental-parsing from publish allowlist
- **File**: `Cargo.toml` (root)
- **Line**: 123 (currently: `"perl-incremental-parsing"` in `[workspace.metadata.publish.allow]`)
- **Action**: Delete the line
- **Verify**: No `perl-incremental-parsing` entry remains

### 3j. Commit and push
- **Commit message**: `refactor(parser,lsp): absorb perl-incremental-parsing → perl-parser::incremental (#4542)` — D3 locked decision, replaces re-export shim with real module, rewires perl-lsp text_sync imports, marks crate unpublished
- **Push**: `git push`
- **Verify**:
  ```bash
  cargo check --workspace --features incremental
  cargo test -p perl-parser --lib --features incremental
  cargo test -p perl-lsp --lib --features incremental
  cargo clippy -p perl-parser -p perl-lsp -- -D warnings
  ```

---

## Step 4: Update published count baseline and G3 assertions

### 4a. Update published-crate-baseline.txt
- **File**: `xtask/published-crate-baseline.txt`
- **Current**: `37` (literal count)
- **Action**: Change to `34`
- **Verify**: Manual inspection

### 4b. Update G3 published count assertion test
- **File**: `crates/perl-lsp-rs-core/tests/g3_published_count.rs`
- **Search**: Find line asserting `== 37` in test `g3_published_count_is_37` (or similar name)
- **Action**: Change assertion from `37` to `34`
- **Verify**: Test name and assertion are updated consistently

### 4c. Update G3 baseline enforcement test
- **File**: `crates/perl-lsp-rs-core/tests/g3_publish_baseline_enforcement.rs`
- **Search**: Find lines asserting `== 37` or `>= 37` (baseline file checks and baseline not regressed checks)
- **Action**: Update to `34` (allow exact match or ratchet-forward-only pattern)
- **Verify**: Both baseline file check and regression check are updated

### 4d. Commit
- **Commit message**: `fix(baseline): correct published-crate-baseline 37 → 34 after Wave 4-Completion (#4542)`
- **Push**: `git push`
- **Verify**:
  ```bash
  cargo xtask published-crate-count    # Should return: OK (34 crates, baseline 34)
  cargo test -p perl-lsp-rs-core --test g3_published_count --lib
  cargo test -p perl-lsp-rs-core --test g3_publish_baseline_enforcement --lib
  ```

---

## Step 5: Add absorption validation tests

### 5a. Create test file
- **File**: CREATE `crates/perl-parser/tests/wave4_completion_absorption_tests.rs`
- **Template**:
  ```rust
  //! Wave 4-Completion absorption validation tests
  //! Confirm that perl-dead-code, perl-refactoring, perl-incremental-parsing
  //! are accessible via perl-parser::* modules.

  use std::fs;
  use std::path::Path;

  #[test]
  fn dead_code_accessible_via_parser() {
      // Confirm perl_parser::dead_code module is publicly accessible
      use perl_parser::dead_code;
      // If this compiles, the module is accessible; if it doesn't, absorption failed.
      let _ = dead_code;  // silence unused warning
  }

  #[test]
  fn refactoring_accessible_via_parser() {
      // Confirm perl_parser::refactor module is publicly accessible
      use perl_parser::refactor;
      let _ = refactor;
  }

  #[test]
  #[cfg(feature = "incremental")]
  fn incremental_accessible_via_parser() {
      // Confirm perl_parser::incremental module is accessible when feature is enabled
      use perl_parser::incremental;
      let _ = incremental;
  }

  #[test]
  fn wave4_crates_have_publish_false() {
      // Verify that absorbed crates are marked publish = false
      let crates = vec![
          "crates/perl-dead-code/Cargo.toml",
          "crates/perl-refactoring/Cargo.toml",
          "crates/perl-incremental-parsing/Cargo.toml",
      ];

      for crate_path in crates {
          let content = fs::read_to_string(crate_path)
              .expect(&format!("Failed to read {}", crate_path));
          assert!(
              content.contains("publish = false"),
              "Crate {} should have 'publish = false', but doesn't",
              crate_path
          );
      }
  }

  #[test]
  fn wave4_crates_not_in_publish_allowlist() {
      // Verify that absorbed crates are removed from root Cargo.toml allowlist
      let root_toml = fs::read_to_string("Cargo.toml")
          .expect("Failed to read root Cargo.toml");

      // Check that allowlist section does NOT contain these crate names
      let disallowed_names = vec![
          "perl-dead-code",
          "perl-refactoring",
          "perl-incremental-parsing",
      ];

      // Extract the [workspace.metadata.publish.allow] section
      let allow_start = root_toml.find("[workspace.metadata.publish.allow]")
          .expect("Allowlist section not found");
      let allow_end = root_toml[allow_start..].find("]")
          .expect("Allowlist closing bracket not found");
      let allow_section = &root_toml[allow_start..allow_start + allow_end + 1];

      for name in disallowed_names {
          assert!(
              !allow_section.contains(&format!("\"{}\"", name)),
              "Crate {} should NOT be in publish allowlist",
              name
          );
      }
  }

  #[test]
  fn published_count_is_34_after_wave4() {
      // Verify that baseline.txt shows 34 (after 3 absorptions from 37)
      let baseline_path = "xtask/published-crate-baseline.txt";
      let baseline_content = fs::read_to_string(baseline_path)
          .expect(&format!("Failed to read {}", baseline_path));

      let baseline_count: u32 = baseline_content
          .trim()
          .parse()
          .expect("Failed to parse baseline count as integer");

      assert_eq!(
          baseline_count, 34,
          "Expected baseline to be 34 after Wave 4 absorptions, got {}",
          baseline_count
      );
  }
  ```
- **Verify**: Test file syntax is valid

### 5b. Commit
- **Commit message**: `test(parser): add Wave 4-Completion absorption validation tests (#4542)`
- **Push**: `git push`
- **Verify**:
  ```bash
  cargo test -p perl-parser --test wave4_completion_absorption_tests
  ```

---

## Final Verification (all steps complete)

### Verify 1: Workspace compilation
```bash
cargo check --workspace
```
**Expected**: All crates compile cleanly.

### Verify 2: Workspace tests (LSP threading)
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --lib
```
**Expected**: All tests pass.

### Verify 3: Layer-check (no new cycles)
```bash
cargo xtask layer-check
```
**Expected**: No violations reported.

### Verify 4: Published count
```bash
cargo xtask published-crate-count
```
**Expected**: Output `OK (34 crates, baseline 34)`.

### Verify 5: Publish closure (no broken dependencies)
```bash
cargo xtask publish-closure
```
**Expected**: Output `OK (0 violations)`.

### Verify 6: Lint & format
```bash
cargo clippy --workspace --lib -D warnings
cargo xtask fmt
```
**Expected**: No clippy warnings, no formatting changes.

---

## Notes for Builder

1. **Three separate pushes**: Each of steps 1, 2, 3 is a complete, testable commit. Push after each so context exhaustion doesn't accumulate. Baseline and tests are a 4th and 5th push.

2. **Incremental rewiring is localized**: Only text_sync.rs imports from the satellite crate. All other rewiring is within perl-parser dependencies.

3. **Feature gating preserved**: The `incremental` feature gating on lib.rs exports is maintained throughout. Tests can run with `--features incremental` to verify feature-gated modules work.

4. **Backward compatibility**: Re-export aliases (`pub use dead_code as dead_code_detector;`) are preserved so outside code can still use `perl_parser::dead_code_detector`. The module absorption is internal restructuring.

5. **Test file pattern**: Tests read Cargo.toml files and baseline.txt to confirm state. No custom test utilities needed, all standard library.
