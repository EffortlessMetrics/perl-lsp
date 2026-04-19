# Acceptance Criteria: Issue #4497 — Facade-Only Public API Ratchet

Extracted from issue body and plan-reviewer comment.

**Status:** All criteria are verifiable via command line or local test.

---

## Core Acceptance Criteria

- [ ] All 5 facade crates have baselines in `.ci/public-api-baselines/{perl-lsp-rs,perl-parser,perl-uri,perl-dap,perllsp}.txt`

- [ ] Each baseline file is non-empty and contains only lines starting with `^pub ` (public API items)

- [ ] `perllsp.txt` baseline is ~2 lines (re-exports only) — this is expected and correct, not an error

- [ ] Baseline capture is idempotent: running `cargo public-api -p <crate> --simplified 2>/dev/null | grep "^pub "` twice produces identical output

- [ ] `just public-api-check` exits 0 on a clean tree (no uncommitted baseline changes)

- [ ] `just public-api-check` exits 1 and prints a diff when a public function is removed from any facade crate (spot-check: remove one function from `perl-uri`, run check, verify diff is printed)

- [ ] `just public-api-update` regenerates all 5 baseline files without error

- [ ] New `public-api-check:` CI job exists in `.github/workflows/ci-nightly.yml` with no `continue-on-error: true` (hard-fail only)

- [ ] CI job `public-api-check:` is triggered by `github.event_name == 'schedule'` (nightly runs) OR `ci:public-api` label on PRs

- [ ] `semver-check:` CI job in `.github/workflows/ci-nightly.yml` now covers 5 crates (perl-parser, perl-lexer, perl-parser-core, perl-lsp-rs, perllsp)

- [ ] `cargo-public-api 0.50.1` is pinned in both `justfile` install helper and CI step

- [ ] `CONTRIBUTING.md` documents `just public-api-update` workflow under new "### Public API Surface Ratchet" subsection

- [ ] Workspace compiles cleanly: `cargo check --workspace` exits 0

- [ ] All existing tests pass: `cargo test --workspace --lib` (no source code changes, so no new tests required)

---

## Implementation Verification

- [ ] Spec files committed to `.spec/4497-public-api-ratchet/`: checklist.md, acceptance.md, context.md

- [ ] Branch name is `impl/4497-public-api-ratchet` and is pushed to origin

- [ ] All file paths and line numbers in checklist match the actual codebase at commit HEAD

---

## Scope Boundary Verification

- [ ] No changes to `crates/*/` source directories

- [ ] No changes to any `Cargo.toml` files

- [ ] No changes to `xtask/` directory

- [ ] No changes to `features.toml`

- [ ] No changes to CI workflows other than `.github/workflows/ci-nightly.yml`

---

**Total Criteria:** 20+
**Checkable via:** `just public-api-check`, `grep` patterns, `wc -l`, `cargo check`
