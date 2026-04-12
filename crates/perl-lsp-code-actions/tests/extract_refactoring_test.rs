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

// Issue #3031: Extract variable — binary arithmetic expression
// Selects "$a * $b" and extracts it into a new variable declaration.
#[test]
fn extract_variable_from_arithmetic_expression() {
    let source = "my $a = 5;\nmy $b = 3;\nmy $total = $a * $b;\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // "$a * $b" starts where the last "$a" appears; ends at the last ";"
    let rhs_start = source.rfind("$a").unwrap_or(0);
    let rhs_end = source.rfind(';').unwrap_or(source.len());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (rhs_start, rhs_end));

    let extract_actions: Vec<_> = actions
        .iter()
        .filter(|a| {
            matches!(a.kind, CodeActionKind::RefactorExtract) && a.title.contains("variable")
        })
        .collect();

    assert!(
        !extract_actions.is_empty(),
        "Expected extract-variable action for arithmetic expression, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );

    let action = &extract_actions[0];
    assert_eq!(
        action.edit.changes.len(),
        2,
        "Expected exactly 2 edits (declaration + replacement)"
    );

    let decl = &action.edit.changes[0];
    assert!(
        decl.new_text.starts_with("my $"),
        "Declaration should start with 'my $', got: {}",
        decl.new_text
    );
    assert!(
        decl.new_text.contains("$a") || decl.new_text.contains("$b"),
        "Declaration should contain extracted expression variables, got: {}",
        decl.new_text
    );
    assert!(
        decl.new_text.ends_with(";\n"),
        "Declaration should end with semicolon+newline, got: {}",
        decl.new_text
    );

    let replace = &action.edit.changes[1];
    assert!(
        replace.new_text.starts_with('$'),
        "Replacement should be a variable reference, got: {}",
        replace.new_text
    );
    assert_eq!(
        replace.location.start, rhs_start,
        "Replacement start should match expression start"
    );
    assert_eq!(replace.location.end, rhs_end, "Replacement end should match expression end");
}

// Issue #3031: Extract variable title must contain "variable" for LSP client matching.
#[test]
fn extract_variable_action_title_contains_variable_keyword() {
    let source = "my $x = length($str) + 1;";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 24));

    let extract_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorExtract) && a.title.contains("variable")
    }));

    assert!(
        extract_action.title.contains("Extract"),
        "Title must contain 'Extract': {}",
        extract_action.title
    );
    assert!(
        extract_action.title.contains("variable"),
        "Title must contain 'variable': {}",
        extract_action.title
    );
}

// Issue #3031: Extract subroutine — detect return values.
// A variable declared inside the block that appears as a bare expression in the
// last statement should be detected as a return value.
#[test]
fn extract_subroutine_detects_return_values() {
    let source = "{\n    my $result = $x * 2;\n    $result\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

    let extract = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorExtract)
            && (a.title.contains("subroutine") || a.title.contains("Extract to"))
    }));

    assert!(extract.edit.changes.len() >= 2, "Expected at least 2 edits");

    // The call replacement should capture the return value
    let call_edit = &extract.edit.changes[1];
    assert!(
        call_edit.new_text.contains("result") || call_edit.new_text.contains('='),
        "Call site should capture return value, got: {}",
        call_edit.new_text
    );
}

// Issue #3031: Extract subroutine sub body must contain the original code verbatim.
#[test]
fn extract_subroutine_body_contains_original_code() {
    let source = "{\n    my $x = 1;\n    print $x;\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

    let extract = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorExtract)
            && (a.title.contains("subroutine") || a.title.contains("Extract to"))
    }));

    let sub_edit = &extract.edit.changes[0];
    assert!(
        sub_edit.new_text.contains("sub "),
        "Sub edit should contain 'sub' keyword, got: {}",
        sub_edit.new_text
    );
    assert!(
        sub_edit.new_text.contains("my $x = 1"),
        "Sub body should contain 'my $x = 1', got: {}",
        sub_edit.new_text
    );
    assert!(
        sub_edit.new_text.contains("print $x"),
        "Sub body should contain 'print $x', got: {}",
        sub_edit.new_text
    );
}

