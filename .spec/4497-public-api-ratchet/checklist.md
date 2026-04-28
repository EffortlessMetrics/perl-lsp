# Implementation Checklist: Issue #4497 — Facade-Only Public API Ratchet

**Issue:** https://github.com/EffortlessMetrics/perl-lsp/issues/4497
**Branch:** `impl/4497-public-api-ratchet`
**Scope:** 5 facade crates (perl-lsp-rs, perl-parser, perl-uri, perl-dap, perllsp)
**Plan-reviewer note:** Critical command correction from issue #4497 plan-review comment — `cargo public-api diff` (file-based) does not exist; use capture-and-diff pattern with `--simplified` flag.

---

## Part 1: Create `.ci/public-api-baselines/` directory and baseline files

**Why:** Store committed baselines for the 5 facade crates. This directory will be checked into version control as the reference for CI drift detection.

**Files to create:**
- `.ci/public-api-baselines/perl-lsp-rs.txt` (~1220 lines) — LSP server library public surface
- `.ci/public-api-baselines/perl-parser.txt` (~300 lines) — Parser library public surface
- `.ci/public-api-baselines/perl-uri.txt` (~13 lines) — URI utilities public surface
- `.ci/public-api-baselines/perl-dap.txt` (~3000 lines) — DAP server library public surface
- `.ci/public-api-baselines/perllsp.txt` (~2 lines) — Binary wrapper (re-exports only; expected small size)

**Step 1.1: Create directory**
- Command: `mkdir -p .ci/public-api-baselines`
- Verify: `[ -d .ci/public-api-baselines ] && echo "OK" || echo "FAIL"`

**Step 1.2: Generate baseline files from workspace**
- For each crate in {perl-lsp-rs, perl-parser, perl-uri, perl-dap, perllsp}:
  - Run: `cargo public-api -p <crate> --simplified 2>/dev/null | grep "^pub " > .ci/public-api-baselines/<crate>.txt`
  - This filters output to only lines starting with "^pub " (removes comments, blank lines, impl blocks)
  - The `--simplified` flag is mandatory (reduces perl-dap from ~10,246 to ~3,184 lines, critical for baseline stability)
  - Redirect stderr to `/dev/null` to suppress build noise
- Verify each baseline is non-empty: `wc -l .ci/public-api-baselines/*.txt`
  - Expected: all 5 files should have line counts close to the estimates above
  - **CRITICAL:** perllsp.txt will be ~2 lines (re-exports only) — this is correct, not an error

**Step 1.3: Commit baselines**
- Stage files: `git add .ci/public-api-baselines/`
- Status check: `git status .ci/public-api-baselines/`
- **Do NOT commit yet** — these go in the final commit after all other changes

---

## Part 2: Add `public-api-check:` CI job to `.github/workflows/ci-nightly.yml`

**Why:** Wire a new hard-fail CI check that compares current public API surface against committed baselines. Unlike the existing `semver-check:` job (which has `continue-on-error: true`), this job hard-fails CI when the surface drifts without a baseline update.

**File:** `.github/workflows/ci-nightly.yml`
**Insertion point:** After line 345 (after the `semver-check:` job block closes, before `clippy-strict:`)

**Step 2.1: Insert new `public-api-check:` job**

Add this YAML block starting at line 346 (push `clippy-strict:` down):

```yaml
  public-api-check:
    name: Public API Surface Check
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    if: |
      github.event_name == 'workflow_dispatch' ||
      github.event_name == 'schedule' ||
      (github.event_name == 'pull_request' &&
       contains(github.event.pull_request.labels.*.name, 'ci:public-api'))
    steps:
      - uses: actions/checkout@v6

      - name: Install Rust
        uses: dtolnay/rust-toolchain@3c5f7ea28cd621ae0bf5283f0e981fb97b8a7af9  # stable (master)
        with:
          toolchain: stable

      - name: Cache cargo dependencies
        uses: Swatinem/rust-cache@c19371144df3bb44fab255c43d04cbc2ab54d1c4  # v2.9.1
        with:
          key: public-api-${{ hashFiles('Cargo.lock') }}
          cache-on-failure: true

      - name: Install cargo-public-api
        run: cargo install cargo-public-api --locked --version 0.50.1

      - name: Check public API surface
        run: just public-api-check
```

