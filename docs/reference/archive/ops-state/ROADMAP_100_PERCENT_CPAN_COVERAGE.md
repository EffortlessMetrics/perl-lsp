# Path to 100% CPAN Corpus Coverage

**Date**: 2026-03-19
**Current State**: 72.1% (3139/4355 clean files)
**Target**: 100% coverage with documented "unfixable floor"
**Estimated Effort**: 8-12 weeks parallel execution, 20-30 weeks serial

---

## Executive Summary

The perl-lsp CPAN corpus is at 72.1% coverage (3139 clean files out of 4355 total). The remaining 1212 failing files (27.9%) are distributed across 33 error buckets. This roadmap breaks down:

1. **Current state analysis** — exact metrics and error distribution
2. **Fixable vs. unfixable assessment** — which errors can be fixed vs. which are inherent to static parsing
3. **Phased execution plan** — phases A-D targeting 90%, 95%, 98%, and "ceiling"
4. **Test strategy** — regression prevention and corpus integrity
5. **Corpus expansion** — growing beyond top-1000 distributions

### Key Finding

**Top 10 error buckets account for 74.8% of all failures (907/1212 files)**. Fixing just these 10 categories should reach ~85-87% coverage. Diminishing returns begin after bucket #5.

---

## Phase 1: Understand Current State

### 1.1 Baseline Metrics (as of 2026-03-18)

| Metric | Value |
|--------|-------|
| Total files | 4,355 |
| Clean files | 3,139 (72.1%) |
| Files with errors | 1,212 (27.8%) |
| Unreadable files | 4 (0.1%) |
| Total error nodes | 6,817 |
| Distinct error buckets | 33 |
| Distributions tracked | 1,005 (CPAN top-1000) |
| Modules in clean manifest | 1,849 |

### 1.2 Error Distribution — Top 15 Buckets

| Rank | Bucket | Count | % | Cumulative % |
|------|--------|-------|---|--------------|
| 1 | unexpected_token_in_expr | 146 | 12.0% | 12.0% |
| 2 | unclosed_paren_identifier | 140 | 11.6% | 23.6% |
| 3 | unexpected_question_expr | 109 | 9.0% | 32.6% |
| 4 | unclosed_paren | 106 | 8.7% | 41.3% |
| 5 | unexpected_rbrace_expr | 83 | 6.8% | 48.2% |
| 6 | unexpected_comma_expr | 70 | 5.8% | 54.0% |
| 7 | expected_left_brace | 66 | 5.4% | 59.4% |
| 8 | expected_variable | 66 | 5.4% | 64.9% |
| 9 | unexpected_fat_arrow_expr | 66 | 5.4% | 70.3% |
| 10 | expected_comma_or_close_paren | 55 | 4.5% | 74.8% |
| 11 | unclosed_bracket | 38 | 3.1% | 77.9% |
| 12 | unclosed_brace | 32 | 2.6% | 80.5% |
| 13 | unclosed_brace_semicolon | 32 | 2.6% | 83.1% |
| 14 | expected_identifier | 30 | 2.5% | 85.6% |
| 15 | unexpected_word_op_or | 28 | 2.3% | 87.9% |

**Remaining 18 buckets**: 148 files (12.1%)

### 1.3 Coverage Goals & File Targets

| Goal | Target Clean | Files Needed | Buckets to Fix |
|------|-------------|-------------|----------------|
| 75% | 3,266 | 127 | Top 1-2 |
| 80% | 3,484 | 345 | Top 1-5 |
| 85% | 3,701 | 562 | Top 1-8 |
| 90% | 3,919 | 780 | Top 1-10 + selected Tier 2 |
| 95% | 4,137 | 998 | All Tier 1-2, some Tier 3 |
| 98% | 4,268 | 1,129 | Tier 1-3 + targeted edge cases |
| 100% | 4,355 | 1,212 | All fixable + documented unfixable |

---

## Phase 2: Categorize the Remaining 17.9%

This phase analyzes each error bucket by **fixability**: can the parser be improved to handle it, or is it an inherent limitation?

### 2.1 Tier 1: High-Impact Fixable (Fast Wins — 300+ files, 1-2 weeks)

#### Bucket #1: unexpected_token_in_expr (146 files, 12.0%)

**Root Cause**: Catch-all for "found unexpected token while parsing expression". Actually a cluster of 10 sub-categories.

**Sub-categories** (from scout analysis — Feb 2026):
1. Keywords used as bareword identifiers (25-40 files) — EASY fix
2. Postfix operators with statement modifiers (20-30 files) — MEDIUM fix
3. Complex paren/brace nesting in expressions (15-25 files) — MEDIUM fix
4. Built-in function edge cases (15-20 files) — MEDIUM fix
5. Method call chains (10-15 files) — MEDIUM fix
6. Dereference chains (10-15 files) — MEDIUM fix
7. Special variables in expressions (8-12 files) — EASY fix
8. Regex binding with complex LHS (5-10 files) — MEDIUM fix
9. Perl 5.20+ signatures (5-10 files) — HARD fix
10. Source filters / legitimately unparseable (3-8 files) — UNFIXABLE