// Issue #3031: Extract subroutine — spec example (process_order).
// Inner block extracted to its own subroutine.
#[test]
fn extract_subroutine_spec_example_inner_block() {
    let source = "sub process_order {\n    my $order = shift;\n    {\n        my $validated = validate_order($order);\n        my $total = calculate_total($order);\n    }\n    return 1;\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let block_start = source.find("    {\n").unwrap_or(0);
    let block_end = source.find("    return 1").unwrap_or(source.len());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (block_start, block_end));

    let action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorExtract)
            && (a.title.contains("subroutine") || a.title.contains("Extract to"))
    }));
    let sub_edit = &action.edit.changes[0];
    assert!(
        sub_edit.new_text.contains("sub "),
        "Should insert sub definition, got: {}",
        sub_edit.new_text
    );

    let call_edit = &action.edit.changes[1];
    assert!(
        call_edit.new_text.contains('(') && call_edit.new_text.contains(')'),
        "Replacement should be a function call, got: {}",
        call_edit.new_text
    );
}

// Issue #3471: Range-bounded traversal — actions from outside the requested range must
// not appear in the result set.
#[test]
fn range_bounded_traversal_excludes_out_of_range_nodes() {
    // Two separate if-blocks. The range covers only the first one.
    // The second block contains a convertible if-statement, but it is outside the range.
    // After the fix, no actions from the second block should appear.
    let source = "if ($debug) { print \"yes\"; }\nif ($trace) { open my $fh, '<', 'file.txt'; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Range covers only the first if-block (bytes 0..28)
    let first_block_end = source.find('\n').unwrap_or(28);
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, first_block_end));

    // Actions from the second block (open with or-die, postfix conversion of second if)
    // must not appear. Verify no action has an edit location starting after first_block_end.
    for action in &actions {
        for edit in &action.edit.changes {
            assert!(
                edit.location.start <= first_block_end,
                "Action '{}' produced an edit at byte {} which is outside the requested range 0..{}",
                action.title,
                edit.location.start,
                first_block_end
            );
        }
    }
}

// Issue #3471: Large-file responsiveness — actions on a narrow range in a large file
// must only return actions for the selected region, not for the entire file.
#[test]
fn large_file_narrow_range_returns_only_in_range_actions() {
    // Build a file with 50 subroutines. The target range covers only sub_000.
    let mut source = String::from("use strict;\nuse warnings;\n\n");
    // sub_000 is inserted first; record where it starts and ends
    let sub_target_start = source.len();
    source.push_str("if ($debug) { print \"target\\n\"; }\n");
    let sub_target_end = source.len() - 1; // up to the closing newline

    // Add 49 more if-blocks that each contain an open() call (triggering error_checking)
    for i in 1..50 {
        source.push_str(&format!("if ($cond_{i}) {{ open my $fh, '<', 'file_{i}.txt'; }}\n"));
    }

    let mut parser = Parser::new(&source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.clone());
    let actions =
        provider.get_enhanced_refactoring_actions(&ast, (sub_target_start, sub_target_end));

    // All returned edits must fall within or at the boundary of the target range
    for action in &actions {
        for edit in &action.edit.changes {
            assert!(
                edit.location.start <= sub_target_end,
                "Action '{}' produced edit at byte {} — outside target range {}..{}",
                action.title,
                edit.location.start,
                sub_target_start,
                sub_target_end
            );
        }
    }

    // The postfix action for the targeted if-block must be present
    assert!(
        actions.iter().any(|a| a.title.contains("postfix")),
        "Expected a postfix action for the targeted if-block"
    );
}

// Issue #3471: Verify that the traversal does not visit children of nodes that are
// entirely before the requested range. This is a structural correctness check:
// a node whose end <= range.start cannot contribute any action.
#[test]
fn traversal_skips_nodes_entirely_before_range() {
    // Three sequential expression statements. The range covers only the third one.
    let source = "my $a = foo();\nmy $b = bar();\nmy $c = baz();\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Find offset of the third statement
    let third_start = source.rfind("my $c").unwrap_or(0);
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (third_start, source.len()));

    // Any extract-variable action must reference the third expression, not $a or $b
    for action in
        actions.iter().filter(|a| a.title.contains("Extract") && a.title.contains("variable"))
    {
        for edit in &action.edit.changes {
            assert!(
                edit.location.start >= third_start || edit.location.end >= third_start,
                "Extract action '{}' has edit at {}..{} which is entirely before the range start {}",
                action.title,
                edit.location.start,
                edit.location.end,
                third_start
            );
        }
    }
}
