use perl_lsp::features::code_actions_provider::{
    CodeActionKind as CodeActionKindV2, CodeActionsProvider as CodeActionsProviderV2,
};
use perl_lsp::features::diagnostics::DiagnosticsProvider;
use perl_parser::Parser;
use std::sync::Arc;

#[test]
fn test_undefined_variable_quick_fix() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nprint $x;";

    // Parse the code
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    // Get diagnostics
    let ast = Arc::new(ast);
    let diag_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diag_provider.get_diagnostics(&ast, &[], source);

    // Find undeclared variable diagnostic
    let undefined_diag = diagnostics
        .iter()
        .find(|d| d.code.as_ref().is_some_and(|c| c == "undeclared-variable"))
        .ok_or("Should have undeclared variable diagnostic")?;

    // Get code actions
    let provider = CodeActionsProviderV2::new(source.to_string());
    let actions = provider.get_code_actions(undefined_diag.range, &diagnostics);

    // Should have at least 2 actions (my and our)
    assert!(actions.len() >= 2, "Should have at least 2 actions, got {}", actions.len());

    // Check first action (declare with 'my')
    assert_eq!(actions[0].title, "Declare '$x' with 'my'");
    assert_eq!(actions[0].kind, CodeActionKindV2::QuickFix);
    assert!(actions[0].edit.new_text.contains("my $x"));

    Ok(())
}

#[test]
fn test_unused_variable_quick_fix() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $unused = 42;\nprint \"done\";";

    // Parse the code
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    // Get diagnostics
    let ast = Arc::new(ast);
    let diag_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diag_provider.get_diagnostics(&ast, &[], source);

    // Find unused variable diagnostic
    let unused_diag = diagnostics
        .iter()
        .find(|d| d.code.as_ref().is_some_and(|c| c == "unused-variable"))
        .ok_or("Should have unused variable diagnostic")?;

    // Get code actions
    let provider = CodeActionsProviderV2::new(source.to_string());
    let actions = provider.get_code_actions(unused_diag.range, &diagnostics);

    // Should have at least 2 actions (remove and rename)
    assert!(actions.len() >= 2, "Should have at least 2 actions");

    // Check rename action
    let rename_action =
        actions.iter().find(|a| a.title.contains("$_unused")).ok_or("Should have rename action")?;
    assert_eq!(rename_action.kind, CodeActionKindV2::QuickFix);

    Ok(())
}

#[test]
fn test_variable_shadowing_quick_fix() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n{ my $x = 2; }";

    // Parse the code
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    // Get diagnostics
    let ast = Arc::new(ast);
    let diag_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diag_provider.get_diagnostics(&ast, &[], source);

    // Find shadowing diagnostic
    let shadow_diag = diagnostics
        .iter()
        .find(|d| d.code.as_ref().is_some_and(|c| c == "variable-shadowing"))
        .ok_or("Should have variable shadowing diagnostic")?;

    // Get code actions
    let provider = CodeActionsProviderV2::new(source.to_string());
    let actions = provider.get_code_actions(shadow_diag.range, &diagnostics);

    // Should have rename suggestions
    assert!(!actions.is_empty(), "Should have rename suggestions");

    // Check that we have suggestions like inner_x, local_x, x_2
    let has_inner = actions.iter().any(|a| a.title.contains("inner_"));
    let has_local = actions.iter().any(|a| a.title.contains("local_"));
    let has_numbered = actions.iter().any(|a| a.title.contains("_2"));

    assert!(
        has_inner || has_local || has_numbered,
        "Should have rename suggestions with inner_, local_, or _2"
    );

    Ok(())
}

#[test]
fn test_parse_error_semicolon_fix() -> Result<(), Box<dyn std::error::Error>> {
    let source = "print 'hello'\nprint 'world';";

    // Parse the code (will have error)
    let mut parser = Parser::new(source);
    let _ast = parser.parse().unwrap_or_else(|_| {
        // Create error node for test
        perl_parser::Node {
            kind: perl_parser::NodeKind::Program { statements: vec![] },
            location: perl_parser::SourceLocation { start: 0, end: source.len() },
        }
    });

    // Create diagnostic manually for missing semicolon
    let diagnostic = perl_lsp::features::diagnostics::Diagnostic {
        range: (13, 14),
        severity: perl_lsp::features::diagnostics::DiagnosticSeverity::Error,
        code: Some("parse-error-missingsemicolon".to_string()),
        message: "Missing semicolon".to_string(),
        related_information: vec![],
        tags: vec![],
    };

    // Get code actions
    let provider = CodeActionsProviderV2::new(source.to_string());
    let actions = provider.get_code_actions(diagnostic.range, &[diagnostic]);

    // Should have semicolon fix
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].title, "Add missing semicolon");
    assert_eq!(actions[0].edit.new_text, ";");

    Ok(())
}

#[test]
fn test_multiple_diagnostics_multiple_actions() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nprint $x;\nmy $unused = 42;";

    // Parse the code
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    // Get diagnostics
    let ast = Arc::new(ast);
    let diag_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diag_provider.get_diagnostics(&ast, &[], source);

    // Should have undeclared variable diagnostic
    let has_undeclared =
        diagnostics.iter().any(|d| d.code.as_ref().is_some_and(|c| c == "undeclared-variable"));

    assert!(has_undeclared, "Should have undeclared variable diagnostic");
    // Note: unused variable detection not yet implemented

    // Get code actions for entire range
    let provider = CodeActionsProviderV2::new(source.to_string());
    let actions = provider.get_code_actions((0, source.len()), &diagnostics);

    // Should have actions for undeclared variable
    assert!(actions.len() >= 2, "Should have at least 2 actions for undeclared variable");

    // Check we have declare actions
    let has_declare_action = actions.iter().any(|a| a.title.contains("Declare"));

    assert!(has_declare_action, "Should have declare action for undeclared variable");

    Ok(())
}
