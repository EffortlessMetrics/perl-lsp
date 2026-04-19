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
    let diagnostics = [diag(0, 16, "PL102", "Unused variable '$unused'")];
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
    let diagnostics = [diag(0, 16, "PL102", "Unused variable '$unused'")];
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
    let diagnostics = [diag(0, 13, "PL102", "Unused variable '$temp'")];
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
    let diagnostics = [diag(6, 14, "PL103", "Undefined variable '$missing'")];
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
    let diagnostics = [diag(6, 14, "PL103", "Undefined variable '$missing'")];
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
    let diagnostics = [diag(6, 14, "PL103", "Undefined variable '$missing'")];
    let actions = actions_for(source, &diagnostics);

    let pref = preferred_action(&actions);
    assert!(
        pref.is_some_and(|a| a.title.contains("my")),
        "preferred action should be 'my' declaration, got: {:?}",
        pref.map(|a| &a.title)
    );
}

#[test]
fn when_cursor_on_undeclared_variable_alias_offers_same_declarations() {
    let source = "print $missing;";
    // PL103 is the stable code for both undefined-variable and undeclared-variable
    let diagnostics = [diag(6, 14, "PL103", "Undefined variable '$missing'")];
    let actions = actions_for(source, &diagnostics);

    assert!(has_action(&actions, "Declare '$missing' with 'my'"));
    assert!(has_action(&actions, "Declare '$missing' with 'our'"));
}

// ===========================================================================
// Quick-fix scenarios: assignment in condition
// ===========================================================================

#[test]
fn when_assignment_in_condition_offers_comparison_fix() {
    let source = "if ($x = 5) { }";
    let diagnostics = [diag(4, 10, "PL403", "Assignment in condition")];
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
    let diagnostics = [diag(4, 10, "PL403", "Assignment in condition")];
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
    let diagnostics = [diag(0, 10, "PL100", "Missing 'use strict'")];
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
    let diagnostics = [diag(0, 10, "PL101", "Missing 'use warnings'")];
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
        diag(0, 10, "PL100", "Missing 'use strict'"),
        diag(0, 10, "PL101", "Missing 'use warnings'"),
    ];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "use strict"),
        "should offer use strict"
    );
    assert!(
        has_action(&actions, "use warnings"),
        "should offer use warnings"
    );
}

// ===========================================================================
// Quick-fix scenarios: parse errors
// ===========================================================================

