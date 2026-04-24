use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
use perl_tdd_support::must;

fn scope_issues_with_pragmas(code: &str) -> Vec<ScopeIssue> {
    let ast = must(Parser::new(code).parse());
    let pragma_map = PragmaTracker::build(&ast);
    ScopeAnalyzer::new().analyze(&ast, code, &pragma_map)
}

fn has_issue(issues: &[ScopeIssue], kind: IssueKind, needle: &str) -> bool {
    issues.iter().any(|issue| issue.kind == kind && issue.variable_name.contains(needle))
}

#[test]
fn signatures_enable_strict_vars_and_strict_subs_checks() -> Result<(), Box<dyn std::error::Error>>
{
    let code = r#"
use feature 'signatures';
sub run ($arg) {
    $undeclared = $arg;
    bareword_call;
}
"#;

    let issues = scope_issues_with_pragmas(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "undeclared"),
        "signatures_strict should enable strict vars behavior; issues: {issues:?}"
    );
    assert!(
        has_issue(&issues, IssueKind::UnquotedBareword, "bareword_call"),
        "signatures_strict should enable strict subs behavior; issues: {issues:?}"
    );

    Ok(())
}

#[test]
fn lexical_no_strict_vars_disables_checks_only_inside_its_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
{
    no strict 'vars';
    $inside = 1;
}
$outside = 2;
"#;

    let issues = scope_issues_with_pragmas(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "inside"),
        "lexical no strict 'vars' should suppress undeclared-variable checks in block; issues: {issues:?}"
    );
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "outside"),
        "strict vars should be restored after block and catch outside usage; issues: {issues:?}"
    );

    Ok(())
}

#[test]
fn conditional_no_and_eval_string_changes_are_visible_in_scope_analysis()
-> Result<(), Box<dyn std::error::Error>> {
    let conditional_disable = r#"
use strict;
no if 1, 'strict', 'vars';
$conditional = 1;
"#;
    let conditional_issues = scope_issues_with_pragmas(conditional_disable);
    assert!(
        !has_issue(&conditional_issues, IssueKind::UndeclaredVariable, "conditional"),
        "conditional no strict vars should disable strict-vars checks downstream; issues: {conditional_issues:?}"
    );

    let eval_string_disable = r#"
use strict;
eval "no strict 'vars';";
$still_strict = 1;
"#;
    let eval_string_issues = scope_issues_with_pragmas(eval_string_disable);
    assert!(
        has_issue(&eval_string_issues, IssueKind::UndeclaredVariable, "still_strict"),
        "eval STRING pragma text should not relax outer strict-vars checks; issues: {eval_string_issues:?}"
    );

    Ok(())
}