**Fixable estimate**: ~110-150 files (75-100% of bucket)
**Difficulty**: EASY-MEDIUM
**Effort**: 2-3 builders × 2 weeks
**ROI**: HIGH (single bucket fix = 3-4% corpus improvement)

**Action**:
- Spawn 3-5 parallel builder agents, one per sub-category
- Each builder creates PR with fix + test cases + corpus validation
- Merge in waves of 2-3 PRs to stay within CI capacity

---

#### Bucket #2: unclosed_paren_identifier (140 files, 11.6%)

**Root Cause**: Expected `)` but found identifier. Occurs when parser can't complete parenthesized expression.

**Sub-patterns**:
- Implicit `$_` in constructs: `grep { ... }` without explicit `@_` source
- Bareword function calls in complex positions: `foo bar baz;` (method chain)
- Interpolated context confusion: `"${var}"` in paren position
- QW-like list operators: `qw(foo bar)` parsing issues in certain contexts

**Root cause investigation needed**: Requires corpus sampling (top 10-20 files from bucket)

**Likely fixes**:
- Improve bareword identifier recognition in function argument position
- Better context detection for implicit `$_` sources
- Handle interpolated variable dereference in parentheses

**Fixable estimate**: ~80-110 files (57-79% of bucket)
**Difficulty**: MEDIUM
**Effort**: 1 builder × 2-3 weeks (requires root cause investigation first)
**ROI**: HIGH (11.6% of remaining failures)

**Action**:
- Scout first: sample 10-15 corpus files, identify exact parse failure points
- Create builder-ready GitHub issue with categorized examples
- Spawn builder after scout completes

---

#### Bucket #3: unexpected_question_expr (109 files, 9.0%)

**Root Cause**: Ternary operator `?:` not fully supported in all contexts.

**Likely patterns**:
- Nested ternary: `$a ? $b ? $c : $d : $e`
- Ternary in list context: `@list = ($x ? A : B, $y ? C : D)`
- Ternary as default: `$value //= ($condition ? X : Y)`
- Chained ternary with method calls: `$obj->method($x ? A : B)`

**Historical context**: Ternary has been partially supported; issue is incomplete precedence handling.

**Fixable estimate**: ~70-100 files (64-92% of bucket)
**Difficulty**: MEDIUM (precedence adjustment)
**Effort**: 1 builder × 2 weeks
**ROI**: HIGH (9.0% of remaining failures)

**Action**:
- Root cause confirmation via corpus sampling
- Precedence refactoring in expression parser
- Comprehensive ternary test suite

---

#### Bucket #4: unclosed_paren (106 files, 8.7%)

**Root Cause**: Expected `)` but found something else (not just identifier — catch-all).

**Related to Bucket #2** but slightly different pattern. Bucket #2 is specifically `unclosed_paren_identifier` (found identifier when expecting close paren). This is the general "found anything else" case.

**Likely patterns**:
- Semicolon inside parens: `method(arg1, arg2;)` — typo or DSL syntax
- Nested structure imbalance: `func({ key => [ val1, val2) ])` — bracket mismatch
- Incomplete list: `(a, b,` — intentional multi-line (should parse as incomplete expr)
- XS module boundary: `.pm` file with embedded C code generating odd syntax

**Fixable estimate**: ~60-85 files (57-80% of bucket)
**Difficulty**: MEDIUM
**Effort**: 1 builder × 2 weeks
**ROI**: HIGH (8.7% of remaining failures)

**Action**:
- Scout: categorize the "something else" patterns
- Identify which are parser gaps vs. genuinely malformed/XS boundary issues
- Build targeted fixes for top patterns

---

#### Bucket #5: unexpected_rbrace_expr (83 files, 6.8%)

**Root Cause**: Found `}` when parsing expression. Usually indicates hash vs. block disambiguation issue.

