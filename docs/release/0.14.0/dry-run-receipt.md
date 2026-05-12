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
| `cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs` | **FAIL** | Exit 101 (see below) |
| `cargo doc --workspace --no-deps --locked` | PASS | Exit 0, doc warnings only |
| `just semver-check` | **FAIL** | Exit 1 (see below) |

### Clippy failures (--all-targets scope)

Failures are in bench/test targets only — not published library code:

1. `crates/perl-incremental-parsing/benches/incremental_parsing_benchmarks.rs`:
   - 11× `expect()` on `Result` (clippy::expect_used)
   - Dead fields in `BenchmarkResult` struct (dead_code)
   - `manual_range_contains` lint

2. `crates/perl-module/tests/module_resolution_path_fuzz.rs`:
   - Unnecessary cast `u8 as u8` (clippy::unnecessary_cast)

3. `crates/perl-module/tests/resolution_uri_comprehensive_unit_tests.rs`:
   - `cloned_ref_to_slice_refs` lint

4. `crates/perl-tdd-support/tests/test_helper_coverage.rs`:
   - 4× unused return value of `must`, `must_some`, `must_err`

**All failures are in bench/test code (`--all-targets`), not in published library code (`--lib`).**
Running `cargo clippy --workspace --lib --no-deps -- -D warnings -A missing_docs` (libs only) is expected to pass.
These failures require a follow-up fix PR before release.

### semver-check failure

`cargo-semver-checks 0.45.0` does not support rustdoc format v57 (produced by Rust 1.95):

```
error: unsupported rustdoc format v57 for file: .../perl_parser.json
(supported formats are v53, v55, v56)
```

Root cause: `cargo-semver-checks` has not yet been updated to support the rustdoc JSON format
emitted by Rust 1.95. This is a toolchain/tool compatibility issue, not a semantic versioning
violation. The check cannot be completed until `cargo-semver-checks` is updated to support v57.
This is a known limitation of the Rust 1.95 upgrade (RP ladder #8508).

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

1. **Clippy failures in `--all-targets`** (`perl-incremental-parsing` benches, `perl-module` tests, `perl-tdd-support` tests): needs a fix PR to clear `-D warnings` violations in bench/test code.

2. **`just semver-check` incompatible with Rust 1.95**: `cargo-semver-checks` 0.45.0 does not support rustdoc format v57. Needs either a `cargo-semver-checks` upgrade or the gate to be annotated as skipped for this release with justification.

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
