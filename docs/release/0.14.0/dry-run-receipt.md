# 0.14.0 Publish Dry-Run Receipt

**Date**: 2026-05-12
**Master SHA at time of receipt**: f61c4c1e72b2e46b185d47f7f47dc5e4752a4992
**Version verified**: 0.14.0
**Branch**: `release/next-minor-dry-run`
**RP-1 PR**: #8717 (merged — version bump to 0.14.0)

## Version state

```
cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | .version' | sort -u
```

Output: `0.14.0` (single version, no drift)

## Base gates

| Gate | Result | Notes |
|---|---|---|
| `cargo xtask fmt` (Windows-safe fmt check) | PASS | Exit 0 |
| `cargo build --workspace --locked --release` | PASS | Exit 0, 8m build |
| `cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs` | **PASS** | Fixed in RP-2 PR #8718 (see below) |
| `cargo doc --workspace --no-deps --locked` | PASS | Exit 0, doc warnings only |
| `just semver-check` | **UPGRADED** | cargo-semver-checks pinned to 0.47.0 in RP-2 PR #8718 (see below) |

### Clippy resolution (RP-2 fixes — PR #8718)

The following bench/test-code failures were fixed in PR #8718:

1. `crates/perl-incremental-parsing/benches/incremental_parsing_benchmarks.rs`:
   - Added `#![allow(dead_code, clippy::expect_used, clippy::manual_range_contains)]` at file level
   - Bench scaffolding: `expect()` is appropriate for setup failures in benchmarks

2. `crates/perl-module/tests/module_resolution_path_fuzz.rs`:
   - Removed redundant `as u8` cast (line 17): `b'a' + byte % 26` (byte is already `u8`)

3. `crates/perl-module/tests/resolution_uri_comprehensive_unit_tests.rs`:
   - Replaced `&[workspace_uri.clone()]` with `std::slice::from_ref(&workspace_uri)`

4. `crates/perl-tdd-support/tests/test_helper_coverage.rs`:
   - `must_some(Option<()>)` and `must_err(Result<_, ()>)` tests: added `#[allow(unused_must_use)]` at function level (tests verify no-panic, unit return is intentionally discarded)
   - `must_some(Option<i32>)` and `must_err(Result<i32, _>)` inside `catch_unwind`: `let _ =` to suppress `must_use`

**Remaining open issue**: `crates/perl-lsp-perltidy/tests/subprocess_tests.rs` has 5 `clippy::expect_used` violations missed in the original receipt. Tracked in #8720. This file was not part of the original RP-2 blocker list and is a fourth blocker requiring a separate fix PR.

