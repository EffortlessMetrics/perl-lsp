# Implementation Checklist — Editor Intelligence Scorecard PR 1: Hover Correctness (#4066)

## Pre-Check: #4065 Dependency
**Status check**: Before starting, verify that #4065 (diagnostics scorecard) has created `test_corpus/gold/` and `crates/perl-corpus/src/gold.rs`.

**If #4065 not landed**: Builder must create these themselves:
- `test_corpus/gold/` directory
- `crates/perl-corpus/src/gold.rs` with `load_hover_gold_fixtures()` function
- Cargo.toml entry for perl-corpus crate (coordinate with maintainer to avoid race)

**If #4065 landed**: Skip to Step 1.

---

## Step 1: Create Hover Fixtures
**Files**: `test_corpus/gold/<fixture-name>/` (10 directories total)
**Action**: CREATE fixture directories with `.pl` files and `expected_hover.json` sidecars
**Dependencies**: `test_corpus/gold/` directory exists (from #4065 or created by builder)

**Fixture list** (~10 fixtures):

| Fixture dir | `.pl` content | Hover assertion (line:char) | Expected assertion |
|---|---|---|---|
| `method_inheritance` | Method call on inherited object (anchors #4077) | `4:8` on `$obj->bar()` | `hover_contains`: parent class name |
| `use_constant` | Constant declaration and usage | Position of constant use | `hover_non_null`: shows constant value |
| `imported_sub` | Sub imported via `use Module qw(sub)` | Position of imported call | `hover_contains`: "Imported from Module::Name" |
| `builtin_func` | Perl builtin like `push` or `join` | Position of builtin | `hover_contains`: perldoc description |
| `scalar_var` | Scalar variable `$x` declaration and use | Position of `$x` in expression | `hover_contains`: "Scalar variable" |
| `our_variable` | Package variable `our $global` | Position of use | `hover_contains`: package-scoped indicator |
| `lexical_variable` | Lexical `my $x` | Position of use | `hover_non_null`: scope info |
| `hash_var` | Hash variable `%h` | Position of `%h` | `hover_contains`: "Hash variable" |
| `array_var` | Array variable `@a` | Position of `@a` | `hover_contains`: "Array variable" |
| `package_name` | Package qualified call `Foo::bar()` | Position of package name | `hover_non_null`: package documentation |

**Each fixture dir structure**:
```
test_corpus/gold/method_inheritance/
  fixture.pl
  expected_hover.json
  expected_diagnostics.json (optional, from #4065 if available)
```

**example fixture.pl** (~10 lines):
```perl
package Parent;
sub bar { "parent implementation" }

package Child;
our @ISA = ('Parent');

package main;
my $obj = bless {}, 'Child';
$obj->bar();  # Line 9, char 8: hover should show parent class name
```

**example expected_hover.json** (~15 lines):
```json
{
  "version": 1,
  "fixture": "method_inheritance",
  "assertions": [
    {
      "kind": "hover_contains",
      "line": 9,
      "character": 8,
      "needle": "Parent",
      "rationale": "inherited method hover must show origin class"
    },
    {
      "kind": "hover_non_null",
      "line": 9,
      "character": 8,
      "rationale": "hover must return something for an inherited method"
    }
  ]
}
```

**Verify**: `ls test_corpus/gold/*/expected_hover.json | wc -l` returns 10

---

## Step 2: Add Hover Loader Function
**File**: `crates/perl-corpus/src/gold.rs` (or create if #4065 not landed)
**Action**: ADD (or CREATE if missing) `load_hover_gold_fixtures()` function
**Dependencies**: Step 1 complete
**Signature**:
```rust
pub fn load_hover_gold_fixtures() -> Result<Vec<HoverGoldFixture>> {
    // 1. Iterate test_corpus/gold/*/
    // 2. For each dir, read expected_hover.json (skip if absent)
    // 3. Deserialize into HoverGoldFixture struct
    // 4. Load fixture.pl content
    // 5. Collect into Vec, return
}

pub struct HoverGoldFixture {
    pub name: String,
    pub pl_content: String,
    pub assertions: Vec<HoverAssertion>,
}

#[derive(Serialize, Deserialize)]
pub struct HoverAssertion {
    pub kind: HoverAssertionKind,
    pub line: u32,
    pub character: u32,
    #[serde(default)]
    pub needle: Option<String>,
    pub rationale: String,
}

pub enum HoverAssertionKind {
    HoverNonNull,
    HoverContains,
    HoverAbsent,
    HoverNull,
}
```

**Error handling**: Silently skip fixtures without `expected_hover.json` (not all fixtures have hover tests)

**Verify**: `cargo build -p perl-corpus`, `cargo test -p perl-corpus -- load_hover_fixtures`

---

## Step 3: Create Test Harness
**File**: `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs`
**Action**: CREATE — new test file (~200 lines)
**Dependencies**: Steps 1-2 complete, #4065 or builder has created `test_corpus/gold/`

**Structure**:
```rust
#[cfg(test)]
mod hover_correctness {
    use perl_corpus::gold::*;
    use perl_lsp_rs::tests::common::*;

    #[test]
    fn hover_gold_corpus() -> Result<()> {
        let fixtures = load_hover_gold_fixtures()?;
        let mut server = LspServer::start_lsp_server()?;
        let mut results = Vec::new();
        
        for fixture in fixtures {
            // 1. Send didOpen with fixture.pl_content
            // 2. For each assertion in fixture.assertions:
            //    a. Send textDocument/hover request at (line, char)
            //    b. Get response
            //    c. Check assertion (non_null, contains, absent, null)
            //    d. Record pass/fail
            // 3. Collect results into Vec<FixtureResult>
        }
        
        // 4. Emit scorecard summary to stdout
        emit_scorecard(&results);
        
        // 5. Save metrics to target/editor_intelligence_metrics.json
        save_metrics(&results)?;
        
        // 6. Assert all pass (fail test if any assertion failed)
        assert!(results.iter().all(|r| r.passed));
        
        Ok(())
    }

    fn emit_scorecard(results: &[FixtureResult]) {
        let total = results.iter().map(|r| r.assertions.len()).sum::<usize>();
        let passed = results.iter().filter(|r| r.passed).map(|r| r.assertions.len()).sum::<usize>();
        eprintln!("Hover gold corpus: {}/{} assertions passed ({}%)", 
            passed, total, (passed * 100) / total);
        
        for result in results {
            if !result.passed {
                for (assertion, failed) in result.assertions.iter().zip(result.failures.iter()) {
                    if *failed {
                        eprintln!("FAIL: [{}] {:?} at line:{} char:{}", 
                            result.fixture_name, assertion.kind, assertion.line, assertion.character);
                    }
                }
            }
        }
    }

    fn save_metrics(results: &[FixtureResult]) -> Result<()> {
        let metrics = serde_json::json!({
            "scorecard": "editor_intelligence",
            "phase": "hover_correctness",
            "timestamp": std::time::SystemTime::now(),
            "results": results,
        });
        std::fs::write("target/editor_intelligence_metrics.json", metrics.to_string())?;
        Ok(())
    }
}
```

**Implementation details**:
- Use existing LSP test harness from `crates/perl-lsp/tests/common/` (RUST_TEST_THREADS=2 handling already there)
- For each assertion at (line, char), send `textDocument/hover` request
- Extract hover response content (if non-null), check against assertion kind
- Record pass/fail for each assertion (not per-fixture, per-assertion)

**Verify**: `cargo build -p perl-lsp-rs`, `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard`

---

## Step 4: Create Status Page
**File**: `docs/project/status/editor.md`
**Action**: CREATE — new markdown file (~30 lines)
**Dependencies**: Steps 1-3 complete
**Content**:

```markdown
# Editor Intelligence Scorecard

## Hover Correctness

<!-- BEGIN: HOVER_SCORECARD -->
(Results will be inserted after test harness runs)
<!-- END: HOVER_SCORECARD -->

## Goto Definition

(Coming in PR 2)

## Completion Relevance

(Coming in PR 3)

## Latency Measurements

(Coming in PR 4)

## Metrics Details

Hover correctness is measured via a gold corpus of 10 representative fixtures covering:
- Method inheritance (regression guard for #4077)
- Symbol resolution (constants, imports, builtins)
- Variable types (scalar, array, hash, lexical, package)
- Package-qualified names

Each fixture includes assertions for expected hover behavior.
Results are binary: assertion passes or fails.
```

**Verify**: `ls docs/project/status/editor.md`

---

## Step 5: Wire Metrics Export
**File**: `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs`
**Action**: ADD metrics export to `target/editor_intelligence_metrics.json`
**Dependencies**: Step 3 complete
**Format**: JSON with schema:
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
    "passed_assertions": 19,
    "pass_rate": 0.95
  }
}
```

**Verify**: After test runs, `cat target/editor_intelligence_metrics.json | jq .`

---

## Step 6: Verify End-to-End
**File**: None (execution only)
**Action**: Run test suite and verify all outputs
**Dependencies**: Steps 1-5 complete
**Commands**:
```bash
# Build
cargo build -p perl-corpus
cargo build -p perl-lsp-rs

# Run test
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard -- --nocapture

# Check metrics export
cat target/editor_intelligence_metrics.json | jq .summary

# Check status page
cat docs/project/status/editor.md | grep "Hover Correctness"
```

**Verify**: All tests pass, metrics file created, status page has results

---

## Step 7: Clippy and Format
**File**: None (verification only)
**Action**: Lint and format check
**Dependencies**: Steps 1-6 complete
**Commands**:
```bash
cargo clippy -p perl-corpus -- -D warnings
cargo clippy -p perl-lsp-rs -- -D warnings
cargo xtask fmt
```

**Verify**: Exit code 0

---

## Summary

| File | Action | Lines | Step |
|------|--------|-------|------|
| `test_corpus/gold/<fixture>/fixture.pl` (10 files) | CREATE | ~100 total | 1 |
| `test_corpus/gold/<fixture>/expected_hover.json` (10 files) | CREATE | ~150 total | 1 |
| `crates/perl-corpus/src/gold.rs` | ADD hover loader | ~50 | 2 |
| `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs` | CREATE | ~200 | 3 |
| `docs/project/status/editor.md` | CREATE | 30 | 4 |
| Metrics export logic | ADD to harness | ~20 | 5 |

**Total scope**: ~550 lines across 6 files + 10 fixture directories

**Compilation gates** (verify at each step):
- Step 2: `cargo build -p perl-corpus`
- Step 3: `cargo build -p perl-lsp-rs`
- Step 5: (included in step 3)
- Step 6: Integration test execution
- Step 7: Clippy and format

**Builder next**: Implement Steps 1-7 in order (Step 1 may require coordination with #4065 or be done independently if #4065 has landed), verify all fixtures pass, commit and push.

---

## Sequencing Note: Future PRs

This is PR 1 of 4. Subsequent PRs build on this infrastructure:

- **PR 2 (Goto Definition)**: Reuses same harness, adds `expected_goto.json` sidecars, `GotoAssertion` struct
- **PR 3 (Completion Relevance)**: Reuses same harness, adds `expected_completion.json`, top-1/top-5 scoring
- **PR 4 (Latency)**: Reuses same harness, instruments with timing probes (cold/warm/incremental)

Each PR is independent and can land individually; they don't have to be sequenced.