**Verify after insertion:**
- Command: `grep -n "public-api-check:" .github/workflows/ci-nightly.yml`
- Expected: Should see both the job name and the step name
- Command: `grep -n "continue-on-error" .github/workflows/ci-nightly.yml | grep public-api`
- Expected: NO match (the new job has NO continue-on-error; hard-fail only)

---

## Part 3: Sync `semver-check:` CI job to cover 5 crates

**Why:** The `justfile:semver-check-all` recipe covers 5 crates (perl-parser, perl-lexer, perl-parser-core, perl-lsp-rs, perllsp), but the CI `semver-check:` job only hardcodes 3. This sync brings them into alignment.

**File:** `.github/workflows/ci-nightly.yml`
**Insertion point:** Lines 329-330 (after the `perl-parser-core` check, before the "Generate breaking changes report" step)

**Step 3.1: Add `perl-lsp-rs` and `perllsp` checks to `semver-check:` job**

Add these two blocks after line 329:

```yaml
      - name: Check perl-lsp-rs API compatibility
        run: |
          cargo semver-checks check-release -p perl-lsp-rs \
            --baseline-rev ${{ steps.baseline.outputs.baseline }} \
            || echo "::warning::Breaking changes detected in perl-lsp-rs"

      - name: Check perllsp API compatibility
        run: |
          cargo semver-checks check-release -p perllsp \
            --baseline-rev ${{ steps.baseline.outputs.baseline }} \
            || echo "::warning::Breaking changes detected in perllsp"
```

**Verify after insertion:**
- Command: `grep "cargo semver-checks check-release -p" .github/workflows/ci-nightly.yml | wc -l`
- Expected: 5 (perl-parser, perl-lexer, perl-parser-core, perl-lsp-rs, perllsp)

---

## Part 4: Add justfile recipes for local public API checks

**Why:** Provide local commands for developers to check and regenerate baselines without pushing to CI.

**File:** `justfile`
**Insertion point:** After line 1795 (after `_semver-check-install` block, before the next recipe)

**Step 4.1: Add three new recipes**

Insert these recipes starting at line 1796:

```
# Private helper: install cargo-public-api if not present
[private]
_public-api-install:
    @if ! command -v cargo-public-api >/dev/null 2>&1; then \
        echo "Installing cargo-public-api..."; \
        cargo install cargo-public-api --locked --version 0.50.1; \
    fi

# Check public API surface of facade crates against committed baselines
public-api-check:
    #!/usr/bin/env bash
    set -euo pipefail
    just _public-api-install
    echo "Checking public API surface for facade crates..."
    FAILED=0
    for crate in perl-lsp-rs perl-parser perl-uri perl-dap perllsp; do
        BASELINE=".ci/public-api-baselines/${crate}.txt"
        if [ ! -f "$BASELINE" ]; then
            echo "FAIL Missing baseline: $BASELINE (run: just public-api-update)"
            FAILED=1
            continue
        fi
        cargo public-api -p "$crate" --simplified 2>/dev/null | grep "^pub " > "/tmp/${crate}-current.txt"
        if ! diff -u "$BASELINE" "/tmp/${crate}-current.txt" > "/tmp/${crate}-diff.txt" 2>&1; then
            echo "FAIL Public API changed in ${crate}:"
            cat "/tmp/${crate}-diff.txt"
            FAILED=1
        else
            echo "OK ${crate}: API surface unchanged"
        fi
    done
    [ $FAILED -eq 0 ] || { echo "Run 'just public-api-update' to regenerate baselines if the change is intentional."; exit 1; }

# Regenerate all public API baselines from current workspace state
public-api-update:
    #!/usr/bin/env bash
    set -euo pipefail
    just _public-api-install
    echo "Regenerating public API baselines..."
    mkdir -p .ci/public-api-baselines
    for crate in perl-lsp-rs perl-parser perl-uri perl-dap perllsp; do
        cargo public-api -p "$crate" --simplified 2>/dev/null | grep "^pub " \
            > ".ci/public-api-baselines/${crate}.txt"
        echo "Updated ${crate}: $(wc -l < .ci/public-api-baselines/${crate}.txt) lines"
    done
    echo "Commit .ci/public-api-baselines/ with your PR."
```

