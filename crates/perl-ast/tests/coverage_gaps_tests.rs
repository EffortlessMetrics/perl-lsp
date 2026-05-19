//! Targeted coverage tests for `perl-ast` branches not exercised by existing test files.
//!
//! Covers:
//! - `format_binary_operator` for operators not in the existing test suite
//!   (xor, lt, le, gt, ge, cmp, x, &, |, ^, <<, >>, {}, [], ->{}, ->[], ...) and the `_` fallback
//! - `format_unary_operator` `_` fallback for unknown operators
//! - `NodeKind::Class` with non-empty `parents` (`:isa(...)` branch in `to_sexp`)
//! - `Node::span_len` saturating-sub when `end < start` (malformed span)
//! - `Node::span_len` zero-length span
//! - `Node::contains_offset` exact boundary values

use perl_ast::ast::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn num(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(0, value.len()))
}

fn block_empty() -> Node {
    Node::new(NodeKind::Block { statements: vec![] }, loc(0, 1))
}

fn make_binary(op: &str) -> Node {
    Node::new(
        NodeKind::Binary {
            op: op.to_string(),
            left: Box::new(num("1")),
            right: Box::new(num("2")),
        },
        loc(0, 5),
    )
}

// ===========================================================================
// 1. format_binary_operator — string-comparison operators
// ===========================================================================

