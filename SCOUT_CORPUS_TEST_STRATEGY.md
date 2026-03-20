# Scout Report: Corpus Coverage Locking Test Strategy

**Date:** 2026-03-19
**Objective:** Explore perl-lsp's test infrastructure to lock corpus coverage gains and prevent regression.
**Status:** Complete with recommendations

---

## Executive Summary

The perl-lsp codebase has **strong foundations** for corpus coverage control:
- **Baseline system** (`parser-corpus-baseline.json`): Tracks full system Perl corpus (7095 files, 72.6% clean)
- **Ratcheting manifest** (`cpan-corpus-manifest.txt`): 1849 CPAN modules locked as "must stay clean"
- **Strict enforcement**: `just cpan-corpus-check` fails CI if any manifest module regresses
- **Per-file tests**: 1994 lines across 16 CPAN pattern test suites (`cpan_*.rs`)

**What's missing:**
1. **Per-bucket golden tests**: No tests assert specific error constructs (e.g., "use eval { ... }" must not produce `unexpected_rbrace_expr`)
2. **Corpus file extraction**: No automation to convert corpus errors into minimal test cases
3. **Parse-result snapshots**: No diff-based regression detection for individual module parse outputs
4. **Test file coverage**: No tests for `.t` (test) files or `.pl` (script) files; only `.pm` (modules)
5. **Construct-level assertions**: No "this Perl construct must parse clean" tests at parser semantic level

---

## 1. Current Test Infrastructure

### A. Baseline & Ratchet System

**File:** `xtask/src/tasks/cpan_corpus.rs` (603 lines)

**How it works:**
1. **sweep**: Scans installed `.pm` files, parses each with perl-parser v3, collects error buckets
2. **ratchet**: Runs sweep, extracts clean files, appends new modules to manifest
3. **check**: Re-scans manifest modules, enforces **strict zero-error policy** (line 352-380)
   - Returns violations if ANY manifest module has errors
   - Fails CI immediately (used in `just ci-gate`)

**Key insight:** Ratchet is **one-way only** — modules never removed, only added. This prevents accidental regression coverage loss.

**Manifest format** (`.ci/cpan-corpus-manifest.txt`, 1849 modules):
```
# One module name per line. Lines starting with # are comments.
# Added by ratchet on 2026-03-17T22:04:21...
Any::URI::Escape
App::Cmd::ArgProcessor
...
```

**Baseline structure** (`.ci/cpan-corpus-baseline.json`):
```json
{
  "schema_version": "1.1.0",
  "total_files": 7095,
  "clean_files": 5139,      // 72.6%
  "files_with_errors": 1908,
  "total_error_nodes": 28383,
  "first_error_buckets": {
    "unexpected_token_in_expr": 706,     // Largest bucket
    "unclosed_paren_identifier": 319,
    "unexpected_arrow_expr": 145,
    ...
  }
}
```

**Enforcement:** `just common-corpus-check`
- Runs parser-corpus-sweep against **common files** (tree-sitter test corpus)
- Enforces **strict policy**: 0 errors allowed
- Used in PR gate; blocks merge if violated

### B. Per-File Corpus Tests

**16 test suites** (1994 lines total) in `crates/perl-parser-core/tests/`:
- `cpan_module_patterns.rs` (257 lines): `use strict`, `use parent`, imports
- `cpan_data_structures.rs` (95 lines): hashes, arrays, refs
- `cpan_oo_patterns.rs` (128 lines): `bless`, package variables
- `cpan_real_world_programs.rs` (165 lines): scripts from real CPAN dists
- `cpan_moose_moo.rs` (160 lines): Moose/Moo OO frameworks
- `cpan_misc_idioms.rs` (360 lines): common Perl idioms (largest)
- `cpan_try_tiny.rs` (128 lines): error handling patterns
- `cpan_regex_patterns.rs` (65 lines): regex literals and ops
- And 8 more...

**Pattern:** Each test file directly calls `assert_clean_parse()` on curated code snippets.

**Helper library** (`cpan_test_helpers/mod.rs`, 92 lines):
```rust
/// Assert that a parsed AST has no Error / Missing* nodes.
pub fn assert_clean_parse(source: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // Check for error sentinels: (error, (Error, (missing_*, ...
    for marker in &error_markers { ... }
}

/// Assert that parsed AST contains an Error node with given substring.
pub fn assert_has_error(source: &str, needle: &str) { ... }
```

**Gap:** Tests are **hand-curated**, not extracted from corpus. No automation to:
- Parse failing corpus files
- Extract the minimal failing construct
- Generate a test case

