//! Comprehensive unit tests for perl-lsp-code-actions
//!
//! Tests cover:
//! - CodeActionsProvider: quick fixes for all diagnostic codes
//! - EnhancedCodeActionsProvider: refactoring, pragma, and import actions
//! - CodeActionKind variants and CodeActionEdit construction
//! - Edge cases: empty source, empty diagnostics, boundary offsets

use perl_lsp_code_actions::{
    CodeAction, CodeActionKind, CodeActionsProvider, EnhancedCodeActionsProvider,
};
use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser_core::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_diag(start: usize, end: usize, code: &str, msg: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Error,
        code: Some(code.to_string()),
        message: msg.to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

fn parse_and_get_actions(source: &str, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, (0, source.len()), diagnostics)
}

fn enhanced_actions(source: &str, range: (usize, usize)) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = EnhancedCodeActionsProvider::new(source.to_string());
    provider.get_enhanced_refactoring_actions(&ast, range)
}

fn has_action_matching(actions: &[CodeAction], pred: impl Fn(&CodeAction) -> bool) -> bool {
    actions.iter().any(pred)
}

// ===========================================================================
// CodeActionsProvider – quick-fix tests
// ===========================================================================

// ---- undefined-variable ---------------------------------------------------

#[test]
fn undefined_variable_produces_my_and_our_declarations() {
    let src = "use strict;\nprint $undefined;";
    let diags = [make_diag(
        18,
        28,
        "PL103",
        "Undefined variable '$undefined'",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("my")
            && a.kind == CodeActionKind::QuickFix),
        "Expected 'my' declaration action, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
    assert!(
        has_action_matching(&actions, |a| a.title.contains("our")),
        "Expected 'our' declaration action"
    );
}

#[test]
fn undefined_variable_preferred_is_my() {
    let src = "print $x;";
    let diags = [make_diag(6, 8, "PL103", "Undefined variable '$x'")];
    let actions = parse_and_get_actions(src, &diags);

    let my_action = actions.iter().find(|a| a.title.contains("my"));
    assert!(my_action.is_some(), "Expected my action");
    assert!(
        my_action.is_some_and(|a| a.is_preferred),
        "'my' should be preferred"
    );

    let our_action = actions.iter().find(|a| a.title.contains("our"));
    assert!(our_action.is_some());
    assert!(
        !our_action.is_none_or(|a| a.is_preferred),
        "'our' should NOT be preferred"
    );
}

#[test]
fn undefined_variable_no_quotes_in_message_yields_no_declaration() {
    let src = "print $x;";
    // Message without single-quoted variable name → split('\'').nth(1) is None
    let diags = [make_diag(6, 8, "PL103", "Undefined variable x")];
    let actions = parse_and_get_actions(src, &diags);

    let decl_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.title.contains("Declare"))
        .collect();
    assert!(
        decl_actions.is_empty(),
        "No declarations when message lacks quoted var"
    );
}

// ---- unused-variable ------------------------------------------------------

#[test]
fn unused_variable_remove_and_rename() {
    let src = "my $unused = 42;\nprint 1;";
    let diags = [make_diag(0, 16, "PL102", "Unused variable '$unused'")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title == "Remove unused variable"),
        "Expected remove action"
    );
    assert!(
        has_action_matching(&actions, |a| a.title.contains("_$unused")),
        "Expected rename-with-underscore action"
    );
}

#[test]
fn unused_variable_remove_is_preferred() {
    let src = "my $unused = 42;\nprint 1;";
    let diags = [make_diag(0, 16, "PL102", "Unused variable '$unused'")];
    let actions = parse_and_get_actions(src, &diags);

    let remove = actions.iter().find(|a| a.title == "Remove unused variable");
    assert!(
        remove.is_some_and(|a| a.is_preferred),
        "Remove should be preferred"
    );
}

// ---- assignment-in-condition ----------------------------------------------

