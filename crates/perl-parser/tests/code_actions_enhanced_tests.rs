use perl_parser::code_actions_provider::CodeActionsProvider as CodeActionsProviderV2;
use perl_parser::{DiagnosticsProvider, Parser};

#[test]
fn test_duplicate_parameter_code_actions() {
    let source = r#"sub test($x, $y, $x) {
    print $x;
}"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let code_actions_provider = CodeActionsProviderV2::new(source.to_string());
    let actions = code_actions_provider.get_code_actions((0, source.len()), &diagnostics);

    // Should offer to remove or rename the duplicate
    let duplicate_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostic_id.as_deref() == Some("duplicate-parameter"))
        .collect();

    assert!(duplicate_actions.len() >= 2);
    assert!(duplicate_actions.iter().any(|a| a.title.contains("Remove duplicate")));
    assert!(duplicate_actions.iter().any(|a| a.title.contains("Rename duplicate")));
}

#[test]
fn test_parameter_shadowing_code_actions() {
    let source = r#"my $data = 42;

sub process($data) {
    return $data * 2;
}"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let code_actions_provider = CodeActionsProviderV2::new(source.to_string());
    let actions = code_actions_provider.get_code_actions((0, source.len()), &diagnostics);

    // Should offer to rename the parameter
    let shadow_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostic_id.as_deref() == Some("parameter-shadows-global"))
        .collect();

    assert!(!shadow_actions.is_empty());
    assert!(shadow_actions.iter().any(|a| a.title.contains("Rename parameter")));
    assert!(
        shadow_actions
            .iter()
            .any(|a| a.title.contains("$p_data") || a.title.contains("$data_param"))
    );
}

#[test]
fn test_unused_parameter_code_actions() {
    let source = r#"sub calculate($x, $y, $unused) {
    return $x + $y;
}"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let code_actions_provider = CodeActionsProviderV2::new(source.to_string());
    let actions = code_actions_provider.get_code_actions((0, source.len()), &diagnostics);

    // Should offer to rename with underscore or add comment
    let unused_actions: Vec<_> =
        actions.iter().filter(|a| a.diagnostic_id.as_deref() == Some("unused-parameter")).collect();

    assert!(!unused_actions.is_empty());
    assert!(unused_actions.iter().any(|a| a.title.contains("$_unused")));
    assert!(unused_actions.iter().any(|a| a.title.contains("comment")));
}

#[test]
fn test_bareword_code_actions() {
    let source = r#"use strict;
print FOO;"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let code_actions_provider = CodeActionsProviderV2::new(source.to_string());
    let actions = code_actions_provider.get_code_actions((0, source.len()), &diagnostics);

    // Should offer to quote the bareword
    let bareword_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostic_id.as_deref() == Some("unquoted-bareword"))
        .collect();

    assert!(bareword_actions.len() >= 2);
    assert!(bareword_actions.iter().any(|a| a.title.contains("'FOO'")));
    assert!(bareword_actions.iter().any(|a| a.title.contains("\"FOO\"")));
    // For uppercase barewords, should also offer filehandle declaration
    assert!(bareword_actions.iter().any(|a| a.title.contains("filehandle")));
}

#[test]
fn test_multiple_parameter_issues() {
    let source = r#"my $x = 1;
my $y = 2;

sub test($x, $y, $x, $unused) {
    return $x + $y;
}"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let code_actions_provider = CodeActionsProviderV2::new(source.to_string());
    let actions = code_actions_provider.get_code_actions((0, source.len()), &diagnostics);

    // Should have actions for all issues
    assert!(actions.iter().any(|a| a.diagnostic_id.as_deref() == Some("duplicate-parameter")));
    assert!(actions.iter().any(|a| a.diagnostic_id.as_deref() == Some("parameter-shadows-global")));
    assert!(actions.iter().any(|a| a.diagnostic_id.as_deref() == Some("unused-parameter")));
}

#[test]
fn test_bareword_filehandle_suggestion() {
    let source = r#"use strict;
print LOGFILE "Starting process";"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let code_actions_provider = CodeActionsProviderV2::new(source.to_string());
    let actions = code_actions_provider.get_code_actions((0, source.len()), &diagnostics);

    // Should suggest declaring as filehandle for uppercase barewords
    let filehandle_actions: Vec<_> =
        actions.iter().filter(|a| a.title.contains("filehandle")).collect();

    assert!(!filehandle_actions.is_empty());
    assert!(filehandle_actions[0].edit.new_text.contains("open"));
}

#[test]
fn test_edit_ranges_for_parameter_fixes() {
    let source = r#"sub test($duplicate, $duplicate) {
    return $duplicate;
}"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let code_actions_provider = CodeActionsProviderV2::new(source.to_string());
    let actions = code_actions_provider.get_code_actions((0, source.len()), &diagnostics);

    // Check that the edit ranges are valid
    for action in actions {
        assert!(action.edit.range.0 <= action.edit.range.1);
        assert!(action.edit.range.1 <= source.len());
    }
}
