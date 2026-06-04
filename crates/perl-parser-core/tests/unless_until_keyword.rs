//! Tests for keyword identity preservation on block `unless`/`until`.
//!
//! Verifies that:
//! - Block `unless` sets `keyword: Some("unless")` on the `If` node
//! - Block `until` sets `keyword: Some("until")` on the `While` node
//! - The condition is still negated (unary `!` preserved)
//! - Plain `if`/`while`/`elsif`/`else` carry `keyword: None`
//! - Postfix `unless`/`until` modifiers are unchanged
//! - Nested and elsif-on-unless constructs work correctly
//!
//! Issue #710 — Approach B: identity tag on existing If/While NodeKind.

mod cpan_test_helpers;

use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

/// Parse source and return the root node.
fn parse(source: &str) -> perl_parser_core::Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

/// Extract the first top-level statement from a Program node.
fn first_stmt(source: &str) -> perl_parser_core::Node {
    let root = parse(source);
    match root.kind {
        NodeKind::Program { mut statements } => {
            assert!(!statements.is_empty(), "expected at least one statement");
            statements.remove(0)
        }
        other => panic!("expected Program, got {}", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// 1. Block `unless` retains keyword
// ---------------------------------------------------------------------------

#[test]
fn block_unless_carries_keyword_tag() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"unless ($x) { print "no" }"#);
    match &node.kind {
        NodeKind::If { keyword, .. } => {
            assert_eq!(
                keyword.as_deref(),
                Some("unless"),
                "expected keyword 'unless', got {:?}",
                keyword
            );
        }
        other => {
            return Err(format!("expected If node, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Block `unless` sexp uses "unless" not "if"
// ---------------------------------------------------------------------------

#[test]
fn block_unless_sexp_uses_unless_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"unless ($x) { print "no" }"#);
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(unless "), "sexp should start with '(unless', got: {sexp}");
    assert!(
        !sexp.starts_with("(if "),
        "sexp must not start with '(if' for unless block, got: {sexp}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Block `until` retains keyword
// ---------------------------------------------------------------------------

#[test]
fn block_until_carries_keyword_tag() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"until ($done) { work() }"#);
    match &node.kind {
        NodeKind::While { keyword, .. } => {
            assert_eq!(
                keyword.as_deref(),
                Some("until"),
                "expected keyword 'until', got {:?}",
                keyword
            );
        }
        other => {
            return Err(format!("expected While node, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Block `until` sexp uses "until" not "while"
// ---------------------------------------------------------------------------

#[test]
fn block_until_sexp_uses_until_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"until ($done) { work() }"#);
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(until "), "sexp should start with '(until', got: {sexp}");
    assert!(
        !sexp.starts_with("(while "),
        "sexp must not start with '(while' for until block, got: {sexp}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Block `if` has no keyword tag (None)
// ---------------------------------------------------------------------------

#[test]
fn block_if_has_no_keyword_tag() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"if ($x) { print "yes" }"#);
    match &node.kind {
        NodeKind::If { keyword, .. } => {
            assert_eq!(
                keyword.as_deref(),
                None,
                "plain 'if' must have keyword: None, got {:?}",
                keyword
            );
        }
        other => {
            return Err(format!("expected If node, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Block `while` has no keyword tag (None)
// ---------------------------------------------------------------------------

#[test]
fn block_while_has_no_keyword_tag() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"while ($x) { work() }"#);
    match &node.kind {
        NodeKind::While { keyword, .. } => {
            assert_eq!(
                keyword.as_deref(),
                None,
                "plain 'while' must have keyword: None, got {:?}",
                keyword
            );
        }
        other => {
            return Err(format!("expected While node, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. `unless` with elsif/else chain: keyword preserved and branches intact
// ---------------------------------------------------------------------------

#[test]
fn block_unless_with_elsif_and_else_chain() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"unless ($x) { A() } elsif ($y) { B() } else { C() }"#);
    match &node.kind {
        NodeKind::If { keyword, elsif_branches, else_branch, .. } => {
            assert_eq!(
                keyword.as_deref(),
                Some("unless"),
                "keyword should be 'unless', got {:?}",
                keyword
            );
            assert_eq!(elsif_branches.len(), 1, "expected 1 elsif branch");
            assert!(else_branch.is_some(), "expected else branch");
        }
        other => {
            return Err(format!("expected If node, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 8. Condition is still negated (unary_not present in sexp)
// ---------------------------------------------------------------------------

#[test]
fn block_unless_condition_is_still_negated() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"unless ($x) { }"#);
    let sexp = node.to_sexp();
    assert!(sexp.contains("unary_not"), "condition must still be negated (unary_not), got: {sexp}");
    Ok(())
}

#[test]
fn block_until_condition_is_still_negated() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"until ($done) { }"#);
    let sexp = node.to_sexp();
    assert!(sexp.contains("unary_not"), "condition must still be negated (unary_not), got: {sexp}");
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. Postfix `unless` modifier is unchanged
// ---------------------------------------------------------------------------

#[test]
fn postfix_unless_modifier_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"print "no" unless $x;"#);
    match &node.kind {
        NodeKind::StatementModifier { modifier, .. } => {
            assert_eq!(
                modifier.as_str(),
                "unless",
                "postfix unless modifier should be 'unless', got: {modifier}"
            );
        }
        other => {
            return Err(
                format!("expected StatementModifier node, got {}", other.kind_name()).into()
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 10. Postfix `until` modifier is unchanged
// ---------------------------------------------------------------------------

#[test]
fn postfix_until_modifier_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"work() until $done;"#);
    match &node.kind {
        NodeKind::StatementModifier { modifier, .. } => {
            assert_eq!(
                modifier.as_str(),
                "until",
                "postfix until modifier should be 'until', got: {modifier}"
            );
        }
        other => {
            return Err(
                format!("expected StatementModifier node, got {}", other.kind_name()).into()
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 11. Clean parse — no error nodes anywhere
// ---------------------------------------------------------------------------

#[test]
fn unless_block_parses_cleanly() {
    cpan_test_helpers::assert_clean_parse(r#"unless ($x) { print "no" }"#);
}

#[test]
fn until_block_parses_cleanly() {
    cpan_test_helpers::assert_clean_parse(r#"until ($done) { work() }"#);
}

#[test]
fn orphaned_else_recovery_has_no_keyword_tag() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"else { recover() }"#);
    match &node.kind {
        NodeKind::If { keyword, .. } => {
            assert_eq!(keyword.as_deref(), None);
        }
        other => {
            return Err(format!("expected recovered If node, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}

#[test]
fn orphaned_elsif_recovery_has_no_keyword_tag() -> Result<(), Box<dyn std::error::Error>> {
    let node = first_stmt(r#"elsif ($x) { recover() } else { fallback() }"#);
    match &node.kind {
        NodeKind::If { keyword, elsif_branches, else_branch, .. } => {
            assert_eq!(keyword.as_deref(), None);
            assert!(elsif_branches.is_empty());
            assert!(else_branch.is_some());
        }
        other => {
            return Err(format!("expected recovered If node, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}
