//! Quick fixes for diagnostic issues
//!
//! Provides automated fixes for common Perl issues driven by diagnostic codes.

use super::types::{CodeAction, CodeActionEdit, CodeActionKind, QuickFixDiagnostic};
use crate::providers::rename::TextEdit;
use perl_diagnostics::codes::DiagnosticCode;
use perl_parser::ast_utils::{find_declaration_position, get_indent_at};
use perl_parser_core::SourceLocation;

/// Fix undefined variable by declaring it
pub fn fix_undefined_variable(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract variable name from diagnostic message
    if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
        // Find the best place to insert declaration
        let insert_pos = find_declaration_position(source, diagnostic.range.0);

        // Add 'my' declaration
        actions.push(CodeAction {
            title: format!("Declare '{}' with 'my'", var_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::UndefinedVariable.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("my {};\n", var_name),
                }],
            },
            is_preferred: true,
        });

        // Add 'our' declaration
        actions.push(CodeAction {
            title: format!("Declare '{}' with 'our'", var_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::UndefinedVariable.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("our {};\n", var_name),
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Fix unused variable by removing it
pub fn fix_unused_variable(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Find the declaration line
    let line_start = source[..diagnostic.range.0].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_end = source[diagnostic.range.1..]
        .find('\n')
        .map(|p| diagnostic.range.1 + p)
        .unwrap_or(source.len());
    let delete_end = if line_end < source.len() { line_end + 1 } else { line_end };

    actions.push(CodeAction {
        title: "Remove unused variable".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::UnusedVariable.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: line_start, end: delete_end },
                new_text: String::new(),
            }],
        },
        is_preferred: true,
    });

    // Add underscore prefix to mark as intentionally unused
    if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
        let unused_name = mark_intentionally_unused(var_name);
        actions.push(CodeAction {
            title: format!("Rename to '{}'", unused_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::UnusedVariable.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: unused_name,
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

fn mark_intentionally_unused(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(sigil @ ('$' | '@' | '%' | '&' | '*')) => {
            let rest = chars.as_str();
            format!("{sigil}_{rest}")
        }
        _ => format!("_{name}"),
    }
}

fn split_sigil(name: &str) -> (&str, &str) {
    let bare = name.trim_start_matches(['$', '@', '%', '&', '*']);
    let sigil_len = name.len() - bare.len();
    (&name[..sigil_len], bare)
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    fn diagnostic_for(range: (usize, usize), message: &str) -> QuickFixDiagnostic {
        QuickFixDiagnostic { range, message: message.to_string(), code: None }
    }

    #[test]
    fn fix_duplicate_hash_keys_rename_bareword_key() {
        let source = "my %h = (\n    foo => 1,\n    foo => 2,\n);\n";
        let key_start = must_some(source.rfind("foo"));
        let key_end = key_start + "foo".len();
        let diagnostic = diagnostic_for(
            (key_start, key_end),
            "Duplicate hash key 'foo' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);

        let rename = must_some(actions.iter().find(|a| a.title.contains("Rename")));
        assert_eq!(rename.edit.changes[0].new_text, "foo_2");
        assert_eq!(rename.edit.changes[0].location.start, key_start);
        assert_eq!(rename.edit.changes[0].location.end, key_end);
    }

    #[test]
    fn fix_duplicate_hash_keys_delete_removes_correct_line() {
        let source = "my %h = (\n    foo => 1,\n    foo => 2,\n);\n";
        let key_start = must_some(source.rfind("foo"));
        let key_end = key_start + "foo".len();
        let diagnostic = diagnostic_for(
            (key_start, key_end),
            "Duplicate hash key 'foo' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);

        let delete = must_some(actions.iter().find(|a| a.title.contains("Remove")));
        assert!(delete.is_preferred);
        assert_eq!(delete.edit.changes[0].new_text, "");

        // Applying the delete should remove only the duplicate line.
        let edit = &delete.edit.changes[0];
        let remaining =
            format!("{}{}", &source[..edit.location.start], &source[edit.location.end..]);
        assert_eq!(remaining, "my %h = (\n    foo => 1,\n);\n");
    }

    #[test]
    fn fix_duplicate_hash_keys_no_delete_for_inline_hash() {
        // All pairs on one line: delete action is suppressed to avoid corrupting inline hash.
        let source = "my %h = (foo => 1, foo => 2);\n";
        let key_start = must_some(source.rfind("foo"));
        let key_end = key_start + "foo".len();
        let diagnostic = diagnostic_for(
            (key_start, key_end),
            "Duplicate hash key 'foo' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);

        assert!(
            !actions.iter().any(|a| a.title.contains("Remove")),
            "should not offer delete for inline hash"
        );
        assert!(actions.iter().any(|a| a.title.contains("Rename")));
    }

    #[test]
    fn fix_duplicate_hash_keys_preserves_single_quote_style() {
        let source = "my %h = (\n    'foo' => 1,\n    'foo' => 2,\n);\n";
        let key_start = must_some(source.rfind("'foo'"));
        let key_end = key_start + "'foo'".len();
        let diagnostic = diagnostic_for(
            (key_start, key_end),
            "Duplicate hash key 'foo' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);

        let rename = must_some(actions.iter().find(|a| a.title.contains("Rename")));
        assert_eq!(rename.edit.changes[0].new_text, "'foo_2'");
    }

    #[test]
    fn fix_duplicate_hash_keys_preserves_double_quote_style() {
        let source = "my %h = (\n    \"foo\" => 1,\n    \"foo\" => 2,\n);\n";
        let key_start = must_some(source.rfind("\"foo\""));
        let key_end = key_start + "\"foo\"".len();
        let diagnostic = diagnostic_for(
            (key_start, key_end),
            "Duplicate hash key 'foo' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);

        let rename = must_some(actions.iter().find(|a| a.title.contains("Rename")));
        assert_eq!(rename.edit.changes[0].new_text, "\"foo_2\"");
    }

    #[test]
    fn fix_duplicate_hash_keys_empty_on_unparseable_message() {
        let source = "my %h = (foo => 1, foo => 2);\n";
        let diagnostic = diagnostic_for((19, 22), "No key name here");

        let actions = fix_duplicate_hash_keys(source, &diagnostic);
        assert!(actions.is_empty());
    }

    #[test]
    fn fix_duplicate_hash_keys_empty_on_invalid_range() {
        let source = "my %h = (foo => 1, foo => 2);\n";
        let diagnostic = diagnostic_for(
            (source.len() + 1, source.len() + 4),
            "Duplicate hash key 'foo' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);
        assert!(actions.is_empty());
    }

    #[test]
    fn fix_duplicate_hash_keys_empty_on_non_char_boundary_range() {
        let source = "my %h = (\"\u{e9}\" => 1, \"\u{e9}\" => 2);\n";
        let char_start = must_some(source.find('\u{e9}'));
        let diagnostic = diagnostic_for(
            (char_start + 1, char_start + 2),
            "Duplicate hash key 'e' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);
        assert!(actions.is_empty());
    }

    #[test]
    fn fix_duplicate_hash_keys_no_delete_for_multiline_value() {
        let source = "my %h = (\n    foo => 1,\n    foo => {\n        nested => 1,\n    },\n);\n";
        let key_start = must_some(source.rfind("foo => {"));
        let key_end = key_start + "foo".len();
        let diagnostic = diagnostic_for(
            (key_start, key_end),
            "Duplicate hash key 'foo' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);

        assert!(
            !actions.iter().any(|a| a.title.contains("Remove")),
            "should not offer line delete for multiline duplicate values"
        );
        assert!(actions.iter().any(|a| a.title.contains("Rename")));
    }

    #[test]
    fn fix_duplicate_hash_keys_quotes_numeric_rename_candidate() {
        let source = "my %h = (\n    42 => 1,\n    42 => 2,\n);\n";
        let key_start = must_some(source.rfind("42"));
        let key_end = key_start + "42".len();
        let diagnostic = diagnostic_for(
            (key_start, key_end),
            "Duplicate hash key '42' -- only the last value will be used",
        );

        let actions = fix_duplicate_hash_keys(source, &diagnostic);

        let rename = must_some(actions.iter().find(|a| a.title.contains("Rename")));
        assert_eq!(rename.edit.changes[0].new_text, "'42_2'");
    }

    #[test]
    fn fix_unused_variable_removal_does_not_overrun_end_of_file() {
        // Source has no trailing newline — delete_end must clamp to source.len().
        let source = "my $unused = 1;";
        let start = 3;
        let end = 10;
        let diagnostic = diagnostic_for((start, end), "Unused variable '$unused'");

        let actions = fix_unused_variable(source, &diagnostic);
        assert!(!actions.is_empty());

        let remove_action = &actions[0];
        let edit = &remove_action.edit.changes[0];

        assert_eq!(edit.location.start, 0);
        assert_eq!(edit.location.end, source.len());
        assert!(edit.location.end <= source.len(), "edit end must not exceed source length");
        assert_eq!(edit.new_text, "");
    }

    #[test]
    fn fix_unused_variable_removal_includes_newline_when_present() {
        // Source has trailing newline — delete_end should include the newline character
        // (line_end + 1) and must still be within source.len().
        let source = "my $unused = 1;\n";
        let start = 3;
        let end = 10;
        let diagnostic = diagnostic_for((start, end), "Unused variable '$unused'");

        let actions = fix_unused_variable(source, &diagnostic);
        assert!(!actions.is_empty());

        let remove_action = &actions[0];
        let edit = &remove_action.edit.changes[0];

        assert_eq!(edit.location.start, 0);
        // The newline at position 15 is found; delete_end = 15 + 1 = 16 = source.len()
        assert_eq!(edit.location.end, source.len());
        assert!(edit.location.end <= source.len(), "edit end must not exceed source length");
        assert_eq!(edit.new_text, "");
    }

    #[test]
    fn fix_unused_variable_removal_multiline_removes_correct_line() {
        // Unused variable on the second line of a multiline source.
        let source = "use strict;\nmy $unused = 1;\nmy $used = 2;\n";
        let start = 16; // start of '$unused'
        let end = 23; // end of '$unused'
        let diagnostic = diagnostic_for((start, end), "Unused variable '$unused'");

        let actions = fix_unused_variable(source, &diagnostic);
        assert!(!actions.is_empty());

        let remove_action = &actions[0];
        let edit = &remove_action.edit.changes[0];

        // Line starts after 'use strict;\n' at offset 12
        assert_eq!(edit.location.start, 12);
        // Line ends at the '\n' after 'my $unused = 1;' — include it: offset 28
        assert_eq!(edit.location.end, 28);
        assert!(edit.location.end <= source.len(), "edit end must not exceed source length");
        assert_eq!(edit.new_text, "");
    }

    // --- fix_printf_format_arity ---

    #[test]
    fn fix_printf_format_arity_listop_one_missing() {
        // Listop form: no parens, call node ends before the semicolon.
        let source = r#"printf "%s %s", $name;"#;
        // Call range covers "printf "%s %s", $name" (everything before ;)
        let call_end = source.len() - 1;
        let diagnostic = diagnostic_for(
            (0, call_end),
            r#"`printf` format string has 2 specifiers but 1 argument supplied"#,
        );

        let actions = fix_printf_format_arity(source, &diagnostic);

        assert!(!actions.is_empty(), "should offer a fix for 1 missing arg");
        assert_eq!(actions[0].title, "Add 1 missing argument as undef");
        let edit = &actions[0].edit.changes[0];
        assert_eq!(edit.new_text, ", undef");
        // Insertion is at the end of the call node (before ';')
        assert_eq!(edit.location.start, call_end);
        assert_eq!(edit.location.end, call_end);
    }

    #[test]
    fn fix_printf_format_arity_listop_range_includes_semicolon() {
        let source = r#"printf "%s %s", $name;"#;
        let diagnostic = diagnostic_for(
            (0, source.len()),
            r#"`printf` format string has 2 specifiers but 1 argument supplied"#,
        );

        let actions = fix_printf_format_arity(source, &diagnostic);

        let edit = &must_some(actions.first()).edit.changes[0];
        assert_eq!(edit.new_text, ", undef");
        assert_eq!(&source[edit.location.start..edit.location.start + 1], ";");
    }

    #[test]
    fn fix_printf_format_arity_parens_one_missing() {
        // Parens form: insert before the closing ')'.
        let source = r#"printf("%s %s", $a)"#;
        let diagnostic = diagnostic_for(
            (0, source.len()),
            r#"`printf` format string has 2 specifiers but 1 argument supplied"#,
        );

        let actions = fix_printf_format_arity(source, &diagnostic);

        assert!(!actions.is_empty(), "should offer a fix for parens form");
        assert_eq!(actions[0].title, "Add 1 missing argument as undef");
        let edit = &actions[0].edit.changes[0];
        assert_eq!(edit.new_text, ", undef");
        // Insertion position should be at the closing ')'
        assert_eq!(&source[edit.location.start..edit.location.start + 1], ")");
    }

    #[test]
    fn fix_printf_format_arity_ignores_range_not_starting_at_call() {
        let source = r#"my $n = printf "%s %s", $name;"#;
        let diagnostic = diagnostic_for(
            (0, source.len()),
            r#"`printf` format string has 2 specifiers but 1 argument supplied"#,
        );

        let actions = fix_printf_format_arity(source, &diagnostic);

        assert!(actions.is_empty());
    }

    #[test]
    fn fix_printf_format_arity_two_missing_appends_two_undefs() {
        let source = r#"sprintf "%s %s %s", $a"#;
        let diagnostic = diagnostic_for(
            (0, source.len()),
            r#"`sprintf` format string has 3 specifiers but 1 argument supplied"#,
        );

        let actions = fix_printf_format_arity(source, &diagnostic);

        assert!(!actions.is_empty(), "should offer a fix for 2 missing args");
        assert_eq!(actions[0].title, "Add 2 missing arguments as undef");
        assert_eq!(actions[0].edit.changes[0].new_text, ", undef, undef");
    }

    #[test]
    fn fix_printf_format_arity_too_many_args_returns_no_fix() {
        // When args > specifiers we don't auto-remove — too destructive.
        let source = r#"printf "%s", $a, $b"#;
        let diagnostic = diagnostic_for(
            (0, source.len()),
            r#"`printf` format string has 1 specifier but 2 arguments supplied"#,
        );

        let actions = fix_printf_format_arity(source, &diagnostic);

        assert!(actions.is_empty(), "should not suggest fix when args exceed specifiers");
    }

    #[test]
    fn fix_printf_format_arity_equal_counts_returns_no_fix() {
        let source = r#"printf "%s", $a"#;
        let diagnostic = diagnostic_for(
            (0, source.len()),
            r#"`printf` format string has 1 specifier but 1 argument supplied"#,
        );
        // Equal counts would not normally be flagged, but the function must handle it gracefully.
        let actions = fix_printf_format_arity(source, &diagnostic);
        assert!(actions.is_empty());
    }
}

/// Fix assignment in condition
pub fn fix_assignment_in_condition(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Change = to ==
    let assignment_pos =
        source[diagnostic.range.0..diagnostic.range.1].find('=').map(|p| diagnostic.range.0 + p);

    if let Some(pos) = assignment_pos {
        actions.push(CodeAction {
            title: "Change to comparison (==)".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::AssignmentInCondition.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: pos, end: pos + 1 },
                    new_text: "==".to_string(),
                }],
            },
            is_preferred: true,
        });

        // Wrap in parentheses to make intention clear
        actions.push(CodeAction {
            title: "Keep assignment (add parentheses)".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::AssignmentInCondition.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![
                    TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.0,
                            end: diagnostic.range.0,
                        },
                        new_text: "(".to_string(),
                    },
                    TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: ")".to_string(),
                    },
                ],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Add 'use strict' pragma
pub fn add_use_strict() -> Vec<CodeAction> {
    vec![CodeAction {
        title: "Add 'use strict'".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::MissingStrict.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: 0, end: 0 },
                new_text: "use strict;\n".to_string(),
            }],
        },
        is_preferred: true,
    }]
}

/// Add 'use warnings' pragma
pub fn add_use_warnings() -> Vec<CodeAction> {
    vec![CodeAction {
        title: "Add 'use warnings'".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::MissingWarnings.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: 0, end: 0 },
                new_text: "use warnings;\n".to_string(),
            }],
        },
        is_preferred: true,
    }]
}

