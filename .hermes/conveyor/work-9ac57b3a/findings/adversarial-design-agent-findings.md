# Adversarial Design Findings — work-9ac57b3a

## Current Approach

The plan proposes to fix `expected_colon` errors by adding **3-token lookahead** to `is_label_start()` in `statements.rs`. The function currently checks only `Identifier Colon` (2 tokens) and returns true. The proposed fix checks a 3rd token after the colon: if the token after colon is `Question`, `Colon`, `Semicolon`, `Comma`, `FatArrow`, `RightParen`, `RightBracket`, `RightBrace`, or `Eof`, then it's NOT a label start.

The rationale: Perl labels MUST be followed by a statement, so if the token after the colon cannot start a statement, the colon belongs to something else (ternary, hash constructor, etc.).

## Alternative Approaches

### Alternative 1: Tentative Parsing with Backtracking in `parse_labeled_statement()`

**Core idea:** Instead of predicting whether a colon is a label colon before consuming it, CONSUME it tentatively in `parse_labeled_statement()` and backtrack if `parse_statement()` fails.

**Why it might be better:**
- No need to guess which tokens can/cannot start a statement
- Handles ALL cases correctly, including edge cases the heuristic might miss
- More robust: if label parsing fails, the parser state is restored and another interpretation is tried
- The colon consumption is the point of no return — parsing past it should confirm the decision

**Why it might be worse:**
- Requires implementing backtracking infrastructure (saving/restoring parser state)
- May have performance cost if backtracking is frequent
- More complex implementation than a simple heuristic

**What it sacrifices:**
- The clean 2-token lookahead of `is_label_start()` — which is fast and works for the common case
- But: the common case (valid labels) is already handled correctly; only edge cases fail

---

### Alternative 2: Defer Label Check to `parse_labeled_statement()` Itself

**Core idea:** Move the disambiguation logic from `is_label_start()` (a pure lookahead check) into `parse_labeled_statement()` (which actually consumes tokens). If what follows the colon can't start a statement, signal failure and let the caller (statement parser) try another interpretation.

**Why it might be better:**
- The decision happens at the right level — where tokens are actually consumed
- No need for 3-token lookahead (which is already used in `is_indirect_call_pattern` for a different purpose)
- More composable: `parse_labeled_statement()` returns an error rather than silently consuming wrong tokens
- Doesn't affect the statement-level `is_label_start()` call, which is correct for the majority of cases

**Why it might be worse:**
- `parse_labeled_statement()` would need to check if the next token can start a statement before committing
- Still requires some form of lookahead
- More coupling: the label parsing knows about statement structure

**What it sacrifices:**
- The simplicity of `is_label_start()` as a pure 2-token predicate
- But: simplicity at the wrong layer is a liability, not a virtue

---

### Alternative 3: Improve Indirect Call Detection for User Functions in Expression Context

**Core idea:** The `expected_colon` error might not be about labels at all — it might be that user-defined function calls without parentheses (like `camelize $name` in a ternary's then-branch) are not being recognized as indirect calls, causing the expression parser to fail to consume arguments, leading to unexpected token sequences.

**Why it might be better:**
- `is_indirect_call_pattern()` currently only enables indirect call detection for non-builtins when `at_stmt_start` is true (line 18 of calls.rs: `if !self.at_stmt_start && !is_filehandle_builtin && name != "new" { return false; }`)
- Inside a ternary's then-branch, `at_stmt_start` is false — so user functions like `camelize` fall through to `parse_expression()`, not `parse_indirect_call()`
- If this is wrong, the fix is to improve indirect call detection in expression contexts, not to restrict `is_label_start()`

**Why it might be worse:**
- This is a different bug than the one described (ternary/label disambiguation)
- Would require significant changes to expression parsing
- May have unintended consequences for other expression contexts

**What it sacrifices:**
- The current behavior that `is_indirect_call_pattern()` is conservative outside statement start
- But: this conservatism might be causing the bug described

## Strongest Argument Against Current Approach

**The proposed 3-token heuristic is fundamentally unsound.** The plan claims:

> "A valid Perl label must be followed by a statement. Examples of what is NOT a valid label start: `foo: ? bar : baz` — 'foo' IDENT Colon → but ? can't start a statement"

This is **incorrect Perl semantics**. In Perl, `foo: ? bar : baz` IS a valid labeled statement where `foo` is the label and `? bar : baz` is the ternary expression as the statement body. The `?` CAN start a statement in Perl — a ternary expression IS a valid statement!

The plan's heuristic would INCORRECTLY reject valid Perl code like:
```perl
foo: ? bar : baz   # foo is label, ternary is the statement
```

The only case where `foo: ? bar` fails is if `? bar` isn't a valid ternary (missing `:`). But that's a separate parse error, not a label error.

Furthermore, the plan lists `RightBrace` as an "invalid 3rd token" for label starts — but `{` CAN start a block statement! The friction log acknowledges this, but the problem is deeper: **the entire heuristic is built on a misunderstanding of what can follow a Perl label.**

## Recommended Action

**Modify the approach substantially, or replace it.**

The 3-token lookahead heuristic as proposed is wrong because:
1. `RightBrace` (actually `{`) can start a statement
2. `?` CAN start a valid statement (ternary expression)
3. The approach treats a symptom (`is_label_start()` returning true) rather than understanding why the colon was consumed incorrectly

**Recommended modification:**

If the 3-token approach is to be used, it must be drastically revised to only reject tokens that genuinely cannot start ANY Perl expression that is also a valid statement. This is a much narrower set than the plan proposes.

**Better alternative:** Consider tentative parsing (Alternative 1) or moving the check to `parse_labeled_statement()` (Alternative 2). These approaches handle the problem at the point where tokens are actually consumed, rather than trying to predict the future.

**Before implementing anything:** The team MUST produce a concrete failing test case from the actual CPAN files. Without a failing test, the fix cannot be validated. The claim that "tests in fix_expected_colon.rs all pass (158/158)" contradicts the premise that the issue is actively failing.

## Long-Term Cost Assessment

**If we do it the current way:**

- **6 months:** The fix is deployed but may not address the actual bug (since CPAN files don't exist and tests pass). Token budget is wasted on a heuristic that might not catch real issues.

- **2 years:** The heuristic becomes technical debt. Developers who don't know Perl semantics deeply will trust the heuristic and add more tokens to the "invalid" list when new edge cases appear, making it ever more incorrect. The codebase will have a misleading function name (`is_label_start`) with a 3-token lookahead that doesn't actually determine if it's a label start.

**If we do tentative parsing or move the check to `parse_labeled_statement()`:**

- **6 months:** More complex implementation but more robust. The parser correctly handles all edge cases from the start.

- **2 years:** No accumulated technical debt. Future developers don't need to understand an arcane heuristic — the code does the right thing at the right place.