#[test]
fn assignment_in_condition_comparison_and_paren_fixes() {
    let src = "if ($x = 5) { }";
    let diags = [make_diag(4, 10, "PL403", "Assignment in condition")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("==")),
        "Expected == comparison fix"
    );
    assert!(
        has_action_matching(&actions, |a| a.title.contains("parentheses")),
        "Expected parentheses wrapping fix"
    );
}

#[test]
fn assignment_in_condition_comparison_is_preferred() {
    let src = "if ($x = 5) { }";
    let diags = [make_diag(4, 10, "PL403", "Assignment in condition")];
    let actions = parse_and_get_actions(src, &diags);

    let comparison = actions.iter().find(|a| a.title.contains("=="));
    assert!(comparison.is_some_and(|a| a.is_preferred));
    let parens = actions.iter().find(|a| a.title.contains("parentheses"));
    assert!(!parens.is_none_or(|a| a.is_preferred));
}

// ---- missing-strict / missing-warnings ------------------------------------

#[test]
fn missing_strict_adds_use_strict() {
    let src = "print 1;";
    let diags = [make_diag(0, 8, "PL100", "Missing use strict")];
    let actions = parse_and_get_actions(src, &diags);

    let strict = actions.iter().find(|a| a.title.contains("use strict"));
    assert!(strict.is_some(), "Expected 'use strict' action");
    assert!(strict.is_some_and(|a| a.is_preferred));

    let edit = &strict.map(|a| &a.edit);
    assert!(
        edit.is_some_and(|e| e.changes.iter().any(|c| c.new_text.contains("use strict;"))),
        "Edit should insert 'use strict;'"
    );
}

#[test]
fn missing_warnings_adds_use_warnings() {
    let src = "print 1;";
    let diags = [make_diag(0, 8, "PL101", "Missing use warnings")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("use warnings")));
}

// ---- deprecated-defined ---------------------------------------------------

#[test]
fn deprecated_defined_replaces_with_variable() {
    let src = "if (defined @array) { }";
    let diags = [make_diag(4, 18, "PL500", "deprecated defined(@array)")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("@array")),
        "Expected action referencing @array, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ---- numeric-undef --------------------------------------------------------

#[test]
fn numeric_undef_adds_defined_check() {
    let src = "$x == 0";
    let diags = [make_diag(0, 7, "PL404", "Numeric comparison may be undef")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("defined")),
        "Expected 'Add defined check' action"
    );
}

#[test]
fn numeric_undef_with_eq_offers_defined_or() {
    let src = "$x == 0";
    let diags = [make_diag(0, 7, "PL404", "Numeric comparison may be undef")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("//")),
        "Expected defined-or operator action when '==' present"
    );
}

#[test]
fn numeric_undef_without_eq_no_defined_or() {
    let src = "$x + 0";
    let diags = [make_diag(0, 6, "PL404", "Numeric comparison may be undef")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        !has_action_matching(&actions, |a| a.title.contains("//")),
        "Should NOT offer defined-or when no '==' present"
    );
}

// ---- unquoted-bareword ----------------------------------------------------

#[test]
fn bareword_produces_single_and_double_quote_fixes() {
    let src = "my $x = foo;";
    let diags = [make_diag(8, 11, "PL109", "Bareword 'foo'")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("single quotes")),
        "Expected single-quote action"
    );
    assert!(
        has_action_matching(&actions, |a| a.title.contains("double quotes")),
        "Expected double-quote action"
    );
}

#[test]
fn bareword_uppercase_produces_filehandle_option() {
    let src = "print STDOUT;";
    let diags = [make_diag(6, 12, "PL109", "Bareword 'STDOUT'")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("filehandle")),
        "Expected filehandle declaration for uppercase bareword"
    );
}

#[test]
fn bareword_lowercase_no_filehandle_option() {
    let src = "my $x = foo;";
    let diags = [make_diag(8, 11, "PL109", "Bareword 'foo'")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        !has_action_matching(&actions, |a| a.title.contains("filehandle")),
        "Should NOT offer filehandle for lowercase bareword"
    );
}

