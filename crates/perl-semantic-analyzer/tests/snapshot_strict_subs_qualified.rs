//! Snapshot tests for `strict 'subs'` package-qualified function call validation.
//!
//! These tests capture the expected output of the scope analyzer for various
//! package-qualified function call patterns under `use strict 'subs'`.
//!
//! Run `INSTA_UPDATE=always cargo test -p perl-semantic-analyzer --test snapshot_strict_subs_qualified`
//! to update snapshots when the output changes intentionally.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
use perl_tdd_support::must;

/// Run scope analysis with strict mode enabled by building a pragma map from
/// `use strict;` in the source.
fn scope_issues_strict(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let pragma_map = PragmaTracker::build(&ast);
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &pragma_map)
}

/// Format issues for snapshot comparison - produces a deterministic string
/// representation of the issues found.
fn format_issues(issues: &[ScopeIssue]) -> String {
    if issues.is_empty() {
        return "no issues".to_string();
    }
    let mut lines = Vec::new();
    for issue in issues {
        lines.push(format!(
            "{}:{}: {:?} '{}' range=({:?}) - {}",
            issue.line,
            issue.variable_name,
            issue.kind,
            issue.variable_name,
            issue.range,
            issue.description
        ));
    }
    lines.join("\n")
}

/// AC1: Foo::bar() MUST be flagged as bareword under strict_subs when bar is
/// not a known builtin.
#[test]
fn snapshot_strict_subs_qualified_flagged() {
    let code = r#"
use strict 'subs';
Foo::bar();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_qualified_flagged", output);
}

/// AC2: Foo::print() MUST NOT be flagged under strict_subs, because `print`
/// is a known builtin.
#[test]
fn snapshot_strict_subs_qualified_builtin_not_flagged() {
    let code = r#"
use strict 'subs';
Foo::print();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_qualified_builtin_not_flagged", output);
}

/// AC3: Bar::print() MUST NOT be flagged because `print` is a known builtin.
#[test]
fn snapshot_strict_subs_bar_print_not_flagged() {
    let code = r#"
use strict 'subs';
Bar::print();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_bar_print_not_flagged", output);
}

/// AC4: Hash key context MUST be excluded — Foo::bar in hash key context MUST
/// NOT be flagged even if bar is not a builtin.
#[test]
fn snapshot_strict_subs_qualified_hash_key_excluded() {
    let code = r#"
use strict 'subs';
my %h = (Foo::bar => 1);
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_qualified_hash_key_excluded", output);
}

/// AC5: Method calls MUST NOT be affected — $obj->method() is a different
/// node type and MUST NOT be flagged under strict_subs.
#[test]
fn snapshot_strict_subs_method_calls_not_affected() {
    let code = r#"
use strict 'subs';
my $obj = Some::Class->new();
$obj->method();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_method_calls_not_affected", output);
}

/// AC6: Package-qualified variables MUST NOT be affected — $Foo::bar is a
/// variable and MUST NOT be flagged under strict_subs.
#[test]
fn snapshot_strict_subs_package_qualified_variables_not_affected() {
    let code = r#"
use strict 'subs';
print $Foo::bar;
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_package_qualified_variables_not_affected", output);
}

/// Verify deeply nested qualified names are flagged correctly.
#[test]
fn snapshot_strict_subs_deeply_nested_qualified() {
    let code = r#"
use strict 'subs';
Very::Long::Package::Name::function();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_deeply_nested_qualified", output);
}

/// Verify multiple qualified calls on the same line are both flagged.
#[test]
fn snapshot_strict_subs_multiple_qualified_same_expr() {
    let code = r#"
use strict 'subs';
Foo::bar(); Baz::quux();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_multiple_qualified_same_expr", output);
}

/// Verify that uppercase package with non-uppercase identifier is flagged.
#[test]
fn snapshot_strict_subs_uppercase_package_builtin_identifier() {
    let code = r#"
use strict 'subs';
DBI::connect();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_uppercase_package_builtin_identifier", output);
}

/// Edge case: Empty identifier after colons (Foo::()).
#[test]
fn snapshot_strict_subs_edge_case_empty_identifier_after_colons() {
    let code = r#"
use strict 'subs';
Foo::();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_edge_case_empty_identifier_after_colons", output);
}

/// Edge case: Leading double colon (::Foo()).
#[test]
fn snapshot_strict_subs_edge_case_leading_double_colon() {
    let code = r#"
use strict 'subs';
::Foo();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_edge_case_leading_double_colon", output);
}

/// Edge case: Multiple consecutive colons (Foo:::bar()).
#[test]
fn snapshot_strict_subs_edge_case_multiple_consecutive_colons() {
    let code = r#"
use strict 'subs';
Foo:::bar();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_edge_case_multiple_consecutive_colons", output);
}

/// Verify that unqualified barewords are still flagged (existing behavior).
#[test]
fn snapshot_strict_subs_unqualified_still_flagged() {
    let code = r#"
use strict 'subs';
FOO();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_unqualified_still_flagged", output);
}

/// Verify that unqualified builtin is not flagged (existing behavior).
#[test]
fn snapshot_strict_subs_unqualified_builtin_not_flagged() {
    let code = r#"
use strict 'subs';
print();
"#;
    let issues = scope_issues_strict(code);
    let output = format_issues(&issues);
    insta::assert_snapshot!("strict_subs_unqualified_builtin_not_flagged", output);
}
