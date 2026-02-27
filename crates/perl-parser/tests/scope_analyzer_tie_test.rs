use perl_parser::{
    Parser,
    pragma_tracker::PragmaTracker,
    scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue},
};

fn analyze_code(code: &str) -> Vec<ScopeIssue> {
    use perl_tdd_support::must;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = PragmaTracker::build(&ast);
    analyzer.analyze(&ast, code, &pragma_map)
}

#[test]
fn test_tie_declaration() {
    let code = r#"
use strict;
tie my %hash, 'Some::Package';
print %hash;
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UninitializedVariable)));
}

#[test]
fn test_tie_assignment() {
    let code = r#"
use strict;
my %hash;
tie %hash, 'Some::Package';
print %hash;
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UninitializedVariable)));
}

#[test]
fn test_untie() {
    let code = r#"
use strict;
my %hash;
tie %hash, 'Some::Package';
untie %hash;
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_tie_args_usage() {
    let code = r#"
use strict;
my $x = 10;
tie my $y, 'Some::Package', $x;
"#;

    let issues = analyze_code(code);
    // $x is used in tie args, so it shouldn't be unused
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UnusedVariable) && i.variable_name == "$x")
    );
}

#[test]
fn test_tied_function() {
    let code = r#"
use strict;
my %hash;
tie %hash, 'Some::Package';
if (tied %hash) {
    print "Tied";
}
"#;
    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}