#[test]
fn bareword_single_quote_is_preferred() {
    let src = "my $x = foo;";
    let diags = [make_diag(8, 11, "PL109", "Bareword 'foo'")];
    let actions = parse_and_get_actions(src, &diags);

    let sq = actions.iter().find(|a| a.title.contains("single quotes"));
    assert!(sq.is_some_and(|a| a.is_preferred));
    let dq = actions.iter().find(|a| a.title.contains("double quotes"));
    assert!(!dq.is_none_or(|a| a.is_preferred));
}

// ---- parse-error-* --------------------------------------------------------

#[test]
fn parse_error_missing_semicolon() {
    let src = "my $x = 1\n";
    let diags = [make_diag(
        0,
        9,
        "parse-error-missingsemicolon",
        "Missing semicolon",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("semicolon")),
        "Expected semicolon action"
    );
}

#[test]
fn parse_error_unclosed_string() {
    let src = r#"my $x = "hello"#;
    let diags = [make_diag(
        8,
        14,
        "parse-error-unclosedstring",
        "Unclosed string",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("closing quote")));
}

#[test]
fn parse_error_unclosed_paren() {
    let src = "my $x = (1 + 2";
    let diags = [make_diag(
        8,
        14,
        "parse-error-unclosedparenthesis",
        "Unclosed paren",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("parenthesis")));
}

#[test]
fn parse_error_unclosed_bracket() {
    let src = "my @a = [1, 2";
    let diags = [make_diag(
        8,
        13,
        "parse-error-unclosedbracket",
        "Unclosed bracket",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("bracket")));
}

#[test]
fn parse_error_unclosed_brace() {
    let src = "if ($x) {";
    let diags = [make_diag(
        8,
        9,
        "parse-error-unclosedbrace",
        "Unclosed brace",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a.title.contains("brace")));
}

#[test]
fn parse_error_unclosed_block() {
    let src = "sub foo {";
    let diags = [make_diag(
        8,
        9,
        "parse-error-unclosedblock",
        "Unclosed block",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a.title.contains("brace")));
}

