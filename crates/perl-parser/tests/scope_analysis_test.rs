use perl_parser::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaState;

fn analyze(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(_) => return vec![],
    };
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![]; // We can assume strict/warnings for tests if needed, or pass empty
    analyzer.analyze(&ast, code, &pragma_map)
}

fn analyze_strict(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(_) => return vec![],
    };
    let analyzer = ScopeAnalyzer::new();

    // Create a pragma map with strict enabled for the whole file
    let pragma_map = vec![(
        0..code.len(),
        PragmaState { strict_refs: true, strict_subs: true, strict_vars: true, warnings: true },
    )];

    analyzer.analyze(&ast, code, &pragma_map)
}

#[test]
fn test_use_before_definition_same_scope() {
    let code = r#"
        use strict;
        print $x;
        my $x = 10;
    "#;

    let issues = analyze_strict(code);

    // We expect UndeclaredVariable because at the point of 'print $x', $x is not in scope
    let undeclared =
        issues.iter().find(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$x");
    assert!(undeclared.is_some(), "Should detect usage before definition as undeclared variable");
}

#[test]
fn test_use_before_initialization() {
    let code = r#"
        use strict;
        my $x;
        print $x;
    "#;

    let issues = analyze_strict(code);

    let uninitialized = issues
        .iter()
        .find(|i| i.kind == IssueKind::UninitializedVariable && i.variable_name == "$x");
    assert!(uninitialized.is_some(), "Should detect usage before initialization");
}

#[test]
fn test_variable_shadowing() {
    let code = r#"
        my $x = 10;
        {
            my $x = 20;
            print $x;
        }
    "#;

    let issues = analyze(code);
    let shadowing = issues.iter().find(|i| i.kind == IssueKind::VariableShadowing);
    assert!(shadowing.is_some(), "Should detect shadowing");
}

#[test]
fn test_variable_redeclaration_same_scope() {
    let code = r#"
        my $x = 10;
        my $x = 20;
    "#;

    let issues = analyze(code);
    let redecl = issues.iter().find(|i| i.kind == IssueKind::VariableRedeclaration);
    assert!(redecl.is_some(), "Should detect redeclaration");
}

#[test]
fn test_undeclared_assignment() {
    let code = r#"
        use strict;
        $undeclared = 10;
    "#;

    let issues = analyze_strict(code);

    let undeclared = issues
        .iter()
        .find(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$undeclared");
    assert!(undeclared.is_some(), "Should detect assignment to undeclared variable");
}

#[test]
fn test_list_assignment_initialization() {
    let code = r#"
        use strict;
        my $x;
        my $y;
        ($x, $y) = (1, 2);
        print $x;
        print $y;
    "#;

    let issues = analyze_strict(code);

    let uninit_x = issues
        .iter()
        .find(|i| i.kind == IssueKind::UninitializedVariable && i.variable_name == "$x");
    let uninit_y = issues
        .iter()
        .find(|i| i.kind == IssueKind::UninitializedVariable && i.variable_name == "$y");

    assert!(uninit_x.is_none(), "List assignment should initialize $x");
    assert!(uninit_y.is_none(), "List assignment should initialize $y");
}