fn file_scope_pragma_insertion_offset(source: &str) -> usize {
    if source.starts_with("#!") {
        source.find('\n').map(|offset| offset + 1).unwrap_or(source.len())
    } else {
        0
    }
}

/// Move a phase-scoped `use strict` pragma to file scope.
pub fn move_use_strict_to_file_scope(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let insert_at = file_scope_pragma_insertion_offset(source);
    let delete_end = if source.as_bytes().get(diagnostic.range.1).copied() == Some(b';') {
        diagnostic.range.1 + 1
    } else {
        diagnostic.range.1
    };

    vec![CodeAction {
        title: "Move 'use strict' to file scope".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::PhaseScopedStrictPragma.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![
                TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: delete_end },
                    new_text: String::new(),
                },
                TextEdit {
                    location: SourceLocation { start: insert_at, end: insert_at },
                    new_text: "use strict;\n".to_string(),
                },
            ],
        },
        is_preferred: true,
    }]
}

/// Move a phase-scoped `use warnings` pragma to file scope.
pub fn move_use_warnings_to_file_scope(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let insert_at = file_scope_pragma_insertion_offset(source);
    let delete_end = if source.as_bytes().get(diagnostic.range.1).copied() == Some(b';') {
        diagnostic.range.1 + 1
    } else {
        diagnostic.range.1
    };

    vec![CodeAction {
        title: "Move 'use warnings' to file scope".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::PhaseScopedWarningsPragma.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![
                TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: delete_end },
                    new_text: String::new(),
                },
                TextEdit {
                    location: SourceLocation { start: insert_at, end: insert_at },
                    new_text: "use warnings;\n".to_string(),
                },
            ],
        },
        is_preferred: true,
    }]
}

