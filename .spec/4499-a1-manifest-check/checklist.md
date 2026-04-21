# Implementation Checklist: xtask publish-manifest-check (#4499)

**Issue:** #4499 (A1 offline manifest-lint)  
**Branch:** `impl/4499-a1-manifest-check`  
**Base:** master 2a57448c8  
**Effort:** ~150-200 LOC Rust + ~30 LOC justfile + 1-line workflow  

---

## Changes overview

Consolidate existing Python allowlist-drift check from `.github/workflows/publish-dry-run.yml` lines 61-70 into a new `cargo xtask publish-manifest-check` subcommand. Add LICENSE-present check. Factor shared `load_publish_allowlist()` helper into `xtask/src/utils.rs`.

**Scope:** Allowlist drift + LICENSE-present only. No keyword count, wildcard deps, description, repository, or SemVer checks (cargo catches those).

---

## Part 1: Factor shared allowlist loader

**File:** `/h/Code/Rust/perl-lsp/xtask/src/utils.rs`  
**Action:** APPEND (do NOT create `utils/` subdirectory — utils.rs is a single file)

1. Read the current file to locate `constrained_env_vars()` function (last function before module end).
2. After that function, append three new serde-derived types and one helper function:
   - `AllowlistMetadata` struct with `workspace_metadata: Option<WorkspacePublishMeta>`
   - `WorkspacePublishMeta` struct with `publish: Option<AllowList>`
   - `AllowList` struct with `allow: Option<Vec<String>>`
   - `pub fn load_publish_allowlist() -> color_eyre::eyre::Result<Vec<String>>`

   The function must:
   - Call `run_cargo_metadata(true)?` to get JSON bytes
   - Deserialize to `AllowlistMetadata`
   - Chain `.workspace_metadata.and_then().and_then()` to extract the allow list
   - Return `Err` if list is absent or empty
   - Return `Ok(Vec<String>)` with crate names

3. Verify: `cargo build -p xtask` compiles

---

## Part 2: Refactor publish_closure.rs

**File:** `/h/Code/Rust/perl-lsp/xtask/src/tasks/publish_closure.rs`  
**Action:** REMOVE duplicate structs, ADD use statement

1. Locate lines ~45-53 (the `WorkspacePublishMeta` and `AllowList` struct definitions).
2. Remove those two structs entirely.
3. Add `use crate::utils::{AllowlistMetadata, load_publish_allowlist};` at the top of the file.
4. Update `load_metadata()` function to:
   - Call `run_cargo_metadata(true)?` instead of inline deserialization
   - Use `crate::utils::load_publish_allowlist()` to get the allowlist
   - Deserialize only the `FullMetadata` (packages + resolve), not the metadata structs

5. Verify: `cargo test -p xtask --lib` passes (existing publish_closure tests should pass)

---

## Part 3: Refactor count_ratchet.rs

**File:** `/h/Code/Rust/perl-lsp/xtask/src/tasks/count_ratchet.rs`  
**Action:** REMOVE duplicate structs, ADD use statement

1. Locate lines ~31-43 (the `WorkspacePublishMeta` and `AllowList` struct definitions).
2. Remove those two structs entirely.
3. Add `use crate::utils::load_publish_allowlist;` at the top of the file.
4. Update `current_count()` function to call `load_publish_allowlist()` instead of inline deserialization.
5. Verify: `cargo test -p xtask --lib` passes (existing count_ratchet tests should pass)

---

## Part 4: Create publish_manifest_check.rs

**File:** `/h/Code/Rust/perl-lsp/xtask/src/tasks/publish_manifest_check.rs` (NEW)  
**Action:** CREATE with two-check logic

1. Create the new file with module docs explaining allowlist-drift + LICENSE-present checks.
2. Define serde structs for NO-DEPS metadata parsing:
   - `NoDepsMetadata` with `packages: Vec<NoDepsPackage>` and `workspace_members: Vec<String>`
   - `NoDepsPackage` with `name`, `id`, `publish: Option<Vec<String>>`, `license: Option<String>`, `license_file: Option<String>`