#[test]
fn sexp_binary_lt_operator() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("lt").to_sexp();
    assert!(sexp.starts_with("(binary_lt "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_le_operator() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("le").to_sexp();
    assert!(sexp.starts_with("(binary_le "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_gt_operator() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("gt").to_sexp();
    assert!(sexp.starts_with("(binary_gt "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_ge_operator() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("ge").to_sexp();
    assert!(sexp.starts_with("(binary_ge "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_cmp_operator() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("cmp").to_sexp();
    assert!(sexp.starts_with("(binary_cmp "), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 2. format_binary_operator — logical xor
// ===========================================================================

#[test]
fn sexp_binary_xor_operator() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("xor").to_sexp();
    assert!(sexp.starts_with("(binary_xor "), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 3. format_binary_operator — bitwise operators
// ===========================================================================

#[test]
fn sexp_binary_bitwise_and() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("&").to_sexp();
    assert!(sexp.starts_with("(binary_& "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_bitwise_or() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("|").to_sexp();
    assert!(sexp.starts_with("(binary_| "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_bitwise_xor() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("^").to_sexp();
    assert!(sexp.starts_with("(binary_^ "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_left_shift() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("<<").to_sexp();
    assert!(sexp.starts_with("(binary_<< "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_right_shift() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary(">>").to_sexp();
    assert!(sexp.starts_with("(binary_>> "), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 4. format_binary_operator — string repetition (x), range (...),
//    hash/array subscript ({},[]), arrow deref (->{},->[] )
// ===========================================================================

#[test]
fn sexp_binary_string_repetition_x() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("x").to_sexp();
    assert!(sexp.starts_with("(binary_x "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_exclusive_range() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("...").to_sexp();
    assert!(sexp.starts_with("(binary_... "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_hash_subscript() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("{}").to_sexp();
    assert!(sexp.starts_with("(binary_{} "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_array_subscript() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("[]").to_sexp();
    assert!(sexp.starts_with("(binary_[] "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_arrow_hash_deref() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("->{}").to_sexp();
    assert!(sexp.starts_with("(arrow_hash_deref "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_arrow_array_deref() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_binary("->[]").to_sexp();
    assert!(sexp.starts_with("(arrow_array_deref "), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 5. format_binary_operator — fallthrough `_` default branch
// ===========================================================================

#[test]
fn sexp_binary_unknown_operator_uses_fallback_format() -> Result<(), Box<dyn std::error::Error>> {
    // An operator not listed in any arm — should hit the `_ => format!("binary_{}", ...)` arm
    let sexp = make_binary("not_a_real_op").to_sexp();
    assert!(sexp.starts_with("(binary_not_a_real_op "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_binary_operator_with_spaces_in_name_replaces_spaces()
-> Result<(), Box<dyn std::error::Error>> {
    // Verify the `op.replace(' ', "_")` in the default arm is exercised
    let sexp = make_binary("op with spaces").to_sexp();
    assert!(sexp.starts_with("(binary_op_with_spaces "), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 6. format_unary_operator — fallthrough `_` default branch
// ===========================================================================

fn make_unary(op: &str) -> Node {
    Node::new(NodeKind::Unary { op: op.to_string(), operand: Box::new(num("1")) }, loc(0, 5))
}

#[test]
fn sexp_unary_unknown_operator_uses_fallback_format() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_unary("some_custom_op").to_sexp();
    assert!(sexp.starts_with("(unary_some_custom_op "), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_unary_unknown_operator_with_spaces_replaces_spaces()
-> Result<(), Box<dyn std::error::Error>> {
    let sexp = make_unary("my op").to_sexp();
    assert!(sexp.starts_with("(unary_my_op "), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 7. NodeKind::Class with non-empty parents — the `:isa(...)` branch
// ===========================================================================

#[test]
fn sexp_class_with_single_parent() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Class {
            name: "Animal".to_string(),
            parents: vec!["Mammal".to_string()],
            body: Box::new(block_empty()),
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains(":isa("), "expected :isa( in sexp, got: {sexp}");
    assert!(sexp.contains("Mammal"), "expected parent name, got: {sexp}");
    Ok(())
}

#[test]
fn sexp_class_with_multiple_parents() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Class {
            name: "Mule".to_string(),
            parents: vec!["Horse".to_string(), "Donkey".to_string()],
            body: Box::new(block_empty()),
        },
        loc(0, 40),
    );
    let sexp = node.to_sexp();
    // Parents are joined with commas inside :isa(...)
    assert!(sexp.contains(":isa(Horse,Donkey)"), "expected comma-joined parents, got: {sexp}");
    Ok(())
}

#[test]
fn sexp_class_without_parents_has_no_isa() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Class {
            name: "Standalone".to_string(),
            parents: vec![],
            body: Box::new(block_empty()),
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(!sexp.contains(":isa("), "no-parent class must not contain :isa, got: {sexp}");
    assert!(sexp.starts_with("(class Standalone "), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 8. Node::span_len — saturating-sub edge cases
// ===========================================================================

#[test]
fn span_len_normal_span() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Undef, loc(5, 10));
    assert_eq!(node.span_len(), 5);
    Ok(())
}

#[test]
fn span_len_zero_length_span() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Undef, loc(7, 7));
    assert_eq!(node.span_len(), 0);
    Ok(())
}

#[test]
fn span_len_inverted_span_saturates_to_zero() -> Result<(), Box<dyn std::error::Error>> {
    // end < start — saturating_sub prevents underflow, returns 0
    let node = Node::new(NodeKind::Undef, loc(10, 3));
    assert_eq!(node.span_len(), 0);
    Ok(())
}

// ===========================================================================
// 9. Node::contains_offset — boundary values
// ===========================================================================

#[test]
fn contains_offset_start_is_inclusive() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Undef, loc(5, 10));
    assert!(node.contains_offset(5), "start should be inclusive");
    Ok(())
}

#[test]
fn contains_offset_end_is_exclusive() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Undef, loc(5, 10));
    assert!(!node.contains_offset(10), "end should be exclusive");
    Ok(())
}

#[test]
fn contains_offset_just_before_start_returns_false() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Undef, loc(5, 10));
    assert!(!node.contains_offset(4), "offset before start must be false");
    Ok(())
}

#[test]
fn contains_offset_just_before_end_returns_true() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Undef, loc(5, 10));
    assert!(node.contains_offset(9), "offset one before end must be true");
    Ok(())
}

#[test]
fn contains_offset_zero_length_span_is_always_false() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Undef, loc(5, 5));
    // Zero-length span: start == end, so no offset satisfies start <= off < end
    assert!(!node.contains_offset(5), "zero-length span contains no offset, not even start");
    assert!(!node.contains_offset(4));
    assert!(!node.contains_offset(6));
    Ok(())
}