**Verify after insertion:**
- Command: `grep -n "^_public-api-install:\|^public-api-check:\|^public-api-update:" justfile`
- Expected: 3 lines showing all three recipes exist
- Command: `cargo install --locked --version 0.50.1 --version 0.50.1 cargo-public-api` (smoke test cargo-public-api is available)
- Note: This installs locally; the CI job will also install it separately

---

## Part 5: Update `CONTRIBUTING.md` with public API surface workflow

**Why:** Document the workflow for developers who modify public facade APIs, so they know when and how to regenerate baselines.

**File:** `CONTRIBUTING.md`
**Insertion point:** After line 302 (after the `## SemVer and Breaking Changes` section closes, before `## Release Workflow`)

**Step 5.1: Insert new subsection**

Add this markdown block at line 303:

```markdown
### Public API Surface Ratchet

The five user-facing facade crates (`perl-lsp-rs`, `perl-parser`, `perl-uri`, `perl-dap`, `perllsp`) have their public API surface locked in text baselines at `.ci/public-api-baselines/`. The nightly CI job fails if the surface changes without a baseline update.

When you intentionally add or remove items from a facade crate's public API:

1. Run `just public-api-update` to regenerate all 5 baselines.
2. Include the updated `.ci/public-api-baselines/*.txt` files in your PR.
3. In your PR description, describe what changed and why.
4. Add the `ci:public-api` label to trigger the surface check in CI.

The check uses `cargo public-api -p <crate> --simplified` (omits blanket-impl noise).
```

**Verify after insertion:**
- Command: `grep -n "### Public API Surface Ratchet" CONTRIBUTING.md`
- Expected: Should find the new subsection
- Command: `grep -A 5 "### Public API Surface Ratchet" CONTRIBUTING.md | head -6`
- Expected: Should show the inserted subsection text

---

## Part 6: Final verification and commit

**Why:** Ensure all changes are in place, compile/test clean, and ready for builder handoff.

**Step 6.1: Baseline files exist and are non-empty**
- Command: `wc -l .ci/public-api-baselines/*.txt`
- Expected: 5 files, all with line counts > 0
- Expected perllsp.txt: ~2 lines (correct)
- Expected perl-dap.txt: ~3000 lines (with `--simplified`)

**Step 6.2: CI job exists and is correctly configured**
- Command: `grep -A 25 "^  public-api-check:" .github/workflows/ci-nightly.yml | head -26`
- Expected: Should show the new job with `runs-on: ubuntu-24.04`, `timeout-minutes: 20`, and the `just public-api-check` command

**Step 6.3: Justfile recipes exist**
- Command: `grep -A 2 "^public-api-check:" justfile`
- Expected: Should show the bash recipe block
- Command: `grep "cargo-public-api --locked --version 0.50.1" justfile`
- Expected: Should find the version pin in the install helper

**Step 6.4: CONTRIBUTING.md updated**
- Command: `grep "### Public API Surface Ratchet" CONTRIBUTING.md`
- Expected: Should find the new subsection
- Command: `grep -c "just public-api-update" CONTRIBUTING.md`
- Expected: Should be >= 2 (at least in the new subsection)

**Step 6.5: Syntax check justfile**
- Command: `just --list 2>&1 | grep -E "public-api-check|public-api-update|_public-api-install"`
- Expected: 3 recipes should be listed

**Step 6.6: Syntax check YAML**
- Command: `python3 -m py_compile .github/workflows/ci-nightly.yml 2>&1` (or use a YAML validator)
- Alternative (safer): `grep -c "public-api-check:" .github/workflows/ci-nightly.yml`
- Expected: 2 (job name + step name)

**Step 6.7: Stage and commit**
- Stage spec and baseline files: `git add .spec/4497-public-api-ratchet/ .ci/public-api-baselines/`
- Also stage modified files: `git add .github/workflows/ci-nightly.yml justfile CONTRIBUTING.md`
- Verify staging: `git status`
- Commit with message:
  ```
  plan(ci): add implementation spec for #4497 (public-api-ratchet)
  ```
- Verify commit: `git log --oneline -1`
- Push to remote: `git push -u origin impl/4497-public-api-ratchet`

---

## Build Execution Order

This is a spec-planner task — no implementation code is written. The builder will:

1. **Read spec files** (this document, acceptance.md, context.md)
2. **Red-TDD:** Write failing tests that verify:
   - `just public-api-check` exits 1 when a public function is removed from perl-uri
   - `just public-api-check` exits 0 on a clean tree
   - `just public-api-update` regenerates all 5 baseline files
   - CI job `public-api-check:` is triggered by `ci:public-api` label
3. **Implementation:** Execute steps 1-6 above exactly as specified
4. **Green:** Run all tests; they should pass
5. **Verify:** `cargo test -p perl-parser --lib`, `cargo test -p perl-lsp-rs --lib`, etc. (no changes to source code, so existing tests should still pass)

---

## Constraints and Scope Boundaries

**Touch ONLY:**
- `.github/workflows/ci-nightly.yml` (add public-api-check job; sync semver-check steps)
- `justfile` (add 3 new recipes)
- `CONTRIBUTING.md` (add subsection)
- `.ci/public-api-baselines/` (new directory + 5 text files)

**Do NOT touch:**
- Any `crates/` source code
- Any `Cargo.toml` files
- `xtask/` directory or scripts
- `features.toml`
- Other CI workflow files (`.github/workflows/*.yml` except ci-nightly.yml)

**Assumptions:**
- Stable Rust 1.92.0 or later is available
- `cargo-public-api` 0.50.1 installs cleanly from crates.io
- The 5 facade crates (perl-lsp-rs, perl-parser, perl-uri, perl-dap, perllsp) have stable public surfaces at current master commit 961c85dfc

---

## Notes for Builder

1. **Baseline generation is idempotent:** Running the capture command twice on the same crate produces identical output (same line order, same function signatures).

2. **The `--simplified` flag is critical:** Without it, perl-dap baseline grows to ~10,246 lines and adding cosmetic changes (e.g., `#[derive(Debug)]` to a DAP struct) breaks the check for false reasons. The `--simplified` flag filters blanket impls and reduces noise. Use it consistently in both capture and check.

3. **`perllsp` near-empty baseline is expected:** The `perllsp` crate is a thin binary wrapper around `perl-lsp-rs`. It has both `[lib]` and `[[bin]]` targets. `cargo public-api` reports only the lib target, which is just re-exports. ~2 lines is correct. Do not try to expand it.

4. **Diff tool sensitivity:** The `diff -u` command in the justfile recipes outputs to stderr and sets exit code 1 on any difference. This is intentional — the `public-api-check:` recipe captures the diff and prints it before failing.

5. **CI job label trigger:** The `ci:public-api` label in the PR causes the CI job to run. This mirrors the existing `ci:semver` label for the semver-check job. Document this in the PR checklist template if needed.

6. **No version pinning needed in Cargo.toml:** `cargo-public-api` is installed via `cargo install --locked`, not as a workspace dependency. Version 0.50.1 is pinned at the CI and justfile levels only.

---

## Verify Commands (TDD Builder)

Use these commands to verify implementation correctness:

```bash
# Baselines exist and are non-empty
wc -l .ci/public-api-baselines/*.txt

# CI job added to workflow
grep -n "public-api-check:" .github/workflows/ci-nightly.yml

# Justfile recipes exist
grep -n "^public-api-check:\|^public-api-update:\|^_public-api-install:" justfile

# CONTRIBUTING.md has new subsection
grep "### Public API Surface Ratchet" CONTRIBUTING.md

# Local check passes on clean tree
just public-api-check

# Semver-check job now covers 5 crates
grep "cargo semver-checks check-release -p" .github/workflows/ci-nightly.yml | wc -l
# Expected: 5

# Syntax validation (justfile)
just --list 2>&1 | grep public-api

# Syntax validation (YAML)
grep -c "public-api-check:" .github/workflows/ci-nightly.yml
# Expected: 2 (job name + step name)
```

---

**End of Checklist**

Version: 1.0
Last updated: 2026-04-18
Status: Ready for red-TDD