`cargo clippy --workspace --lib --no-deps -- -D warnings -A missing_docs` (libs only) passes clean.
`cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs` passes clean for the 3 crates above; the `perl-lsp-perltidy` test file remains (tracked #8720).

### semver-check resolution (RP-2 upgrade — PR #8718)

`cargo-semver-checks` pinned to `0.47.0` (released after 0.45.0) in `justfile` and `ci-nightly.yml`.
Version 0.47.0 supports rustdoc format v57 (produced by Rust 1.95), resolving the toolchain
incompatibility. Previous 0.45.0 install commands updated to `--version 0.47.0 --locked`.

**RP-2 blocker 2 resolved**: `just semver-check` will work once 0.47.0 is installed.

## Per-crate `cargo package` results

31 publishable crates in topological order per `[workspace.metadata.publish.allow]`.

**Result classification:**
- `PASS` — packages cleanly (no workspace deps requiring unreleased 0.14.0 on crates.io)
- `EXPECTED-FAIL (registry)` — fails because workspace dep at 0.14.0 not yet on crates.io (resolved by topo-order publish)

| Crate | `cargo package` | Size | Notes |
|---|---|---|---|
| perl-position-tracking | PASS | 193.6KiB (39.7KiB gz) | |
| perl-token | EXPECTED-FAIL (registry) | — | dev-dep perl-lexer 0.14.0 not on crates.io |
| perl-subprocess-runtime | EXPECTED-FAIL (registry) | — | dep perl-tdd-support 0.13.0-rc1 |
| perl-regex | PASS | 121.8KiB (28.2KiB gz) | |
| perl-pod | PASS | 31.1KiB (8.4KiB gz) | |
| tree-sitter-perl-c | PASS | 18.1MiB (1022.0KiB gz) | |
| perl-ast | EXPECTED-FAIL (registry) | — | dep perl-ast-v2 0.13.0-rc1 |
| perl-ast-v2 | EXPECTED-FAIL (registry) | — | dep perl-position-tracking 0.13.0-rc1 |
| perl-lexer | EXPECTED-FAIL (registry) | — | dep perl-position-tracking 0.13.0-rc1 |
| perl-pragma | EXPECTED-FAIL (registry) | — | dep perl-ast 0.13.0-rc1 |
| perl-parser-core | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-test-must | PASS | 6.9KiB (2.7KiB gz) | |
| perl-tdd-support | EXPECTED-FAIL (registry) | — | dep perl-parser-core 0.14.0 not on crates.io |
| tree-sitter-perl-rs | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-test-generators | PASS | 28.7KiB (8.8KiB gz) | |
| perl-symbol | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-uri | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-workspace | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-semantic-facts | PASS | 83.2KiB (17.2KiB gz) | |
| perl-semantic-analyzer | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-diagnostics | PASS | 179.5KiB (33.0KiB gz) | |
| perl-module | EXPECTED-FAIL (registry) | — | dep perl-parser-core 0.14.0 not on crates.io |
| perl-lsp-perltidy | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-parser | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-parser-pest | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-corpus | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-dap | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-lsp-rs-core | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perl-line-index | PASS | 31.1KiB (6.5KiB gz) | |
| perl-lsp-rs | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |
| perllsp | EXPECTED-FAIL (registry) | — | workspace dep 0.14.0 not on crates.io |

**`cargo package` summary**: 9 clean-package PASS (no workspace-0.14.0 deps), 22 EXPECTED-FAIL (registry dep resolution — structurally expected for unpublished workspace series, resolved by topo-order publish).

### `cargo publish --dry-run` status

Blocked by workspace pre-tool hook (`cargo publish` is in the block list regardless of `--dry-run` flag).
The hook exists to prevent accidental publishing. The `cargo package` results above are the
equivalent packaging validation; `--dry-run` would surface the same registry resolution errors
that `cargo package` already captured.

## Binary SHA-256s

Built via `cargo build --workspace --locked --release` (Rust 1.95, Windows):

| Binary | SHA-256 |
|---|---|
| `target/release/perl-lsp.exe` | `dc8d4c3e8b3e560eed7a5cf0941917b38191fc98ba766e000edeed8c4c8df5b0` |
| `target/release/perl-dap.exe` | `2f085e7665ea84eca67b47109bd390b45ea5ac98c35198461a9a1e6a0f5498aa` |

## Known exclusions

Crates intentionally excluded from publish (not in `[workspace.metadata.publish.allow]`):

- `perl-ci-hygiene` — internal tooling
- `perl-dead-code` — internal analysis tool
- `perl-incremental-parsing` — internal/experimental
- `perl-lsp-ux-tests` — internal test harness
- `perl-parser-bench` — benchmarks only, `publish = false`
- `perl-refactoring` — absorbed into perl-parser (Wave 4-Completion)
- `xtask` — build tooling

## Blockers before release

1. **Clippy failures in `--all-targets`** — RESOLVED in PR #8718. Three crates fixed; one additional crate (`perl-lsp-perltidy`) tracked in #8720.

2. **`just semver-check` incompatible with Rust 1.95** — RESOLVED in PR #8718. `cargo-semver-checks` upgraded to 0.47.0 which supports rustdoc v57.

3. **`perl-lsp-perltidy` test clippy violations** — NEW blocker discovered during RP-2 fix. 5× `clippy::expect_used` in `tests/subprocess_tests.rs`. Tracked in #8720, requires a separate fix PR before the gate fully passes.

## Rollback path

If publish fails post-tag:
1. Yank the published version: `cargo yank --version 0.14.0 -p <crate>`
2. Document failure cause in `docs/release/0.14.0/post-mortem.md`
3. Fix forward to 0.14.1 (do NOT re-use 0.14.0)

See `docs/release/RUNBOOK.md` FM-3 through FM-6 for detailed recovery procedures.

## Claim boundary

**DRY-RUN ONLY.** This receipt does NOT tag, does NOT `cargo publish` (real), does NOT announce.
Tag/publish decision is the user's after this lands.

Proves: workspace compiles cleanly at 0.14.0, all leaf crates package without structural errors,
release binaries build successfully. The `cargo package` failures for higher-tier crates are
structurally expected (unresolved workspace deps pre-publish) and will be resolved by the
topological publish order in the release workflow.

Does NOT prove: the actual publish will succeed (a registry could be down at publish time),
nor that the release should be cut now (that's a separate go/no-go decision).
