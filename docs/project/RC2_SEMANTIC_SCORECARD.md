# 0.13.2 Semantic Scorecard

The semantic spine landed as the baseline for the 0.13.2 expansion train. Its
proof rail is split into two deterministic xtask surfaces:

```bash
cargo xtask semantic-scorecard
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare
cargo xtask semantic-shadow-compare --check
```

`semantic-scorecard` indexes the committed semantic fixture corpus and writes:

- `docs/project/status/semantic_scorecard.json`
- `docs/project/status/semantic_scorecard.md`

`semantic-shadow-compare` writes deterministic old-path/new-path receipt shapes
for semantic provider proof:

- `docs/project/status/semantic_shadow_compare.json`
- `docs/project/status/semantic_shadow_compare.md`

`--check` recomputes the same payloads and fails when the committed artifacts are stale.

## Release-Readiness Rows

The scorecard tracks release-readiness rows reviewers need for semantic UX and
provider behavior:

- `semantic_fact_counts_nonzero`
- `visible_symbols_fixture_pass_rate`
- `definition_shadow_regressions`
- `reference_shadow_regressions`
- `completion_import_fixture_pass_rate`
- `undefined_symbol_false_positive_fixture_rate`
- `rename_unsafe_edit_count`
- `safe_delete_blocker_fixture_pass_rate`

Provider cutover should stay conservative: dynamic, unavailable, or ambiguous cases must keep returning fallback, unavailable, warning, or blocker results rather than false precision.

Shadow compare includes both release-readiness receipts and schema fixture
receipts. Release-readiness rows count only provider-gating receipts; schema
fixture receipts exist to keep `same`, `improved`, `regression`, `ambiguous`,
and `unavailable` output shapes deterministic.

## Slow Analyzer Fuzz

The local PR gate is the targeted analyzer lib/property/integration set. Full `perl-semantic-analyzer` package tests may include long-running fuzz-style coverage such as `semantic_pipeline_fuzz_tests`; keep that as a slow/nightly proof lane unless a reviewer explicitly asks for it locally.