3. Implement `pub fn run() -> Result<()>`:
   - Load allowlist via `load_publish_allowlist()?`
   - Get metadata via `run_cargo_metadata(true)?`
   - Deserialize to `NoDepsMetadata`
   - Call `check_metadata(&meta, &allowlist)` to collect violations
   - Print violations to stderr (each prefixed "ERROR: publish-manifest-check: ")
   - Exit non-zero if violations not empty; else print success and exit 0

4. Implement `pub fn check_metadata(meta: &NoDepsMetadata, allowlist: &[String]) -> Vec<String>`:
   - DRIFT CHECK A: Iterate allowlist; for each crate not in publishable set, add violation
   - DRIFT CHECK B: Iterate publishable set; for each crate not in allowlist, add violation
   - LICENSE CHECK: For each allowlist crate, verify `license` or `license_file` is non-empty (workspace inheritance is already resolved by cargo metadata)
   - Return all violations collected

5. Add inline unit tests (in `#[cfg(test)]` block):
   - `clean_metadata_no_violations()` — one allowlisted crate with license
   - `drift_a_allowlist_has_publish_false_crate()` — allowlist contains a publish=false crate
   - `drift_b_publishable_crate_absent_from_allowlist()` — publishable crate missing from allowlist
   - `missing_license_detected()` — allowlist crate with no license or license_file

6. Verify: `cargo test -p xtask --lib publish_manifest_check` passes

---

## Part 5: Register in mod.rs

**File:** `/h/Code/Rust/perl-lsp/xtask/src/tasks/mod.rs`  
**Action:** ADD module declaration

1. Locate the line `pub mod publish_closure;` (around line 30).
2. Add `pub mod publish_manifest_check;` immediately after or nearby.
3. Verify: `cargo build -p xtask` compiles

---

## Part 6: Register in main.rs

**File:** `/h/Code/Rust/perl-lsp/xtask/src/main.rs`  
**Action:** ADD two items (enum variant + dispatch)

1. Locate the `Commands` enum (around line 732).
2. Find the line `PublishedCrateCount,` (after the `LayerCheck` variant).
3. Add immediately after:
   ```rust
   /// Offline manifest validation: allowlist drift + LICENSE present.
   ///
   /// Consolidates the Python --check-drift step from publish-dry-run.yml
   /// into the xtask gate suite. Wired into pr-fast and ci-gate.
   PublishManifestCheck,
   ```

4. Locate the match dispatch block (around line 1401, inside the match on `self`).
5. Find the line `Commands::PublishedCrateCount => tasks::published_crate_count::run(),`.
6. Add immediately after:
   ```rust
   Commands::PublishManifestCheck => tasks::publish_manifest_check::run(),
   ```

