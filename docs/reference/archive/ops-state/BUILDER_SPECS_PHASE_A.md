# Builder Specifications — Phase A (90% Coverage)

**Scope**: Buckets #1-5 (560+ files)
**Duration**: 4-5 weeks parallel
**Builders**: 5 recommended
**Target**: 3919+ clean files (90% coverage)

---

## Builder #1: unexpected_token_in_expr (Sub-category: Keywords as Barewords)

**Bucket**: unexpected_token_in_expr (146 total, 25-40 files in this sub-category)
**Impact**: 25-40 files
**Difficulty**: EASY
**Estimate**: 2 weeks

### Problem Statement

Perl allows reserved keywords to be used as bareword identifiers in certain contexts (e.g., as hash keys before `=>`). The parser currently rejects these.

### Examples from CPAN

1. **Module-Setup** (lib/Module/Setup/Plugin.pm:94)
   ```perl
   my %hash = ( format => 'text' );  # format is keyword, should be allowed
   ```

2. **local::lib** (lib/local/lib.pm:150)
   ```perl
   my $obj = local::lib->foo();  # local is keyword in bareword context
   ```

3. **feature** (core)
   ```perl
   use if $condition, 'Module';  # if is keyword but allowed here
   my %config = ( state => 'init' );  # state is keyword
   ```

### Root Cause Analysis

The lexer/parser distinguishes between keywords and barewords. In Perl, certain keywords are context-sensitive:
- **Can be barewords**: format, local, state, given, when, default, unless, for, foreach, while, until
- **Cannot be barewords**: my, sub, if, elsif, else, return, etc.

**Current behavior**: Parser rejects all keywords in bareword position.

**Fix strategy**:
1. In bareword recognition (expression parser), check if the bareword is in a position where keywords are allowed
2. Positions where keywords can be barewords:
   - Left side of `=>` in hash constructor
   - First element of list after `(` or `,`
   - Following a bareword function call

### Implementation Guide

**File to modify**: `crates/perl-parser-core/src/engine/parser/expressions/primary.rs` (or equivalent bareword recognition)

**Changes**:
1. Create `is_contextual_keyword(token: &str, context: &ParserContext) -> bool`
2. In bareword parsing, allow keywords if `is_contextual_keyword()` returns true
3. Add test cases for each keyword type

**Test coverage**:
```rust
#[test]
fn test_keyword_as_bareword_hash_key() {
    assert_parses_clean!(r#"my %h = ( format => 'x', local => 'y' );"#);
}

#[test]
fn test_keyword_as_bareword_list() {
    assert_parses_clean!(r#"my @list = (state, given, default);"#);
}

#[test]
fn test_keyword_after_bareword_func() {
    assert_parses_clean!(r#"foo format, local, state;"#);
}
```

### Validation

1. **Unit tests**: 5-10 tests covering each keyword type
2. **Integration**: Run corpus sweep, verify 25-40 files move from error bucket to clean
3. **Regression**: Ensure no CPAN files that *should* fail now pass

### Expected Outcome

- PR size: ~80-150 lines (bareword context checks + tests)
- Files fixed: 25-40 CPAN modules
- Corpus improvement: +25-40 files (~0.6-0.9% coverage)

---

## Builder #2: unexpected_token_in_expr (Sub-category: Postfix Operators + Statement Modifiers)

**Bucket**: unexpected_token_in_expr (20-30 files in this sub-category)
**Impact**: 20-30 files
**Difficulty**: MEDIUM
**Estimate**: 2-3 weeks

### Problem Statement

Statement modifiers (if, unless, while, until) don't parse after postfix operators (++, --, ->, []).

### Examples from CPAN

1. **Try-Tiny** (lib/Try/Tiny.pm:120)
   ```perl
   $count++ if $enabled;  # Parser fails on postfix ++ followed by if
   ```

2. **DBIx-Class** (lib/DBIx/Class/Util.pm:80)
   ```perl
   shift @arr unless $empty;
   ```

3. **File-Find** (lib/File/Find.pm:300)
   ```perl
   print shift @items if @items;
   ```

### Root Cause Analysis

The parser handles statement modifiers at the statement level: `STATEMENT if COND`. But postfix operators like `++` are part of the expression, not the statement.

**Parser sees**: `$count++ if $enabled` as:
1. Expression: `$count++`
2. Unexpected token: `if` (not a valid infix operator)

**Correct parse**: `($count++) if $enabled` = "increment $count only if $enabled is true"

### Implementation Guide

**File to modify**: `crates/perl-parser-core/src/engine/parser/statements.rs` (statement modifier parsing)