/// Fix deprecated 'defined @array' or 'defined %hash'
pub fn fix_deprecated_defined(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract the array/hash from the diagnostic
    if let Some(start) = source[diagnostic.range.0..diagnostic.range.1].find("defined") {
        let defined_start = diagnostic.range.0 + start;
        let arg_start = defined_start + 7; // "defined".len()

        // Find the argument
        let raw_arg = source[arg_start..diagnostic.range.1].trim();
        let arg_text = normalize_deprecated_defined_arg(raw_arg);

        actions.push(CodeAction {
            title: format!("Replace with '{}'", arg_text),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::DeprecatedDefined.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: defined_start, end: diagnostic.range.1 },
                    new_text: arg_text.to_string(),
                }],
            },
            is_preferred: true,
        });
    }

    actions
}

fn normalize_deprecated_defined_arg(raw_arg: &str) -> &str {
    raw_arg
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .map(str::trim)
        .unwrap_or(raw_arg)
}

/// Fix numeric comparison with undef
pub fn fix_numeric_undef(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Add defined check
    actions.push(CodeAction {
        title: "Add defined check".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::NumericComparisonWithUndef.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![
                TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.0 },
                    new_text: "defined(".to_string(),
                },
                TextEdit {
                    location: SourceLocation { start: diagnostic.range.1, end: diagnostic.range.1 },
                    new_text: ")".to_string(),
                },
            ],
        },
        is_preferred: true,
    });

    // Use // operator
    if source[diagnostic.range.0..diagnostic.range.1].contains("==") {
        actions.push(CodeAction {
            title: "Use defined-or operator (//)".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::NumericComparisonWithUndef.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: "// 0".to_string(), // Default to 0
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Fix explicit numeric comparison with `undef` from native critic.
pub fn fix_native_undef_comparison(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let Some(replacement) =
        native_undef_comparison_replacement(&source[diagnostic.range.0..diagnostic.range.1])
    else {
        return Vec::new();
    };

    vec![CodeAction {
        title: "Use defined() check".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["native.common.undef_comparison".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: replacement,
            }],
        },
        is_preferred: true,
    }]
}

fn native_undef_comparison_replacement(text: &str) -> Option<String> {
    if let Some((left, right)) = text.split_once("==") {
        return native_defined_replacement(left, right, true);
    }
    if let Some((left, right)) = text.split_once("!=") {
        return native_defined_replacement(left, right, false);
    }

    None
}

fn native_defined_replacement(left: &str, right: &str, equal: bool) -> Option<String> {
    let left = left.trim();
    let right = right.trim();
    let compared = if left == "undef" {
        right
    } else if right == "undef" {
        left
    } else {
        return None;
    };
    if compared.is_empty() {
        return None;
    }

    let replacement =
        if equal { format!("!defined({compared})") } else { format!("defined({compared})") };
    Some(replacement)
}

/// Fix unquoted bareword by quoting or declaring as filehandle
///
/// Provides three options for fixing bareword issues under strict mode:
/// 1. Quote with single quotes - wraps bareword in single quotes
/// 2. Quote with double quotes - wraps bareword in double quotes
/// 3. Declare as filehandle - for uppercase barewords, adds filehandle declaration
pub fn fix_bareword(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract bareword text from the source at the diagnostic range
    let bareword = &source[diagnostic.range.0..diagnostic.range.1];

    // Check if bareword is all uppercase (filehandle convention)
    let is_uppercase = bareword.chars().all(|c| c.is_ascii_uppercase() || c == '_');

    // Action 1: Quote with single quotes
    actions.push(CodeAction {
        title: format!("Quote '{}' with single quotes", bareword),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::UnquotedBareword.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("'{}'", bareword),
            }],
        },
        is_preferred: true,
    });

    // Action 2: Quote with double quotes
    actions.push(CodeAction {
        title: format!("Quote '{}' with double quotes", bareword),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::UnquotedBareword.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("\"{}\"", bareword),
            }],
        },
        is_preferred: false,
    });

    // Action 3: Declare as filehandle (only for uppercase barewords)
    if is_uppercase {
        // Find the best position to insert a filehandle declaration
        let insert_pos = find_declaration_position(source, diagnostic.range.0);
        let indent = get_indent_at(source, insert_pos);

        actions.push(CodeAction {
            title: format!("Declare '{}' as filehandle", bareword),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::UnquotedBareword.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("{}open my ${};\n", indent, bareword),
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Fix parse errors with automated corrections
pub fn fix_parse_error(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
    code: &str,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    match code {
        "parse-error-missingsemicolon" => {
            // Add semicolon at the end
            let line_end = source[diagnostic.range.0..]
                .find('\n')
                .map(|p| diagnostic.range.0 + p)
                .unwrap_or(source.len());

            // Find the actual end of the statement (before any trailing whitespace)
            let mut end_pos = line_end;
            while end_pos > diagnostic.range.0
                && source.as_bytes()[end_pos - 1].is_ascii_whitespace()
            {
                end_pos -= 1;
            }

            actions.push(CodeAction {
                title: "Add missing semicolon".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: end_pos, end: end_pos },
                        new_text: ";".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "PL001" | "PL002"
            if diagnostic.message.to_ascii_lowercase().contains("missing semicolon") =>
        {
            // PL001/PL002 are general parse error codes. When the message indicates a missing
            // semicolon, apply the same fix — but skip heredoc contexts where insertion is wrong.
            let at_heredoc = source[diagnostic.range.0..].get(..2).is_some_and(|s| s == "<<");
            if !at_heredoc {
                let line_end = source[diagnostic.range.0..]
                    .find('\n')
                    .map(|p| diagnostic.range.0 + p)
                    .unwrap_or(source.len());

                // Insert before trailing whitespace
                let mut end_pos = line_end;
                while end_pos > diagnostic.range.0
                    && source.as_bytes()[end_pos - 1].is_ascii_whitespace()
                {
                    end_pos -= 1;
                }

                actions.push(CodeAction {
                    title: "Add missing semicolon".to_string(),
                    kind: CodeActionKind::QuickFix,
                    diagnostics: vec![code.to_string()],
                    edit: CodeActionEdit {
                        changes: vec![TextEdit {
                            location: SourceLocation { start: end_pos, end: end_pos },
                            new_text: ";".to_string(),
                        }],
                    },
                    is_preferred: true,
                });
            }
        }
        "parse-error-unclosedstring" => {
            // Add closing quote
            actions.push(CodeAction {
                title: "Add closing quote".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: "\"".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "parse-error-unclosedparenthesis" => {
            actions.push(CodeAction {
                title: "Add closing parenthesis".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: ")".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "parse-error-unclosedbracket" => {
            actions.push(CodeAction {
                title: "Add closing bracket".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: "]".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "parse-error-unclosedbrace" | "parse-error-unclosedblock" => {
            actions.push(CodeAction {
                title: "Add closing brace".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: "}".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        _ => {}
    }

    actions
}

/// Fix unused parameter by adding underscore prefix
pub fn fix_unused_parameter(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    if let Some(param_name) = diagnostic.message.split('\'').nth(1) {
        // Add underscore prefix
        actions.push(CodeAction {
            title: format!("Rename to '_{}'", param_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::UnusedParameter.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: format!("_{}", param_name),
                }],
            },
            is_preferred: true,
        });
    }

    actions
}

/// Fix duplicate parameter by removing or renaming the repeated binding.
pub fn fix_duplicate_parameter(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let Some(param_name) = diagnostic.message.split('\'').nth(1) else {
        return Vec::new();
    };
    let (sigil, base_name) = split_sigil(param_name);
    let new_name = format!("{sigil}{base_name}_2");

    vec![
        CodeAction {
            title: format!("Remove duplicate parameter '{}'", param_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::DuplicateParameter.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: String::new(),
                }],
            },
            is_preferred: true,
        },
        CodeAction {
            title: format!("Rename duplicate to '{}'", new_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::DuplicateParameter.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: new_name,
                }],
            },
            is_preferred: false,
        },
    ]
}

/// Fix parameter shadowing by suggesting unambiguous parameter names.
pub fn fix_parameter_shadowing(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let Some(param_name) = diagnostic.message.split('\'').nth(1) else {
        return Vec::new();
    };
    let (sigil, base_name) = split_sigil(param_name);

    [
        format!("{sigil}p_{base_name}"),
        format!("{sigil}{base_name}_param"),
        format!("{sigil}{base_name}_arg"),
    ]
    .into_iter()
    .map(|new_name| CodeAction {
        title: format!("Rename parameter to '{}'", new_name),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::ParameterShadowsGlobal.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: new_name,
            }],
        },
        is_preferred: false,
    })
    .collect()
}

/// Suggest portable shebang line
///
/// Detects hardcoded perl paths in shebang lines (e.g., `#!/usr/bin/perl`,
/// `#!/usr/local/bin/perl`) and suggests replacing with `#!/usr/bin/env perl`
/// for better portability across systems.
///
/// Only triggers on the first line of the file when it starts with `#!` and
/// contains a path to perl that is not already using `env`.
pub fn fix_hardcoded_shebang(source: &str) -> Vec<CodeAction> {
    let first_line = match source.lines().next() {
        Some(line) => line,
        None => return Vec::new(),
    };

    // Must be a shebang line
    if !first_line.starts_with("#!") {
        return Vec::new();
    }

    // Already portable
    if first_line.contains("/env ") || first_line.contains("/env\t") {
        return Vec::new();
    }

    // Must reference perl
    if !first_line.contains("perl") {
        return Vec::new();
    }

    // Extract any flags after the perl path (e.g., -w, -T)
    let flags = extract_shebang_flags(first_line);
    let new_shebang = if flags.is_empty() {
        "#!/usr/bin/env perl".to_string()
    } else {
        format!("#!/usr/bin/env perl {}", flags)
    };

    vec![CodeAction {
        title: "Use portable shebang (#!/usr/bin/env perl)".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["hardcoded-shebang".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: 0, end: first_line.len() },
                new_text: new_shebang,
            }],
        },
        is_preferred: true,
    }]
}

/// Extract flags from a shebang line (e.g., `-w` from `#!/usr/bin/perl -w`)
fn extract_shebang_flags(shebang_line: &str) -> String {
    // Find "perl" in the line, then grab everything after it
    if let Some(perl_pos) = shebang_line.find("perl") {
        let after_perl = &shebang_line[perl_pos + 4..];
        let trimmed = after_perl.trim();
        if trimmed.is_empty() { String::new() } else { trimmed.to_string() }
    } else {
        String::new()
    }
}

/// Fix variable shadowing by suggesting rename
pub fn fix_variable_shadowing(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
        // Remove sigil for the base name
        let base_name =
            var_name.trim_start_matches('$').trim_start_matches('@').trim_start_matches('%');

        // Suggest alternative names
        let suggestions = vec![
            format!("{}_inner", base_name),
            format!("{}_local", base_name),
            format!("my_{}", base_name),
        ];

        for suggestion in suggestions {
            let new_name = if var_name.starts_with('$') {
                format!("${}", suggestion)
            } else if var_name.starts_with('@') {
                format!("@{}", suggestion)
            } else if var_name.starts_with('%') {
                format!("%{}", suggestion)
            } else {
                suggestion.clone()
            };

            actions.push(CodeAction {
                title: format!("Rename to '{}'", new_name),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![DiagnosticCode::VariableShadowing.as_str().to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.0,
                            end: diagnostic.range.1,
                        },
                        new_text: new_name,
                    }],
                },
                is_preferred: false,
            });
        }
    }

    actions
}

/// Fix bareword filehandle by replacing with lexical filehandle
///
/// Bareword filehandles (e.g., `open FILE, ...`) are a common Perl anti-pattern.
/// This fix suggests replacing the bareword with a lexical variable (`my $fh`).
pub fn fix_bareword_filehandle(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    // Extract filehandle name from message, e.g. "Bareword filehandle 'FILE'"
    let fh_name = diagnostic.message.split('\'').nth(1).unwrap_or("FH");
    // Derive a lowercase lexical name: FILE -> $file_fh, LOGFILE -> $logfile_fh
    let lexical_name = format!("${}_fh", fh_name.to_lowercase());

    vec![CodeAction {
        title: format!("Replace bareword filehandle '{}' with lexical '{}'", fh_name, lexical_name),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::BarewordFilehandle.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("my {}", lexical_name),
            }],
        },
        is_preferred: true,
    }]
}

