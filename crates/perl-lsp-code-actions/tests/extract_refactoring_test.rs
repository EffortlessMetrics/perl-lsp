/// Test extract variable and subroutine refactoring
use perl_lsp_code_actions::{CodeActionKind, CodeActionsProvider, EnhancedCodeActionsProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};

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
    let extract_action = actions
        .iter()
        .find(|a| matches!(a.kind, CodeActionKind::RefactorExtract) && a.title.contains("Extract"));

    assert!(extract_action.is_some(), "Should have extract variable action");

    let action = extract_action.unwrap();
    assert!(!action.edit.changes.is_empty(), "Should have edits");

    // Verify edits look reasonable
    let edits = &action.edit.changes;
    assert!(
        edits.len() >= 2,
        "Should have at least 2 edits (insert declaration + replace expression)"
    );

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
    let extract_action = actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorExtract)
            && (a.title.contains("Extract")
                || a.title.contains("function")
                || a.title.contains("subroutine"))
    });

    if let Some(action) = extract_action {
        assert!(!action.edit.changes.is_empty(), "Should have edits");

        // Verify edits look reasonable
        let edits = &action.edit.changes;
        assert!(
            edits.len() >= 2,
            "Should have at least 2 edits (insert function + replace with call)"
        );

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
    let extract_action = actions.iter().find(|a| matches!(a.kind, CodeActionKind::RefactorExtract));

    if let Some(action) = extract_action {
        assert!(!action.edit.changes.is_empty(), "Should have edits");

        // The action should suggest extracting the function call
        let edits = &action.edit.changes;
        assert!(edits.len() >= 2, "Should have insert + replace edits");
    }
}

// Gap 3: detect_parameters misses FunctionCall/MethodCall nodes
// When a block uses a variable only as a FunctionCall argument, it should be detected as a parameter.
#[test]
fn extract_subroutine_detects_function_call_args_as_params() {
    // $input is only used as an argument to length() — a FunctionCall node.
    // After the fix, collect_variables must recurse into FunctionCall args and detect $input.
    // The generated call site should include $input as a passed argument.
    let source = "{\n    my $len = length($input);\n    print $len;\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    // Select the entire block
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

    // There must be an "Extract to subroutine" action for this block.
    let extract = actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorExtract)
            && (a.title.contains("subroutine") || a.title.contains("Extract to"))
    });

    // must_some panics with "unexpected None" if no extract action is produced
    let action = must_some(extract);

    // The second edit is the function call replacing the block.
    // After the fix, $input must appear as an argument in the call: process_data($input)
    // (The first edit is the sub definition which contains the original body text verbatim,
    //  so we check the CALL edit — changes[1] — not the body definition.)
    assert!(
        action.edit.changes.len() >= 2,
        "expected at least 2 edits (sub definition + call replacement)"
    );
    let call_edit = &action.edit.changes[1];
    assert!(
        call_edit.new_text.contains("input"),
        "generated call should pass $input as detected parameter from FunctionCall arg, got: {}",
        call_edit.new_text
    );
}

// Gap 4a: Extract variable should not emit too many actions for a nested binary expression.
// When selecting "length($str) + 10", BOTH the Binary AND its FunctionCall child overlap the range —
// generating multiple RefactorExtract actions. After the fix only 1 should be emitted.
#[test]
fn extract_variable_no_duplicate_actions_for_nested_binary() {
    // "length($str) + 10" is a Binary whose left child is a FunctionCall.
    // Both nodes overlap the selection range — currently generates 2 separate extract actions.
    let source = "my $x = length($str) + 10;";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    // Select exactly "length($str) + 10" — bytes 8..25
    let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 25));

    let extract_actions: Vec<_> = actions
        .iter()
        .filter(|a| {
            matches!(a.kind, CodeActionKind::RefactorExtract) && a.title.contains("variable")
        })
        .collect();

    assert!(
        extract_actions.len() <= 1,
        "should emit at most 1 extract-variable action for a nested binary selection, got {}: {:?}",
        extract_actions.len(),
        extract_actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// Gap 4b: Extract subroutine should NOT be offered on if-body blocks
#[test]
fn extract_subroutine_not_offered_on_if_body() {
    let source = "if ($cond) { print 'yes'; }";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

    let spurious: Vec<_> = actions
        .iter()
        .filter(|a| {
            matches!(a.kind, CodeActionKind::RefactorExtract)
                && (a.title.contains("subroutine") || a.title.contains("Extract to"))
        })
        .collect();

    assert!(
        spurious.is_empty(),
        "should not offer 'Extract to subroutine' on an if-body block, but got: {:?}",
        spurious.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}
