//! BDD-style behavior specification tests for perl-lsp-code-actions
//!
//! These tests describe *what* the code actions system does from a user's
//! perspective, not *how* it is implemented. Each test name reads as a
//! behavior statement: "when <situation>, the system <does X>."
//!
//! Coverage targets:
//! - Quick-fix code actions triggered by diagnostic codes
//! - Refactoring actions offered based on cursor position / selection
//! - Enhanced actions: pragma insertion, import management, postfix conversion
//! - Edge cases: no diagnostics, empty source, overlapping ranges

use perl_lsp_code_actions::{
    CodeAction, CodeActionKind, CodeActionsProvider, EnhancedCodeActionsProvider,
};
use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser_core::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn diag(start: usize, end: usize, code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Warning,
        code: Some(code.to_string()),
        message: message.to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

fn actions_for(source: &str, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, (0, source.len()), diagnostics)
}

fn enhanced_actions_for(source: &str, range: (usize, usize)) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    provider.get_enhanced_refactoring_actions(&ast, range)
}

fn has_action(actions: &[CodeAction], title_fragment: &str) -> bool {
    actions.iter().any(|a| a.title.contains(title_fragment))
}

fn preferred_action(actions: &[CodeAction]) -> Option<&CodeAction> {
    actions.iter().find(|a| a.is_preferred)
}

// ===========================================================================
// Quick-fix scenarios: unused variable
// ===========================================================================

