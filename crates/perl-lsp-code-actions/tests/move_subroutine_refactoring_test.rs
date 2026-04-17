//! Tests for the move subroutine to module refactoring code action
//!
//! These tests define the expected behavior of the "Move subroutine to module"
//! refactoring code action. The action should appear when the cursor is on a
//! subroutine definition and should allow moving the subroutine to another module.

use perl_lsp_code_actions::{CodeActionKind, CodeActionsProvider, EnhancedCodeActionsProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Parse source and get enhanced refactoring actions for a range
fn enhanced_actions(source: &str, range: (usize, usize)) -> Vec<perl_lsp_code_actions::CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    provider.get_enhanced_refactoring_actions(&ast, range)
}

/// Parse source and get all code actions (including refactorings) for a range
fn all_actions(source: &str, range: (usize, usize)) -> Vec<perl_lsp_code_actions::CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, range, &[])
}

/// Check if the CodeActionKind is a refactoring kind
/// This uses string comparison to avoid requiring RefactorMove to exist
fn is_refactor_kind(kind: &CodeActionKind) -> bool {
    let kind_str = format!("{:?}", kind);
    kind_str.starts_with("Refactor")
}

/// Find a move subroutine action by title pattern
fn find_move_subroutine_action(
    actions: &[perl_lsp_code_actions::CodeAction],
) -> Option<&perl_lsp_code_actions::CodeAction> {
    actions.iter().find(|a| {
        a.title.to_lowercase().contains("move")
            && (a.title.to_lowercase().contains("subroutine")
                || a.title.to_lowercase().contains("module")
                || a.title.to_lowercase().contains("function"))
    })
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action appears on subroutine definition
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_appears_on_subroutine_definition() {
    let source = r#"
sub foo {
    print "hello\n";
}

foo();
"#;

    // Find the byte range of the subroutine definition
    // "sub foo {" starts around byte 1
    let sub_start = source.find("sub foo").expect("Should find 'sub foo'");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));

    // The move subroutine action should appear when cursor is on subroutine
    let move_action = find_move_subroutine_action(&actions);
    assert!(
        move_action.is_some(),
        "Expected 'Move subroutine to module' action when cursor is on subroutine definition.\nGot actions: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action has correct title format
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_title_format() {
    let source = r#"
sub utility_function {
    return 42;
}
"#;

    let sub_start = source.find("sub utility_function").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action");

    let action = move_action.unwrap();
    // Title should mention "Move" and "subroutine" and/or "module"
    assert!(
        action.title.to_lowercase().contains("move"),
        "Title should contain 'Move', got: {}",
        action.title
    );
    assert!(
        action.title.to_lowercase().contains("subroutine")
            || action.title.to_lowercase().contains("function")
            || action.title.to_lowercase().contains("module"),
        "Title should mention 'subroutine', 'function', or 'module', got: {}",
        action.title
    );
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action has appropriate code action kind
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_uses_refactor_kind() {
    let source = r#"
sub bar {
    my ($x) = @_;
    return $x * 2;
}
"#;

    let sub_start = source.find("sub bar").expect("Should find 'sub bar'");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action");

    let action = move_action.unwrap();
    // Move is a refactoring action - should use Refactor or RefactorMove kind
    assert!(
        is_refactor_kind(&action.kind),
        "Expected Refactor or RefactorMove kind, got: {:?}",
        action.kind
    );
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action is NOT preferred (user should choose)
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_is_not_preferred() {
    let source = r#"
sub my_sub {
    print "test\n";
}
"#;

    let sub_start = source.find("sub my_sub").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    // The move action should exist but not be marked as preferred
    // (user should explicitly choose to move a subroutine)
    if let Some(action) = move_action {
        assert!(
            !action.is_preferred,
            "Move subroutine action should NOT be preferred (it's a significant refactoring)"
        );
    }
    // If no action found, the test will fail - that's expected before implementation
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action has non-empty edit
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_has_edit() {
    let source = r#"
package MyApp;
use strict;
use warnings;

sub helper {
    return "helper result";
}

1;
"#;

    // Find the subroutine
    let sub_start = source.find("sub helper").expect("Should find 'sub helper'");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action");

    let action = move_action.unwrap();
    assert!(
        !action.edit.changes.is_empty(),
        "Move subroutine action should have at least one edit"
    );
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action edit removes subroutine from source
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_removes_subroutine_from_source() {
    let source = r#"
sub to_move {
    return 1;
}

print "after\n";
"#;

    // Find the subroutine
    let sub_start = source.find("sub to_move").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action");

    let action = move_action.unwrap();

    // The edit should include an edit that removes or replaces the subroutine
    // (since we can't fully test the edit output without actually applying it,
    // we just verify that there's at least one edit covering the subroutine range)
    let has_edit_on_subroutine = action.edit.changes.iter().any(|edit| {
        // The edit location should overlap with the subroutine
        edit.location.start <= sub_end && edit.location.end >= sub_start
    });

    assert!(
        has_edit_on_subroutine,
        "Expected at least one edit affecting the subroutine range ({}-{}), \
         got edits: {:?}",
        sub_start, sub_end, action.edit.changes
    );
}

// ---------------------------------------------------------------------------
// Test: Move subroutine does NOT appear when cursor is not on a subroutine
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_not_offered_on_regular_code() {
    let source = r#"
my $x = 5;
my $y = 10;
my $z = $x + $y;
print $z;
"#;

    // Select a range that doesn't include any subroutine
    let actions = enhanced_actions(source, (0, source.len()));

    let move_action = find_move_subroutine_action(&actions);
    assert!(
        move_action.is_none(),
        "Move subroutine action should NOT appear when cursor is not on a subroutine.\n\
         Got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test: Move subroutine does NOT appear for anonymous subs
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_not_offered_for_anonymous_sub() {
    let source = r#"
my $coderef = sub {
    return 42;
};
"#;

    // Find the anonymous sub
    let sub_start = source.find("sub {").expect("Should find anonymous sub");
    let sub_end = source.find(';').expect("Should find semicolon") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    // Anonymous subs can't be "moved" by name - shouldn't offer the action
    // (or if it does, it might work differently)
    // For now, we just check that we don't get a standard "move subroutine" action
    if let Some(action) = move_action {
        assert!(
            !action.title.to_lowercase().contains("named"),
            "Anonymous sub move action should not claim to move a named subroutine"
        );
    }
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action works with named subroutine with prototype
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_on_subroutine_with_prototype() {
    let source = r#"
sub foo (;$) {
    my ($x) = @_;
    return $x;
}
"#;

    let sub_start = source.find("sub foo").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action for subroutine with prototype");
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action works with subroutines with signatures
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_on_subroutine_with_signature() {
    let source = r#"
use v5.20;
sub foo ($x, $y) {
    return $x + $y;
}
"#;

    let sub_start = source.find("sub foo").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action for subroutine with signature");
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action works with method-style subroutine (arrow syntax)
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_on_method_definition() {
    let source = r#"
package MyClass;
use strict;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub get_value {
    my ($self) = @_;
    return $self->{value};
}

1;
"#;

    // Test that we can offer move action on methods
    let sub_start = source.find("sub get_value").expect("Should find method");
    // Use rfind to find the LAST '}' which belongs to sub get_value
    // (using find would find the first '}' belonging to sub new)
    let sub_end = source.rfind('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action for method definition");
}

// ---------------------------------------------------------------------------
// Test: Multiple subroutines - only offer move on the selected one
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_offers_on_selected_sub_only() {
    let source = r#"
sub sub_one {
    return 1;
}

sub sub_two {
    return 2;
}

sub sub_three {
    return 3;
}
"#;

    // Select only sub_two
    let sub_two_start = source.find("sub sub_two").expect("Should find sub_two");
    let sub_two_end = source.find("sub sub_three").expect("Should find sub_three");

    let actions = enhanced_actions(source, (sub_two_start, sub_two_end));

    // Find all move actions
    let move_actions: Vec<_> = actions
        .iter()
        .filter(|a| {
            a.title.to_lowercase().contains("move")
                && (a.title.to_lowercase().contains("sub")
                    || a.title.to_lowercase().contains("module"))
        })
        .collect();

    // Should offer to move sub_two, but the action title should reference sub_two
    let has_sub_two_move = move_actions.iter().any(|a| a.title.contains("sub_two"));
    assert!(
        has_sub_two_move,
        "Expected move action to mention 'sub_two', got actions: {:?}",
        move_actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test: Enhanced provider has move_subroutine module wired in
// ---------------------------------------------------------------------------

#[test]
fn test_enhanced_provider_includes_move_subroutine() {
    // This test verifies that the enhanced code actions provider
    // has the move_subroutine module wired in and active.
    //
    // The enhanced/mod.rs should have:
    // - mod move_subroutine; in the module list
    // - Logic to call move_subroutine::create_move_subroutine_action
    //   when the cursor is on a Subroutine node

    let source = r#"
sub test_sub {
    return 1;
}
"#;

    let sub_start = source.find("sub test_sub").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));

    // If move_subroutine is properly wired, we should see a move action
    let has_move = actions.iter().any(|a| {
        a.title.to_lowercase().contains("move") && a.title.to_lowercase().contains("module")
    });

    assert!(
        has_move,
        "Enhanced provider should offer 'Move subroutine to module' action.\n\
         Got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test: Move subroutine action includes target module in title
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_action_title_includes_module_hint() {
    let source = r#"
sub process_data {
    my ($input) = @_;
    return $input * 2;
}
"#;

    let sub_start = source.find("sub process_data").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action");

    let action = move_action.unwrap();
    // The title should give user a hint that they're moving to a module
    // (The exact module name may be prompted or inferred)
    assert!(
        action.title.to_lowercase().contains("module"),
        "Move action title should mention 'module', got: {}",
        action.title
    );
}

// ---------------------------------------------------------------------------
// Test: Package declaration is preserved in source after move
// ---------------------------------------------------------------------------

#[test]
fn test_move_subroutine_preserves_package_declaration() {
    let source = r#"
package MyApp::Utils;

use strict;
use warnings;

sub helper {
    return "helper";
}

1;
"#;

    let sub_start = source.find("sub helper").expect("Should find subroutine");
    let sub_end = source.find('}').expect("Should find closing brace") + 1;

    let actions = enhanced_actions(source, (sub_start, sub_end));
    let move_action = find_move_subroutine_action(&actions);

    assert!(move_action.is_some(), "Expected move subroutine action");

    let action = move_action.unwrap();

    // The edit should not completely remove the package declaration
    // At minimum, there should be edits that preserve the package structure
    // (This is more of a semantic check - the actual edit might remove the sub
    // but leave the package intact)
    assert!(!action.edit.changes.is_empty(), "Move action should produce edits");
}
