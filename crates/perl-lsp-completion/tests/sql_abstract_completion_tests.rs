//! Tests for SQL::Abstract method completion, hover documentation, and signature help.
//!
//! These tests verify the SQL::Abstract support follows the DBI pattern:
//! - Guard pattern using `use SQL::Abstract` to avoid false positives
//! - Variable inference for `$sql`, `$sqla`, `$sql_abs` names
//! - SQL::Abstract->new() assignment detection
//!
//! Acceptance Criteria from ADR-0017:
//! - AC1: Method completion for `$sql->` with `use SQL::Abstract`
//! - AC4: Guard prevents false positives (no `use SQL::Abstract` = no completions)
//! - AC5: Variable name inference without constructor
//!
//! Note: Tests for get_sql_abstract_method_documentation, hover, and signature help
//! will be added separately once the completion tests pass (indicating the basic
//! implementation exists).

use perl_lsp_completion::{CompletionItem, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn completions(code: &str, position: usize) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    provider.get_completions(code, position)
}

fn completions_at_end(code: &str) -> Vec<CompletionItem> {
    completions(code, code.len())
}

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|item| item.label == label)
}

// -----------------------------------------------------------------------------
// SQL::Abstract Methods (expected completions)
// -----------------------------------------------------------------------------

const SQL_ABSTRACT_METHODS: &[&str] =
    &["select", "insert", "update", "delete", "where", "generate", "values", "order_by"];

// -----------------------------------------------------------------------------
// AC1: Method Completion
// -----------------------------------------------------------------------------

/// AC1: Given a Perl file with `use SQL::Abstract` and `$sql = SQL::Abstract->new();`
/// When the user types `$sql->`
/// Then perl-lsp offers completions including: select, insert, update, delete, where, generate, values, order_by
#[test]
fn completes_sql_abstract_methods_for_sql_variable_with_new_assignment() {
    let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->";
    let items = completions_at_end(code);

    for method in SQL_ABSTRACT_METHODS {
        assert!(
            has_label(&items, method),
            "should suggest SQL::Abstract method '{}' for $sql after SQL::Abstract->new(), got: {:?}",
            method,
            labels(&items)
        );
    }
}

/// AC1 variant: $sqla variable name should also trigger SQL::Abstract methods
#[test]
fn completes_sql_abstract_methods_for_sqla_variable() {
    let code = "use SQL::Abstract;\nmy $sqla = SQL::Abstract->new();\n$sqla->";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "select"),
        "should suggest SQL::Abstract select for $sqla, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "insert"),
        "should suggest SQL::Abstract insert for $sqla, got: {:?}",
        labels(&items)
    );
}

/// AC1 variant: $sql_abs variable name should also trigger SQL::Abstract methods
#[test]
fn completes_sql_abstract_methods_for_sql_abs_variable() {
    let code = "use SQL::Abstract;\nmy $sql_abs = SQL::Abstract->new();\n$sql_abs->";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "update"),
        "should suggest SQL::Abstract update for $sql_abs, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "delete"),
        "should suggest SQL::Abstract delete for $sql_abs, got: {:?}",
        labels(&items)
    );
}

// -----------------------------------------------------------------------------
// AC4: Guard Prevents False Positives
// -----------------------------------------------------------------------------

/// AC4: Given a Perl file WITHOUT `use SQL::Abstract` that uses a variable named `$sql`
/// When the user types `$sql->select`
/// Then perl-lsp does NOT offer SQL::Abstract method completions
#[test]
fn guard_pattern_prevents_false_positives_without_use_sql_abstract() {
    let code = "my $sql = 'some value';\n$sql->";
    let items = completions_at_end(code);

    // The guard pattern should prevent SQL::Abstract completions
    // We should NOT see SQL::Abstract-specific methods like insert, update, delete, where
    assert!(
        !has_label(&items, "insert"),
        "should NOT suggest SQL::Abstract insert without use SQL::Abstract, got: {:?}",
        labels(&items)
    );
    assert!(
        !has_label(&items, "update"),
        "should NOT suggest SQL::Abstract update without use SQL::Abstract, got: {:?}",
        labels(&items)
    );
    assert!(
        !has_label(&items, "delete"),
        "should NOT suggest SQL::Abstract delete without use SQL::Abstract, got: {:?}",
        labels(&items)
    );
    assert!(
        !has_label(&items, "where"),
        "should NOT suggest SQL::Abstract where without use SQL::Abstract, got: {:?}",
        labels(&items)
    );
}

