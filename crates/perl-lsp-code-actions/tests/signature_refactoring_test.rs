//! Tests for the add-parameter signature refactoring code action.

use perl_lsp_code_actions::{CodeActionKind, EnhancedCodeActionsProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;

/// Apply a list of byte-offset edits to source (sorts descending by start to avoid index shift).
fn apply_edits(source: &str, action: &perl_lsp_code_actions::CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut output = source.to_string();
    for edit in edits {
        output.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    output
}

// ---------------------------------------------------------------------------
// Happy path — named sub with signature, single call site
// ---------------------------------------------------------------------------

#[test]
fn add_parameter_action_exists_for_named_sub_with_signature() {
    let source = "use feature 'signatures';\nsub process ($data) { return length($data); }\nmy $r = process($v1);\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    // Cursor on the signature line — byte range covering sub keyword to closing paren
    let sub_start = source.find("sub process").expect("sub process in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature");
    assert!(
        action.is_some(),
        "Expected 'Add parameter to signature' action, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn add_parameter_action_kind_is_refactor_rewrite() {
    let source = "use feature 'signatures';\nsub process ($data) { return length($data); }\nmy $r = process($v1);\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let sub_start = source.find("sub process").expect("sub process in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature").unwrap();
    assert_eq!(
        action.kind,
        CodeActionKind::RefactorRewrite,
        "Add parameter action should be RefactorRewrite"
    );
}

#[test]
fn add_parameter_updates_signature_and_one_call_site() {
    // sub with one call site
    let source = "use feature 'signatures';\nsub process ($data) { return length($data); }\nmy $r = process($v1);\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let sub_start = source.find("sub process").expect("sub process in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature").unwrap();

    // Exactly 2 edits: 1 signature + 1 call site
    assert_eq!(
        action.edit.changes.len(),
        2,
        "Expected 2 edits (1 signature + 1 call site), got {} edits",
        action.edit.changes.len()
    );

    let result = apply_edits(source, action);

    // Signature updated
    assert!(
        result.contains("sub process ($data, $options = {})"),
        "Expected signature with new param, got:\n{}",
        result
    );
    // Call site updated
    assert!(result.contains("process($v1, {})"), "Expected call site updated, got:\n{}", result);
}

#[test]
fn add_parameter_updates_three_call_sites() {
    let source = concat!(
        "use feature 'signatures';\n",
        "sub process ($data) { return length($data); }\n",
        "my $r1 = process($v1);\n",
        "my $r2 = process($v2);\n",
        "my $r3 = process($v3);\n",
    );
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let sub_start = source.find("sub process").expect("sub process in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature").unwrap();

    // 4 edits: 1 signature + 3 call sites
    assert_eq!(
        action.edit.changes.len(),
        4,
        "Expected 4 edits (1 signature + 3 call sites), got {} edits: {:?}",
        action.edit.changes.len(),
        action.edit.changes.iter().map(|e| (&e.location, &e.new_text)).collect::<Vec<_>>()
    );

    let result = apply_edits(source, action);

    assert!(result.contains("sub process ($data, $options = {})"));
    assert!(result.contains("process($v1, {})"));
    assert!(result.contains("process($v2, {})"));
    assert!(result.contains("process($v3, {})"));
}

// ---------------------------------------------------------------------------
// Rejection cases
// ---------------------------------------------------------------------------

#[test]
fn no_add_parameter_action_for_anonymous_sub() {
    let source = "my $f = sub ($x) { $x * 2 };\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature");
    assert!(action.is_none(), "Should not offer add-parameter for anonymous sub");
}

#[test]
fn no_add_parameter_action_when_last_param_is_slurpy() {
    let source = "use feature 'signatures';\nsub process ($data, @rest) { }\nprocess(1, 2, 3);\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let sub_start = source.find("sub process").expect("sub process in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature");
    assert!(action.is_none(), "Should not offer add-parameter when last param is slurpy");
}

#[test]
fn no_add_parameter_action_for_sub_without_signature() {
    // Old-style sub with no signature
    let source = "sub process { my $data = shift; return length($data); }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let sub_start = source.find("sub process").expect("sub process in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature");
    assert!(action.is_none(), "Should not offer add-parameter for sub without signature");
}

// ---------------------------------------------------------------------------
// Edit structure
// ---------------------------------------------------------------------------

#[test]
fn add_parameter_edits_have_no_overlapping_ranges() {
    let source = concat!(
        "use feature 'signatures';\n",
        "sub process ($data) { return length($data); }\n",
        "my $r1 = process($v1);\n",
        "my $r2 = process($v2);\n",
    );
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let sub_start = source.find("sub process").expect("sub process in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature").unwrap();

    // Check no overlapping edits
    let mut sorted = action.edit.changes.clone();
    sorted.sort_by_key(|e| e.location.start);
    for pair in sorted.windows(2) {
        assert!(
            pair[0].location.end <= pair[1].location.start,
            "Overlapping edits: [{}, {}] and [{}, {}]",
            pair[0].location.start,
            pair[0].location.end,
            pair[1].location.start,
            pair[1].location.end
        );
    }
}

#[test]
fn add_parameter_signature_edit_inserts_before_closing_paren() {
    let source = "use feature 'signatures';\nsub foo ($x) { $x + 1 }\nfoo(1);\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let sub_start = source.find("sub foo").expect("sub foo in source");
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    let actions = provider.get_enhanced_refactoring_actions(&ast, (sub_start, sub_start + 1));

    let action = actions.iter().find(|a| a.title == "Add parameter to signature").unwrap();
    let result = apply_edits(source, action);

    assert!(
        result.contains("sub foo ($x, $options = {})"),
        "Expected signature with $options = {{}}, got:\n{}",
        result
    );
    assert!(result.contains("foo(1, {})"), "Expected call updated, got:\n{}", result);
}
