//! Snapshot tests for label/ternary disambiguation in `is_label_start()`.
//!
//! These tests capture the S-expression output of the parser for representative
//! Perl snippets involving labels, ternary operators, and hash constructors.
//! Any change to the parser output will be detected by these snapshots.
//!
//! Run `INSTA_UPDATE=always cargo test -p perl-parser-core --test snapshot_label_ternary_disambiguation`
//! to update snapshots when output changes intentionally.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Valid Label Patterns - These should parse as labels
// =============================================================================

/// Valid label: identifier followed by colon and a statement start
#[test]
fn snapshot_valid_label_with_identifier() {
    let source = "LABEL: my $x = 1;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("valid_label_with_identifier", sexp);
}

/// Valid label: identifier followed by colon and a block start
#[test]
fn snapshot_valid_label_with_block() {
    let source = "LABEL: { 1; }";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("valid_label_with_block", sexp);
}

/// Valid label: identifier followed by colon and a keyword statement
#[test]
fn snapshot_valid_label_with_keyword_statement() {
    let source = "LABEL: if (1) { 1; }";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("valid_label_with_keyword_statement", sexp);
}

/// Valid label: identifier followed by colon and a loop
#[test]
fn snapshot_valid_label_with_loop() {
    let source = "LABEL: while (1) { last; }";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("valid_label_with_loop", sexp);
}

/// Valid label: identifier followed by colon and a print statement
#[test]
fn snapshot_valid_label_with_print() {
    let source = "LABEL: print 'hello';";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("valid_label_with_print", sexp);
}

/// Valid label: identifier followed by colon and a statement modifier
#[test]
fn snapshot_valid_label_with_modifier() {
    let source = "LABEL: print 'hi' if $debug;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("valid_label_with_modifier", sexp);
}

// =============================================================================
// Invalid Label Patterns (Ternary/Hash Context) - These should NOT produce expected_colon
// =============================================================================

/// Invalid label pattern: ternary condition looks like label
#[test]
fn snapshot_ternary_condition_as_label() {
    let source = "my $x = COND: ? $then : $else;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("ternary_condition_as_label", sexp);
}

/// Invalid label pattern: chained ternary
#[test]
fn snapshot_chained_ternary() {
    let source = "my $x = $a ? 1 : $b ? 2 : 3;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("chained_ternary", sexp);
}

/// Invalid label pattern: nested ternary
#[test]
fn snapshot_nested_ternary() {
    let source = "my $x = $a ? $b ? 1 : 2 : 3;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("nested_ternary", sexp);
}

/// Invalid label pattern: hash constructor with fat arrow
#[test]
fn snapshot_hash_constructor_fat_arrow() {
    let source = "my %h = (KEY1 => 'value1', KEY2 => 'value2');";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("hash_constructor_fat_arrow", sexp);
}

/// Invalid label pattern: keyword as hash key
#[test]
fn snapshot_keyword_as_hash_key() {
    let source = "my %h = (if => 1, for => 2, return => 3);";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("keyword_as_hash_key", sexp);
}

// =============================================================================
// Edge Cases - Invalid 3rd tokens
// =============================================================================

/// Edge case: identifier colon followed by another colon (chained ternary else)
#[test]
fn snapshot_identifier_colon_colon() {
    let source = "FOO: :";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("identifier_colon_colon", sexp);
}

/// Edge case: identifier colon followed by semicolon
#[test]
fn snapshot_identifier_colon_semicolon() {
    let source = "FOO: ;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("identifier_colon_semicolon", sexp);
}

/// Edge case: identifier colon followed by closing paren
#[test]
fn snapshot_identifier_colon_paren() {
    let source = "FOO: )";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("identifier_colon_paren", sexp);
}

/// Edge case: identifier colon at EOF
#[test]
fn snapshot_identifier_colon_eof() {
    let source = "FOO:";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("identifier_colon_eof", sexp);
}

/// Edge case: identifier colon followed by question (ternary)
#[test]
fn snapshot_identifier_colon_question() {
    let source = "FOO: ?";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("identifier_colon_question", sexp);
}

// =============================================================================
// Control Flow Labels - These should still work
// =============================================================================

/// Control flow: last with label
#[test]
fn snapshot_label_last() {
    let source = "OUTER: for my $i (1..10) { last OUTER if $i == 5; }";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("label_last", sexp);
}

/// Control flow: next with label
#[test]
fn snapshot_label_next() {
    let source = "OUTER: for my $i (1..10) { next OUTER if $i == 5; }";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("label_next", sexp);
}

/// Control flow: redo with label
#[test]
fn snapshot_label_redo() {
    let source = "OUTER: for my $i (1..10) { redo OUTER if $i == 5; }";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("label_redo", sexp);
}

// =============================================================================
// Statement Modifiers with Labels
// =============================================================================

/// Label with unless modifier
#[test]
fn snapshot_label_unless_modifier() {
    let source = "LABEL: 1 unless $y;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("label_unless_modifier", sexp);
}

/// Label with while modifier
#[test]
fn snapshot_label_while_modifier() {
    let source = "LABEL: 1 while $count-- > 0;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("label_while_modifier", sexp);
}

/// Label with until modifier
#[test]
fn snapshot_label_until_modifier() {
    let source = "LABEL: 1 until $done;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("label_until_modifier", sexp);
}

/// Label with for modifier
#[test]
fn snapshot_label_for_modifier() {
    let source = "LABEL: 1 for @items;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("label_for_modifier", sexp);
}

// =============================================================================
// Ternary with Complex Conditions
// =============================================================================

/// Ternary with method call condition
#[test]
fn snapshot_ternary_method_call_condition() {
    let source = "my $x = $obj->method ? $a : $b;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("ternary_method_call_condition", sexp);
}

/// Ternary with subscript condition
#[test]
fn snapshot_ternary_subscript_condition() {
    let source = "my $x = $hash{key} ? $a : $b;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("ternary_subscript_condition", sexp);
}

/// Ternary with array subscript condition
#[test]
fn snapshot_ternary_array_subscript_condition() {
    let source = "my $x = $arr[0] ? $a : $b;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("ternary_array_subscript_condition", sexp);
}

/// Ternary with postfix deref condition
#[test]
fn snapshot_ternary_postfix_deref_condition() {
    let source = "my $x = $obj->$method ? $a : $b;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("ternary_postfix_deref_condition", sexp);
}

// =============================================================================
// Complex Hash and Ternary Mixes
// =============================================================================

/// Hash with ternary values
#[test]
fn snapshot_hash_with_ternary_values() {
    let source = "my %h = (key => $x ? $a : $b);";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("hash_with_ternary_values", sexp);
}

/// Ternary with hash refs in branches
#[test]
fn snapshot_ternary_with_hash_refs() {
    let source = "my $x = $cond ? {a => 1} : {b => 2};";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("ternary_with_hash_refs", sexp);
}

/// Ternary with array refs in branches
#[test]
fn snapshot_ternary_with_array_refs() {
    let source = "my $x = $cond ? [1, 2] : [3, 4];";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    insta::assert_snapshot!("ternary_with_array_refs", sexp);
}