### C. Error Bucket Tracking

**File:** `xtask/src/tasks/parser_corpus_sweep.rs` (lines 33-84)

**Semantic bucket mapping** (first-match wins):
```rust
const SEMANTIC_BUCKETS: &[(&str, &str)] = &[
    ("catastrophic backtracking", "catastrophic_backtracking"),
    ("Expected variable, found", "expected_variable"),
    ("expected expression, found '=>'", "unexpected_fat_arrow_expr"),
    ("expected expression, found", "unexpected_token_in_expr"),  // Catch-all
    ("expected '}'", "unclosed_brace"),
    ...
];
```

**Current baseline buckets** (28 named buckets):
- 706 `unexpected_token_in_expr` (catch-all, too broad)
- 319 `unclosed_paren_identifier`
- 145 `unexpected_arrow_expr`
- 134 `unclosed_paren`
- 106 `unclosed_paren` (above)
- Plus 23 more...

**Problem:** Buckets are **error message aggregates**, not semantic categories. A bucket like `unexpected_token_in_expr` (146 files) lacks sub-categorization.

---

## 2. Per-Bucket Regression Tests

**Current state:** ❌ **None exist**

The codebase does **not** assert specific constructs parse cleanly at the bucket level. For example:
- No test: "use eval { ... } must not produce `unexpected_rbrace_expr`"
- No test: "fat arrow in subroutine args must parse clean"
- No test: "method chaining with `->` must not produce `unexpected_arrow_expr`"

**Why this matters:** A parser change can inadvertently break a previously-clean category without triggering regression on the manifest itself. For example:
- Merge PR #2206 (complex paren args fix)
- All 134 files still parse
- But a new construct now triggers `unexpected_comma_expr` in edge cases
- Corpus clean rate stays ~72%, so baseline passes

**Solution:** Create per-bucket **construct-level tests** that verify specific Perl idioms parse cleanly.

---

## 3. Missing Test Strategies

### A. Automatic Test Case Extraction from Corpus

**Idea:** For each file with errors in corpus:
1. Parse the file
2. Walk AST to find first Error node
3. Extract minimal code snippet around error location
4. Generate a unit test with `assert_has_error(snippet, expected_bucket)`

**Implementation cost:** Moderate (200-300 lines)
- Add function to `parser_corpus_sweep.rs`: `extract_error_snippet(path, error_node) -> String`
- Add code generation function: `generate_test_from_snippet(module_name, snippet, bucket) -> String`
- Add post-sweep hook to write generated tests to `crates/perl-parser-core/tests/generated_corpus_*.rs`

**Benefit:**
- **Auto-keeps tests in sync** with corpus errors
- **Coverage explosion**: 706 `unexpected_token_in_expr` files → 706 targeted test cases
- Eliminates hand-curation bottleneck

**Estimated effort:** 20-30 hours (tool-building, test framework integration, debug)

### B. Per-Module Parse Result Snapshots

**Idea:** Store `.json` snapshot of each manifest module's parse result (clean/error count, first bucket).

**File:** `.ci/cpan-corpus-module-snapshots.jsonl` (one JSON object per line)
```json
{"module": "Any::URI::Escape", "status": "clean", "errors": 0, "bucket": null}
{"module": "App::Cmd::ArgProcessor", "status": "clean", "errors": 0, "bucket": null}
{"module": "Broken::Module", "status": "errors", "errors": 3, "bucket": "unexpected_rbrace_expr"}
```

**Enforcement:**
```bash
just cpan-corpus-check --snapshot-diff
```
Compares new snapshots against committed ones; flags any module that:
- Changed status (clean → error or vice versa)
- Increased error count
- Changed first error bucket

**Benefit:**
- **Granular regression detection** (module-level, not aggregate)
- **Diff review**: "Module X regressed from clean to 2 errors in bucket Y"
- **Root cause tracing**: Parser change X causes regression in Y modules

**Estimated effort:** 10-15 hours (snapshot generation, diff logic, CI integration)

### C. Golden-File Parse Output Tests

**Idea:** For key manifesto modules, commit `.golden` files containing expected AST structure.

**Files:**
```
.ci/corpus-golden/
  ├── Any::URI::Escape.golden      # AST nodes for this module
  ├── Moose.golden
  └── ...
```