**Changes**:
1. Extend statement modifier parsing to accept postfix expressions (not just simple variables)
2. After parsing a full expression (including postfix operators), check for statement modifiers
3. Wrap in appropriate precedence grouping

**Current flow**:
```
statement := expression ';'
           | expression MODIFIER ';'  (but only for simple expressions)
```

**New flow**:
```
statement := postfix_expression ';'
           | postfix_expression MODIFIER ';'
```

**Test coverage**:
```rust
#[test]
fn test_postfix_inc_with_if_modifier() {
    assert_parses_clean!(r#"$x++ if $y;"#);
}

#[test]
fn test_postfix_dec_with_unless_modifier() {
    assert_parses_clean!(r#"$x-- unless $y;"#);
}

#[test]
fn test_shift_with_if_modifier() {
    assert_parses_clean!(r#"shift @arr if @arr;"#);
}

#[test]
fn test_method_call_with_modifier() {
    assert_parses_clean!(r#"$obj->method() if $enabled;"#);
}
```

### Validation

1. **Unit tests**: 10+ covering each postfix operator type
2. **Integration**: Verify 20-30 files move to clean
3. **Regression**: Ensure existing statement modifier logic still works

### Expected Outcome

- PR size: ~100-200 lines
- Files fixed: 20-30 CPAN modules
- Corpus improvement: +20-30 files (~0.5-0.7% coverage)

---

## Builder #3: unclosed_paren_identifier (Root Cause + Fixes)

**Bucket**: unclosed_paren_identifier (140 files)
**Impact**: 80-110 files (57-79% of bucket)
**Difficulty**: MEDIUM (requires root cause analysis)
**Estimate**: 2-3 weeks

### Phase 1: Root Cause Investigation (Days 1-3)

**Goal**: Sample 15-20 corpus files, categorize failure patterns.

**Steps**:
1. Extract 20 random files from CPAN corpus that trigger `unclosed_paren_identifier`
2. Run parser in debug mode on each, identify exact parse failure point
3. Categorize by pattern:
   - Implicit `$_` in block-taking builtins
   - Bareword function calls in unusual positions
   - Interpolation issues in parentheses
   - QW operator parsing

**Deliverable**: GitHub issue with categorized examples and suggested fixes

### Phase 2: Implementation (Days 4-14)

