# Verification Findings — work-9ac57b3a

## Confidence Assessment

**Medium** — The research agent's core analysis of the `is_label_start()` function is correct, and the proposed fix is technically sound. However, there are significant discrepancies in the scope (8 files claimed vs 10 actual, and only 2 of the 4 named files are actually in the baseline). Additionally, the tests in `fix_expected_colon.rs` all pass (158/158), which contradicts the claim that the issue is actively causing failures.

## Confirmed Findings

### Finding 1: `is_label_start()` is too permissive (CONFIRMED)

**Evidence:**
- File: `crates/perl-parser-core/src/engine/parser/statements.rs` at lines 1030-1047
- The function only checks for `Identifier Colon` (2-token lookahead)
- It does NOT verify that what follows the colon can start a statement
- The comment at line 1042-1043 says: "Qualified identifiers use `::` which tokenizes as DoubleColon, so `Identifier Colon` (single colon) is unambiguously a label" — but this is incorrect because it doesn't account for ternary operators, hash constructors, or other expression contexts where colon is not a label

**Code reference:**
```rust
fn is_label_start(&mut self) -> bool {
    if self.peek_kind() != Some(TokenKind::Identifier) {
        return false;
    }
    if let Ok(second_token) = self.tokens.peek_second() {
        if second_token.kind == TokenKind::Colon {
            return true;  // <-- ONLY checks Identifier Colon, no 3rd token check
        }
    }
    false
}
```

### Finding 2: `peek_third()` exists and is usable (CONFIRMED)

**Evidence:**
- File: `crates/perl-tokenizer/src/token_stream.rs` at lines 213-225
- `peek_third()` is already implemented and cached (same pattern as `peek_second()`)
- The method is already used in several places in the codebase (e.g., `expressions/calls.rs` lines 28, 109, 125, 126, 156, 281)

### Finding 3: The fix logic is sound (CONFIRMED)

The proposed fix to check if the 3rd token can start a statement is correct because:
- Perl labels MUST be followed by a statement
- If the token after `:` cannot start a statement (`?`, `:`, `;`, `,`, `=>`, `)`, `]`, `}`, `Eof`), then the colon is NOT a label colon
- This aligns with how `is_stmt_modifier_kind` works (file: `engine/parser/helpers.rs` lines 8-19)

## Corrected Findings

### Correction 1: Scope is wrong — 10 files, not 8 (and 2 named files are wrong)

**Research agent claim:** 4 unique CPAN files (each appearing twice = 8 entries):
- `/usr/share/perl5/IO/Socket/SSL/Intercept.pm`
- `/usr/share/perl5/Mail/Address.pm`
- `/usr/share/perl5/Parse/RecDescent.pm`
- `/usr/share/perl5/Regexp/Common/SEN.pm`

**Actual baseline (`.ci/cpan-corpus-baseline.json`):** 10 unique files (20 entries total):
- `IO/Socket/SSL/Intercept.pm` ✓ (correct)
- `Mojo/Log.pm` ✗ (not mentioned)
- `Mojolicious/Plugin/DefaultHelpers.pm` ✗ (not mentioned)
- `Mojolicious/Routes.pm` ✗ (not mentioned)
- `POE/Test/Loops/z_leolo_wheel_run.pm` ✗ (not mentioned)
- `Regexp/Common/SEN.pm` ✓ (correct)
- `Sort/BySpec.pm` ✗ (not mentioned)
- `Test/Needs.pm` ✗ (not mentioned)
- `AnyEvent/Handle.pm` ✗ (not mentioned)
- `AnyEvent/Log.pm` ✗ (not mentioned)

`Mail/Address.pm` and `Parse/RecDescent.pm` are NOT in the baseline at all.

### Correction 2: Tests pass but baseline still shows errors (CONTRADICTION)

**Research agent claim:** Tests in `fix_expected_colon.rs` represent failing patterns that need to be fixed.

**Verification result:**
```bash
$ cargo test -p perl-parser-core --test fix_expected_colon
test result: ok. 158 passed; 0 failed; 0 ignored; 0 measured
```

**However**, the baseline still shows 10 files with `expected_colon` errors. This suggests either:
1. The tests were written in anticipation of a fix that hasn't been implemented yet
2. The tests cover patterns that are ALREADY fixed
3. The baseline is stale and hasn't been updated