#[test]
fn parse_error_unknown_code_yields_no_actions_for_that_code() {
    let src = "print 1;";
    let diags = [make_diag(
        0,
        8,
        "parse-error-unknown",
        "Unknown parse error",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        !has_action_matching(&actions, |a| {
            a.diagnostics.contains(&"parse-error-unknown".to_string())
        }),
        "Unknown parse-error code should not produce a targeted action"
    );
}

// ---- PL001 / PL002 missing-semicolon via message text --------------------

#[test]
fn pl001_missing_semicolon_message_triggers_fix() {
    let src = "my $x = 1\n";
    let diags = [make_diag(
        0,
        9,
        "PL001",
        "Missing semicolon after statement. Add `;` here (found `my`)",
    )];
    let actions = parse_and_get_actions(src, &diags);
    assert!(
        has_action_matching(&actions, |a| a.title.contains("semicolon")),
        "PL001 with missing-semicolon message must offer fix, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn pl002_missing_semicolon_message_triggers_fix() {
    let src = "my $x = 1\n";
    let diags = [make_diag(
        0,
        9,
        "PL002",
        "Missing semicolon after statement. Add `;` here (found `my`)",
    )];
    let actions = parse_and_get_actions(src, &diags);
    assert!(
        has_action_matching(&actions, |a| a.title.contains("semicolon")),
        "PL002 with missing-semicolon message must offer fix"
    );
}

#[test]
fn pl001_generic_message_does_not_trigger_semicolon_fix() {
    let src = "my $x = 1;\n";
    let diags = [make_diag(0, 9, "PL001", "Unexpected token found `my`")];
    let actions = parse_and_get_actions(src, &diags);
    assert!(
        !has_action_matching(&actions, |a| a.title.contains("semicolon")),
        "PL001 with unrelated message must not offer semicolon fix"
    );
}

#[test]
fn pl001_semicolon_inserted_before_trailing_whitespace() {
    // "my $x = 1   \n" — trailing spaces before newline
    let src = "my $x = 1   \n";
    let diags = [make_diag(
        0,
        9,
        "PL001",
        "Missing semicolon after statement. Add `;` here (found `my`)",
    )];
    let actions = parse_and_get_actions(src, &diags);
    let fix = actions
        .iter()
        .find(|a| a.title.contains("semicolon"))
        .expect("fix must exist");
    // The insertion point should be right after "1" (byte 9), not after trailing spaces
    assert_eq!(
        fix.edit.changes[0].location.start, 9,
        "semicolon must be inserted after last non-whitespace char (byte 9)"
    );
}

#[test]
fn pl001_heredoc_does_not_trigger_semicolon_fix() {
    // Source at range start looks like a heredoc — fix must be skipped
    let src = "<<END\nhello\nEND\n";
    let diags = [make_diag(
        0,
        5,
        "PL001",
        "Missing semicolon after statement. Add `;` here",
    )];
    let actions = parse_and_get_actions(src, &diags);
    assert!(
        !has_action_matching(&actions, |a| a.title.contains("semicolon")),
        "Heredoc context must not produce semicolon fix"
    );
}

#[test]
fn pl001_eof_without_newline_inserts_at_end() {
    // No trailing newline — `unwrap_or(source.len())` path
    let src = "my $x = 1";
    let diags = [make_diag(
        0,
        9,
        "PL001",
        "Missing semicolon after statement. Add `;` here",
    )];
    let actions = parse_and_get_actions(src, &diags);
    let fix = actions
        .iter()
        .find(|a| a.title.contains("semicolon"))
        .expect("fix must exist");
    assert_eq!(
        fix.edit.changes[0].location.start, 9,
        "EOF without newline must insert at source.len()"
    );
}

// ---- unused-parameter -----------------------------------------------------

#[test]
fn unused_parameter_rename_with_underscore() {
    let src = "sub foo { my ($x) = @_; }";
    let diags = [make_diag(14, 16, "PL108", "Unused parameter '$x'")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("_$x")),
        "Expected underscore-prefix rename"
    );
}

#[test]
fn unused_parameter_no_quotes_yields_no_rename() {
    let src = "sub foo { my ($x) = @_; }";
    let diags = [make_diag(14, 16, "PL108", "Unused parameter x")];
    let actions = parse_and_get_actions(src, &diags);

    let rename_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.title.starts_with("Rename to"))
        .collect();
    assert!(
        rename_actions.is_empty(),
        "No rename when message lacks quoted name"
    );
}

// ---- variable-shadowing ---------------------------------------------------

#[test]
fn variable_shadowing_suggests_three_alternatives() {
    let src = "my $x = 1; { my $x = 2; }";
    let diags = [make_diag(17, 19, "PL104", "Variable '$x' shadows outer")];
    let actions = parse_and_get_actions(src, &diags);

    let shadow_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostics.contains(&"PL104".to_string()))
        .collect();
    assert!(
        shadow_actions.len() >= 3,
        "Expected at least 3 suggestions, got {}",
        shadow_actions.len()
    );
    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("$x_inner")));
    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("$x_local")));
    assert!(has_action_matching(&actions, |a| a.title.contains("$my_x")));
}

#[test]
fn variable_shadowing_with_array_sigil() {
    let src = "my @arr = (1); { my @arr = (2); }";
    let diags = [make_diag(21, 25, "PL104", "Variable '@arr' shadows outer")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("@arr_inner")),
        "Expected @arr_inner suggestion"
    );
}

#[test]
fn variable_shadowing_with_hash_sigil() {
    let src = "my %h = (); { my %h = (); }";
    let diags = [make_diag(18, 20, "PL104", "Variable '%h' shadows outer")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("%h_inner")),
        "Expected %h_inner suggestion"
    );
}