**Test:** `cargo test -- golden_corpus`
```rust
#[test]
fn test_golden_any_uri_escape() {
    let path = "target/cpan-corpus/lib/perl5/Any/URI/Escape.pm";
    let ast = parse_file(path);
    let actual = ast.to_sexp();
    let expected = fs::read_to_string(".ci/corpus-golden/Any::URI::Escape.golden");
    assert_eq!(actual, expected, "AST mismatch for module");
}
```

**Benefit:**
- **Structural regression detection**: Catches AST changes even if error count stays same
- **Explicit expectations**: Commits what "correct" looks like for key modules
- Works for both clean and error files (error buckets are part of AST)

**Limitation:** Golden files are brittle; parser refactors require mass updates.

**Estimated effort:** 15-20 hours (infrastructure, golden file generation, update tooling)

### D. Per-Construct Semantic Tests

**Idea:** Add tests for specific Perl constructs that were previously problematic.

**Pattern:** In issue tracking, label problematic constructs with `#CPAN` tag, then test them:

```rust
mod cpan_construct_regressions {
    use cpan_test_helpers::*;

    // From issue #2223 (use eval { ... } with RBRACE)
    #[test]
    fn use_eval_with_block_must_parse_clean() {
        let code = "eval { print 'hello' };";
        assert_clean_parse(code);
    }

    // From issue #2202 (fat arrow in complex args)
    #[test]
    fn fat_arrow_in_nested_paren_args() {
        let code = "func(a => { b => $c }, d => $e);";
        assert_clean_parse(code);
    }

    // From issue #2206 (complex paren args, unexpected_comma)
    #[test]
    fn complex_paren_args_with_trailing_comma() {
        let code = "foo(bar($x,), baz($y,));";
        assert_clean_parse(code);
    }
}
```

**Benefit:**
- **Targeted regression prevention**: Locks down fixes
- **Low overhead**: Just curate the hard cases, not all 7095 files
- **Traceable**: Each test links to issue/PR where fix was validated

**Estimated effort:** 5-10 hours per release cycle (curate problematic constructs)

---

## 4. Preventing Regression

### Current Mechanism

**`just cpan-corpus-check`** (enforced in `ci-gate`):
1. Runs sweep against manifest modules
2. Enforces `enforce_strict_clean()` (lines 352-380 in parser_corpus_sweep.rs)
3. Fails if ANY manifest module has errors
4. Fails CI, blocks merge

**How quickly is regression caught?**
- **Merge time**: Immediately (gate blocks PR)
- **Local dev**: Only if dev runs `just cpan-corpus-check` (not automatic)
- **CI**: Tested post-merge in nightly (`ci-full`) if not in PR gate

**Gaps:**
- ❌ No per-module diff (only aggregate stats)
- ❌ No bucket-level tracking (only 0 vs >0 errors)
- ❌ Manual inspection needed to root-cause regression

### Recommended Additions

**Tier 1 (Critical, effort: 10-15 hours):**
1. **Module-level snapshots** (see B above)
   - Store: `.ci/cpan-corpus-module-snapshots.jsonl`
   - Report: Module X went from clean to N errors, bucket Y
   - Enforce: Automated diff in CI gate

**Tier 2 (High value, effort: 20-30 hours):**
2. **Auto-extracted construct tests** (see A above)
   - Converts 706 catch-all bucket files into 706 specific test cases
   - Tests run locally; catch regressions before merge

**Tier 3 (Polish, effort: 5-10 hours per cycle):**
3. **Per-issue construct tests** (see D above)
   - Curate high-impact fixes
   - Lock down against re-regression

---

## 5. Expanding Coverage Tests

### Current Coverage

**What's tested:**
- ✅ `.pm` files only (modules)
- ✅ Top 1000 CPAN distributions by reverse dependency count
- ✅ System Perl installation (`/usr/share/perl`, `/usr/lib/perl5`)
- ✅ 1849 modules locked in strict manifest

**What's NOT tested:**
- ❌ `.t` files (test scripts within CPAN distributions)
- ❌ `.pl` scripts (standalone programs)
- ❌ `.xs` files (XS binding code — should be ignored)
- ❌ Distributions below top 1000 (long tail of ~500K CPAN dists)
- ❌ Private/internal Perl code (only public CPAN)

### Recommendations

#### A. Add `.t` Test File Coverage

**Effort:** Low (5 hours)

**Implementation:**
```bash
# Modify cpan_corpus.rs to also scan for .t files
let t_files = discover_files_with_ext(&corpus_roots, "t");
let pm_files = discover_files_with_ext(&corpus_roots, "pm");
let all_files = [&pm_files, &t_files].concat();
```

**Benefit:** Test files often use more exotic Perl syntax than modules.