**Likely patterns** (per Feb 2026 scout analysis #2183):
- Hash literal in statement context: `my $h = { a => 1, b => 2 };` (works)
- Hash literal in expression context: `foo({ a => 1 })` (may fail)
- Nested blocks: `do { ... } if $x` — `}` appears in unexpected position
- Postfix `}` in complex nesting: `@arr = map { $_+1 }, @items;` — dangling brace

**Known issue**: Parser has been improving on this (builders have tackled it in recent PRs #2041, #2050).

**Fixable estimate**: ~60-75 files (72-90% of bucket, assuming fixes landed)
**Difficulty**: MEDIUM
**Effort**: 1 builder × 1-2 weeks (may already be partially fixed)
**ROI**: MEDIUM-HIGH (6.8% of remaining)

**Action**:
- Verify current state: have recent PRs fixed this?
- If not: builder attack on remaining patterns
- If yes: corpus ratchet should auto-migrate files to clean

---

### 2.2 Tier 2: Medium-Impact Fixable (Harder Wins — 200+ files, 2-4 weeks)

#### Bucket #6: unexpected_comma_expr (70 files, 5.8%)

**Root Cause**: Found `,` in expression where operator expected.

**Likely patterns**:
- Comma operator in unusual position: `$a = ($x, $y, $z)` (last wins)
- List constructor precedence: `foo($a, $b, $c)` vs. `foo $a, $b, $c` context confusion
- Bareword function call with comma: `foo, bar` (method chaining, comma has meaning)

**Fixable estimate**: ~50-65 files (71-93% of bucket)
**Difficulty**: MEDIUM (precedence handling)
**Effort**: 1 builder × 2 weeks
**ROI**: MEDIUM (5.8% of remaining)

---

#### Bucket #7: expected_left_brace (66 files, 5.4%)

**Root Cause**: Expected `{` for hash/block, found something else.

**Likely patterns**:
- Hash constructor missing braces: `my %h = a => 1, b => 2;` (valid Perl, no braces needed)
- Block-taking builtin without braces: `map _ + 1 @arr` (invalid; should be `map { $_ + 1 } @arr`)
- Whitespace issues or lexer boundary: `{ ... }` not recognized as block delimiter

**Fixable estimate**: ~50-60 files (76-91% of bucket)
**Difficulty**: MEDIUM (context-dependent parsing)
**Effort**: 1 builder × 2-3 weeks
**ROI**: MEDIUM (5.4% of remaining)

---

#### Bucket #8: expected_variable (66 files, 5.4%)

**Root Cause**: Expected `$`, `@`, `%`, `&`, or `*` sigil, found bareword.

**Likely patterns**:
- Bareword in `local` context: `local $my_var` (works), `local $/ = "\n"` (should work, magic var)
- Bareword in `my` context: `my $x` (works), `my ($x, $y)` (works), `my $x, $y, $z` (list, works)
- Missing sigil in assignment: `x = 5;` (invalid Perl, caught correctly)
- Special handling for magic variables: `$/`, `$\`, `$;`, etc. may not be recognized in all positions

**Fixable estimate**: ~40-55 files (61-83% of bucket)
**Difficulty**: MEDIUM (lexer/parser coordination)
**Effort**: 1 builder × 1-2 weeks
**ROI**: MEDIUM (5.4% of remaining)

---

#### Bucket #9: unexpected_fat_arrow_expr (66 files, 5.4%)

**Root Cause**: Found `=>` in expression where operator expected.

**Likely patterns**:
- Bareword coercion before `=>`: `format => 'text'` (format is keyword, should be allowed)
- Fat arrow in unusual position: `($a => $b)` in list context
- Fat arrow after postfix operator: `$x++ => $y` (weird but valid?)

**Known**: Builders have been working this bucket (PR #2198 covers 46 files). Status unclear post-merge.

**Fixable estimate**: ~50-65 files (76-98% of bucket)
**Difficulty**: MEDIUM-HIGH (keyword special-casing)
**Effort**: 1 builder × 1-2 weeks (if not already fixed)
**ROI**: MEDIUM (5.4% of remaining)

---

#### Bucket #10: expected_comma_or_close_paren (55 files, 4.5%)

**Root Cause**: In parenthesized list, expected `,` or `)`, found something else.

**Likely patterns**:
- Postfix operator in arg list: `foo($x++)` (should parse)
- Method call in arg list: `foo($obj->method())` (should parse)
- Nested parens with complex precedence: `foo((a ? b : c), d)` (should parse)
- Bareword in position parser misreads: `foo(bar, baz)` with DSL context

**Fixable estimate**: ~40-50 files (73-91% of bucket)
**Difficulty**: MEDIUM (expression parsing in context)
**Effort**: 1 builder × 1-2 weeks
**ROI**: MEDIUM (4.5% of remaining)

---

### 2.3 Tier 3: Lower-Impact Fixable (Edge Cases — 100+ files, 3-6 weeks)

**Buckets #11-20**: unclosed_bracket (38), unclosed_brace (32), unclosed_brace_semicolon (32), expected_identifier (30), unexpected_word_op_or (28), and 13 others.

**Combined impact**: ~200 files (16.5%)
**Difficulty**: MEDIUM-HARD (specialized edge cases)
**Effort**: 3-4 builders × 2-4 weeks each
**ROI**: MEDIUM (each bucket <5%)

**Key buckets in Tier 3**:
- **unclosed_bracket** (38): Array subscript completion, similar to paren/brace issues
- **expected_identifier** (30): Bareword recognition in unusual positions
- **unexpected_word_op_* (28+)**: Word operators (and, or, not, xor) in wrong positions

---

### 2.4 Tier 4: The Unfixable Floor

**Estimate**: 80-150 files (~7-12% of remaining)

#### Category A: Source Filters (40-60 files)

Perl's source filter mechanism allows modules to modify source code before parsing. Examples:
- Filter::Simple, Filter::Util::Call
- Moose/MooseX with code-generating sugar
- Template::Toolkit with `<% %>`-style blocks
- Prototypes that modify sigils

**Why unparseable**: The filter rewrites the source at compile-time. Static analysis cannot predict the output.

**Examples**:
```perl
use Filter::Simple sub { s/foo/bar/g };
use Moose;  # declares attributes via 'has', rewrites keyword semantics
```

**Status**: UNFIXABLE. Document in "Known Limitations."

---

#### Category B: BEGIN Blocks That Modify the Parser

Perl allows arbitrary code execution at compile time via `BEGIN`. Some modules use this to modify the parser itself:

```perl
BEGIN {
  $^H{feature} = 1;  # enable/disable features
}
use feature 'signatures';  # requires special handling
```

**Why unparseable**: The parser state depends on runtime decisions made in BEGIN blocks.

**Status**: UNFIXABLE. Rationalize as "compile-time metaprogramming."

---

#### Category C: XS Module Boundaries

Some Perl modules contain embedded C code (XS). The `.pm` file wrapper is valid Perl, but the file may have:
- Unusual syntax for loading compiled code
- Embedded POD that confuses line counting
- Code sections that don't parse as Perl

**Examples**:
```perl
# File: Term/ReadKey.pm
package Term::ReadKey;
use strict;
use warnings;
our $VERSION = '2.46';
use DynaLoader ();
our @ISA = qw(DynaLoader);
our $AUTOLOAD;
sub AUTOLOAD {
    my $sym = $AUTOLOAD;
    ...
    return &$sub(...);
}
bootstrap Term::ReadKey $VERSION;
1;
```

The `bootstrap` line and `AUTOLOAD` magic may trigger parser confusion.

**Status**: MOSTLY FIXABLE (improve `bootstrap` keyword handling, `AUTOLOAD` support). But some files may have truly unparseable sections.

**Files in this category**: ~20-40

---

#### Category D: DSL/Template Syntax

Some modules embed domain-specific languages:

```perl
# Dancer2 route definitions
get '/' => sub { ... };
post '/api' => sub { ... };

# Template::Toolkit
[% FOREACH item IN list %]
  [% item.name %]
[% END %]
```

The `get` and `post` are bareword subroutines that consume blocks as arguments. Parser may fail due to unusual precedence or implicit argument handling.

**Status**: MOSTLY FIXABLE (improve bareword builtin handling). Estimated 20-30 files.

---

#### Category E: Genuinely Malformed or Non-Perl

```perl
# Generated file with syntax errors
auto-generated from foobar.idl
# File may have truncated or incomplete code
# Intentional placeholder: DO NOT USE
```

**Files**: ~5-10

---

#### Unfixable Floor Summary

| Category | Files | Fixability |
|----------|-------|-----------|
| Source Filters | 40-60 | 0% (compile-time rewrite) |
| BEGIN-block parser modification | 20-30 | 0% (runtime decision) |
| XS module boundaries | 20-40 | 20-50% (can improve) |
| DSL/Template syntax | 20-30 | 50-70% (can improve) |
| Genuinely malformed | 5-10 | 0% |
| **Total floor** | **105-170** | **~10-20%** |

**Implication**: Maximum realistic coverage = **95-98%** (4,250-4,290 clean files).

---

## Phase 3: Plan for 100% (with Documented Ceiling)

### 3.1 Path to 90% Corpus Coverage

**Goal**: 3919 clean files (780 more from current 3139)

**Required fixes**:
1. Bucket #1 (unexpected_token_in_expr): 100 files
2. Bucket #2 (unclosed_paren_identifier): 100 files
3. Bucket #3 (unexpected_question_expr): 80 files
4. Bucket #4 (unclosed_paren): 80 files
5. Bucket #5 (unexpected_rbrace_expr): 70 files
6. Buckets #6-10 (small fixes): ~350 files

**Effort**: 8-10 builders × 2 weeks = **4-5 weeks parallel**

**Resources needed**:
- 8-10 builder agents (constrained by CI queue, see below)
- 2-3 scout agents for root-cause analysis
- 1 coordinator for merge batching

**CI Capacity Analysis**:
- Current CI queue: 3-wide (maxes out at 3 concurrent PR builds)
- Average PR build time: 5-10 minutes
- Merge rate: 3 PRs every 15-20 minutes
- Max throughput: 12-16 PRs/hour, ~100 PRs/day
- Cost of rapid merges: CI queue backlog, 30+ min wait per PR

**Recommendation**: 8-10 builders is optimal for parallel execution without CI stalls. Beyond 10, merge queue backs up.

---

### 3.2 Path to 95% Corpus Coverage

**Goal**: 4137 clean files (998 more)

**Required fixes**: All of Phase 3.1 + Tier 2 edge cases

**Tier 2 buckets** (buckets #11-20 + misc):
- unclosed_bracket, unclosed_brace, unclosed_brace_semicolon, expected_identifier, etc.

**Effort**: 12-15 builders × 3 weeks = **7-9 weeks parallel**

**Expected outcome**: Most fixable Tier 1-2 problems solved. 95% is realistic; hitting 96-97% requires careful selection of Tier 3 fixes.

---

### 3.3 Path to 98% Corpus Coverage

**Goal**: 4268 clean files (1129 more)

**Required**: 95% + selective Tier 3 fixes + XS/DSL improvements

**Which Tier 3 fixes to prioritize**:
1. By impact: Fix buckets in order of size (unclosed_bracket, unclosed_brace, etc.)
2. By difficulty: Choose MEDIUM over MEDIUM-HARD first
3. By dependencies: Fix prerequisites (e.g., postfix operators before statement modifiers)

**Effort**: 10-12 more builders × 3-4 weeks = **4-6 weeks additional**

**Total effort to 98%**: 11-15 weeks parallel (8-12 weeks serial)

---

### 3.4 Path to 100% (or "Documented Ceiling")

**Reality check**: The unfixable floor (105-170 files) will remain unless:
1. Source filters are statically analyzable (not realistic)
2. BEGIN blocks are pre-executed (breaks tool portability)
3. XS boundaries are manually annotated (possible but high effort)

**Recommendation**:
- Set realistic goal at **97-98%** (realistic with 12-16 weeks of builders)
- Document the 2-3% unfixable floor in `PARSER_LIMITATIONS.md`
- Categorize failures: "Expected parser limitations" (source filters, DSL), "Implementation gaps" (fixable), "Malformed" (genuinely broken)

**If pursuing 100%**:
- Requires 20-30 weeks additional effort
- Focus on: improving XS boundary handling, DSL syntax support, explicit documentation
- Create fallback parser for special file types (source filter markers, BEGIN blocks)
- Not recommended unless customer requirement drives it

---

## Phase 4: Corpus Expansion

Current scope: **CPAN top-1000 distributions** (~1005 modules)

### 4.1 Beyond Top-1000

**Opportunity**: Top-1000 represents the most-depended-on CPAN modules. Many smaller but important modules fall outside.

**Expansion options**:

1. **Top-5000 distributions** (+3000 modules)
   - Adds 4000-5000 `.pm` files
   - Effort: 2-3 weeks (1-2 agents doing build + ratchet)
   - Expected cleanup: 70-80% of new files are clean (unknown parser gaps)
   - ROI: LOW (effort high, benefit unclear)

2. **Perl core modules** (+500 modules)
   - Perl 5.38 includes ~500 standard library modules
   - Already partially covered via top-1000 dependency closure
   - Effort: 1 week (fetch + ratchet)
   - Expected cleanup: 95%+ (core modules are well-maintained)
   - ROI: MEDIUM (high quality, low effort)

3. **Popular frameworks** (+200 modules)
   - Dancer2, Mojolicious, Catalyst, DBIx::Class, etc.
   - Intentionally selected for ecosystem quality
   - Effort: 1-2 weeks (manual curation + ratchet)
   - Expected cleanup: 80-90%
   - ROI: MEDIUM-HIGH (covers real-world use cases)

4. **User/customer code** (unbounded)
   - Not CPAN, but application code using Perl
   - Effort: Custom corpus building
   - Expected cleanup: 50-70% (user code is often less polished)
   - ROI: CUSTOMER-DEPENDENT

### 4.2 Recommended Expansion Path

**Phase A (Week 14-15)**: Add Perl 5.38 core modules
- Fetch via `perl -MFile::Find -e 'find { wanted => sub { push @f, $_ if /\.pm$/ }, no_chdir => 1 }, $Config{privlib}, $Config{sitelib}'`
- Merge into corpus, ratchet
- Expected: +400 clean files

**Phase B (Week 16-17)**: Add top frameworks (Dancer2, Mojolicious, Catalyst, ORM libraries)
- Manual selection of 30-50 popular frameworks
- Install + ratchet
- Expected: +200-300 clean files

**Phase C (Post-98%)**: Top-5000 distributions (optional, low priority)
- Run corpus expansion to top-5000
- Analyze new error buckets
- Decide on ROI for new builder waves

### 4.3 Corpus Maintenance & Regression

**Ratchet mechanism**: Once a module passes, it goes into `.ci/cpan-corpus-manifest.txt`. The `just cpan-corpus-check` gate enforces that ratcheted modules never regress.

**Versioning strategy**:
- Pin specific CPAN release versions in `.ci/cpan-top-1000-distributions.txt`
- Re-run sweep quarterly to catch new bugs in updated distributions
- Ratchet only locks in the specific version that passed

**Corpus freshness**:
- Current data: 2026-03-18 (fresh)
- Cadence: Weekly updates to top-1000 distribution list via `just cpan-corpus-fetch-list`
- Baseline refresh: Monthly or after parser changes

---

## Phase 5: Test Strategy

### 5.1 Regression Testing per Bucket

For each bucket fix, create a regression test file:

**Example structure** (`crates/perl-parser-core/tests/bucket_unexpected_token_in_expr_tests.rs`):

```rust
#[test]
fn test_keyword_as_bareword_format() {
    // From CPAN: Module-Setup/lib/Module/Setup/Plugin.pm:94
    let code = r#"
        my %hash = ( format => 'text' );
    "#;
    assert!(parse(code).is_ok());
}

#[test]
fn test_postfix_operator_with_modifier() {
    // From CPAN: Try-Tiny/lib/Try/Tiny.pm:120
    let code = r#"
        $count++ if $enabled;
    "#;
    assert!(parse(code).is_ok());
}

#[test]
fn test_complex_paren_nesting() {
    // From CPAN: DBIx-Class/lib/DBIx/Class/ResultSet.pm:450
    let code = r#"
        my @sorted = [sort { $a <=> $b } @items];
    "#;
    assert!(parse(code).is_ok());
}
```

**Strategy**:
- 1 test file per error bucket (33 files total)
- Each file contains 5-10 real CPAN examples
- Reference corpus file and line number for reproducibility
- Tests are integration tests in the parser crate

**Integration with CI**:
- Regression tests run as part of `cargo test -p perl-parser-core`
- Gate: `just pr-fast` and `just ci-gate` always run tests
- Ratchet: If a test fails, rerun corpus sweep to validate corpus baseline

---

### 5.2 Corpus Baseline Integrity

**Baseline format** (`.ci/cpan-corpus-baseline.json`):

```json
{
  "schema_version": "1.1.0",
  "commit": "...",
  "timestamp": "2026-03-18T...",
  "corpus_profile": "system",
  "total_files": 4355,
  "clean_files": 3139,
  "files_with_errors": 1212,
  "first_error_buckets": { ... }
}
```

**Enforcement**:
1. After each builder PR merge, run `just cpan-corpus-sweep --output target/new-baseline.json`
2. Compare new-baseline.json vs. `.ci/cpan-corpus-baseline.json`
3. If `clean_files` decreased: **BLOCKER**, revert merge
4. If `clean_files` increased: Run `just cpan-corpus-ratchet` to lock in gains

**CI gate** (`just ci-gate`):
```bash
cargo xtask cpan-corpus sweep --enforce  # Validates manifest + baseline
```

Exits nonzero if:
- Any ratcheted module regressed
- Baseline file missing
- Manifest integrity violated

---

### 5.3 Tracking Progress

**Metrics to track** (via `scripts/update-current-status.py`):

```yaml
corpus:
  total_files: 4355
  clean_files: 3139
  coverage_percent: 72.1
  top_bucket: "unexpected_token_in_expr (146)"
  top_5_combined: 501
  unfixable_floor_estimate: "105-170 (2.4-3.9%)"
  ratcheted_modules: 1849

phases:
  phase_A_90pct:
    target_files: 3919
    effort_weeks_parallel: 4-5
    builders_required: 8-10
    status: "Planning"

  phase_B_95pct:
    target_files: 4137
    effort_weeks_parallel: 7-9
    builders_required: 12-15
    status: "Queued"

  phase_C_98pct:
    target_files: 4268
    effort_weeks_parallel: 11-15
    builders_required: 20+
    status: "Post-B"
```

**Monthly reporting**:
- Update CURRENT_STATUS.md with latest metrics
- Publish progress chart: coverage % vs. week
- List top 3 blocking buckets
- Highlight regressions (should be zero)

---

## Phase 6: Parallel Execution Plan

### 6.1 Builder Wave 1 (Weeks 1-4): Top 5 Buckets

**Goal**: 85-87% coverage (560+ files)

**Assignments**:
- Builder #1: unexpected_token_in_expr (sub-category: keywords as barewords)
- Builder #2: unexpected_token_in_expr (sub-category: postfix operators + modifiers)
- Builder #3: unclosed_paren_identifier (root cause investigation + fixes)
- Builder #4: unexpected_question_expr (ternary operator precedence)
- Builder #5: unclosed_paren (root cause + fixes)

**Timeline**:
- Days 1-3: Scout (root cause analysis)
- Days 4-10: Build (implement parser changes, test cases)
- Days 11-14: Review + merge (2-3 PRs per wave)

**Expected PRs**: 5 PRs, ~80% chance of landing all

**Merge rate**: 1 PR every 2-3 days to avoid CI queue saturation

---

### 6.2 Builder Wave 2 (Weeks 5-8): Buckets 5-10

**Goal**: 90%+ coverage (380+ more files)

**Assignments**:
- Builder #6: unexpected_rbrace_expr
- Builder #7: unexpected_comma_expr
- Builder #8: expected_left_brace
- Builder #9: expected_variable
- Builder #10: unexpected_fat_arrow_expr

**Expected PRs**: 5 PRs, smaller changes (30-80 line diffs)

**Merge rate**: 1 PR per 2 days

---

### 6.3 Builder Wave 3 (Weeks 9-12): Tier 2 & Selected Tier 3

**Goal**: 95%+ coverage (remaining bucket consolidation)

**Assignments**:
- Builder #11-15: Tier 2 buckets (unclosed_bracket, expected_identifier, etc.)

**Effort**: 3-4 weeks per builder × 3-4 builders in parallel

**Expected outcome**: 4,137+ clean files (95% coverage)

---

### 6.4 Coordination & CI Pacing

**CI bottleneck**: 3-wide merge queue

**Coordination strategy**:
- Coordinator tracks PR state (draft, review, approved, ready-to-merge)
- Batches merges in groups of 3 (respects CI capacity)
- Waits 20-30 minutes between batches (allows CI to complete + checks for regressions)
- If any batch fails: investigates, reverts if necessary, continues

**Tool support**:
- Slack/Discord notifications when batch is ready to merge
- Automated CI status checks (failing build = pause merges)
- Dashboard: live view of builder progress, PR state, corpus coverage trend

---

## Decision Trees & Trade-offs

### Should we pursue 100%?

**Factors**:

| Factor | Decision |
|--------|----------|
| **Unfixable floor exists** | 95-98% is realistic ceiling; 100% requires 20-30 weeks |
| **Customer demand** | If no specific customer has "must be 100%", shoot for 98% |
| **ROI** | Last 2-3% fix diminishing returns (weeks of effort for few files) |
| **Documentation trade-off** | Documenting "Known Limitations" is 80% as valuable as fixing all bugs |
| **Effort to market** | Get to 90-95%, ship, gather feedback, iterate |

**Recommendation**: **Pursue 95% as the primary goal. Plan 98% as the secondary milestone. Only attempt 100% if customer or strategic requirement drives it.**

---

### Which Tier 2 buckets to fix?

**Priority heuristic**: `(file_count - 20) * (100 - difficulty) / 100`

Higher score = better ROI.

| Bucket | Files | Difficulty | Score | Rank |
|--------|-------|-----------|-------|------|
| unexpected_comma_expr | 70 | MEDIUM (50) | 3500 | #1 |
| expected_left_brace | 66 | MEDIUM (50) | 3300 | #2 |
| expected_variable | 66 | MEDIUM (50) | 3300 | #2 |
| expected_comma_or_close_paren | 55 | MEDIUM (50) | 2750 | #4 |
| unclosed_bracket | 38 | MEDIUM (50) | 1900 | #5 |

**Recommendation**: Fix Tier 2 buckets in priority order 1→5 if pursuing 95%.

---

### Should we expand the corpus?

**Costs & benefits**:

| Option | Effort | Benefit | Recommendation |
|--------|--------|---------|-----------------|
| **Perl 5.38 core** | 1 week | +400 files, 95%+ clean | YES — do week 14 |
| **Popular frameworks** | 1-2 weeks | +200-300 files, 80-90% clean | YES — do week 15-16 |
| **Top-5000 CPAN** | 2-3 weeks | +2000+ files, ~50% clean (unknown gaps) | MAYBE — low priority |
| **User/customer code** | Custom | Depends | CUSTOMER-DRIVEN |

**Recommendation**: Core + frameworks. Skip top-5000 unless customer requests.

---

## Appendix A: Error Bucket Descriptions

### Complete List of 33 Buckets

All buckets from baseline, ranked by impact:

```
1.  unexpected_token_in_expr (146) — Expression parsing catch-all
2.  unclosed_paren_identifier (140) — `)` expected, found identifier
3.  unexpected_question_expr (109) — Ternary operator issues
4.  unclosed_paren (106) — `)` expected, found other token
5.  unexpected_rbrace_expr (83) — `}` in expression position
6.  unexpected_comma_expr (70) — `,` in expression position
7.  expected_left_brace (66) — `{` expected for block/hash
8.  expected_variable (66) — Variable sigil expected
9.  unexpected_fat_arrow_expr (66) — `=>` in expression position
10. expected_comma_or_close_paren (55) — List context punctuation
11. unclosed_bracket (38) — `]` expected
12. unclosed_brace (32) — `}` expected
13. unclosed_brace_semicolon (32) — `}` expected, found `;`
14. expected_identifier (30) — Bareword expected
15. unexpected_word_op_or (28) — `or` in wrong position
16. unexpected_word_op_and (7) — `and` in wrong position
17. unexpected_word_op_not (8) — `not` in wrong position
18. expected_left_paren (7) — `(` expected
19. expected_semicolon (9) — `;` expected
20. unclosed_angle (8) — `>` expected (qw, regex)
21. expected_import_item (12) — Import list syntax
22. expected_module_name (27) — Module name in use/require
23. expected_colon (26) — `:` expected (labels, refs)
24. substitution_misparse (8) — s///, m//, tr/// syntax
25. expected_comma (2) — `,` expected
26. CHECK must be followed by block (2) — CHECK phaser
27. Missing replacement in substitution (2) — s/// syntax
28. unexpected_arrow_expr (10) — `->` in expression
29. unexpected_slash_expr (4) — `/` in expression
30. unexpected_rparen_expr (2) — `)` in expression
31. unexpected_semicolon_expr (16) — `;` in expression
32. unexpected_return_expr (0?) — return in expression
33. unexpected_eof_expr (0?) — Unexpected EOF
```

(Note: Some buckets in baseline are zero or very small; not all 33 listed above have files.)

---

## Appendix B: Resource Requirements

### Minimum Resource Levels

| Phase | Builders | Scouts | Coordinator | Duration | CI Cost |
|-------|----------|--------|-------------|----------|---------|
| 1-2 | 2-3 | 1-2 | 0.5 | 4-6 weeks | +100 hrs |
| 1-3 | 5-8 | 2 | 1 | 8-12 weeks | +250 hrs |
| 1-4 | 10-15 | 3 | 1 | 12-16 weeks | +400 hrs |
| 1-5 (98%) | 15-20 | 3 | 1 | 16-20 weeks | +500 hrs |

**CI cost**: Estimated GitHub Actions runner-hours (Linux + Windows cross-compilation)

### Team Composition

For **Phase 1-3 (95% coverage)** in 12 weeks:

- 8-10 builder agents (parallel worktrees)
- 2-3 scout agents (root cause investigation)
- 1 orchestrator/coordinator (merge sequencing, metrics)
- 1 QA/test agent (regression testing, corpus validation)

**Total**: 12-15 agents active in parallel

---

## Appendix C: Risk Mitigation

### Risks & Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| CI queue saturation | HIGH | 2-3x slowdown | Batch merges, pace to 3/batch |
| Builder parallelism creates conflicts | MEDIUM | Merge failures, rework | Use worktree isolation (mandatory) |
| Regressions in unfixable-floor discovery | LOW | Wrong decisions | Document floor early, validate categorization |
| Parser changes break other features | MEDIUM | Unexpected failures | Comprehensive test suite, regression testing per bucket |
| Corpus drift (modules update) | LOW | Stale baseline | Monthly refresh of distribution list + sweep |
| Unfinished work (builders run out of time) | MEDIUM | Incomplete queue | Prioritize by ROI upfront; cut Tier 3 if needed |

### Rollback Strategy

If a builder PR causes regressions:

1. **Immediate**: Revert the merge (git revert)
2. **Investigation**: Scout re-analyzes the PR changes vs. corpus failures
3. **Decision**:
   - If fixable in <2 days: Builder redoes, resubmits
   - If complex (>2 days): Defer to next phase, move PR to backlog

---

## Appendix D: Success Metrics

### End-of-Phase Milestones

**Phase A (90% coverage)**:
- ✓ 3919+ clean files
- ✓ Top 5 buckets reduced by 80%+
- ✓ Zero regressions (ratcheted modules stable)
- ✓ 5 merged PRs, 500+ lines of parser improvements
- ✓ Regression test suite for buckets #1-5

**Phase B (95% coverage)**:
- ✓ 4137+ clean files
- ✓ Buckets #1-10 mostly fixed
- ✓ 10 merged PRs, 1000+ lines of improvements
- ✓ Full regression test suite (all 33 buckets)
- ✓ Unfixable floor clearly documented

**Phase C (98% coverage)**:
- ✓ 4268+ clean files
- ✓ Tier 2-3 selective fixes completed
- ✓ XS/DSL boundary improvements validated
- ✓ Corpus expansion (core + frameworks) integrated
- ✓ Monthly metrics dashboard live

---

## Appendix E: Related Documentation

- **Parser architecture**: `docs/reference/PARSER_ARCHITECTURE.md`
- **Error recovery strategy**: `docs/reference/PARSER_ERROR_RECOVERY.md`
- **Known limitations**: `docs/project/PARSER_LIMITATIONS.md` (to be created)
- **CPAN corpus baseline**: `.ci/cpan-corpus-baseline.json`
- **Manifest integrity**: `.ci/cpan-corpus-manifest.txt`
- **Scout findings** (Feb-Mar 2026): Memory files in `.claude/projects/*/memory/`

---

## Summary: Path Forward

**Immediate next steps** (weeks 1-4):

1. **Scout wave 1**: Categorize buckets #1-5 with CPAN file samples (2-3 scouts, 5-7 days)
2. **Builder wave 1**: Spawn 5 builders on buckets #1-5 (concurrent, 2-3 weeks)
3. **Merge coordination**: Batch merges 3-wide, paced 20-30 min apart
4. **Metrics**: Update CURRENT_STATUS.md weekly

**By week 4**: 85-90% coverage target (3700+ clean files)

**By week 8**: 95% coverage target (4137+ clean files)

**By week 12-16**: 97-98% coverage target (4250+ clean files), with unfixable floor documented

**Long-term**: Maintain corpus freshness (monthly sweep), expand to frameworks, consider top-5000 CPAN if customer demand.

---

**Roadmap complete. Ready for team review and builder assignment.**