#[test]
fn when_cursor_on_unused_variable_offers_remove_action() {
    let source = "my $unused = 42;\nprint 'hello';";
    let diagnostics = [diag(0, 16, "unused-variable", "Unused variable '$unused'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Remove unused variable"),
        "should offer to remove the unused variable declaration, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_cursor_on_unused_variable_offers_underscore_rename() {
    let source = "my $unused = 42;\nprint 'hello';";
    let diagnostics = [diag(0, 16, "unused-variable", "Unused variable '$unused'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Rename to '_$unused'"),
        "should offer to prefix with underscore, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_cursor_on_unused_variable_preferred_action_is_remove() {
    let source = "my $temp = 1;";
    let diagnostics = [diag(0, 13, "unused-variable", "Unused variable '$temp'")];
    let actions = actions_for(source, &diagnostics);

    let pref = preferred_action(&actions);
    assert!(
        pref.is_some_and(|a| a.title.contains("Remove")),
        "preferred action should be removal, got: {:?}",
        pref.map(|a| &a.title)
    );
}

// ===========================================================================
// Quick-fix scenarios: undefined / undeclared variable
// ===========================================================================

#[test]
fn when_cursor_on_undefined_variable_offers_my_declaration() {
    let source = "print $missing;";
    let diagnostics = [diag(6, 14, "undefined-variable", "Undefined variable '$missing'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Declare '$missing' with 'my'"),
        "should offer 'my' declaration, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_cursor_on_undefined_variable_offers_our_declaration() {
    let source = "print $missing;";
    let diagnostics = [diag(6, 14, "undefined-variable", "Undefined variable '$missing'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Declare '$missing' with 'our'"),
        "should offer 'our' declaration as alternative, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_cursor_on_undefined_variable_my_is_preferred_over_our() {
    let source = "print $missing;";
    let diagnostics = [diag(6, 14, "undefined-variable", "Undefined variable '$missing'")];
    let actions = actions_for(source, &diagnostics);

    let pref = preferred_action(&actions);
    assert!(
        pref.is_some_and(|a| a.title.contains("my")),
        "preferred action should be 'my' declaration, got: {:?}",
        pref.map(|a| &a.title)
    );
}

// ===========================================================================
// Quick-fix scenarios: assignment in condition
// ===========================================================================

#[test]
fn when_assignment_in_condition_offers_comparison_fix() {
    let source = "if ($x = 5) { }";
    let diagnostics = [diag(4, 10, "assignment-in-condition", "Assignment in condition")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Change to comparison (==)"),
        "should offer to convert = to ==, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_assignment_in_condition_offers_parentheses_wrapper() {
    let source = "if ($x = 5) { }";
    let diagnostics = [diag(4, 10, "assignment-in-condition", "Assignment in condition")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Keep assignment (add parentheses)"),
        "should offer to wrap assignment in parentheses to indicate intent, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: missing pragmas
// ===========================================================================

#[test]
fn when_missing_strict_offers_add_use_strict() {
    let source = "my $x = 1;";
    let diagnostics = [diag(0, 10, "missing-strict", "Missing 'use strict'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add 'use strict'"),
        "should offer to add use strict pragma, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_missing_warnings_offers_add_use_warnings() {
    let source = "my $x = 1;";
    let diagnostics = [diag(0, 10, "missing-warnings", "Missing 'use warnings'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add 'use warnings'"),
        "should offer to add use warnings pragma, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_strict_and_warnings_both_missing_offers_both() {
    let source = "my $x = 1;";
    let diagnostics = [
        diag(0, 10, "missing-strict", "Missing 'use strict'"),
        diag(0, 10, "missing-warnings", "Missing 'use warnings'"),
    ];
    let actions = actions_for(source, &diagnostics);

    assert!(has_action(&actions, "use strict"), "should offer use strict");
    assert!(has_action(&actions, "use warnings"), "should offer use warnings");
}

// ===========================================================================
// Quick-fix scenarios: parse errors
// ===========================================================================

#[test]
fn when_missing_semicolon_offers_add_semicolon() {
    let source = "my $x = 1\nprint $x;";
    let diagnostics = [diag(0, 9, "parse-error-missingsemicolon", "Missing semicolon")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add missing semicolon"),
        "should offer to add a semicolon, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_unclosed_parenthesis_offers_add_closing_paren() {
    let source = "print(42";
    let diagnostics = [diag(0, 8, "parse-error-unclosedparenthesis", "Unclosed parenthesis")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add closing parenthesis"),
        "should offer to close the parenthesis, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_unclosed_brace_offers_add_closing_brace() {
    let source = "sub foo {";
    let diagnostics = [diag(0, 9, "parse-error-unclosedbrace", "Unclosed brace")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add closing brace"),
        "should offer to close the brace, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_unclosed_string_offers_add_closing_quote() {
    let source = r#"my $x = "hello"#;
    let diagnostics = [diag(8, 14, "parse-error-unclosedstring", "Unclosed string")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add closing quote"),
        "should offer to close the string, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_unclosed_bracket_offers_add_closing_bracket() {
    let source = "my @a = (1, 2, [3";
    let diagnostics = [diag(16, 18, "parse-error-unclosedbracket", "Unclosed bracket")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add closing bracket"),
        "should offer to close the bracket, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: deprecated patterns
// ===========================================================================

#[test]
fn when_deprecated_defined_array_offers_replacement() {
    let source = "if (defined @arr) { }";
    // Range covers "defined @arr"
    let diagnostics = [diag(4, 16, "deprecated-defined", "deprecated use of defined() on array")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        actions.iter().any(|a| a.title.contains("Replace")),
        "should offer to replace deprecated defined() call, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: bareword
// ===========================================================================

#[test]
fn when_unquoted_bareword_offers_single_quote() {
    let source = "my $x = hello;";
    let diagnostics = [diag(8, 13, "unquoted-bareword", "Bareword 'hello'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "single quotes"),
        "should offer to wrap bareword in single quotes, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_unquoted_bareword_offers_double_quote() {
    let source = "my $x = hello;";
    let diagnostics = [diag(8, 13, "unquoted-bareword", "Bareword 'hello'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "double quotes"),
        "should offer to wrap bareword in double quotes, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_uppercase_bareword_additionally_offers_filehandle_declaration() {
    let source = "my $x = STDOUT;";
    let diagnostics = [diag(8, 14, "unquoted-bareword", "Bareword 'STDOUT'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "filehandle"),
        "should offer filehandle declaration for uppercase barewords, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_lowercase_bareword_does_not_offer_filehandle_declaration() {
    let source = "my $x = hello;";
    let diagnostics = [diag(8, 13, "unquoted-bareword", "Bareword 'hello'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        !has_action(&actions, "filehandle"),
        "should NOT offer filehandle for lowercase barewords, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: unused parameter
// ===========================================================================

#[test]
fn when_unused_parameter_offers_underscore_rename() {
    let source = "sub foo { my ($bar) = @_; }";
    let diagnostics = [diag(14, 18, "unused-parameter", "Unused parameter '$bar'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Rename to '_$bar'"),
        "should offer underscore prefix for unused parameters, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: variable shadowing
// ===========================================================================

#[test]
fn when_variable_shadows_outer_scope_offers_rename_suggestions() {
    let source = "my $x = 1; { my $x = 2; }";
    let diagnostics =
        [diag(17, 19, "variable-shadowing", "Variable '$x' shadows outer declaration")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Rename to '$x_inner'"),
        "should suggest _inner suffix, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
    assert!(
        has_action(&actions, "Rename to '$x_local'"),
        "should suggest _local suffix, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Enhanced refactoring: extract variable
// ===========================================================================

#[test]
fn when_expression_selected_offers_extract_to_variable() {
    let source = "my $result = length($input) + 10;";
    let actions = enhanced_actions_for(source, (13, 27)); // "length($input)"

    assert!(
        has_action(&actions, "Extract"),
        "should offer extract-to-variable for expression selection, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Enhanced refactoring: postfix conversion
// ===========================================================================

#[test]
fn when_cursor_on_simple_if_block_offers_postfix_conversion() {
    let source = "if ($debug) { print \"Debug\\n\"; }";
    let actions = enhanced_actions_for(source, (0, source.len()));

    assert!(
        has_action(&actions, "postfix"),
        "should offer to convert block-form if to postfix, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Enhanced refactoring: error checking
// ===========================================================================

#[test]
fn when_cursor_on_open_call_offers_error_checking() {
    let source = "open my $fh, '<', 'file.txt';";
    let actions = enhanced_actions_for(source, (0, source.len()));

    assert!(
        has_action(&actions, "error checking"),
        "should offer to add error checking around open(), got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Enhanced refactoring: pragma insertion
// ===========================================================================

#[test]
fn when_source_lacks_strict_enhanced_offers_add_pragmas() {
    let source = "my $x = 1;";
    let actions = enhanced_actions_for(source, (0, source.len()));

    assert!(
        actions.iter().any(|a| a.title.contains("pragmas") || a.title.contains("strict")),
        "should offer to add missing pragmas when strict is absent, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_source_has_strict_and_warnings_does_not_offer_pragma_action() {
    let source = "use strict;\nuse warnings;\nmy $x = 1;";
    let actions = enhanced_actions_for(source, (0, source.len()));

    assert!(
        !actions.iter().any(|a| a.title.contains("missing pragmas")),
        "should not offer add-missing-pragmas when both are present, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn when_no_diagnostics_no_diagnostic_driven_quickfixes_returned() {
    let source = "use strict;\nuse warnings;\nmy $x = 1;";
    let actions = actions_for(source, &[]);

    // With no diagnostics and pragmas present, no quickfix actions referencing
    // specific diagnostic codes should appear.
    let diagnostic_quickfixes: Vec<_> = actions
        .iter()
        .filter(|a| a.kind == CodeActionKind::QuickFix && !a.diagnostics.is_empty())
        .collect();
    assert!(
        diagnostic_quickfixes.is_empty(),
        "without diagnostics, no diagnostic-driven QuickFix actions should be generated, got: {:?}",
        diagnostic_quickfixes.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_empty_source_no_panic() {
    let source = "";
    let actions = actions_for(source, &[]);
    // Simply verifying no panic; empty source may or may not produce actions
    let _ = actions;
}

#[test]
fn when_diagnostic_has_unknown_code_no_action_produced() {
    let source = "my $x = 1;";
    let diagnostics = [diag(0, 10, "totally-unknown-code", "Unknown issue")];
    let actions = actions_for(source, &diagnostics);

    // Should not crash and should not produce a quickfix for an unknown code
    assert!(
        !actions.iter().any(|a| a.kind == CodeActionKind::QuickFix
            && a.diagnostics.contains(&"totally-unknown-code".to_string())),
        "should not produce a quickfix for unknown diagnostic codes, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_multiple_diagnostics_produces_actions_for_each() {
    let source = "print $a;\nprint $b;";
    let diagnostics = [
        diag(6, 8, "undefined-variable", "Undefined variable '$a'"),
        diag(16, 18, "undefined-variable", "Undefined variable '$b'"),
    ];
    let actions = actions_for(source, &diagnostics);

    // Should have actions for both $a and $b
    assert!(
        has_action(&actions, "'$a'"),
        "should produce actions for first diagnostic ($a), got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
    assert!(
        has_action(&actions, "'$b'"),
        "should produce actions for second diagnostic ($b), got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn diagnostic_driven_quick_fix_actions_have_diagnostic_codes() {
    let source = "use strict;\nuse warnings;\nif ($x = 5) { }";
    // Offset adjusted: "use strict;\nuse warnings;\n" = 26 bytes, "$x = 5" at 30..36
    let diagnostics = [diag(30, 36, "assignment-in-condition", "Assignment in condition")];
    let actions = actions_for(source, &diagnostics);

    // Filter to only the actions generated in response to the supplied diagnostics
    let diag_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.kind == CodeActionKind::QuickFix && !a.diagnostics.is_empty())
        .collect();

    assert!(!diag_actions.is_empty(), "should have at least one diagnostic-driven quickfix");
    for action in &diag_actions {
        assert!(
            !action.diagnostics.is_empty(),
            "Diagnostic-driven QuickFix '{}' should reference at least one diagnostic code",
            action.title
        );
    }
}

#[test]
fn all_actions_have_non_empty_titles() {
    let source = "print $undefined;";
    let diagnostics = [diag(6, 16, "undefined-variable", "Undefined variable '$undefined'")];
    let actions = actions_for(source, &diagnostics);

    for action in &actions {
        assert!(!action.title.is_empty(), "every code action must have a non-empty title");
    }
}

#[test]
fn all_actions_have_non_empty_edits() {
    let source = "my $x = 1;";
    let diagnostics = [diag(0, 10, "missing-strict", "Missing 'use strict'")];
    let actions = actions_for(source, &diagnostics);

    for action in &actions {
        assert!(
            !action.edit.changes.is_empty(),
            "code action '{}' must produce at least one edit",
            action.title
        );
    }
}