/// Fix missing package declaration by inserting `package main;` at the top
///
/// When a Perl file has no `package` declaration (PL200), the default package
/// is `main`. This fix makes that intent explicit by inserting `package main;`
/// at the top of the file.
pub fn fix_missing_package_declaration(source: &str) -> Vec<CodeAction> {
    // Insert after shebang if present, otherwise at top
    let insert_pos =
        if source.starts_with("#!") { source.find('\n').map(|p| p + 1).unwrap_or(0) } else { 0 };

    vec![CodeAction {
        title: "Add 'package main;' declaration".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::MissingPackageDeclaration.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: "package main;\n".to_string(),
            }],
        },
        is_preferred: true,
    }]
}

/// Fix variable redeclaration by removing the duplicate `my` keyword
///
/// When a variable is declared twice in the same scope (PL105), the fix
/// is to remove the `my` keyword from the second declaration, turning it
/// into a plain assignment.
pub fn fix_variable_redeclaration(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    if let Some((abs_my_start, abs_my_end)) = find_duplicate_my_span(source, diagnostic) {
        // Remove only the duplicate declarator and keep the assignment/value intact.

        actions.push(CodeAction {
            title: "Remove duplicate 'my' declaration".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::VariableRedeclaration.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: abs_my_start, end: abs_my_end },
                    new_text: String::new(),
                }],
            },
            is_preferred: true,
        });
    }

    actions
}

