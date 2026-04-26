# Acceptance Criteria — Editor Intelligence Scorecard PR 1: Hover Correctness (#4066)

## Behavioral Assertions (Grid-Complete)

This is **PR 1 of 4**. It implements hover correctness measurement only (see context for sequencing).

- [ ] **Hover fixtures loaded from gold corpus** | `load_hover_gold_fixtures()` in `crates/perl-corpus/src/gold.rs` reads `test_corpus/gold/*/expected_hover.json` files without error | `crates/perl-corpus/src/gold.rs:hover_loader()` function | `cargo test -p perl-corpus -- load_hover_fixtures`
- [ ] **Hover non-null assertion** | Test harness detects when `hover_non_null` assertion at (line:4, char:8) receives null response and records FAIL | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs` assertion logic | Synthetic fixture with null response
- [ ] **Hover contains assertion** | Test harness detects when `hover_contains` assertion for needle "Parent" receives response without that needle and records FAIL | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs` assertion logic | Fixture with incorrect content
- [ ] **Hover absent assertion** | Test harness detects when `hover_absent` assertion for needle "wrong" receives response containing "wrong" and records FAIL | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs` assertion logic | Synthetic assertion
- [ ] **Hover null assertion** | Test harness detects when `hover_null` assertion receives non-null response and records FAIL | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs` assertion logic | Synthetic fixture
- [ ] **Pass rate calculation** | Test harness calculates pass rate as: assertions_passed / total_assertions; records 10/10 (100%) for all-pass case, 9/10 (90%) for one failure | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs:emit_scorecard()` | Multiple fixture runs
- [ ] **Scorecard output format** | Test harness emits summary line: "Hover gold corpus: X/Y assertions passed (Z%)" plus per-assertion failures (if any) | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs:emit_scorecard()` | Verify stdout under `--nocapture`
- [ ] **10 hover fixtures pass** | All 10 fixtures in `test_corpus/gold/*/` with `expected_hover.json` pass their assertions: method_inheritance, use_constant, imported_sub, builtin_func, scalar_var, our_variable, lexical_variable, hash_var, array_var, package_name | `test_corpus/gold/<fixture>/expected_hover.json` schema compliance | `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard -- --nocapture`
- [ ] **Method inheritance regression anchor** | Fixture `test_corpus/gold/method_inheritance/expected_hover.json` includes assertion at `$obj->bar()` call expecting content to contain parent class name (anchor for #4077 fix regression guard) | `test_corpus/gold/method_inheritance/` fixture with expected_hover.json | Test passes when #4077 is merged, would fail if #4077 regression occurs
- [ ] **Clippy clean** | `cargo clippy -p perl-lsp-rs` produces zero new warnings | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs` follows Rust style | `cargo clippy -p perl-lsp-rs -- -D warnings` exit 0
- [ ] **Status page created** | `docs/project/status/editor.md` exists with hover correctness results table | `docs/project/status/editor.md` new file with <!-- BEGIN: HOVER_SCORECARD --> markers | File exists with correct structure
- [ ] **CSV metrics exported** | Test harness exports per-assertion results to `target/editor_intelligence_metrics.json` for CI artifact upload | `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs:save_metrics()` | Verify file format and schema

## Structural Assertions (Non-Grid)

- [ ] `test_corpus/gold/` directory structure exists with 10 subdirectories (one per fixture)
- [ ] Each fixture dir contains: `fixture.pl`, `expected_hover.json` (and optionally `expected_diagnostics.json` from #4065 if available)
- [ ] `expected_hover.json` schema validated: `kind` (enum: hover_non_null, hover_contains, hover_absent, hover_null), `line`, `character`, optional `needle`, `rationale`
- [ ] `crates/perl-corpus/src/gold.rs` exports `load_hover_gold_fixtures()` returning `Vec<HoverGoldFixture>`
- [ ] `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs` uses LSP test harness (existing `LspServer` + `send_request()` from `common/mod.rs`)
- [ ] No unwrap/expect/panic in new code (error propagation via Result<T>)

## Gates (Pre-Verify Checklist)

- `ls test_corpus/gold/ | wc -l` returns ≥10 (fixture dirs exist)
- `cargo build -p perl-corpus` compiles without error
- `cargo build -p perl-lsp-rs` compiles without error
- `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard` passes all 10 assertions
- `cargo test -p perl-corpus -- load_hover_fixtures` passes
- `cargo clippy -p perl-lsp-rs -- -D warnings` exit 0
- `ls docs/project/status/editor.md` exists
- `ls target/editor_intelligence_metrics.json` exists (created by test harness)

## Context

> This is PR 1 of a 4-PR sequence for the editor intelligence scorecard. Each PR adds a new feature (hover, goto, completion, latency). This PR is scoped to hover correctness only.

> Depends on #4065 (diagnostics scorecard) landing first, which creates `test_corpus/gold/` directory structure and `crates/perl-corpus/src/gold.rs` fixture loader. This PR reuses that loader and adds hover-specific sidecar files.

> If #4065 does not land before this builder starts, the builder must create `test_corpus/gold/` themselves and coordinate to avoid Cargo.toml rebase conflicts (use separate issue for the loader).

> The hover assertions are binary (pass/fail). Top-1/top-5 relevance scoring (completion) comes in PR 3. Latency measurement (cold/warm/incremental) comes in PR 4.

> Regression guard: method_inheritance fixture is anchored to #4077 (inherited method resolution). If #4077 regresses in the future, this fixture will catch it.