#[test]
fn variable_shadowing_none_preferred() {
    let src = "my $x = 1; { my $x = 2; }";
    let diags = [make_diag(17, 19, "PL104", "Variable '$x' shadows outer")];
    let actions = parse_and_get_actions(src, &diags);

    let shadow_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostics.contains(&"PL104".to_string()))
        .collect();
    assert!(
        shadow_actions.iter().all(|a| !a.is_preferred),
        "Shadowing renames should not be preferred"
    );
}

// ===========================================================================
// CodeActionsProvider – edge cases
// ===========================================================================

#[test]
fn empty_diagnostics_still_returns_refactoring_actions() {
    let src = "my $x = length('hello') + 1;";
    let actions = parse_and_get_actions(src, &[]);

    // Should still contain refactoring/enhanced actions (pragmas, extract, etc.)
    assert!(!actions.is_empty());
}

#[test]
fn empty_source_does_not_panic() {
    let src = "";
    let diags = [make_diag(0, 0, "PL100", "Missing use strict")];
    let actions = parse_and_get_actions(src, &diags);

    // Should return at least the missing-strict action
    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("use strict")));
}

#[test]
fn unknown_diagnostic_code_produces_no_targeted_fix() {
    let src = "print 1;";
    let diags = [make_diag(0, 8, "totally-unknown-code", "Unknown issue")];
    let actions = parse_and_get_actions(src, &diags);

    let targeted: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostics.contains(&"totally-unknown-code".to_string()))
        .collect();
    assert!(targeted.is_empty());
}

#[test]
fn diagnostic_without_code_produces_no_quick_fix() {
    let src = "print 1;";
    let diags = [Diagnostic {
        range: (0, 8),
        severity: DiagnosticSeverity::Warning,
        code: None,
        message: "Something".to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }];
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(src.to_string());
    let actions = provider.get_code_actions(&ast, (0, src.len()), &diags);

    // No code → no quick fixes targeted at it (refactorings may still exist)
    let code_fixes: Vec<_> = actions
        .iter()
        .filter(|a| !a.diagnostics.is_empty())
        .collect();
    let has_none_code = code_fixes
        .iter()
        .any(|a| a.diagnostics.iter().any(|d| d.is_empty()));
    assert!(!has_none_code);
}

#[test]
fn multiple_diagnostics_produce_combined_actions() {
    let src = "print $x;";
    let diags = [
        make_diag(0, 9, "PL100", "Missing use strict"),
        make_diag(0, 9, "PL101", "Missing use warnings"),
        make_diag(6, 8, "PL103", "Undefined variable '$x'"),
    ];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("use strict")));
    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("use warnings")));
    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("Declare")));
}

// ===========================================================================
// CodeActionKind – equality checks
// ===========================================================================

#[test]
fn code_action_kind_equality() {
    assert_eq!(CodeActionKind::QuickFix, CodeActionKind::QuickFix);
    assert_eq!(CodeActionKind::Refactor, CodeActionKind::Refactor);
    assert_eq!(
        CodeActionKind::RefactorExtract,
        CodeActionKind::RefactorExtract
    );
    assert_eq!(
        CodeActionKind::RefactorInline,
        CodeActionKind::RefactorInline
    );
    assert_eq!(
        CodeActionKind::RefactorRewrite,
        CodeActionKind::RefactorRewrite
    );
    assert_eq!(CodeActionKind::Source, CodeActionKind::Source);
    assert_eq!(
        CodeActionKind::SourceOrganizeImports,
        CodeActionKind::SourceOrganizeImports
    );
    assert_eq!(CodeActionKind::SourceFixAll, CodeActionKind::SourceFixAll);

    assert_ne!(CodeActionKind::QuickFix, CodeActionKind::Refactor);
    assert_ne!(
        CodeActionKind::RefactorExtract,
        CodeActionKind::RefactorRewrite
    );
}

// ===========================================================================
// EnhancedCodeActionsProvider
// ===========================================================================