### Correction 3: The specific test pattern cited doesn't match the issue

**Research agent says (line 114-120):**
> But this test currently passes, suggesting the issue is with the `Identifier Colon` pattern specifically, not with the indirect call parsing.

The test `test_ternary_with_user_func_no_parens_in_then` passes:
```rust
assert_clean_parse(r#"my $suffix = $name =~ /^[a-z]/ ? camelize $name : $name;"#);
```

In this pattern, `camelize` is followed by a space and `$name` — there is NO `Identifier Colon` sequence. The colon belongs to the ternary operator. This test does NOT demonstrate the label-disambiguation problem described in the issue.

The problematic pattern would be something like `foo: ? bar : baz` where `foo:` is incorrectly consumed as a label.

## New Findings

### Finding 4: Tests may not cover the actual failing patterns

The test file has 158 passing tests, but the baseline still shows 10 files with `expected_colon` errors. This means the tests in `fix_expected_colon.rs` do NOT cover the patterns that cause failures in the actual CPAN files.

**Specific issue:** The tests in `fix_expected_colon.rs` were likely added as part of issue #4169's fix (which addressed ternary + postfix call disambiguation), but the label-disambiguation issue (which requires 3-token lookahead in `is_label_start()`) was NOT addressed by that fix.

### Finding 5: `RightBrace` as an invalid 3rd token may be too aggressive

**Risk:** The proposed fix includes `RightBrace` in the list of invalid tokens after a label colon. However, in Perl, a label CAN be followed by a closing brace in some contexts:

```perl
LABEL: { say "hello" }
```

The `parse_labeled_statement()` function calls `parse_statement()` after consuming the colon, and a `{` can start a block statement. So `RightBrace` should NOT be in the invalid list.

**Recommended fix:** Remove `RightBrace` from the invalid tokens list, or verify that `parse_statement()` can handle `{` as a valid statement start.

### Finding 6: The `Eof` token kind is `Eof` (not `None`)

**Research agent says:** `Eof / None` — end of input

**Actual token kind:** `TokenKind::Eof` (line 365 of `perl-token/src/lib.rs`)

There is no `TokenKind::None` in the codebase.

## Scope Assessment

**Scope Mismatch — Partial**

The issue title says "parser: expected_colon errors suggest ternary/label disambiguation needs improvement (8 files)".

**Reality:**
- The `is_label_start()` function issue is correctly identified
- The scope should be 10 files (not 8), and only 2 of the 4 named files are correct
- The fix requires modifying `is_label_start()` in `statements.rs` to add 3rd-token lookahead
- The test file exists but all tests pass (contradicting the premise that the issue is active)

**Crate scope:** `perl-parser-core` only (correct)

## Verification Methodology

1. **Verified `is_label_start()` logic:** Read `statements.rs` lines 1030-1047 — confirmed it only checks `Identifier Colon` without 3rd-token lookahead
2. **Verified `peek_third()` exists:** Read `token_stream.rs` lines 213-225 — confirmed it exists and is cached
3. **Verified tests pass:** Ran `cargo test -p perl-parser-core --test fix_expected_colon` — 158/158 passed
4. **Verified baseline scope:** Parsed `.ci/cpan-corpus-baseline.json` — found 10 unique files, not 4 as claimed
5. **Verified test patterns:** Searched `fix_expected_colon.rs` for `camelize` patterns — confirmed the passing test doesn't use `Identifier Colon`
6. **Verified statement modifier tokens:** Read `helpers.rs` lines 8-19 to confirm `is_stmt_modifier_kind` definition
7. **Verified token kinds:** Searched `perl-token` for `Eof` and `None` — found `Eof` exists, `None` does not

## Summary

| Claim | Status |
|-------|--------|
| `is_label_start()` is too permissive | ✓ CONFIRMED |
| `peek_third()` exists | ✓ CONFIRMED |
| Fix logic is sound | ✓ CONFIRMED (with caveat about `RightBrace`) |
| 4 unique files in baseline | ✗ WRONG — actually 10 |
| `Mail/Address.pm` and `Parse/RecDescent.pm` are affected | ✗ WRONG — not in baseline |
| Tests fail | ✗ WRONG — 158/158 pass |
| Scope is 8 files | ✗ WRONG — actually 10 files |