#[test]
fn when_missing_semicolon_offers_add_semicolon() {
    let source = "my $x = 1\nprint $x;";
    let diagnostics = [diag(
        0,
        9,
        "parse-error-missingsemicolon",
        "Missing semicolon",
    )];
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
    let diagnostics = [diag(
        0,
        8,
        "parse-error-unclosedparenthesis",
        "Unclosed parenthesis",
    )];
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
fn when_unclosed_block_offers_add_closing_brace() {
    let source = "if ($x) {";
    let diagnostics = [diag(0, 9, "parse-error-unclosedblock", "Unclosed block")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add closing brace"),
        "should offer to close an unclosed block, got: {:?}",
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
    let diagnostics = [diag(
        16,
        18,
        "parse-error-unclosedbracket",
        "Unclosed bracket",
    )];
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
    let diagnostics = [diag(4, 16, "PL500", "deprecated use of defined() on array")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        actions.iter().any(|a| a.title.contains("Replace")),
        "should offer to replace deprecated defined() call, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: numeric comparison with undef
// ===========================================================================

#[test]
fn when_numeric_comparison_uses_undef_offers_defined_check_and_fallback() {
    let source = "if ($value == undef) { }";
    let diagnostics = [diag(4, 19, "PL404", "Numeric comparison with undef")];
    let actions = actions_for(source, &diagnostics);

    assert!(has_action(&actions, "Add defined check"));
    assert!(has_action(&actions, "defined-or operator"));
}

#[test]
fn when_numeric_undef_range_has_no_equality_operator_skips_defined_or_fallback() {
    let source = "print $value;";
    let diagnostics = [diag(6, 12, "PL404", "Numeric comparison with undef")];
    let actions = actions_for(source, &diagnostics);

    assert!(has_action(&actions, "Add defined check"));
    assert!(!has_action(&actions, "defined-or operator"));
}

#[test]
fn when_general_parse_error_mentions_missing_semicolon_offers_semicolon_fix() {
    let source = "my $x = 1\nprint $x;";
    let diagnostics = [diag(
        0,
        9,
        "PL001",
        "Missing semicolon before next statement",
    )];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add missing semicolon"),
        "should map PL001 missing-semicolon diagnostics to semicolon fix, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_general_parse_error_is_heredoc_context_no_semicolon_fix_is_offered() {
    let source = "<<EOF\nhello\nEOF\n";
    let diagnostics = [diag(0, 2, "PL001", "Missing semicolon near heredoc")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        !has_action(&actions, "Add missing semicolon"),
        "should skip semicolon quickfix in heredoc context, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: bareword
// ===========================================================================

#[test]
fn when_unquoted_bareword_offers_single_quote() {
    let source = "my $x = hello;";
    let diagnostics = [diag(8, 13, "PL109", "Bareword 'hello'")];
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
    let diagnostics = [diag(8, 13, "PL109", "Bareword 'hello'")];
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
    let diagnostics = [diag(8, 14, "PL109", "Bareword 'STDOUT'")];
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
    let diagnostics = [diag(8, 13, "PL109", "Bareword 'hello'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        !has_action(&actions, "filehandle"),
        "should NOT offer filehandle for lowercase barewords, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_diagnostic_is_bareword_filehandle_offers_lexical_replacement() {
    let source = "open FILE, '<', 'data.txt';";
    let diagnostics = [diag(5, 9, "PL400", "Bareword filehandle 'FILE'")];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(
            &actions,
            "Replace bareword filehandle 'FILE' with lexical '$file_fh'"
        ),
        "should offer lexical filehandle replacement, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_diagnostic_is_two_arg_open_offers_three_arg_open_fix() {
    let source = "open($fh, $filename);";
    let diagnostics = [diag(
        0,
        source.len(),
        "PL401",
        "Two-argument open is unsafe",
    )];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Convert to three-argument open() for safety"),
        "should offer safer three-argument open conversion, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_range_starts_after_first_line_hardcoded_shebang_fix_is_not_reported() {
    let source = "#!/usr/local/bin/perl -w\nmy $x = 1;\nprint 'ok';\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    let range_start = source.find("print").unwrap_or(0);
    let actions = provider.get_code_actions(&ast, (range_start, source.len()), &[]);

    assert!(
        !actions.iter().any(|a| a.title.contains("portable shebang")),
        "shebang fix should only be offered when the first line is in range, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Quick-fix scenarios: unused parameter
// ===========================================================================

#[test]
fn when_unused_parameter_offers_underscore_rename() {
    let source = "sub foo { my ($bar) = @_; }";
    let diagnostics = [diag(14, 18, "PL108", "Unused parameter '$bar'")];
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
    let diagnostics = [diag(
        17,
        19,
        "PL104",
        "Variable '$x' shadows outer declaration",
    )];
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

#[test]
fn when_missing_return_in_subroutine_offers_explicit_return_fix() {
    let source = "sub compute {\n    my $x = 1;\n}\n";
    let diagnostics = [diag(
        0,
        source.len() - 2,
        "PL301",
        "Subroutine may not return a value",
    )];
    let actions = actions_for(source, &diagnostics);

    assert!(
        has_action(&actions, "Add explicit 'return' statement"),
        "should offer adding explicit return for PL301, got: {:?}",
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
        actions
            .iter()
            .any(|a| a.title.contains("pragmas") || a.title.contains("strict")),
        "should offer to add missing pragmas when strict is absent, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn when_source_starts_with_shebang_pragmas_are_inserted_after_it() {
    let source = "#!/usr/bin/env perl\nmy $x = 1;";
    let actions = enhanced_actions_for(source, (0, source.len()));
    let pragma_action = actions
        .iter()
        .find(|action| action.title.contains("missing pragmas"));
    assert!(pragma_action.is_some(), "expected pragma insertion action");
    let pragma_action = pragma_action.unwrap_or_else(|| unreachable!());

    assert_eq!(pragma_action.edit.changes.len(), 1);
    let change = &pragma_action.edit.changes[0];
    assert_eq!(change.location.start, "#!/usr/bin/env perl\n".len());
    assert!(change.new_text.contains("use strict;"));
    assert!(change.new_text.contains("use warnings;"));
}

#[test]
fn when_imports_are_out_of_order_enhanced_actions_offer_organization() {
    let source = "use My::Local;\nuse strict;\nuse warnings;\n";
    let actions = enhanced_actions_for(source, (0, source.len()));
    let organize = actions
        .iter()
        .find(|action| action.title == "Organize imports");
    assert!(organize.is_some(), "expected organize imports action");
    let organize = organize.unwrap_or_else(|| unreachable!());

    assert_eq!(organize.kind, CodeActionKind::SourceOrganizeImports);
    let change = &organize.edit.changes[0];
    assert_eq!(
        change.new_text,
        "use strict;\nuse warnings;\nuse My::Local;\n"
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
        diagnostic_quickfixes
            .iter()
            .map(|a| &a.title)
            .collect::<Vec<_>>()
    );
}

#[test]
fn when_empty_source_actions_have_valid_structure() {
    let source = "";
    let actions = actions_for(source, &[]);
    // Empty source may produce refactoring actions (e.g. add pragmas); verify
    // that any returned actions have valid structure.
    for action in &actions {
        assert!(
            !action.title.is_empty(),
            "action titles must be non-empty even for empty source"
        );
    }
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
        diag(6, 8, "PL103", "Undefined variable '$a'"),
        diag(16, 18, "PL103", "Undefined variable '$b'"),
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
    let diagnostics = [diag(30, 36, "PL403", "Assignment in condition")];
    let actions = actions_for(source, &diagnostics);

    // Filter to only the actions generated in response to the supplied diagnostics
    let diag_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.kind == CodeActionKind::QuickFix && !a.diagnostics.is_empty())
        .collect();

    assert!(
        !diag_actions.is_empty(),
        "should have at least one diagnostic-driven quickfix"
    );
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
    let diagnostics = [diag(6, 16, "PL103", "Undefined variable '$undefined'")];
    let actions = actions_for(source, &diagnostics);

    for action in &actions {
        assert!(
            !action.title.is_empty(),
            "every code action must have a non-empty title"
        );
    }
}

#[test]
fn all_actions_have_non_empty_edits() {
    let source = "my $x = 1;";
    let diagnostics = [diag(0, 10, "PL100", "Missing 'use strict'")];
    let actions = actions_for(source, &diagnostics);

    for action in &actions {
        assert!(
            !action.edit.changes.is_empty(),
            "code action '{}' must produce at least one edit",
            action.title
        );
    }
}