#[test]
fn enhanced_extract_variable_for_function_call() {
    let src = "my $x = length($string) + 10;";
    let actions = enhanced_actions(src, (8, 23));

    assert!(
        has_action_matching(&actions, |a| a.title.contains("Extract")),
        "Expected extract action for function call"
    );
}

#[test]
fn enhanced_extract_variable_for_binary_expression() {
    let src = "my $x = $a + $b;";
    let actions = enhanced_actions(src, (8, 15));

    assert!(
        has_action_matching(&actions, |a| a.title.contains("Extract")
            && a.kind == CodeActionKind::RefactorExtract),
        "Expected extract action for binary expr"
    );
}

#[test]
fn enhanced_error_checking_for_open() {
    let src = "open my $fh, '<', 'file.txt';";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        has_action_matching(&actions, |a| a.title.contains("error checking")),
        "Expected error checking action for open()"
    );
}

#[test]
fn enhanced_error_checking_not_offered_when_die_present() {
    let src = "open my $fh, '<', 'file.txt' or die 'Failed: $!';";
    let actions = enhanced_actions(src, (0, src.len()));

    // error checking should not be offered since "die" is nearby
    assert!(
        !has_action_matching(&actions, |a| a
            .title
            .contains("Add error checking to 'open'")),
        "Should not offer error checking when 'die' already present"
    );
}

#[test]
fn enhanced_postfix_conversion_for_simple_if() {
    let src = "if ($debug) { print \"Debug\\n\"; }";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        has_action_matching(&actions, |a| a.title.contains("postfix")),
        "Expected postfix conversion action"
    );
}

#[test]
fn enhanced_extract_subroutine_for_block() {
    let src = "sub main { { my $x = 1; my $y = 2; } }";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        has_action_matching(&actions, |a| a.title.contains("subroutine")),
        "Expected extract-subroutine action for block"
    );
}

#[test]
fn enhanced_add_pragmas_when_missing() {
    // Source without 'use strict' or 'use warnings'
    let src = "print 'hello';";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        has_action_matching(&actions, |a| a.title.contains("pragma")),
        "Expected pragma action when strict/warnings missing, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn enhanced_no_pragma_when_both_present() {
    let src = "use strict;\nuse warnings;\nprint 'hello';";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        !has_action_matching(&actions, |a| a.title.contains("pragma")),
        "Should NOT suggest pragmas when both already present"
    );
}

#[test]
fn enhanced_utf8_action_for_non_ascii() {
    let src = "my $msg = 'héllo';";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        has_action_matching(&actions, |a| a.title.contains("UTF-8")),
        "Expected UTF-8 support action for non-ASCII content"
    );
}

#[test]
fn enhanced_no_utf8_when_already_present() {
    let src = "use utf8;\nmy $msg = 'héllo';";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        !has_action_matching(&actions, |a| a.title.contains("UTF-8")),
        "Should NOT suggest UTF-8 when 'use utf8' present"
    );
}

#[test]
fn enhanced_no_utf8_for_ascii_only() {
    let src = "my $msg = 'hello';";
    let actions = enhanced_actions(src, (0, src.len()));

    assert!(
        !has_action_matching(&actions, |a| a.title.contains("UTF-8")),
        "Should NOT suggest UTF-8 for ASCII-only content"
    );
}

// ===========================================================================
// CodeActionEdit structural checks
// ===========================================================================

#[test]
fn code_action_edit_has_nonempty_changes() {
    let src = "print $x;";
    let diags = [make_diag(6, 8, "PL103", "Undefined variable '$x'")];
    let actions = parse_and_get_actions(src, &diags);

    for action in &actions {
        assert!(
            !action.edit.changes.is_empty(),
            "Action '{}' should have at least one edit change",
            action.title
        );
    }
}

