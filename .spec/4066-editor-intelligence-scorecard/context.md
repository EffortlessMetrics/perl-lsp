# Context — Editor Intelligence Scorecard PR 1: Hover Correctness (#4066)

## Problem Statement

Developers and maintainers need visibility into the quality of LSP editor intelligence features (hover, goto-definition, completion). These features are tested in integration tests but not tracked as a scorecard metric. A regression in inherited method resolution (#4077) or symbol import tracking (#3472) has no visible impact on a dashboard.

This scorecard makes those regressions detectable and prevents them from regressing silently.

## MVP Scope: PR 1 (Hover Correctness)

This spec implements **PR 1 of a 4-PR sequence**:
- **PR 1** (this spec): Hover correctness — binary pass/fail on 10 fixtures
- **PR 2**: Goto-definition exactness — binary pass/fail on 8 fixtures
- **PR 3**: Completion relevance — top-1 and top-5 ranking metrics
- **PR 4**: Latency profiling — cold/warm/incremental measurements

Each PR is independent (can land individually) but reuses the same harness infrastructure.

## Key Decisions

### 1. Gold Corpus Location and Format

**Location**: `test_corpus/gold/<fixture-name>/` (root level, per #4065 plan-review decision)

**Structure**: Each fixture dir contains:
- `fixture.pl` — Perl code to test
- `expected_hover.json` — Hover assertions (sidecar for this feature)
- `expected_diagnostics.json` — Diagnostic assertions (sidecar from #4065, if available)
- `expected_goto.json` — Goto assertions (sidecar from PR 2, will be added later)
- `expected_completion.json` — Completion assertions (sidecar from PR 3, will be added later)

**Benefit**: Single fixture file serves multiple scorecards. A PR that improves both hover and diagnostics increments both scorecards' metrics.

### 2. Hover Assertion Types

**Binary assertions only** (for PR 1):
- `hover_non_null` — response must have non-empty content (passes if not null)
- `hover_contains` — response content must include needle string (case-sensitive substring match)
- `hover_absent` — response content must NOT include needle string
- `hover_null` — response must be null (no hover available at this position)

**Not in PR 1** (defer to PR 3 / completion relevance):
- Ranked assertions (top-1 vs top-5) — these apply to completion, not hover

### 3. Regression Anchor: #4077

**Fixture**: `test_corpus/gold/method_inheritance/fixture.pl`
**Assertion**: At `$obj->bar()` call, hover content must contain parent class name (indicator that inherited method origin is shown)

**Why**: #4077 fixed a critical bug where inherited methods were not resolved correctly. This fixture becomes the permanent regression guard. If the bug is ever re-introduced, this fixture will fail.

**Precedent**: Existing test suite has `go_to_definition_cross_file_inherited_and_role_method_call_in_framework_workspace` (cross_file_goto_definition_tests.rs:1206) — PR 2 will convert this to a gold corpus fixture.

### 4. Test Harness Architecture

**Reuse existing LSP infrastructure**: Don't build new harness — use existing `LspServer`, `send_request()`, threading constraints from `crates/perl-lsp/tests/common/`.

**Threading**: RUST_TEST_THREADS=2 (LSP server serializes; parallel tests cause contention). Already documented in crates/perl-lsp/CLAUDE.md.

**Fixture loading**: Use `load_hover_gold_fixtures()` from `crates/perl-corpus/src/gold.rs` (either reuses #4065 loader or builder creates it).

**Per-assertion tracking**: Record pass/fail for each assertion individually (not per-fixture). A fixture with 2 assertions can have pass=1, fail=1.

### 5. Metrics Export Format

**Output file**: `target/editor_intelligence_metrics.json` (gitignored, uploaded as CI artifact)

**Schema**:
```json
{
  "scorecard": "editor_intelligence",
  "phase": "hover_correctness",
  "timestamp": "ISO-8601",
  "fixtures": [
    {
      "name": "method_inheritance",
      "total_assertions": 2,
      "passed_assertions": 2,
      "failures": []
    }
  ],
  "summary": {
    "total_assertions": 20,
    "passed_assertions": 20,
    "pass_rate": 1.0
  }
}
```

**Dashboard integration**: This file feeds into `docs/project/status/editor.md` via a post-test-run step (builder sets this up or ops team uses artifact for manual updates).

### 6. Dependency on #4065

**Critical question**: Has #4065 (diagnostics scorecard) created `test_corpus/gold/` and the loader?

**If #4065 landed**: Builder reuses the existing loader and adds hover sidecars.

**If #4065 NOT landed**: Builder has two options:
1. Create `test_corpus/gold/` and `crates/perl-corpus/src/gold.rs` themselves (coordinate with maintainer to avoid merge conflict race)
2. Wait for #4065 to land first (risk: sequencing delay)

**Recommended**: Assume #4065 lands first (it's in flight as of the plan-review). If it doesn't, the builder should escalate to orchestrator rather than create duplicate infrastructure.

## Alternatives Considered and Rejected

| Alternative | Why rejected |
|-------------|-------------|
| Inline `# expect:hover:contains=...` markers in fixture.pl | Fragile (comments are refactored/deleted), ambiguous (multiple assertions on same line), violates separation of concerns. Sidecar JSON is cleaner. |
| Single monolithic `expected.json` with mixed assertion types | Creates tight coupling between features. Separate sidecars allow independent feature addition. |
| Hard-code fixture data in the test | Loss of auditability, makes corpus invisible to developers, prevents reuse for other scorecards. |
| Use existing `test_corpus/` instead of `test_corpus/gold/` | #4065 decision: gold corpus is a curated set of high-value fixtures (hand-annotated). `test_corpus/` is auto-generated from CPAN. Different purposes, separate directories. |
| Top-1/top-5 ranking metrics in PR 1 | Requires manual labeling of expected completions (large effort). Defer to PR 3. PR 1 focuses on binary correctness (simpler). |
| Benchmark latency in PR 1 | Requires P50/P95 baseline and noise tolerance logic. Defer to PR 4. |

## Testing Strategy

**Unit tests** (in `cargo test -p perl-corpus`):
- Load fixtures from gold directory without error
- Deserialize JSON assertions correctly
- Skip fixtures without expected_hover.json (no error)

**Integration tests** (in `cargo test -p perl-lsp-rs`):
- All 10 hover fixtures pass their assertions
- Summary output shows 100% pass rate
- Metrics JSON exported correctly

**Regression tests** (permanent):
- method_inheritance fixture ensures #4077 doesn't regress
- All other fixtures are regression guards against future changes

## Coordination with #4065 (Diagnostics Scorecard)

#4065 creates `test_corpus/gold/` and the loader. This builder reuses it.

**Coupling point**: Both scorecards use the same fixture directory structure and loader. The first PR to land (either #4065 or #4066) creates the infrastructure; the second reuses it.

**Recommended sequencing**: Land #4065 first (diagnostics scorecard), then this builder uses the existing loader. If both land in parallel, coordinate to avoid:
- Duplicate directory creation (git merge conflict)
- Duplicate Cargo.toml entries (parse error)
- Duplicate loader functions (compilation error)

**No blocking**: If coordination fails, the builder can create the infrastructure themselves. It's simple enough that duplication is low-risk (search/replace in post-landing cleanup if needed).

## Verification Path

1. **Fixtures exist**: `ls test_corpus/gold/ | wc -l` ≥ 10
2. **Loader compiles**: `cargo build -p perl-corpus`
3. **Test harness compiles**: `cargo build -p perl-lsp-rs`
4. **Tests pass**: `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard`
5. **Metrics exported**: `ls target/editor_intelligence_metrics.json`
6. **Status page created**: `ls docs/project/status/editor.md`
7. **All assertions pass**: Test output shows "20/20 assertions passed (100%)"

## Scope Exclusions

- Does NOT implement goto-definition (PR 2)
- Does NOT implement completion scoring (PR 3)
- Does NOT implement latency measurement (PR 4)
- Does NOT add the metrics to the floor-enforcement gate (#4105)
- Does NOT modify the LSP server code itself (only tests the existing code)
- Does NOT change CI configuration (gold fixtures are just data)

## Downstream Impact

- **PR 2** (goto-definition) reuses the same harness, adds goto sidecars
- **PR 3** (completion) reuses the same harness, adds completion sidecars
- **PR 4** (latency) reuses the same harness, adds timing instrumentation
- **#4105** (metrics ratchet) will later add floor-enforcement if desired
- **Dashboard** (#4070 phase 2) will surface metrics alongside other health indicators

## Open Questions Resolved

- Q: Should hover assertions be per-fixture or per-assertion?
  A: Per-assertion. A fixture can have multiple assertions; tracking them separately reveals which specific assertions fail.

- Q: Should we hard-assert test failures or soft-assert?
  A: Hard-assert (test fails if any assertion fails). This is a regression guard; failures must be caught.

- Q: Can we reuse existing tests (e.g., lsp_hover_tests.rs)?
  A: Those are different: they test specific LSP functionality. Gold corpus is a curated baseline for scorecard measurement. Both can coexist.

- Q: What if a fixture is ambiguous (could test multiple features)?
  A: Create separate fixtures for each feature. Clarity is worth the duplication.

- Q: Can we test hover at multiple positions in one fixture?
  A: Yes. The fixture can have assertions at positions (line:4, char:8), (line:5, char:3), etc. Each assertion is tracked separately.

## Historical Context

#4077 (merged) fixed inherited method resolution. This scorecard's method_inheritance fixture is the permanent guard against that regression. The gold corpus pattern is now the standard way to add regression guards to the editor intelligence scorecard.

The scout report identified LSP testing infrastructure as already cheap enough (existing harness, ~5ms per request). The build is straightforward: load fixtures, send requests, check responses.