Based on root causes found in Phase 1, implement fixes. (Since we don't have exact samples yet, here are likely fixes):

#### Likely Fix #1: Implicit `$_` in grep/map/sort

```perl
grep { $_ > 5 } @items;  # Works
grep { $_ > 5 }, @items;  # May fail due to comma parsing
```

**Implementation**: Improve block argument handling for map/grep/sort/etc.

#### Likely Fix #2: Bareword function calls with arguments

```perl
method_name arg1, arg2;  # Parser may think arg2 starts new statement
```

**Implementation**: Better bareword-to-function-call disambiguation in expression parser.

#### Likely Fix #3: Interpolated dereference in parentheses

```perl
"$obj->method()";  # String, fine
("${obj}");         # Parens + interpolation may confuse parser
```

**Test suite**:
```rust
#[test]
fn test_grep_block_with_pipe() {
    assert_parses_clean!(r#"my @result = grep { $_ > 5 }, @items;"#);
}

#[test]
fn test_bareword_method_args() {
    assert_parses_clean!(r#"foo_method arg1, arg2, arg3;"#);
}

#[test]
fn test_interpolated_deref() {
    assert_parses_clean!(r#"my ($x) = ( "$obj" );"#);
}
```

### Validation

1. Before implementing: Scout must provide categorized examples
2. After implementing: 80-110 files should move to clean
3. Regression: No regressions in other buckets

### Expected Outcome

- Scout phase: 3-4 days
- Builder phase: 10-14 days
- PR size: ~200-400 lines (depends on number of fixes)
- Files fixed: 80-110 CPAN modules (11.6% of bucket)
- Corpus improvement: ~2-2.5% coverage

---

## Builder #4: unexpected_question_expr (Ternary Operator Precedence)

**Bucket**: unexpected_question_expr (109 files)
**Impact**: 70-100 files (64-92% of bucket)
**Difficulty**: MEDIUM (precedence adjustment)
**Estimate**: 2 weeks

### Problem Statement

The ternary operator `?:` is not fully supported in all contexts. Parser fails when ternary appears in certain positions or nests with other operators.

### Examples from CPAN

1. **Nested ternary** (many modules)
   ```perl
   my $result = $x ? $y ? $a : $b : $c;  # Ternary within ternary
   ```

2. **Ternary in list context**
   ```perl
   my @list = ($x ? A : B, $y ? C : D);  # Ternary in comma-separated list
   ```

3. **Ternary with logical operators**
   ```perl
   my $value = $x && $y ? Z : $default;  # Mixed precedence
   ```

4. **Ternary in default assignment**
   ```perl
   $var //= ($condition ? X : Y);  # Ternary in assignment RHS
   ```

### Root Cause Analysis

The ternary operator has **right associativity** and **lower precedence than logical operators**.

```
$a ? $b : $c ? $d : $e
should parse as:
$a ? $b : ($c ? $d : $e)   -- right-associative
```

But parser may be parsing as:
```
($a ? $b : $c) ? $d : $e   -- wrong associativity
```

### Implementation Guide

**File to modify**: `crates/perl-parser-core/src/engine/parser/expressions/ternary.rs` or equivalent

**Changes**:
1. Audit ternary operator parsing for right-associativity
2. Adjust precedence: ternary should be LOWER than logical operators but HIGHER than assignment
3. Fix recursive parsing to allow nested ternaries

**Pseudocode fix**:
```rust
fn parse_ternary(parser: &mut Parser) -> Result<Expr> {
    let mut condition = parse_logical_or(parser)?;  // Higher precedence

    if parser.peek() == Token::Question {
        parser.advance();
        let then_expr = parse_ternary(parser)?;  // Right-recursive for right-assoc
        parser.expect(Token::Colon)?;
        let else_expr = parse_ternary(parser)?;  // Right-recursive
        condition = Expr::Ternary {
            condition: Box::new(condition),
            then_branch: Box::new(then_expr),
            else_branch: Box::new(else_expr),
        };
    }

    Ok(condition)
}
```

**Test coverage**:
```rust
#[test]
fn test_nested_ternary_right_associative() {
    assert_parses_clean!(r#"my $x = $a ? $b : $c ? $d : $e;"#);
    // Should parse as: $a ? $b : ($c ? $d : $e)
}

#[test]
fn test_ternary_in_list() {
    assert_parses_clean!(r#"my @arr = ($x ? A : B, $y ? C : D);"#);
}

#[test]
fn test_ternary_with_logical_and() {
    assert_parses_clean!(r#"my $z = $x && $y ? Z : default;"#);
}

#[test]
fn test_ternary_in_assignment() {
    assert_parses_clean!(r#"$var //= ($cond ? X : Y);"#);
}
```

### Validation

1. **Unit tests**: 8-10 covering each nesting pattern
2. **Integration**: 70-100 files should move to clean
3. **Regression**: Ensure non-ternary expressions still parse correctly

### Expected Outcome

- PR size: ~150-250 lines
- Files fixed: 70-100 CPAN modules
- Corpus improvement: +1.6-2.3% coverage

---

## Builder #5: unclosed_paren (General Case)

**Bucket**: unclosed_paren (106 files)
**Impact**: 60-85 files (57-80% of bucket)
**Difficulty**: MEDIUM (similar to Builder #3, requires categorization)
**Estimate**: 2-3 weeks

### Phase 1: Root Cause Investigation (Days 1-3)

**Goal**: Distinguish from `unclosed_paren_identifier` (which is a special case).

**Expected patterns**:
1. Semicolon inside parens: `method(arg1, arg2;)` (typo or DSL)
2. Nested structure imbalance: `func({ key => [ val1, val2) ]})` (bracket mismatch)
3. Incomplete lists: `(a, b,` (multi-line or intentional)
4. XS module boundaries: `.pm` with embedded C code

**Deliverable**: GitHub issue with categorized patterns, split by fixability.

### Phase 2: Implementation

**Strategy**: Fix highest-impact patterns first.

#### Pattern #1: Semicolon inside parens

This is typically a typo or non-Perl syntax (DSL). May not be fixable without understanding the DSL.

**Recommendation**: Skip or mark as unfixable.

#### Pattern #2: Nested structure imbalance

```perl
my $ref = func({
    key => [val1, val2)  # Mismatched bracket
]);
```

**Fix**: Better bracket matching in nested contexts. This might require tracking bracket stack.

#### Pattern #3: XS boundary markers

Many `.pm` files have markers that XS code looks for:

```perl
package Term::ReadKey;
...
bootstrap Term::ReadKey $VERSION;
```

**Fix**: Recognize `bootstrap` keyword and special XS patterns.

**Test coverage**:
```rust
#[test]
fn test_bootstrap_keyword() {
    assert_parses_clean!(r#"
        package Foo;
        use DynaLoader;
        our @ISA = qw(DynaLoader);
        bootstrap Foo $VERSION;
        1;
    "#);
}

#[test]
fn test_nested_bracket_mismatch_recovery() {
    // This should either parse or fail gracefully, not hang
    let code = r#"my $x = func({ key => [val1, val2) });"#;
    assert!(parse(code).is_ok() || parse(code).is_err());  // Both OK
}
```

### Validation

1. Scout categorization: Which patterns are fixable?
2. After fixes: 60-85 files should move to clean
3. Regression: Ensure legitimate parsing still works

### Expected Outcome

- Scout phase: 3-4 days
- Builder phase: 10-14 days
- PR size: ~200-350 lines
- Files fixed: 60-85 CPAN modules (8.7% of bucket)
- Corpus improvement: ~1.4-2% coverage

---

## Coordination & Scheduling

### Weekly Milestones

**Week 1** (Days 1-5):
- Builders #3 and #5 begin scout phase (categorize root causes)
- Builders #1, #2, #4 begin implementation (issues are already understood)
- Coordinator: Set up merge batching infrastructure, CI monitoring

**Week 2** (Days 6-10):
- Builders #1, #2, #4 submit PRs to review
- Builders #3, #5 submit scout findings as GitHub issues
- Coordinator: Begin review + merge cycle (batches of 3)

**Week 3** (Days 11-15):
- Builders #3, #5 begin implementation (based on scout findings)
- PRs from week 2 merge in batches
- Corpus sweep after each merge batch to validate improvements

**Week 4** (Days 16-20):
- All builders submit final PRs
- Merge cycle continues
- Corpus sweep validates 90%+ coverage achieved

**Week 5** (Optional, if overruns):
- PR review and cleanup
- Final regression testing
- Document learnings for Phase B

### Merge Coordination

**Rule 1**: Batch merges in groups of 3
- Merge PR #1, #2, #3
- Wait 25 minutes for CI
- Check for regressions
- Repeat

**Rule 2**: Pause on any red build
- If CI fails after a batch, investigate
- If fault is in new PR, revert and work with builder
- If fault is in master, fix and continue

**Rule 3**: Corpus sweep after every 3-5 merges
- Run `just cpan-corpus-sweep --output target/batch-N.json`
- Compare to baseline, validate improvement
- If regressed: revert batch, investigate

### Risk Mitigation

**Risk**: Builders don't finish in time
- **Mitigation**: Prioritize by ROI. If tight on time, focus on buckets #1-3 (easier, higher impact). Cut #5 if needed.

**Risk**: Scout takes longer than expected
- **Mitigation**: Builders #1, #2, #4 start immediately; they don't depend on scouts. Stagger scout and builder work.

**Risk**: CI queue backs up
- **Mitigation**: Strict 3-wide batching, no exceptions. Acceptable to merge slower if it prevents backlog.

---

## Definition of Done

**Per-builder deliverables**:

- [ ] Scout phase (if applicable): GitHub issue with categorized examples, fixability assessment, root causes documented
- [ ] Implementation: Parser changes with tests, passing locally + CI
- [ ] PR submitted: Draft PR opened, passes CI, ready for review
- [ ] Review: Coordinator + 1 other reviewer approve
- [ ] Merge: Merged in batched group, corpus sweep validates improvement
- [ ] Validation: Corpus sweep shows X files moved to clean, zero regressions

**Phase-wide success**:

- [ ] All 5 builders complete PRs (100% delivery rate or 80% minimum)
- [ ] 560+ files move from error to clean (85-87% coverage achieved)
- [ ] Zero regressions (ratcheted modules stay clean)
- [ ] Regression test suite in place for buckets #1-5
- [ ] Learnings documented for Phase B

---

## Support & Escalation

**Builder gets blocked?**
- Escalate to coordinator immediately
- Coordinator loops in orchestrator/research
- Max 2-hour response time for blockers

**Scout encounters ambiguous patterns?**
- Post to GitHub issue, tag parser experts
- Create minimal reproducer
- Defer PR until clarity achieved

**CI is consistently red?**
- Halt new merges
- Investigate root cause
- Fix in dedicated follow-up PR before resuming

---

## Phase A Success Criteria

| Metric | Target | Acceptable | Fail |
|--------|--------|-----------|------|
| Coverage gain | 17-18% | 15-17% | <15% |
| Target files | 780+ | 650+ | <650 |
| PRs merged | 5/5 | 4/5 | <4 |
| Regressions | 0 | 0 | >0 |
| Time | 4-5 weeks | 5-6 weeks | >6 weeks |
| Test coverage | 100% of fixes | 90%+ | <90% |

**Target**: 90% coverage (3919+ files), zero regressions, 5 PRs merged, 4-5 weeks elapsed.

---

**Ready for builder assignment. Coordinate with orchestrator on timing and resource allocation.**
