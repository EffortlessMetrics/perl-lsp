//! Tests for inline variable refactoring code action
//!
//! These tests define the expected behavior of the inline variable refactoring
//! code action. When a user selects a variable declaration, the LSP should offer
//! to inline it — replacing all usages with the initializer expression and removing
//! the declaration.
//!
//! The refactoring logic exists in perl-refactoring, but the code action to
//! expose it via LSP does not yet exist.

use perl_lsp_code_actions::{CodeActionKind, CodeActionsProvider, EnhancedCodeActionsProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};

/// Basic case: single variable with single usage
/// The action should be offered when the cursor is on the variable declaration.
#[test]
fn inline_variable_action_offered_on_declaration() {
    // my $temp = 5;
    // print $temp;
    let source = "my $temp = 5;\nprint $temp;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Position on the variable declaration (bytes 0..12)
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 12));

    // Should have at least one RefactorInline action
    let inline_actions: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a.kind, CodeActionKind::RefactorInline))
        .collect();

    assert!(
        !inline_actions.is_empty(),
        "Expected inline variable action for variable declaration, got actions: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

/// The inline variable action title should contain "Inline" for LSP client matching.
#[test]
fn inline_variable_action_title_contains_inline_keyword() {
    let source = "my $temp = 5;\nprint $temp;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 12));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
            && a.title.to_lowercase().contains("inline")
    }));

    assert!(
        inline_action.title.to_lowercase().contains("variable"),
        "Inline action title should contain 'variable', got: {}",
        inline_action.title
    );
}

/// When inlining a variable with a simple literal initializer,
/// all usages should be replaced with the literal and declaration removed.
#[test]
fn inline_variable_replaces_usage_with_literal() {
    // Before:
    //   my $x = 42;
    //   print $x;
    // After:
    //   print 42;
    let source = "my $x = 42;\nprint $x;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 10));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // The edits should:
    // 1. Remove the declaration line
    // 2. Replace "$x" in the print statement with "42"
    assert!(
        inline_action.edit.changes.len() >= 2,
        "Expected at least 2 edits (remove declaration + replace usages), got {}",
        inline_action.edit.changes.len()
    );

    // Find the edit that removes the declaration (should replace with empty string)
    let has_removal = inline_action.edit.changes.iter().any(|e| {
        e.new_text.is_empty() && e.location.start == 0
    });
    assert!(has_removal, "Should have an edit that removes the declaration at start");

    // Find the edit that replaces $x with 42
    let has_replacement = inline_action.edit.changes.iter().any(|e| {
        e.new_text.contains("42")
    });
    assert!(has_replacement, "Should have an edit that replaces $x with 42");
}

/// When inlining a variable with an expression initializer,
/// all usages should be replaced with the expression.
#[test]
fn inline_variable_replaces_usage_with_expression() {
    // my $sum = $a + $b;
    // print $sum;
    // =>
    // print $a + $b;
    let source = "my $sum = $a + $b;\nprint $sum;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 18));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // Should replace $sum with "$a + $b"
    let has_expr_replacement = inline_action.edit.changes.iter().any(|e| {
        e.new_text.contains("$a") && e.new_text.contains("$b")
    });
    assert!(
        has_expr_replacement,
        "Should replace $sum with '$a + $b', got edits: {:?}",
        inline_action.edit.changes
    );
}