#[test]
fn text_edit_locations_are_within_source_bounds() {
    let src = "my $unused = 42;\nprint 1;";
    let diags = [make_diag(0, 16, "PL102", "Unused variable '$unused'")];
    let actions = parse_and_get_actions(src, &diags);

    for action in &actions {
        for change in &action.edit.changes {
            assert!(
                change.location.start <= src.len() + 1,
                "Edit start {} exceeds source length {} in action '{}'",
                change.location.start,
                src.len(),
                action.title
            );
        }
    }
}

// ===========================================================================
// Integration: full pipeline with provider construction
// ===========================================================================

#[test]
fn provider_can_be_constructed_with_large_source() {
    let src = "use strict;\nuse warnings;\n".repeat(1000);
    let _provider = CodeActionsProvider::new(src);
    // Should not panic or OOM for reasonably large input
}

#[test]
fn enhanced_provider_can_be_constructed_with_large_source() {
    let src = "use strict;\nuse warnings;\n".repeat(1000);
    let _provider = EnhancedCodeActionsProvider::new(src);
}

#[test]
fn actions_for_multiline_source() {
    let src = "use strict;\nuse warnings;\nmy $x = 1;\nmy $y = $x + 2;\nprint $y;\n";
    let diags = [make_diag(26, 36, "PL102", "Unused variable '$x'")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title == "Remove unused variable"),
        "Expected remove action in multiline source"
    );
}

// ===========================================================================
// Quick-fix edit content validation
// ===========================================================================

#[test]
fn use_strict_edit_inserts_at_position_zero() {
    let src = "print 1;";
    let diags = [make_diag(0, 8, "PL100", "Missing use strict")];
    let actions = parse_and_get_actions(src, &diags);

    let strict = actions.iter().find(|a| a.title.contains("use strict"));
    assert!(strict.is_some());
    let edit = &strict.map(|a| &a.edit.changes[0]);
    assert!(edit.is_some());
    let te = edit.as_ref();
    assert_eq!(te.map(|t| t.location.start), Some(0));
    assert_eq!(te.map(|t| t.location.end), Some(0));
    assert_eq!(te.map(|t| t.new_text.as_str()), Some("use strict;\n"));
}

#[test]
fn use_warnings_edit_inserts_at_position_zero() {
    let src = "print 1;";
    let diags = [make_diag(0, 8, "PL101", "Missing use warnings")];
    let actions = parse_and_get_actions(src, &diags);

    let warnings = actions.iter().find(|a| a.title.contains("use warnings"));
    assert!(warnings.is_some());
    let te = &warnings.map(|a| &a.edit.changes[0]);
    assert_eq!(te.map(|t| t.new_text.as_str()), Some("use warnings;\n"));
}

#[test]
fn assignment_in_condition_eq_edit_replaces_single_char() {
    let src = "if ($x = 5) { }";
    let diags = [make_diag(4, 10, "PL403", "Assignment in condition")];
    let actions = parse_and_get_actions(src, &diags);

    let eq_action = actions.iter().find(|a| a.title.contains("=="));
    assert!(eq_action.is_some());
    let changes = &eq_action.map(|a| &a.edit.changes);
    assert!(changes.is_some());
    let edits = changes.as_ref().map(|c| c.as_slice());
    // The change should replace 1 char '=' with '=='
    if let Some(edits) = edits {
        let first = &edits[0];
        assert_eq!(
            first.location.end - first.location.start,
            1,
            "Should replace 1 char"
        );
        assert_eq!(first.new_text, "==");
    }
}

// ===========================================================================
// Semicolon insertion edge case
// ===========================================================================

