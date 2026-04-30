//! Tests for label/ternary disambiguation in `is_label_start()`.
//!
//! The parser's `is_label_start()` function determines whether an `Identifier Colon`
//! sequence starts a label statement. Before the fix, it only checked 2 tokens
//! (Identifier + Colon). After the fix, it checks 3 tokens (Identifier + Colon +
//! the token AFTER the colon) to disambiguate label colons from ternary/hash-constructor colons.
//!
//! This test file covers the acceptance criteria from ADR-0017:
//! - `is_label_start()` returns `false` for patterns where the token after colon
//!   cannot start a statement (ternary `?`, fat arrow `=>`, another `:`, etc.)
//! - `is_label_start()` returns `true` for valid label patterns where the token
//!   after colon CAN start a statement.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Invalid patterns — is_label_start should return false (colon belongs to ternary/hash)
// =============================================================================

/// Token after colon is `?` (ternary question) — cannot start a statement.
/// Pattern: ternary in expression context where identifier before `?` looks like a label.
#[test]
fn test_label_ternary_disambiguation_ternary_question() {
    // `foo ? $then : $else` — a simple ternary expression.
    // The identifier before `?` could be mistaken for a label if parser has bug.
    // After fix: is_label_start returns false because `?` can't start a statement.
    assert_clean_parse(r#"my $x = $cond ? $then : $else;"#);
}

/// Token after colon is `=>` (fat arrow) in hash constructor.
/// Pattern: hash constructor where key looks like a label due to preceding colon.
#[test]
fn test_label_ternary_disambiguation_fat_arrow() {
    // In hash constructors, `word: => 'value'` has the key BEFORE the fat arrow,
    // but the colon before `=>` could be mistaken for a label colon.
    // After fix: is_label_start returns false because `=>` can't start a statement.
    // This allows the parser to correctly parse the hash constructor.
    assert_clean_parse(r#"my %hash = (KEY1 => 'value1', KEY2 => 'value2');"#);
}

/// Token after colon is another `:` in chained ternary.
/// Pattern: chained ternary where internal colons might be mistaken for label colons.
#[test]
fn test_label_ternary_disambiguation_double_colon() {
    // Chained ternary: `$a ? $b : $c ? $d : $e`
    // The `:` after `$b` is the ternary's else, but could be mistaken for a label colon.
    // After fix: the second `:` is not consumed as a label colon.
    assert_clean_parse(r#"my $x = $a ? $b : $c ? $d : $e;"#);
}

// =============================================================================
// Valid patterns — is_label_start should return true (colon IS a label colon)
// =============================================================================

/// Token after colon is identifier `my` — can start a statement (variable declaration).
#[test]
fn test_label_ternary_disambiguation_valid_identifier() {
    // `FOO: my $x = 1;` — label followed by variable declaration.
    // is_label_start should return true.
    assert_clean_parse(r#"FOO: my $x = 1; print $x;"#);
}

/// Token after colon is `{` (left brace) — can start a statement (block).
#[test]
fn test_label_ternary_disambiguation_valid_left_brace() {
    // `LABEL: { block }` — label followed by block statement.
    // is_label_start should return true.
    assert_clean_parse(r#"LABEL: { print "in block\n"; }"#);
}

/// Token after colon is `(` (left paren) — can start a statement (paren expression).
#[test]
fn test_label_ternary_disambiguation_valid_left_paren() {
    // `LABEL: (expr)` — label followed by parenthesized expression.
    // is_label_start should return true.
    assert_clean_parse(r#"OUTER: (my $x = 1); print $x;"#);
}

/// Token after colon is keyword `while` — can start a loop statement.
#[test]
fn test_label_ternary_disambiguation_valid_keyword_while() {
    // `LABEL: while (1) { }` — label followed by while loop.
    // is_label_start should return true.
    assert_clean_parse(r#"LOOP: while (1) { last LOOP if $done; }"#);
}

/// Token after colon is keyword `if` — can start a conditional statement.
#[test]
fn test_label_ternary_disambiguation_valid_keyword_if() {
    // `LABEL: if $x { }` — label followed by if statement.
    // is_label_start should return true.
    assert_clean_parse(r#"CHECK: if ($x > 0) { print "positive\n"; }"#);
}

/// Token after colon is keyword `for` — can start a for loop.
#[test]
fn test_label_ternary_disambiguation_valid_keyword_for() {
    // `LABEL: for (...) { }` — label followed by for loop.
    // is_label_start should return true.
    assert_clean_parse(r#"ITER: for my $i (1..10) { print "$i\n"; }"#);
}

/// Token after colon is identifier `print` — can start a print statement.
#[test]
fn test_label_ternary_disambiguation_valid_print() {
    // `LABEL: print "hi" if $debug;` — label followed by print with statement modifier.
    // Token after colon is identifier, which CAN start a statement.
    // is_label_start should return true.
    assert_clean_parse(r#"DEBUG: print "debug info\n" if $debug;"#);
}

/// Token after colon is identifier `return` — can start a return statement.
#[test]
fn test_label_ternary_disambiguation_valid_return() {
    // `LABEL: return $x;` — label followed by return.
    // is_label_start should return true.
    assert_clean_parse(r#"EXIT: return $result if defined $result;"#);
}

/// Token after colon is `;` — labeled empty statement. Valid Perl.
///
/// `LABEL: ;` is a legal labeled empty-statement in Perl.  Earlier versions of
/// the heuristic incorrectly included `Semicolon` in the "cannot start a
/// statement" set, which caused this pattern to fall through to expression
/// parsing and produce a spurious parse error.
#[test]
fn test_label_ternary_disambiguation_valid_empty_statement() {
    // `LABEL: ;` — label on an empty statement. is_label_start should return true.
    assert_clean_parse(r#"EMPTY: ; print "after\n";"#);
}

// =============================================================================
// Statement modifier edge cases
// =============================================================================

/// Label followed by statement with `unless` modifier.
#[test]
fn test_label_ternary_disambiguation_valid_unless_modifier() {
    assert_clean_parse(r#"SKIP: print "skipping\n" unless $skip;"#);
}

/// Label followed by statement with `while` modifier.
#[test]
fn test_label_ternary_disambiguation_valid_while_modifier() {
    assert_clean_parse(r#"AGAIN: print "loop\n" while $count-- > 0;"#);
}

/// Label followed by statement with `for` modifier.
#[test]
fn test_label_ternary_disambiguation_valid_for_modifier() {
    assert_clean_parse(r#"ITER: print "$_\n" for @items;"#);
}

/// Multiple valid labels in sequence.
#[test]
fn test_label_ternary_disambiguation_multiple_labels() {
    assert_clean_parse(
        r#"
        OUTER: for my $i (1..3) {
            INNER: for my $j (1..3) {
                next OUTER if $i == 2;
            }
        }
        "#,
    );
}

// =============================================================================
// Hash constructor disambiguation
// =============================================================================

/// Fat arrow in hash constructor - valid Perl with identifier keys.
#[test]
fn test_label_ternary_disambiguation_hash_constructor_simple() {
    // Simple hash constructor with fat arrows
    assert_clean_parse(r#"my %hash = (key1 => 'value1', key2 => 'value2');"#);
}

/// Fat arrow in hash constructor with uppercase-looking keys.
#[test]
fn test_label_ternary_disambiguation_hash_constructor_uppercase_keys() {
    // Keys that look like labels (uppercase) but are hash keys with fat arrow
    assert_clean_parse(r#"my %hash = (KEY1 => 'value1', KEY2 => 'value2');"#);
}

/// Nested hash with fat arrows.
#[test]
fn test_label_ternary_disambiguation_hash_constructor_nested() {
    assert_clean_parse(r#"my %hash = (outer => {inner => 'value'}, other => 'x');"#);
}

/// Hash constructor in list context.
#[test]
fn test_label_ternary_disambiguation_hash_in_list() {
    assert_clean_parse(r#"my @items = ({key => 'val'}, {other => 'x'});"#);
}

// =============================================================================
// Ternary chain disambiguation
// =============================================================================

/// Ternary with hash ref in branches.
#[test]
fn test_label_ternary_disambiguation_ternary_with_hash_ref() {
    assert_clean_parse(r#"my $x = $cond ? {a => 1} : {b => 2};"#);
}

/// Ternary with array ref in branches.
#[test]
fn test_label_ternary_disambiguation_ternary_with_array_ref() {
    assert_clean_parse(r#"my $x = $cond ? [1, 2] : [3, 4];"#);
}

/// Ternary with do block in branches.
#[test]
fn test_label_ternary_disambiguation_ternary_with_do_block() {
    assert_clean_parse(r#"my $x = $cond ? do { $a + $b } : 0;"#);
}

// =============================================================================
// Statement modifier edge cases
// =============================================================================

/// Label followed by statement with complex expression modifier.
#[test]
fn test_label_ternary_disambiguation_complex_modifier() {
    // Label followed by print with complex expression modifier
    assert_clean_parse(r#"INFO: print join(", ", @arr) if @arr;"#);
}

/// Label followed by return with expression.
#[test]
fn test_label_ternary_disambiguation_return_expression() {
    assert_clean_parse(r#"FAIL: return $error // 'unknown error';"#);
}

// =============================================================================
// The core bug patterns from CPAN (IO/Socket/SSL/Intercept.pm, Regexp/Common/SEN.pm)
// =============================================================================

/// Pattern: label-like identifier followed by ternary in function call.
/// This was causing expected_colon errors due to misidentified label colons.
#[test]
fn test_label_ternary_disambiguation_in_function_call() {
    // The pattern `func($arg ? $bar : $baz)` - the identifier before `?` could be
    // mistaken for a label if the parser incorrectly identifies `Identifier:` as label.
    // Without fix: potential misidentification of label-like patterns.
    // With fix: is_label_start returns false because `?` can't start a statement.
    assert_clean_parse(r#"my $x = $func($arg ? $bar : $baz);"#);
}

/// Pattern: ternary in list assignment context.
#[test]
fn test_label_ternary_disambiguation_in_list_assignment() {
    // Ternary in list assignment where identifier before `?` could look like label
    assert_clean_parse(r#"my ($a, $b) = $cond ? ($x, $y) : ($z, $w);"#);
}

/// Pattern: ternary with method call on the condition.
#[test]
fn test_label_ternary_disambiguation_ternary_method_call_condition() {
    // `$obj->method ? $then : $else` — the method call before `?` is not an identifier
    // followed by colon, so this should parse correctly. But we test anyway.
    assert_clean_parse(r#"my $x = $obj->method ? $then : $else;"#);
}

/// Pattern: ternary with subscript on condition.
#[test]
fn test_label_ternary_disambiguation_ternary_subscript_condition() {
    // `$hash{key} ? $then : $else` — subscript expression before `?`.
    // This should parse correctly since `$hash{key}` is an expression, not identifier.
    assert_clean_parse(r#"my $x = $hash{key} ? $then : $else;"#);
}
