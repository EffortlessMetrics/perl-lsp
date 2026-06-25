//! HIR `if`/`unless` branch lowering tests.
//!
//! These tests pin the branch-shell substrate slice for `if`/`unless` block
//! statements: each construct lowers to exactly one `BranchShell` HIR item,
//! child constructs inside the branch bodies recurse and produce their own
//! HIR items, and every emitted item carries a valid source range and scope
//! context.
//!
//! The implementation lives in `crates/perl-parser-core/src/hir/lower.rs`
//! (`NodeKind::If` arm) and emits `HirKind::BranchShell`.  These tests are
//! the designated regression surface for issue #8224 (HIR lowering coverage)
//! and the prerequisite for PIR control-flow lowering (PLSP-SPEC-0025).

use perl_parser_core::Parser;
use perl_parser_core::hir::{BranchKeyword, HirFile, HirItem, HirKind, lower_ast};
use perl_tdd_support::must_some;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn lower_source(source: &str) -> HirFile {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    lower_ast(&output.ast)
}

fn first_branch_item(file: &HirFile) -> Result<&HirItem, Box<dyn std::error::Error>> {
    file.items
        .iter()
        .find(|item| matches!(item.kind, HirKind::BranchShell(_)))
        .ok_or_else(|| "expected a BranchShell HIR item".into())
}

// ---------------------------------------------------------------------------
// Basic `if` block
// ---------------------------------------------------------------------------

#[test]
fn plain_if_lowers_to_branch_shell() -> TestResult {
    let file = lower_source("if ($x) { 1 }\n");
    let item = first_branch_item(&file)?;
    let HirKind::BranchShell(shell) = &item.kind else {
        return Err("expected BranchShell".into());
    };
    assert_eq!(shell.keyword, BranchKeyword::If, "keyword should be If");
    assert_eq!(shell.elsif_count, 0, "no elsif branches");
    assert!(!shell.has_else, "no else branch");
    Ok(())
}

// ---------------------------------------------------------------------------
// `unless` block
// ---------------------------------------------------------------------------

#[test]
fn unless_lowers_to_branch_shell_with_unless_keyword() -> TestResult {
    let file = lower_source("unless ($x) { 1 }\n");
    let item = first_branch_item(&file)?;
    let HirKind::BranchShell(shell) = &item.kind else {
        return Err("expected BranchShell".into());
    };
    assert_eq!(shell.keyword, BranchKeyword::Unless, "keyword should be Unless");
    assert_eq!(shell.elsif_count, 0);
    assert!(!shell.has_else);
    Ok(())
}

// ---------------------------------------------------------------------------
// `elsif` chains and `else`
// ---------------------------------------------------------------------------

#[test]
fn if_with_two_elsifs_and_else_records_counts() -> TestResult {
    let source = "if ($a) {} elsif ($b) {} elsif ($c) {} else {}\n";
    let file = lower_source(source);
    let item = first_branch_item(&file)?;
    let HirKind::BranchShell(shell) = &item.kind else {
        return Err("expected BranchShell".into());
    };
    assert_eq!(shell.keyword, BranchKeyword::If);
    assert_eq!(shell.elsif_count, 2, "two elsif branches");
    assert!(shell.has_else, "else branch present");
    Ok(())
}

// ---------------------------------------------------------------------------
// Nested child recursion
//
// A `VariableDecl` inside the then-branch must still lower as its own HIR
// item — child bodies are not swallowed by the branch shell.
// ---------------------------------------------------------------------------

#[test]
fn nested_variable_decl_inside_if_body_still_lowers() -> TestResult {
    let file = lower_source("if ($x) { my $y = 1; }\n");

    // There should be a BranchShell for the `if`.
    let _branch = first_branch_item(&file)?;

    // And a VariableDecl for `my $y`.
    let var_decl = must_some(file.items.iter().find_map(|item| match &item.kind {
        HirKind::VariableDecl(decl) => Some(decl),
        _ => None,
    }));
    assert!(
        var_decl.variables.iter().any(|v| v.name == "y"),
        "expected variable 'y' in VariableDecl, got: {:?}",
        var_decl.variables,
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Source range validity
// ---------------------------------------------------------------------------

#[test]
fn branch_shell_item_has_valid_source_range() -> TestResult {
    let file = lower_source("if ($x) { 1 }\n");
    let item = first_branch_item(&file)?;
    assert!(
        item.range.end >= item.range.start,
        "HIR item range must be non-empty and ordered; got {:?}",
        item.range,
    );
    // The anchor should point back at an `If` AST node.
    assert_eq!(item.anchor.node_kind, "If", "anchor node_kind should be 'If'");
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope context
//
// `if`/`unless` branch shells are emitted with a scope context because
// the then-body is itself a `Block` that creates a new scope frame.
// The item's `scope_context` records the enclosing scope at lowering time.
// ---------------------------------------------------------------------------

#[test]
fn branch_shell_item_has_scope_context() -> TestResult {
    let file = lower_source("if ($x) { 1 }\n");
    let item = first_branch_item(&file)?;
    assert!(
        item.scope_context.is_some(),
        "BranchShell HIR item should carry a scope context; got None",
    );
    Ok(())
}