#[test]
fn missing_semicolon_inserts_before_trailing_whitespace() {
    let src = "my $x = 1   \nprint 2;";
    let diags = [make_diag(
        0,
        9,
        "parse-error-missingsemicolon",
        "Missing semicolon",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let semi = actions.iter().find(|a| a.title.contains("semicolon"));
    assert!(semi.is_some());
    let te = &semi.map(|a| &a.edit.changes[0]);
    // The insert position should be at byte offset 9 (after "my $x = 1", before spaces)
    if let Some(te) = te {
        assert_eq!(te.new_text, ";");
        assert!(
            te.location.start <= 9,
            "Semicolon should be inserted before trailing whitespace"
        );
    }
}

#[test]
fn missing_semicolon_at_end_of_file_no_newline() {
    let src = "my $x = 1";
    let diags = [make_diag(
        0,
        9,
        "parse-error-missingsemicolon",
        "Missing semicolon",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(has_action_matching(&actions, |a| a
        .title
        .contains("semicolon")));
}

// ===========================================================================
// Quick-fix: bareword-filehandle (PL400)
// ===========================================================================

#[test]
fn bareword_filehandle_offers_lexical_replacement() {
    // "open FILE, ..." uses a bareword filehandle (FILE)
    let src = "open FILE, '<', 'data.txt';";
    // The bareword filehandle "FILE" spans bytes 5..9
    let diags = [make_diag(5, 9, "PL400", "Bareword filehandle 'FILE'")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("lexical")
            || a.title.contains("my $")
            || a.title.contains("$fh")),
        "Expected action to replace bareword filehandle with lexical, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn bareword_filehandle_action_is_quickfix_kind() {
    let src = "open FILE, '<', 'data.txt';";
    let diags = [make_diag(5, 9, "PL400", "Bareword filehandle 'FILE'")];
    let actions = parse_and_get_actions(src, &diags);

    let fh_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostics.contains(&"PL400".to_string()))
        .collect();
    assert!(
        !fh_actions.is_empty(),
        "Expected at least one bareword-filehandle action"
    );
    assert!(
        fh_actions
            .iter()
            .all(|a| a.kind == CodeActionKind::QuickFix),
        "bareword-filehandle actions should be QuickFix kind"
    );
}

#[test]
fn bareword_filehandle_action_is_preferred() {
    let src = "open LOGFILE, '<', 'log.txt';";
    let diags = [make_diag(5, 12, "PL400", "Bareword filehandle 'LOGFILE'")];
    let actions = parse_and_get_actions(src, &diags);

    let preferred: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostics.contains(&"PL400".to_string()) && a.is_preferred)
        .collect();
    assert!(
        !preferred.is_empty(),
        "At least one bareword-filehandle action should be preferred"
    );
}

// ===========================================================================
// Quick-fix: two-arg-open (PL401)
// ===========================================================================

#[test]
fn two_arg_open_offers_three_arg_upgrade() {
    // "open $fh, $filename" is the two-arg form; should suggest three-arg
    let src = "open my $fh, $filename;";
    let diags = [make_diag(0, 22, "PL401", "Two-argument open() is unsafe")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_matching(&actions, |a| a.title.contains("three")
            || a.title.contains("3-arg")
            || a.title.contains("three-argument")),
        "Expected action to convert to three-arg open, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn two_arg_open_action_is_quickfix_kind() {
    let src = "open my $fh, $filename;";
    let diags = [make_diag(0, 22, "PL401", "Two-argument open() is unsafe")];
    let actions = parse_and_get_actions(src, &diags);

    let open_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostics.contains(&"PL401".to_string()))
        .collect();
    assert!(
        !open_actions.is_empty(),
        "Expected at least one two-arg-open action"
    );
    assert!(
        open_actions
            .iter()
            .all(|a| a.kind == CodeActionKind::QuickFix),
        "two-arg-open actions should be QuickFix kind"
    );
}

#[test]
fn two_arg_open_action_is_preferred() {
    let src = "open my $fh, $filename;";
    let diags = [make_diag(0, 22, "PL401", "Two-argument open() is unsafe")];
    let actions = parse_and_get_actions(src, &diags);

    let preferred: Vec<_> = actions
        .iter()
        .filter(|a| a.diagnostics.contains(&"PL401".to_string()) && a.is_preferred)
        .collect();
    assert!(
        !preferred.is_empty(),
        "At least one two-arg-open action should be preferred"
    );
}