/// AC4: Using use SQL::Abstract with imports should still work
#[test]
fn completes_sql_abstract_methods_with_use_sql_abstract_qw_imports() {
    let code = "use SQL::Abstract qw(select insert);\nmy $sql = SQL::Abstract->new();\n$sql->";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "select"),
        "should suggest select with use SQL::Abstract qw(...), got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "insert"),
        "should suggest insert with use SQL::Abstract qw(...), got: {:?}",
        labels(&items)
    );
    // Also should have other methods not explicitly imported
    assert!(
        has_label(&items, "update"),
        "should suggest update (not in qw list) with use SQL::Abstract, got: {:?}",
        labels(&items)
    );
}

// -----------------------------------------------------------------------------
// AC5: Variable Name Inference Without Constructor
// -----------------------------------------------------------------------------

/// AC5: Given a Perl file with `use SQL::Abstract` and `my $sqla;` (no assignment from SQL::Abstract->new)
/// When the user types `$sqla->`
/// Then perl-lsp offers SQL::Abstract method completions
#[test]
fn completes_sql_abstract_methods_for_sqla_variable_without_new() {
    let code = "use SQL::Abstract;\nmy $sqla;\n$sqla->";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "select"),
        "should suggest SQL::Abstract select for $sqla without new(), got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "generate"),
        "should suggest SQL::Abstract generate for $sqla without new(), got: {:?}",
        labels(&items)
    );
}

/// AC5: Given a Perl file with `use SQL::Abstract` and `my $sql_abs;` (no assignment from SQL::Abstract->new)
/// When the user types `$sql_abs->`
/// Then perl-lsp offers SQL::Abstract method completions
#[test]
fn completes_sql_abstract_methods_for_sql_abs_variable_without_new() {
    let code = "use SQL::Abstract;\nmy $sql_abs;\n$sql_abs->";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "where"),
        "should suggest SQL::Abstract where for $sql_abs without new(), got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "values"),
        "should suggest SQL::Abstract values for $sql_abs without new(), got: {:?}",
        labels(&items)
    );
}

// -----------------------------------------------------------------------------
// Completion item detail/documentation should be set
// -----------------------------------------------------------------------------

/// Verify that SQL::Abstract completion items have documentation set
#[test]
fn sql_abstract_completion_items_have_documentation() {
    let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->";
    let items = completions_at_end(code);

    let select_item = find_item(&items, "select");
    assert!(select_item.is_some(), "should have select completion item, got: {:?}", labels(&items));

    let select = select_item.unwrap();
    assert!(select.documentation.is_some(), "select item should have documentation");
}

#[test]
fn sql_abstract_completion_items_have_method_detail() {
    let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->";
    let items = completions_at_end(code);

    let insert_item = find_item(&items, "insert");
    assert!(insert_item.is_some(), "should have insert completion item");

    let insert = insert_item.unwrap();
    assert!(insert.detail.is_some(), "insert item should have detail");
}

// -----------------------------------------------------------------------------
// Variable name $s should NOT trigger SQL::Abstract inference (per ADR decision)
// -----------------------------------------------------------------------------

/// ADR decision: $s is too common in Perl (loop variable, subroutine arg, generic scalar)
/// and will NOT be used for SQL::Abstract inference
#[test]
fn dollar_s_variable_should_not_trigger_sql_abstract_completion() {
    let code = "use SQL::Abstract;\nmy $s = shift;\n$s->";
    let items = completions_at_end(code);

    // $s should NOT get SQL::Abstract methods even with use SQL::Abstract
    // because $s is too common a variable name in Perl
    assert!(
        !has_label(&items, "select")
            || !has_label(&items, "insert")
            || !has_label(&items, "update"),
        "$s should NOT trigger SQL::Abstract completions per ADR-0017, got: {:?}",
        labels(&items)
    );
}