fn find_duplicate_my_span(source: &str, diagnostic: &QuickFixDiagnostic) -> Option<(usize, usize)> {
    let variable_start = diagnostic.range.0.min(source.len());
    let line_start = source[..variable_start].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let before_var = &source[line_start..variable_start];
    let my_offset = before_var.rfind("my ")?;

    if before_var[my_offset + 3..].chars().all(char::is_whitespace) {
        let start = line_start + my_offset;
        return Some((start, start + 3));
    }

    None
}

/// Fix misspelled pragma by replacing with the correctly spelled name
///
/// The MisspelledPragma diagnostic (PL111) message has the format:
/// `"Did you mean 'use <correct>;'? '<typo>' is not a known pragma"`
/// This fix extracts the correct name and replaces the entire `use <typo>` statement.
pub fn fix_misspelled_pragma(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Parse correct pragma from message: "Did you mean 'use <correct>;'?"
    let msg = &diagnostic.message;
    if let Some(after_use) = msg.strip_prefix("Did you mean 'use ")
        && let Some(correct_name) = after_use.split(';').next()
    {
        let correct_pragma = correct_name.trim();
        actions.push(CodeAction {
            title: format!("Fix pragma spelling: 'use {};'", correct_pragma),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::MisspelledPragma.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: format!("use {};", correct_pragma),
                }],
            },
            is_preferred: true,
        });
    }

    // Unused parameter suppression: source is used indirectly through the
    // diagnostic message which was produced from the same source text.
    let _ = source;

    actions
}

