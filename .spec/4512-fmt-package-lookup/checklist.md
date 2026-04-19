# Implementation Checklist: #4512 -- fix(hook): pre-push cargo fmt uses dir basename instead of Cargo.toml package name

## Change order (compiles at each step)

### Step 1: Make resolve_package_names public and add a single-dir helper
- **File:** `xtask/src/tasks/targeted_checks.rs`
- **Change:** Change `fn resolve_package_names` to `pub fn resolve_package_names`. Add a new public helper `pub fn resolve_single_package_name`.
- **Details:**
  Signature:
  ```rust
  pub fn resolve_single_package_name(project_root: &Path, crate_dir: &str) -> Result<String>
  ```
  Body: insert `crate_dir.trim_end_matches('/').to_string()` into a BTreeSet, call `resolve_package_names`, return the first (and only) element or `eyre!("No workspace package found for crate directory: {crate_dir}")`.
- **Verify:** `cargo check -p xtask`

### Step 2: Add ResolvePackageName CLI subcommand to xtask
- **File:** `xtask/src/main.rs`
- **Change:** Add `ResolvePackageName { crate_dir: String }` variant to `Commands` enum and dispatch it.
- **Details:**
  Variant doc: "Resolve the Cargo package name for a crate directory. Used by the pre-push hook to convert a directory basename into the correct -p argument. Example: cargo xtask resolve-package-name crates/perl-lsp -> perl-lsp-rs"
  Dispatch arm: call `tasks::targeted_checks::resolve_single_package_name(&root, &crate_dir)?` where `root = utils::project_root()?`, then `println!("{}", name)`.
  Check if `utils` is already in scope in main.rs; use the existing import pattern.
- **Depends on:** Step 1
- **Verify:** `cargo check -p xtask`

### Step 3: Fix the pre-push hook
- **File:** `hooks/pre-push` (canonical source; `.git/hooks/pre-push` auto-updates on next push)
- **Change:** Replace basename-derived `SINGLE_CRATE_NAME` with xtask-resolved package name.
- **Details:**
  Current line ~151:
  ```
  SINGLE_CRATE_NAME="$(printf '%s' "$SINGLE_CRATE_NAMES" | tr -d '[:space:]')"
  ```
  Replace with:
  ```
  SINGLE_CRATE_DIR="$(printf '%s' "$SINGLE_CRATE_NAMES" | tr -d '[:space:]')"
  # Resolve the Cargo package name from Cargo.toml, not the directory basename.
  # Fallback to dirname if xtask is unavailable (issue #4512).
  SINGLE_CRATE_NAME="$(cargo xtask resolve-package-name "crates/${SINGLE_CRATE_DIR}" 2>/dev/null || printf '%s' "$SINGLE_CRATE_DIR")"
  ```
  Update the echo line to show both for diagnostics:
  ```
  echo "Single-crate push (${SINGLE_CRATE_DIR} -> ${SINGLE_CRATE_NAME}) -- running targeted gate"
  ```
  Lines 156-158 (cargo fmt/clippy/test) use $SINGLE_CRATE_NAME unchanged -- they will now correctly use the resolved package name.
- **Depends on:** Step 2
- **Verify:** `cargo xtask resolve-package-name crates/perl-lsp` outputs `perl-lsp-rs`

### Step 4: Add lib.rs if xtask is binary-only
- **File:** `xtask/src/lib.rs` (create if missing)
- **Change:** If `xtask/src/lib.rs` does not exist, create it to expose the tasks and utils modules to integration tests.
- **Details:**
  Content if creating:
  ```rust
  //! xtask library surface -- exposed for integration tests.
  pub mod tasks;
  pub mod utils;
  ```
  Also add to `xtask/Cargo.toml` if needed:
  ```toml
  [lib]
  name = "xtask"
  path = "src/lib.rs"
  ```
  If lib.rs already exists, just verify `pub mod tasks` is present.
- **Depends on:** Step 1
- **Verify:** `cargo check -p xtask`

### Step 5: Add regression test
- **File:** `xtask/tests/fmt_package_lookup_tests.rs`
- **Change:** New integration test file.
- **Details:**
  Three tests:
  1. `resolve_uses_cargo_toml_name_not_dir_basename` -- synthetic workspace where dir="my-dir", package name="my-package". Assert result == "my-package".
  2. `resolve_when_dir_and_name_match` -- normal case where dir and package name are the same.
  3. `resolve_returns_error_for_unknown_dir` -- dir not in workspace members, assert Err.
  Use `tempfile::tempdir()` to create synthetic workspaces. `tempfile` is already a dev-dep in xtask.
  The test imports: `use xtask::tasks::targeted_checks::resolve_single_package_name;`
- **Depends on:** Steps 1 and 4
- **Verify:** `cargo test -p xtask --test fmt_package_lookup_tests`

### Step 6: Final verification
- **Verify:**
  ```
  cargo check -p xtask
  cargo test -p xtask --test fmt_package_lookup_tests
  cargo xtask resolve-package-name crates/perl-lsp  (output must be: perl-lsp-rs)
  cargo clippy -p xtask -- -D warnings
  cargo xtask fmt
  ```

## Callers and consumers

- `resolve_package_names` (targeted_checks.rs:102) -- called only from `targeted_checks::run` at line 245. Making it pub does not break callers.
- `SINGLE_CRATE_NAME` in `hooks/pre-push` -- used only within the single-crate gate block (lines ~151-174).
- `fmt::run(check)` -- NOT changed; already correct (uses --manifest-path, not -p).

## Scope boundary

Files IN scope:
- `xtask/src/tasks/targeted_checks.rs`
- `xtask/src/main.rs`
- `hooks/pre-push`
- `xtask/tests/fmt_package_lookup_tests.rs` (new)
- `xtask/src/lib.rs` (create if missing)

Files OUT of scope:
- `xtask/src/tasks/fmt.rs` -- already correct
- `.git/hooks/pre-push` -- auto-updated from hooks/pre-push
- `Cargo.toml` / `xtask/Cargo.toml` -- no new runtime deps needed
- `crates/perl-lsp/` rename -- tracked as #4511

## Flags for builder

1. **Check lib.rs first**: Run `ls xtask/src/lib.rs`. If absent, create minimal one before writing tests.
2. **Windows path separator**: `resolve_package_names` uses `strip_prefix('/')`. On Windows manifest paths may use backslash. Check the existing function for Windows compat; normalize with `.replace('\', '/')` if needed before stripping.
3. **tempfile in dev-deps**: Confirmed `tempfile.workspace = true` in xtask/Cargo.toml `[dependencies]`. It may already be available for tests. Do NOT add a duplicate.
4. **Do NOT use cargo_metadata crate**: The existing serde_json shell-out pattern is what to follow. No new crate dep needed.
5. **Fallback in hook is intentional**: The `|| printf '%s' "$SINGLE_CRATE_DIR"` fallback in Step 3 gracefully degrades. Keep it.
