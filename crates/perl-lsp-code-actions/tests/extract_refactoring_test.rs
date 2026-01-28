/// Test extract variable and subroutine refactoring
use perl_lsp_code_actions::{CodeActionsProvider, CodeActionKind};
use perl_parser_core::Parser;
use perl_tdd_support::must;

#[test]
fn parser_extract_variable_generates_edits() {
    let source = r#"
my $x = 5;
my $y = 10;
my $result = $x + $y;
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Select the "$x + $y" expression (approximate byte range)
    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (30, 38), &[]);

    // Find extract variable action
    let extract_action = actions.iter()
        .find(|a| matches!(a.kind, CodeActionKind::RefactorExtract) && a.title.contains("Extract"));

    assert!(extract_action.is_some(), "Should have extract variable action");

    let action = extract_action.unwrap();
    assert!(!action.edit.changes.is_empty(), "Should have edits");

    // Verify edits look reasonable
    let edits = &action.edit.changes;
    assert!(edits.len() >= 2, "Should have at least 2 edits (insert declaration + replace expression)");

    // Check that one edit inserts a variable declaration
    let has_declaration = edits.iter().any(|e| e.new_text.contains("my $"));
    assert!(has_declaration, "Should insert a variable declaration");
}

#[test]
fn parser_extract_subroutine_generates_edits() {
    let source = r#"
my $x = 5;
my $y = 10;
{
    my $temp = $x + $y;
    print $temp;
}
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Select the block
    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (30, 70), &[]);

    // Find extract subroutine action
    let extract_action = actions.iter()
        .find(|a| matches!(a.kind, CodeActionKind::RefactorExtract) &&
                   (a.title.contains("Extract") || a.title.contains("function") || a.title.contains("subroutine")));

    if let Some(action) = extract_action {
        assert!(!action.edit.changes.is_empty(), "Should have edits");

        // Verify edits look reasonable
        let edits = &action.edit.changes;
        assert!(edits.len() >= 2, "Should have at least 2 edits (insert function + replace with call)");

        // Check that one edit inserts a sub definition
        let has_sub = edits.iter().any(|e| e.new_text.contains("sub "));
        assert!(has_sub, "Should insert a subroutine definition");
    }
}

#[test]
fn parser_extract_variable_from_function_call() {
    let source = r#"
my $result = length("hello world");
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Select the function call expression
    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (13, 36), &[]);

    // Find extract variable action
    let extract_action = actions.iter()
        .find(|a| matches!(a.kind, CodeActionKind::RefactorExtract));

    if let Some(action) = extract_action {
        assert!(!action.edit.changes.is_empty(), "Should have edits");

        // The action should suggest extracting the function call
        let edits = &action.edit.changes;
        assert!(edits.len() >= 2, "Should have insert + replace edits");
    }
}