#### B. Add `.pl` Script Coverage

**Effort:** Low-Moderate (8-10 hours)

**Implementation:**
1. Add heuristic: files with `#!/usr/bin/perl` or matching `bin/` dirs
2. Parse them as complete programs (not just module-level)
3. Track separately in baseline (scripts often have more errors)

**Benefit:** Scripts use different patterns (main block, ARGV processing).

#### C. Extend Beyond Top 1000

**Effort:** Moderate (15-20 hours infrastructure, then 2 weeks of corpus time)

**Options:**
1. **Tier 2 (next 1000-5000)**: Add distributions #1001-5000 by river score
2. **Tier 3 (long tail)**: Sample randomly from remaining 500K+ distributions
3. **Focus areas**: Search for distributions that depend on problematic features (e.g., use of `eval BLOCK`, regex modifiers)

**Trade-off:** More coverage vs. slower sweep times.

---

## 6. Test Strategy Summary & Recommendations

| Strategy | Effort | Impact | Priority | Estimated Time |
|----------|--------|--------|----------|-----------------|
| **Module snapshots** (diff-based regression) | 10-15h | High (catches module-level regressions) | Tier 1 | 2-3 days |
| **Auto-extracted construct tests** (706→706 tests) | 20-30h | Very High (explosive coverage growth) | Tier 1 | 3-5 days |
| **Per-issue construct tests** (curated) | 5-10h/cycle | High (locks down hard cases) | Tier 2 | 4-8 hours |
| **Golden-file snapshots** (AST diff) | 15-20h | Medium (brittle on refactors) | Tier 3 | 2-3 days |
| **`.t` file coverage** (test scripts) | 5h | Medium (tests are often simpler) | Tier 2 | 1 day |
| **`.pl` script coverage** | 8-10h | Medium (scripts different patterns) | Tier 2 | 1-2 days |
| **Long-tail coverage** (Tier 2-3 CPAN) | 15-20h + 2 weeks | Low-Medium (finds rare edge cases) | Tier 3 | 3-4 weeks |

### Phased Rollout (Recommended)

**Phase 1 (Weeks 1-2, ~40 hours):**
- ✅ Module snapshots + diff gate (Tier 1)
- ✅ Auto-extracted construct tests (Tier 1)
- ✅ Per-issue construct tests for current 5 open issues (Tier 2)

**Outcome:** Catch module-level regressions immediately, 706 new tests.

**Phase 2 (Weeks 3-4, ~20 hours):**
- ✅ Add `.t` and `.pl` file coverage
- ✅ Golden-file infrastructure (optional polish)

**Outcome:** Test coverage expanded to scripts, more realistic parse patterns.

**Phase 3 (Weeks 5+, ongoing):**
- ✅ Curate per-issue construct tests as issues are closed
- ✅ Explore long-tail CPAN coverage (time-permitting)

**Outcome:** Sustained regression prevention, issue-driven test growth.

---

## Files & Paths

**Key implementation files:**
- `xtask/src/tasks/cpan_corpus.rs` — Ratchet & manifest logic
- `xtask/src/tasks/parser_corpus_sweep.rs` — Sweep engine & error buckets
- `crates/perl-parser-core/tests/cpan_test_helpers/mod.rs` — Test helpers
- `crates/perl-parser-core/tests/cpan_*.rs` — 16 test suites (1994 lines)
- `.ci/cpan-corpus-manifest.txt` — 1849 locked modules
- `.ci/cpan-corpus-baseline.json` — Baseline stats
- `.ci/parser-corpus-baseline.json` — System Perl stats
- `.ci/common-corpus-manifest.txt` — Strict common-files manifest

**New files to create:**
- `.ci/cpan-corpus-module-snapshots.jsonl` — Per-module parse results
- `crates/perl-parser-core/tests/generated_corpus_*.rs` — Auto-extracted tests
- `.ci/corpus-golden/` — Golden-file snapshots (optional)

---

## Conclusion

**Perl-lsp has a solid foundation** for corpus-driven regression prevention:
- Ratcheting manifest (1849 modules locked)
- Strict CI enforcement (blocks on ANY regression)
- 16 curated CPAN test suites (1994 lines)

**To lock gains and prevent regression, prioritize:**
1. **Module-level snapshots** (10-15h) — Catch regression immediately
2. **Auto-extracted tests** (20-30h) — Convert error files into tests
3. **Per-issue curation** (5-10h/cycle) — Lock down hard fixes

This 3-pronged approach scales from 1849 locked modules to 7095+ actively-tested files.

