# Specs — work-1efc01c2

## Feature / Behavior

Formalize lint enforcement for `println!`/`eprintln!`/`dbg!` in `perl-lsp-launcher` library code, completing the pattern that PR #2446 established but left incomplete for this crate. The single intentional exception (the startup banner) is explicitly annotated.

## Summary

Add `#![deny(clippy::print_stderr, clippy::print_stdout)]` to `crates/perl-lsp-launcher/src/lib.rs` and add `#[allow(clippy::print_stderr)]` to the `startup_banner` function. Close GitHub issue #3224 as substantially resolved.

## Background

GitHub issue #3224 reported 210 debug print statements in library code. Investigation (research + verification agents) confirms that PR #2446 already migrated 209 of them. The one remaining genuine library-code print (`perl-lsp-launcher/src/lib.rs:779`) is intentional because it fires before the tracing subscriber is configured.

The issue's own scope exclusions (test code, `#[cfg(debug_assertions)]`, doc comments, CLI binaries) account for all other findings.

## Acceptance Criteria

1. **`perl-lsp-launcher/src/lib.rs` has lint enforcement.** The crate's lib.rs contains:
   - `#![deny(clippy::print_stderr, clippy::print_stdout)]`
   - `#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]`
   - Both directives placed after the crate's existing doc comment and `#![deny(unsafe_code)]` line, following the pattern established in `perl-lsp/src/lib.rs` and `perl-dap/src/lib.rs`.

2. **`startup_banner` function is explicitly exempted.** The function at line 775 has:
   - `#[allow(clippy::print_stderr)]` annotation above `pub fn startup_banner(...)`
   - The existing doc comment (lines 770–774) remains unchanged, explaining why the exception exists.

3. **`cargo clippy --workspace` passes.** Running `cargo clippy --workspace -D warnings` in the repo root produces no new lint errors introduced by these changes.

4. **Issue #3224 is closed** as substantially resolved, referencing PR #2446 and the lint enforcement gap that this ADR addresses.

## Non-Goals

- This spec does **not** migrate any additional print statements — the mechanical migration was already completed in PR #2446.
- This spec does **not** modify test code, `#[cfg(test)]` blocks, `#[cfg(bench)]` blocks, `#[cfg(debug_assertions)]` blocks, or doc comments.
- This spec does **not** add lint enforcement to CLI binaries (`crates/perl-ci-hygiene/`, `crates/perl-lsp/src/cli.rs`).
- This spec does **not** modify `perl-ts-advanced-parsers` or any other crate not mentioned in issue #3224's scope.

## Dependencies

- `clippy` — the lint is already active in sibling crates; no new toolchain requirements
- `tracing` — already a dependency of `perl-lsp-launcher` (for `tracing_subscriber`)
- No new dependencies, Cargo.lock changes, or build artifacts

## Files Modified

| File | Change |
|---|---|
| `crates/perl-lsp-launcher/src/lib.rs` | Add lint enforcement directives (2 lines after line 8); add `#[allow(clippy::print_stderr)]` to `startup_banner` function (line 775) |

## Verification

Run:
```bash
cd /home/hermes/repos/perl-lsp
cargo clippy --workspace -D warnings 2>&1 | grep -E "(print_stderr|print_stdout)"
```
Expected: No output (no print-stderr/print_stdout lint errors in library code after the allow annotation on `startup_banner`).