/// Fix unreachable code by removing the unreachable statement
///
/// PL406 fires when a statement follows an unconditional exit (return, die, exit).
/// The fix removes the entire line containing the unreachable statement.
pub fn fix_unreachable_code(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    // Find the full line containing the unreachable statement
    let line_start = source[..diagnostic.range.0].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_end = source[diagnostic.range.1..]
        .find('\n')
        .map(|p| diagnostic.range.1 + p + 1)
        .unwrap_or(source.len());

    vec![CodeAction {
        title: "Remove unreachable code".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::UnreachableCode.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: line_start, end: line_end },
                new_text: String::new(),
            }],
        },
        is_preferred: true,
    }]
}

/// Fix duplicate subroutine by suggesting rename of the second definition
///
/// PL300 fires when a subroutine is defined more than once. The fix renames the
/// second definition to avoid the conflict, preserving both implementations.
pub fn fix_duplicate_subroutine(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract sub name from message: "Subroutine 'foo' is defined more than once..."
    let sub_name = diagnostic.message.split('\'').nth(1).unwrap_or("sub");

    actions.push(CodeAction {
        title: format!("Rename duplicate subroutine '{}' to '{}_2'", sub_name, sub_name),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::DuplicateSubroutine.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("{}_2", sub_name),
            }],
        },
        is_preferred: true,
    });

    actions
}

/// Fix duplicate hash key by renaming or removing the duplicate entry (PL408).
///
/// Offers two actions when the same static key appears more than once in a hash literal:
///
/// 1. **Remove duplicate entry** - deletes the entire line containing the duplicate key,
///    keeping the *first* value. Offered only for simple one-line entries
///    so we don't corrupt inline hashes or multiline values.
/// 2. **Rename duplicate key** - appends `_2` to the key name so both entries are kept
///    under distinct names. The original quote style (single-quoted, double-quoted, or
///    bareword) is preserved.
///
/// The diagnostic range covers the duplicate key token (second occurrence).
/// The first occurrence location is available in `related_information` on the full
/// `Diagnostic`, but is not needed here - we fix the duplicate in-place.
pub fn fix_duplicate_hash_keys(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let Some(key_name) = duplicate_hash_key_name(&diagnostic.message) else {
        return Vec::new();
    };

    let mut actions = Vec::new();

    // Determine the key's source representation to preserve quote style.
    let Some(key_source) = source.get(diagnostic.range.0..diagnostic.range.1) else {
        return Vec::new();
    };
    let new_key = duplicate_hash_key_rename_text(key_source, key_name);

    // Offer line deletion only when the duplicate pair occupies its own line.
    if let Some((line_start, delete_end)) = duplicate_hash_key_delete_range(source, diagnostic) {
        actions.push(CodeAction {
            title: format!("Remove duplicate entry '{key_name}' (keep first value)"),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::DuplicateHashKey.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: line_start, end: delete_end },
                    new_text: String::new(),
                }],
            },
            is_preferred: true,
        });
    }

    // Rename action keeps the duplicate entry and edits only the key token.
    actions.push(CodeAction {
        title: format!("Rename duplicate key to '{new_key}'"),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::DuplicateHashKey.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: new_key,
            }],
        },
        is_preferred: false,
    });

    actions
}

fn duplicate_hash_key_name(message: &str) -> Option<&str> {
    const PREFIX: &str = "Duplicate hash key '";
    const SUFFIX: &str = "' -- only the last value will be used";

    message.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)
}

fn duplicate_hash_key_rename_text(key_source: &str, key_name: &str) -> String {
    if is_quoted_with(key_source, '\'') || is_quoted_with(key_source, '"') {
        let insert_at = key_source.len().saturating_sub(1);
        let mut renamed = String::with_capacity(key_source.len() + 2);
        renamed.push_str(&key_source[..insert_at]);
        renamed.push_str("_2");
        renamed.push_str(&key_source[insert_at..]);
        return renamed;
    }

    if is_bareword_hash_key(key_source) {
        format!("{key_source}_2")
    } else {
        format!("'{}_2'", escape_single_quoted_perl(key_name))
    }
}

fn is_quoted_with(value: &str, quote: char) -> bool {
    value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote)
}