7. Verify: `cargo xtask publish-manifest-check --help` shows the command (should work via clap's auto-derive from variant name)

---

## Part 7: Create integration test

**File:** `/h/Code/Rust/perl-lsp/xtask/tests/publish_manifest_check_test.rs` (NEW)  
**Action:** CREATE integration test

1. Create the new file with:
   - Import `use assert_cmd::Command;`
   - One test: `publish_manifest_check_passes_on_master()` that spawns `cargo xtask publish-manifest-check` and asserts `.success()`

2. Verify: `cargo test -p xtask --test publish_manifest_check_test` passes on master

---

## Part 8: Edit publish-dry-run.yml workflow

**File:** `/h/Code/Rust/perl-lsp/.github/workflows/publish-dry-run.yml`  
**Action:** REPLACE lines 61-70 ONLY; keep 72 onward

1. Locate lines 61-70 (the "Check allowlist drift" step with Python --check-drift invocation).
2. Replace the entire step with:
   ```yaml
   - name: Check allowlist drift (manifest check)
     run: cargo xtask publish-manifest-check
   ```

3. **IMPORTANT:** Do NOT touch lines 72 onward. The topo-sort work stays in Python:
   ```yaml
   - name: Compute topological publish order
     id: topo
     run: |
       cargo metadata --format-version=1 --no-deps \
         | python3 scripts/publish-topo.py > /tmp/crates.json
   ```

4. Update the `paths:` trigger (lines 15-25):
   - Add `'xtask/src/tasks/publish_manifest_check.rs'` to the list
   - Keep `'scripts/publish-topo.py'` (it's still used by topo-sort step)
   - Keep `'scripts/tests/test-publish-topo.py'`

5. Verify workflow syntax: `gh workflow view publish-dry-run.yml` (no parse errors)

---

## Part 9: Add justfile recipes

**File:** `/h/Code/Rust/perl-lsp/justfile`  
**Action:** ADD recipe and wire into gates

1. Locate line ~881 (after `ci-published-crate-count` recipe).
2. Add:
   ```just
   ci-publish-manifest-check:
       @echo "Checking publish manifest (drift + LICENSE)..."
       @cargo xtask publish-manifest-check
   ```

3. Locate the `pr-fast` recipe (line ~55).
4. Find the line with `just _timed "published-crate-count" ...`.
5. Add immediately after:
   ```
   just _timed "publish-manifest-check" "just ci-publish-manifest-check" && \
   ```
   (Keep the `&& \` continuation)

6. Locate the `ci-gate` recipe (line ~769).
7. Find the line `just ci-published-crate-count`.
8. Add immediately after:
   ```
   just ci-publish-manifest-check
   ```

9. Locate the existing `publish-allowlist-check` recipe (line ~2132).
10. Replace its body to delegate:
    ```just
    publish-allowlist-check:
        @cargo xtask publish-manifest-check
    ```

11. Verify: `just ci-publish-manifest-check` runs without error on master

---

## Verify commands (in order)

After each step, run the corresponding verify command:

| Step | Command | Expected |
|------|---------|----------|
| 1 | `cargo build -p xtask` | Compiles |
| 2 | `cargo test -p xtask --lib` | All tests pass |
| 3 | `cargo test -p xtask --lib` | All tests pass |
| 4 | `cargo test -p xtask --lib publish_manifest_check` | 4 unit tests pass |
| 5 | `cargo build -p xtask` | Compiles |
| 6 | `cargo xtask publish-manifest-check --help` | Shows command (via clap) |
| 7 | `cargo test -p xtask --test publish_manifest_check_test` | Integration test passes |
| 8 | `cargo xtask publish-manifest-check` | Exits 0 on master (no violations) |
| 9 | `just ci-publish-manifest-check` | Exits 0 on master |

**Final gates:**
```bash
cargo test -p xtask                                      # All xtask tests pass
cargo xtask publish-manifest-check                       # Happy path on master
cargo clippy -p xtask -- -D warnings                     # No clippy violations
cargo xtask fmt                                          # Code is formatted
just pr-fast                                             # pr-fast gate passes
```

---

## Acceptance criteria

- [ ] `load_publish_allowlist()` defined in `xtask/src/utils.rs`
- [ ] `AllowlistMetadata`, `WorkspacePublishMeta`, `AllowList` types defined in utils.rs
- [ ] `publish_closure.rs` refactored to use shared types (removed duplicate structs)
- [ ] `count_ratchet.rs` refactored to use shared types (removed duplicate structs)
- [ ] `publish_manifest_check.rs` created with `run()` and `check_metadata()` functions
- [ ] `check_metadata()` returns violations: drift A, drift B, license
- [ ] Inline unit tests pass: 4 test functions in publish_manifest_check.rs
- [ ] `PublishManifestCheck` variant added to `Commands` enum in main.rs
- [ ] Dispatch implemented: `Commands::PublishManifestCheck => tasks::publish_manifest_check::run()`
- [ ] Integration test created: `publish_manifest_check_test.rs`
- [ ] `.github/workflows/publish-dry-run.yml` lines 61-70 replaced with single-line xtask call
- [ ] `scripts/publish-topo.py` **NOT deleted** (still used by topo-sort step)
- [ ] `ci-publish-manifest-check` recipe added to justfile
- [ ] `pr-fast` recipe includes `publish-manifest-check` gate
- [ ] `ci-gate` recipe includes `publish-manifest-check` gate
- [ ] `publish-allowlist-check` recipe delegates to xtask command
- [ ] No `unwrap()`, `expect()`, `panic!()` in production code
- [ ] `cargo test -p xtask` passes (all unit + integration tests)
- [ ] `cargo xtask publish-manifest-check` exits 0 on master
- [ ] `just pr-fast` gate passes
- [ ] `just ci-gate` includes the new check

---

## Key notes for builder

1. **`xtask/src/utils.rs` is a SINGLE FILE, not a directory.** Do NOT create `utils/publish.rs` or `utils/` subdirectory. Append directly to utils.rs.

2. **`scripts/publish-topo.py` has 4 callers beyond `--check-drift`:**
   - `--check-drift` at workflow line 61-70 (being replaced) ← ONLY THIS MOVES TO XTASK
   - Topo-sort at workflow line 72-86 (stays in Python)
   - `justfile:2150` (stays in Python)
   - `scripts/tests/test-publish-topo.py` (stays in Python)
   - CI trigger in `ci-gate-self-tests.yml` (stays in Python)
   
   **Do NOT delete publish-topo.py.** Only the `--check-drift` invocation moves.

3. **Workspace license inheritance is safe:**
   - 44 crates use `license.workspace = true` in this workspace
   - Cargo metadata resolves this to `"MIT OR Apache-2.0"` in JSON before Rust code sees it
   - No false positives; the license field in cargo metadata output is already expanded

4. **Happy path confirmed on master 2a57448c8:**
   - Zero allowlist drift violations
   - Zero missing LICENSE violations
   - Integration test will pass immediately

5. **Refactor both `publish_closure.rs` AND `count_ratchet.rs`:**
   - Both have the same `WorkspacePublishMeta` / `AllowList` struct pair
   - Both must be refactored to use the shared `load_publish_allowlist()` from utils.rs
   - Do NOT touch `publish.rs` — it has a different `CargoMetadata` struct with `packages` field needed by `publish_crates()`

6. **`check_metadata()` must be `pub fn`:**
   - Not `pub(crate)` — tests in the same file AND integration test file both call it
   - Export for visibility to tests

7. **Test fixtures already verified:**
   - Master has no drift and no missing licenses
   - Unit tests mock metadata with helper functions (`make_pkg()`, `make_meta()`)
   - No need for separate `.toml` fixture files in `xtask/tests/fixtures/`
   - Inline unit tests in publish_manifest_check.rs cover all violation classes

8. **Clap auto-derive:**
   - `PublishManifestCheck` enum variant becomes `publish-manifest-check` subcommand automatically
   - No need for manual clap attribute configuration
   - `--help` will show it

---

## Gotchas to avoid

- **Do NOT** create `xtask/src/utils/publish.rs` or a `utils/` subdirectory
- **Do NOT** delete or move `scripts/publish-topo.py`
- **Do NOT** remove the topo-sort step from the workflow (lines 72 onward)
- **Do NOT** touch `xtask/src/tasks/publish.rs` — it has different structs
- **Do NOT** use `unwrap()`, `expect()`, `panic!()` in production code (use `?` and `Result`)
- **Do NOT** forget to export `check_metadata()` as `pub fn` for test visibility
- **Do NOT** add keyword count, wildcard deps, description, repository, or SemVer checks (they're out of scope)

---

## Compilation order

Build in this order to ensure compilation succeeds at each step:

1. Part 1: `cargo build -p xtask` (add helpers to utils.rs)
2. Part 2: `cargo test -p xtask --lib` (refactor publish_closure.rs)
3. Part 3: `cargo test -p xtask --lib` (refactor count_ratchet.rs)
4. Part 4: `cargo test -p xtask --lib publish_manifest_check` (create new task)
5. Part 5: `cargo build -p xtask` (register in mod.rs)
6. Part 6: `cargo xtask publish-manifest-check --help` (register in main.rs dispatch)
7. Part 7: `cargo test -p xtask --test publish_manifest_check_test` (integration test)
8. Part 8: Workflow syntax check (edit .github workflow)
9. Part 9: `just pr-fast` (wire into recipes)

---

## Success criteria

All of the following pass on the builder's impl branch before PR creation:

```bash
cargo test -p xtask
cargo xtask publish-manifest-check
cargo clippy -p xtask -- -D warnings
cargo xtask fmt
just pr-fast
just ci-gate
```

Expected result: All commands exit 0 on master HEAD.