/// Inline action should NOT be offered when the variable is used in its own initializer
/// (this would create circular references).
#[test]
fn inline_variable_not_offered_on_self_referential() {
    // my $x = $x + 1;  -- self-referential, cannot inline
    let source = "my $x = $x + 1;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 16));

    let inline_actions: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a.kind, CodeActionKind::RefactorInline))
        .collect();

    assert!(
        inline_actions.is_empty(),
        "Should NOT offer inline action for self-referential variable, got: {:?}",
        inline_actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

/// Inline variable should work with function call initializers.
#[test]
fn inline_variable_with_function_call_initializer() {
    // my $len = length("hello");
    // print $len;
    // =>
    // print length("hello");
    let source = "my $len = length(\"hello\");\nprint $len;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 25));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // The replacement should contain length("hello")
    let has_fn_replacement = inline_action.edit.changes.iter().any(|e| {
        e.new_text.contains("length")
    });
    assert!(
        has_fn_replacement,
        "Should replace $len with length(\"hello\"), got: {:?}",
        inline_action.edit.changes
    );
}

/// Inline variable should be usable from the basic CodeActionsProvider
/// (not just EnhancedCodeActionsProvider), as it's a standard LSP refactoring.
#[test]
fn inline_variable_available_via_code_actions_provider() {
    let source = "my $temp = 10;\nprint $temp;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, 12), &[]);

    let inline_actions: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a.kind, CodeActionKind::RefactorInline))
        .collect();

    assert!(
        !inline_actions.is_empty(),
        "Inline variable should be available via CodeActionsProvider, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

/// Multiple usages of the same variable should all be replaced.
#[test]
fn inline_variable_replaces_all_usages() {
    // my $msg = "hello";
    // print $msg;
    // print $msg;
    // =>
    // print "hello";
    // print "hello";
    let source = "my $msg = \"hello\";\nprint $msg;\nprint $msg;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 17));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // Count replacements of $msg with "hello"
    let replacement_count = inline_action.edit.changes.iter()
        .filter(|e| e.new_text.contains("hello"))
        .count();

    assert!(
        replacement_count >= 2,
        "Expected at least 2 replacements of $msg with \"hello\", got {}",
        replacement_count
    );
}

/// Variables with no usages (unused) should still be inlineable -
/// the action just removes the declaration.
#[test]
fn inline_variable_with_no_usages_removes_only_declaration() {
    // my $unused = 42;
    // (no usages)
    let source = "my $unused = 42;\nprint \"done\";\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 15));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // Should only have one edit - removing the declaration
    assert!(
        !inline_action.edit.changes.is_empty(),
        "Should have at least one edit to remove declaration"
    );

    // All edits should either remove the declaration or not touch $unused elsewhere
    // (since there are no other usages)
    let has_removal = inline_action.edit.changes.iter().any(|e| e.new_text.is_empty());
    assert!(has_removal, "Should remove the unused variable declaration");
}

/// Inline action should be offered when cursor is on a variable usage,
/// not just on the declaration.
#[test]
fn inline_variable_action_offered_on_usage() {
    // my $temp = 5;
    // print $temp;
    //        ^-- cursor here
    let source = "my $temp = 5;\nprint $temp;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Position on "$temp" in the print statement (bytes 17..22)
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (17, 22));

    let inline_actions: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a.kind, CodeActionKind::RefactorInline))
        .collect();

    assert!(
        !inline_actions.is_empty(),
        "Expected inline action when cursor is on usage, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

/// Inline variable with hash access initializer.
#[test]
fn inline_variable_with_hash_access() {
    // my $val = $hash{$key};
    // print $val;
    // =>
    // print $hash{$key};
    let source = "my $val = $hash{$key};\nprint $val;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 21));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // The replacement should contain $hash{$key}
    let has_hash_replacement = inline_action.edit.changes.iter().any(|e| {
        e.new_text.contains("$hash")
    });
    assert!(
        has_hash_replacement,
        "Should replace $val with $hash{{$key}}, got: {:?}",
        inline_action.edit.changes
    );
}

/// Inline variable with array access initializer.
#[test]
fn inline_variable_with_array_access() {
    // my $item = $arr[0];
    // print $item;
    // =>
    // print $arr[0];
    let source = "my $item = $arr[0];\nprint $item;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 18));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // The replacement should contain $arr[0]
    let has_array_replacement = inline_action.edit.changes.iter().any(|e| {
        e.new_text.contains("$arr")
    });
    assert!(
        has_array_replacement,
        "Should replace $item with $arr[0], got: {:?}",
        inline_action.edit.changes
    );
}

/// Inline should work with my($var) style declaration without initializer default.
#[test]
fn inline_variable_with_list_assignment() {
    // my ($a, $b) = (1, 2);
    // print $a;
    // =>
    // print 1;
    let source = "my ($a, $b) = (1, 2);\nprint $a;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 23));

    let inline_action = must_some(actions.iter().find(|a| {
        matches!(a.kind, CodeActionKind::RefactorInline)
    }));

    // Should have edits to replace $a usages
    assert!(
        !inline_action.edit.changes.is_empty(),
        "Should have edits for inline refactoring"
    );
}