fn is_bareword_hash_key(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn escape_single_quoted_perl(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn duplicate_hash_key_delete_range(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Option<(usize, usize)> {
    let before_key = source.get(..diagnostic.range.0)?;
    let after_key = source.get(diagnostic.range.1..)?;
    let line_start = before_key.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_end = after_key.find('\n').map(|p| diagnostic.range.1 + p).unwrap_or(source.len());
    let delete_end = if line_end < source.len() { line_end + 1 } else { line_end };
    let line_content = source.get(line_start..line_end)?;

    if is_simple_duplicate_hash_entry_line(line_content) {
        Some((line_start, delete_end))
    } else {
        None
    }
}

fn is_simple_duplicate_hash_entry_line(line_content: &str) -> bool {
    let trimmed = line_content.trim_end();

    line_content.matches("=>").count() == 1
        && trimmed.ends_with(',')
        && !line_content.contains(['(', ')', '[', ']', '{', '}'])
}

/// Fix missing return statement by adding an explicit `return` before the closing brace
///
/// PL301 fires when a subroutine has no explicit return statement. The diagnostic
/// range covers the subroutine body. This inserts `return;` at the end of the range.
pub fn fix_missing_return(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    // Find indentation to match surrounding code style
    let insert_pos = diagnostic.range.1.min(source.len());
    let indent = get_indent_at(source, insert_pos.saturating_sub(1));

    vec![CodeAction {
        title: "Add explicit 'return' statement".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::MissingReturn.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: format!("{}return;\n", indent),
            }],
        },
        is_preferred: true,
    }]
}

/// Suggest upgrading two-argument open() to three-argument form
///
/// Two-argument `open($fh, $filename)` is unsafe because `$filename` can
/// contain shell metacharacters. The three-argument form separates the mode
/// from the filename, e.g. `open(my $fh, '<', $filename)`.
pub fn fix_two_arg_open(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let Some((range, new_text)) = two_arg_open_replacement(source, diagnostic.range) else {
        return Vec::new();
    };

    vec![CodeAction {
        title: "Convert to three-argument open() for safety".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::TwoArgOpen.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: range.0, end: range.1 },
                new_text,
            }],
        },
        is_preferred: true,
    }]
}

fn two_arg_open_replacement(
    source: &str,
    range: (usize, usize),
) -> Option<((usize, usize), String)> {
    let snippet = source.get(range.0..range.1)?;
    if let Some(((start, end), new_text)) = parse_two_arg_open_call(snippet) {
        return Some(((range.0 + start, range.0 + end), new_text));
    }

    let start = range.0.min(source.len());
    let line_start = source[..start].rfind('\n').map_or(0, |idx| idx + 1);
    let line_end = source[start..].find('\n').map_or(source.len(), |offset| start + offset);
    let diagnostic_offset = start.saturating_sub(line_start);
    source.get(start..line_end).and_then(parse_two_arg_open_call).map(
        |((call_start, call_end), new_text)| {
            (
                (
                    line_start + diagnostic_offset + call_start,
                    line_start + diagnostic_offset + call_end,
                ),
                new_text,
            )
        },
    )
}

fn parse_two_arg_open_call(snippet: &str) -> Option<((usize, usize), String)> {
    let call_start = first_non_whitespace(snippet)?;
    let call = &snippet[call_start..];
    let after_open = call.strip_prefix("open")?;
    let next = after_open.chars().next()?;
    if !next.is_whitespace() && next != '(' {
        return None;
    }

    let body_start = call_start + "open".len();
    let body_start = body_start + first_non_whitespace(&snippet[body_start..])?;
    let body = &snippet[body_start..];

    let (args, call_end) = if body.starts_with('(') {
        let close = find_matching_parenthesis(body)?;
        if has_non_statement_trailing_text(&body[close + 1..]) {
            return None;
        }
        (&body[1..close], body_start + close + 1)
    } else {
        let args_end = bare_call_args_end(body)?;
        (&body[..args_end], body_start + args_end)
    };
    let (handle, path) = split_two_top_level_args(args)?;

    Some(((call_start, call_end), format!("open({}, '<', {})", handle.trim(), path.trim())))
}

fn first_non_whitespace(input: &str) -> Option<usize> {
    input.char_indices().find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
}

fn has_non_statement_trailing_text(input: &str) -> bool {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return false;
    }

    let Some(after_semicolon) = trimmed.strip_prefix(';') else {
        return true;
    };

    !after_semicolon.trim().is_empty()
}

fn bare_call_args_end(input: &str) -> Option<usize> {
    if input.trim().is_empty() {
        return None;
    }

    find_statement_semicolon(input).map_or_else(
        || {
            if contains_unquoted_comment(input) { None } else { Some(input.trim_end().len()) }
        },
        |semicolon| {
            let trailing = input[semicolon + 1..].trim();
            if trailing.is_empty() && !contains_unquoted_comment(&input[..semicolon]) {
                Some(input[..semicolon].trim_end().len())
            } else {
                None
            }
        },
    )
}

fn find_statement_semicolon(input: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => return Some(idx),
            _ => {}
        }
    }

    None
}

fn contains_unquoted_comment(input: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '#' => return true,
            _ => {}
        }
    }

    false
}

fn find_matching_parenthesis(input: &str) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' => paren_depth += 1,
            '[' => bracket_depth += 1,
            '{' => brace_depth += 1,
            ')' if paren_depth == 1 && bracket_depth == 0 && brace_depth == 0 => return Some(idx),
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
    }

    None
}

fn split_two_top_level_args(input: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut split = None;

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if split.replace(idx).is_some() {
                    return None;
                }
            }
            _ => {}
        }
    }

    let idx = split?;
    let first = &input[..idx];
    let second = &input[idx + 1..];

    if first.trim().is_empty() || second.trim().is_empty() {
        return None;
    }

    Some((first, second))
}

/// Remove an unused `use Module;` import statement (PL700).
///
/// When a module is imported but never referenced in the file, this fix
/// deletes the entire `use Module;` line so no blank line is left behind.
/// The module name is extracted from the diagnostic message
/// (`"Module 'Foo::Bar' appears to be unused"`).
pub fn fix_unused_import(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let Some((line_start, line_end)) = diagnostic_line_range(source, diagnostic.range) else {
        return Vec::new();
    };
    let Some(line_text) = source.get(line_start..line_end) else {
        return Vec::new();
    };
    if !line_text.trim_start().starts_with("use ") {
        return Vec::new();
    }

    let title = diagnostic.message.split('\'').nth(1).map_or_else(
        || "Remove unused import".to_string(),
        |module| format!("Remove unused 'use {module};'"),
    );

    vec![CodeAction {
        title,
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::UnusedImport.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: line_start, end: line_end },
                new_text: String::new(),
            }],
        },
        is_preferred: true,
    }]
}

/// Remove a deprecated `$[ = 0;` array-base variable assignment (PL501).
///
/// `$[` was a Perl variable that changed the starting index for arrays and
/// string operations. It has been deprecated since Perl 5.12. This fix removes
/// only a standalone `$[ = 0;` line. Non-zero assignments are intentionally not
/// auto-fixed because removing them can require renumbering array subscripts.
pub fn fix_deprecated_array_base(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let Some((line_start, line_end)) = diagnostic_line_range(source, diagnostic.range) else {
        return Vec::new();
    };
    let Some(line_text) = source.get(line_start..line_end) else {
        return Vec::new();
    };
    if !is_zero_array_base_assignment(line_text) {
        return Vec::new();
    }

    vec![CodeAction {
        title: "Remove deprecated '$[' array base assignment".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::DeprecatedArrayBase.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: line_start, end: line_end },
                new_text: String::new(),
            }],
        },
        is_preferred: true,
    }]
}

fn is_zero_array_base_assignment(line_text: &str) -> bool {
    let Some(rest) = line_text.trim_start().strip_prefix("$[") else {
        return false;
    };
    let Some(rest) = rest.trim_start().strip_prefix('=') else {
        return false;
    };
    let Some(rest) = rest.trim_start().strip_prefix('0') else {
        return false;
    };

    rest.trim_start().starts_with(';')
}

/// Scope a global signal handler assignment with `local` (PL602).
///
/// A bare `$SIG{__DIE__} = ...` or `$SIG{__WARN__} = ...` at file scope
/// changes signal handling for the whole process. Prepending `local` limits
/// the effect to the enclosing block so the original handler is restored on
/// scope exit, preventing the hook from leaking into unrelated call stacks.
pub fn fix_security_signal_handler(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let Some((insert_pos, _)) = valid_diagnostic_range(source, diagnostic.range) else {
        return Vec::new();
    };
    let Some(at_pos) = source.get(insert_pos..) else {
        return Vec::new();
    };
    if !at_pos.starts_with("$SIG")
        && !at_pos.starts_with("$main::SIG")
        && !at_pos.starts_with("$::SIG")
    {
        return Vec::new();
    }

    vec![CodeAction {
        title: "Add 'local' to scope signal handler to the current block".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::SecuritySignalHandler.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: "local ".to_string(),
            }],
        },
        is_preferred: true,
    }]
}

/// Add missing arguments to a `printf`/`sprintf` call (PL405 / `native.common.printf_format_arity`).
///
/// When the format string has more specifiers than supplied arguments, this fix
/// appends the required number of `undef` placeholders. When arguments exceed
/// specifiers the fix is skipped; removing arguments is too destructive to automate.
///
/// Handles both listop form (`printf "%s", $a`) and parenthesised form
/// (`printf("%s", $a)`), inserting before the closing `)`, statement semicolon,
/// or range end as appropriate.
pub fn fix_printf_format_arity(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let Some((missing, call_name)) = parse_printf_format_mismatch(&diagnostic.message) else {
        return Vec::new();
    };
    if missing == 0 {
        return Vec::new();
    }
    let Some((range_start, range_end)) = valid_diagnostic_range(source, diagnostic.range) else {
        return Vec::new();
    };
    let Some(insert_pos) =
        printf_format_insert_position(source, range_start, range_end, &call_name)
    else {
        return Vec::new();
    };

    let undef_args = ", undef".repeat(missing);
    let plural = if missing == 1 { "" } else { "s" };

    vec![CodeAction {
        title: format!("Add {missing} missing argument{plural} as undef"),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::PrintfFormatMismatch.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: undef_args,
            }],
        },
        is_preferred: true,
    }]
}

fn printf_format_insert_position(
    source: &str,
    range_start: usize,
    range_end: usize,
    call_name: &str,
) -> Option<usize> {
    let call_text = source.get(range_start..range_end)?;
    let call_offset = first_non_whitespace(call_text)?;
    let call_body = call_text.get(call_offset..)?;
    if !call_body.starts_with(call_name) {
        return None;
    }

    // Skip past the function name to detect whether the call uses parens.
    let after_name = call_body.get(call_name.len()..)?;
    let after_name_trimmed = after_name.trim_start();
    if after_name_trimmed.starts_with('(') {
        // Parenthesized form: insert the new args before the closing ')'.
        let paren_start_offset = range_end - after_name_trimmed.len() - range_start;
        let close = find_matching_parenthesis(after_name_trimmed)?;
        if has_non_statement_trailing_text(&after_name_trimmed[close + 1..]) {
            return None;
        }
        Some(range_start + paren_start_offset + close)
    } else {
        // Listop form: insert before a statement semicolon when one is inside
        // the diagnostic range, otherwise at the trimmed range end.
        bare_call_args_end(call_text).map(|end| range_start + end)
    }
}

/// Parse a `printf`/`sprintf` format-arity message and return `(missing_count, call_name)`.
///
/// Returns `None` when the message cannot be parsed or when args exceed specifiers
/// (the caller decides not to offer a fix in the "too many args" case).
fn parse_printf_format_mismatch(message: &str) -> Option<(usize, String)> {
    // Message form: "`printf` format string has N specifier(s) but M argument(s) supplied"
    let backtick_start = message.find('`')? + 1;
    let backtick_end = backtick_start + message[backtick_start..].find('`')?;
    let call_name = message[backtick_start..backtick_end].to_string();

    let after_has = message.split("has ").nth(1)?;
    let specifiers: usize = after_has.split_whitespace().next()?.parse().ok()?;

    let after_but = message.split("but ").nth(1)?;
    let supplied: usize = after_but.split_whitespace().next()?.parse().ok()?;

    specifiers.checked_sub(supplied).filter(|&n| n > 0).map(|n| (n, call_name))
}

fn diagnostic_line_range(source: &str, range: (usize, usize)) -> Option<(usize, usize)> {
    let (start, end) = valid_diagnostic_range(source, range)?;
    let line_start = source[..start].rfind('\n').map_or(0, |idx| idx + 1);
    let newline_offset = source[end..].find('\n');
    let line_end = newline_offset.map_or(source.len(), |offset| end + offset + 1);
    Some((line_start, line_end))
}

fn valid_diagnostic_range(source: &str, range: (usize, usize)) -> Option<(usize, usize)> {
    let (start, end) = range;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return None;
    }
    Some((start, end))
}
