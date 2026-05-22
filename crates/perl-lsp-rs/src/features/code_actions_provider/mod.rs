//! Code action provider model and generation entry points.

use crate::features::diagnostics::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;

mod fixes;
mod parse_errors;
mod source_utils;
mod types;

pub use types::{CodeAction, CodeActionKind, TextEdit};

/// Provides code actions (quick-fixes) for diagnostics
///
/// Analyzes Perl source code and diagnostics to provide automated fixes
/// and refactoring actions.
pub struct CodeActionsProvider {
    source: String,
}

impl CodeActionsProvider {
    /// Creates a new code actions provider
    ///
    /// # Arguments
    ///
    /// * `source` - The Perl source code to analyze for code actions
    ///
    /// # Returns
    ///
    /// A new `CodeActionsProvider` instance ready to generate actions
    pub fn new(source: String) -> Self {
        Self { source }
    }

    /// Get all available code actions for a given range
    pub fn get_code_actions(
        &self,
        range: (usize, usize),
        diagnostics: &[Diagnostic],
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        for diagnostic in diagnostics {
            if source_utils::ranges_overlap(diagnostic.range, range) {
                actions.extend(self.get_actions_for_diagnostic(diagnostic));
            }
        }

        actions
    }

    /// Get code actions for a specific diagnostic
    fn get_actions_for_diagnostic(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        if !source_utils::is_valid_source_range(self.source(), diagnostic.range) {
            return Vec::new();
        }

        let code = diagnostic.code.as_deref();
        match diagnostic.code.as_deref() {
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::UndefinedVariable.as_str(),
                    "undefined-variable",
                    "undeclared-variable",
                ],
            ) =>
            {
                fixes::fix_undefined_variable(self, diagnostic)
            }
            _ if has_any_code(
                code,
                &[DiagnosticCode::UnusedVariable.as_str(), "unused-variable"],
            ) =>
            {
                fixes::fix_unused_variable(self, diagnostic)
            }
            Some("native.variables.unused_lexical") => fixes::fix_unused_variable(self, diagnostic),
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::AssignmentInCondition.as_str(),
                    "assignment-in-condition",
                    "native.common.assignment_in_condition",
                ],
            ) =>
            {
                fixes::fix_assignment_in_condition(self, diagnostic)
            }
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::DeprecatedDefined.as_str(),
                    "deprecated-defined",
                    "native.common.deprecated_defined",
                ],
            ) =>
            {
                fixes::fix_deprecated_defined(self, diagnostic)
            }
            Some("native.common.undef_comparison") => {
                fixes::fix_native_undef_comparison(self, diagnostic)
            }
            Some("native.testing.require_use_strict") => fixes::add_use_strict(diagnostic),
            Some("native.testing.require_use_warnings") => fixes::add_use_warnings(diagnostic),
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::VariableShadowing.as_str(),
                    "variable-shadowing",
                    "native.variables.shadowed_lexical",
                ],
            ) =>
            {
                fixes::fix_variable_shadowing(diagnostic)
            }
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::VariableRedeclaration.as_str(),
                    "variable-redeclaration",
                    "native.variables.duplicate_lexical",
                ],
            ) =>
            {
                fixes::fix_variable_redeclaration(self, diagnostic)
            }
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::DuplicateParameter.as_str(),
                    "duplicate-parameter",
                    "native.variables.duplicate_parameter",
                ],
            ) =>
            {
                fixes::fix_duplicate_parameter(diagnostic)
            }
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::ParameterShadowsGlobal.as_str(),
                    "parameter-shadows-global",
                    "native.variables.parameter_shadows_global",
                ],
            ) =>
            {
                fixes::fix_parameter_shadowing(diagnostic)
            }
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::UnusedParameter.as_str(),
                    "unused-parameter",
                    "native.variables.unused_parameter",
                ],
            ) =>
            {
                fixes::fix_unused_parameter(diagnostic)
            }
            _ if has_any_code(
                code,
                &[DiagnosticCode::UnquotedBareword.as_str(), "unquoted-bareword"],
            ) =>
            {
                fixes::fix_unquoted_bareword(self, diagnostic)
            }
            _ if has_any_code(
                code,
                &[
                    DiagnosticCode::BarewordFilehandle.as_str(),
                    "bareword-filehandle",
                    "native.io.bareword_filehandle",
                ],
            ) =>
            {
                fixes::fix_bareword_filehandle(diagnostic)
            }
            _ if has_any_code(
                code,
                &[DiagnosticCode::TwoArgOpen.as_str(), "two-arg-open", "native.io.two_arg_open"],
            ) =>
            {
                fixes::fix_two_arg_open(self, diagnostic)
            }
            Some(code) if code.starts_with("parse-error-") => {
                fixes::fix_parse_error(self, diagnostic, code)
            }
            // PL001 / PL002 are general parse error codes. Route known parse
            // message patterns through the same targeted parse quick-fixes.
            Some("PL001") | Some("PL002") => {
                parse_errors::parse_error_fix_code_from_message(&diagnostic.message)
                    .map_or_else(Vec::new, |code| fixes::fix_parse_error(self, diagnostic, code))
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn source(&self) -> &str {
        &self.source
    }
}

fn has_any_code(code: Option<&str>, aliases: &[&str]) -> bool {
    code.is_some_and(|candidate| aliases.contains(&candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::DiagnosticSeverity;
    use perl_tdd_support::must_some;

    /// Helper to build a diagnostic with minimal boilerplate.
    fn make_diagnostic(
        range: (usize, usize),
        severity: DiagnosticSeverity,
        code: &str,
        message: &str,
    ) -> Diagnostic {
        Diagnostic {
            range,
            severity,
            code: Some(code.to_string()),
            message: message.to_string(),
            related_information: vec![],
            tags: vec![],
            suggestion: None,
        }
    }

    fn provider_covering(range: (usize, usize)) -> CodeActionsProvider {
        CodeActionsProvider::new(" ".repeat(range.1))
    }

    #[test]
    fn test_invalid_diagnostic_ranges_do_not_panic() {
        let source = "use strict;\nprint $x;".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let invalid_ranges =
            [(source.len() + 1, source.len() + 3), (8, 3), (usize::MAX, usize::MAX)];

        for range in invalid_ranges {
            let diagnostic = make_diagnostic(
                range,
                DiagnosticSeverity::Error,
                "undefined-variable",
                "Variable '$x' is undefined",
            );

            let actions = provider.get_actions_for_diagnostic(&diagnostic);

            assert!(actions.is_empty());
        }
    }

    #[test]
    fn test_non_char_boundary_diagnostic_ranges_do_not_panic() {
        let source = "use strict;\nprint \"\u{00e9}\";".to_string();
        let accent_start = must_some(source.find('\u{00e9}'));
        let provider = CodeActionsProvider::new(source);
        let diagnostic = make_diagnostic(
            (accent_start + 1, accent_start + '\u{00e9}'.len_utf8()),
            DiagnosticSeverity::Error,
            "undefined-variable",
            "Variable '$x' is undefined",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert!(actions.is_empty());
    }

    // ── Quick-fix: undefined / undeclared variable ──────────────────────

    #[test]
    fn test_undefined_variable_fix() {
        let source = "use strict;\nprint $x;".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (18, 20),
            DiagnosticSeverity::Error,
            "undefined-variable",
            "Variable '$x' is undefined",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].title, "Declare '$x' with 'my'");
        assert_eq!(actions[1].title, "Declare '$x' with 'our'");
        assert_eq!(actions[0].kind, CodeActionKind::QuickFix);
        assert_eq!(actions[1].kind, CodeActionKind::QuickFix);
    }

    #[test]
    fn test_undeclared_variable_fix_same_as_undefined() {
        let source = "use strict;\nprint $y;".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (18, 20),
            DiagnosticSeverity::Error,
            "undeclared-variable",
            "Variable '$y' is undeclared",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].title, "Declare '$y' with 'my'");
        assert_eq!(actions[1].title, "Declare '$y' with 'our'");
    }

    #[test]
    fn test_undefined_variable_fix_inserts_at_line_start() {
        // "use strict;\n" is 12 bytes, so $x starts at offset 18.
        // The declaration should be inserted at the start of the line containing $x.
        let source = "use strict;\nprint $x;".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (18, 20),
            DiagnosticSeverity::Error,
            "undefined-variable",
            "Variable '$x' is undefined",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        // Insert position should be right after the '\n' (offset 12)
        assert_eq!(actions[0].edit.range, (12, 12));
        assert_eq!(actions[0].edit.new_text, "my $x;\n");
    }

    #[test]
    fn test_undefined_variable_fix_no_quoted_value_returns_empty() {
        let source = "print $x;".to_string();
        let provider = CodeActionsProvider::new(source);

        // Message without quotes around the variable name
        let diagnostic = make_diagnostic(
            (6, 8),
            DiagnosticSeverity::Error,
            "undefined-variable",
            "Variable x is undefined",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert!(actions.is_empty());
    }

    // ── Quick-fix: unused variable ──────────────────────────────────────

    #[test]
    fn test_unused_variable_fix() {
        let source = "my $unused = 42;".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (3, 10),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$unused' is declared but never used",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 2);
        assert!(actions[0].title.contains("Remove"));
        assert!(actions[1].title.contains("$_unused"));
    }

    #[test]
    fn test_unused_variable_rename_produces_underscore_prefix() {
        let source = "my $count = 0;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (3, 9),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$count' is declared but never used",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 2);
        // The rename action should produce $_count
        assert_eq!(actions[1].edit.new_text, "$_count");
    }

    #[test]
    fn test_unused_variable_remove_action_clears_declaration() {
        let source = "my $unused = 42;\nprint 1;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (3, 10),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$unused' is declared but never used",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        let remove = must_some(
            actions.iter().find(|action| action.title.contains("Remove unused variable")),
        );

        let declaration_end = must_some(provider.source().find('\n')) + 1;
        assert_eq!(remove.edit.range, (0, declaration_end));
        assert!(remove.edit.new_text.is_empty());
    }

    #[test]
    fn test_unused_variable_remove_action_uses_nearest_same_line_declaration() {
        let source = "my $x = 1; { my $x = 2; }\n".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let inner_decl = must_some(source.rfind("my $x"));

        let diagnostic = make_diagnostic(
            (inner_decl + 3, inner_decl + 5),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$x' is declared but never used",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        let remove = must_some(
            actions.iter().find(|action| action.title.contains("Remove unused variable")),
        );

        assert_eq!(remove.edit.range.0, inner_decl);
        assert_eq!(&provider.source()[remove.edit.range.0..remove.edit.range.1], "my $x = 2;");
    }

    #[test]
    fn test_unused_variable_fix_skips_remove_when_declaration_is_not_simple_my() {
        let source = "my ($used, $unused) = @_;\n".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let start = must_some(source.find("$unused"));

        let diagnostic = make_diagnostic(
            (start, start + "$unused".len()),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$unused' is declared but never used",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Rename to '$_unused' (mark as intentionally unused)");
    }

    #[test]
    fn test_native_critic_strict_warnings_quick_fixes() {
        let provider = CodeActionsProvider::new("print 'hello';\n".to_string());
        let diagnostics = vec![
            make_diagnostic(
                (0, 0),
                DiagnosticSeverity::Warning,
                "native.testing.require_use_strict",
                "Code does not use strict",
            ),
            make_diagnostic(
                (0, 0),
                DiagnosticSeverity::Warning,
                "native.testing.require_use_warnings",
                "Code does not use warnings",
            ),
        ];

        let actions = provider.get_code_actions((0, 1), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Add 'use strict'" && action.edit.new_text == "use strict;\n"
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Add 'use warnings'" && action.edit.new_text == "use warnings;\n"
        }));
    }

    #[test]
    fn test_native_critic_unused_lexical_quick_fix() {
        let source = "use strict;\nuse warnings;\nmy $unused = 1;\n".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let start = must_some(source.find("$unused"));
        let diagnostic = make_diagnostic(
            (start, start + "$unused".len()),
            DiagnosticSeverity::Warning,
            "native.variables.unused_lexical",
            "Lexical variable '$unused' is declared but never used",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        let declaration_start = must_some(source.find("my $unused"));
        assert!(actions.iter().any(|action| {
            action.title == "Remove unused variable '$unused'"
                && action.edit.range == (declaration_start, source.len())
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Rename to '$_unused' (mark as intentionally unused)"
                && action.edit.range == diagnostic.range
                && action.edit.new_text == "$_unused"
        }));
    }

    #[test]
    fn test_native_critic_duplicate_lexical_quick_fix() {
        let source = "use strict;\nuse warnings;\nmy $dup = 1;\nmy $dup = 2;\n".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let start = must_some(source.rfind("$dup"));
        let diagnostic = make_diagnostic(
            (start, start + "$dup".len()),
            DiagnosticSeverity::Error,
            "native.variables.duplicate_lexical",
            "Lexical variable '$dup' is declared more than once in the same scope",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Remove redundant 'my'");
        let declaration_start = must_some(source.rfind("my $dup"));
        assert_eq!(actions[0].edit.range, (declaration_start, start));
        assert_eq!(actions[0].edit.new_text, "");
    }

    // ── Quick-fix: variable shadowing ───────────────────────────────────

    #[test]
    fn test_variable_shadowing_fix_offers_three_alternatives() {
        let diagnostic = make_diagnostic(
            (20, 24),
            DiagnosticSeverity::Warning,
            "variable-shadowing",
            "Variable '$foo' shadows outer variable",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].title, "Rename shadowing variable to '$inner_foo'");
        assert_eq!(actions[1].title, "Rename shadowing variable to '$local_foo'");
        assert_eq!(actions[2].title, "Rename shadowing variable to '$foo_2'");
    }

    #[test]
    fn test_native_critic_shadowed_lexical_quick_fix() {
        let diagnostic = make_diagnostic(
            (20, 26),
            DiagnosticSeverity::Warning,
            "native.variables.shadowed_lexical",
            "Lexical variable '$value' shadows an outer declaration",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].title, "Rename shadowing variable to '$inner_value'");
        assert_eq!(actions[1].title, "Rename shadowing variable to '$local_value'");
        assert_eq!(actions[2].title, "Rename shadowing variable to '$value_2'");
    }

    #[test]
    fn test_variable_shadowing_fix_preserves_sigil() {
        let diagnostic = make_diagnostic(
            (10, 15),
            DiagnosticSeverity::Warning,
            "variable-shadowing",
            "Variable '@items' shadows outer variable",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions[0].edit.new_text, "@inner_items");
        assert_eq!(actions[1].edit.new_text, "@local_items");
        assert_eq!(actions[2].edit.new_text, "@items_2");
    }

    #[test]
    fn test_variable_shadowing_fix_hash_sigil() {
        let diagnostic = make_diagnostic(
            (5, 10),
            DiagnosticSeverity::Warning,
            "variable-shadowing",
            "Variable '%cfg' shadows outer variable",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions[0].edit.new_text, "%inner_cfg");
    }

    // ── Quick-fix: variable redeclaration ───────────────────────────────

    #[test]
    fn test_variable_redeclaration_fix_removes_redundant_my() {
        let source = "my $x = 1;\nmy $x = 2;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (11, 21),
            DiagnosticSeverity::Error,
            "variable-redeclaration",
            "Variable '$x' is redeclared",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Remove redundant 'my'");
        // Should remove "my " (3 bytes) from the start of the range
        assert_eq!(actions[0].edit.range, (11, 14));
        assert!(actions[0].edit.new_text.is_empty());
    }

    #[test]
    fn test_variable_redeclaration_fix_no_action_when_not_my() {
        let source = "our $x = 1;\nour $x = 2;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (12, 23),
            DiagnosticSeverity::Error,
            "variable-redeclaration",
            "Variable '$x' is redeclared",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert!(actions.is_empty());
    }

    // ── Quick-fix: duplicate parameter ──────────────────────────────────

    #[test]
    fn test_duplicate_parameter_fix_offers_remove_and_rename() {
        let diagnostic = make_diagnostic(
            (30, 34),
            DiagnosticSeverity::Error,
            "duplicate-parameter",
            "Parameter '$arg' is duplicated",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 2);
        assert!(actions[0].title.contains("Remove duplicate"));
        assert!(actions[1].title.contains("Rename duplicate to '$arg_2'"));
    }

    #[test]
    fn test_duplicate_parameter_rename_preserves_sigil() {
        let diagnostic = make_diagnostic(
            (10, 16),
            DiagnosticSeverity::Error,
            "duplicate-parameter",
            "Parameter '@vals' is duplicated",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions[1].edit.new_text, "@vals_2");
    }

    #[test]
    fn test_native_critic_duplicate_parameter_quick_fix() {
        let diagnostic = make_diagnostic(
            (30, 34),
            DiagnosticSeverity::Error,
            "native.variables.duplicate_parameter",
            "Parameter '$arg' appears more than once in this signature",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].title, "Remove duplicate parameter '$arg'");
        assert_eq!(actions[0].edit.range, (30, 34));
        assert_eq!(actions[0].edit.new_text, "");
        assert_eq!(actions[1].title, "Rename duplicate to '$arg_2'");
        assert_eq!(actions[1].edit.new_text, "$arg_2");
    }

    // ── Quick-fix: parameter shadows global ─────────────────────────────

    #[test]
    fn test_parameter_shadowing_fix_offers_three_alternatives() {
        let diagnostic = make_diagnostic(
            (15, 20),
            DiagnosticSeverity::Warning,
            "parameter-shadows-global",
            "Parameter '$name' shadows global variable",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].title, "Rename parameter to '$p_name'");
        assert_eq!(actions[1].title, "Rename parameter to '$name_param'");
        assert_eq!(actions[2].title, "Rename parameter to '$name_arg'");
    }

    #[test]
    fn test_parameter_shadowing_fix_preserves_hash_sigil() {
        let diagnostic = make_diagnostic(
            (5, 12),
            DiagnosticSeverity::Warning,
            "parameter-shadows-global",
            "Parameter '%opts' shadows global variable",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions[0].edit.new_text, "%p_opts");
        assert_eq!(actions[1].edit.new_text, "%opts_param");
        assert_eq!(actions[2].edit.new_text, "%opts_arg");
    }

    #[test]
    fn test_native_critic_parameter_shadows_global_quick_fix() {
        let diagnostic = make_diagnostic(
            (20, 25),
            DiagnosticSeverity::Warning,
            "native.variables.parameter_shadows_global",
            "Parameter '$name' shadows an outer declaration",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].title, "Rename parameter to '$p_name'");
        assert_eq!(actions[0].edit.new_text, "$p_name");
        assert_eq!(actions[1].title, "Rename parameter to '$name_param'");
        assert_eq!(actions[2].title, "Rename parameter to '$name_arg'");
    }

    #[test]
    fn test_native_critic_assignment_in_condition_quick_fix() {
        let source = "if ($x = 5) { }";
        let diagnostic = make_diagnostic(
            (4, 10),
            DiagnosticSeverity::Warning,
            "native.common.assignment_in_condition",
            "Assignment in condition - did you mean '=='?",
        );

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].title, "Change to comparison (==)");
        assert_eq!(actions[0].edit.range, (7, 8));
        assert_eq!(actions[0].edit.new_text, "==");
        assert_eq!(actions[1].title, "Keep assignment (add parentheses)");
        assert_eq!(actions[1].edit.range, (4, 10));
        assert_eq!(actions[1].edit.new_text, "($x = 5)");
    }

    #[test]
    fn test_native_critic_deprecated_defined_quick_fix() {
        let source = "if (defined @items) { print @items; }";
        let diagnostic = make_diagnostic(
            (4, 18),
            DiagnosticSeverity::Warning,
            "native.common.deprecated_defined",
            "Use of 'defined @items' is deprecated",
        );

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Replace with '@items'");
        assert_eq!(actions[0].edit.range, (4, 18));
        assert_eq!(actions[0].edit.new_text, "@items");
    }

    #[test]
    fn test_native_critic_deprecated_defined_quick_fix_normalizes_parentheses() {
        let source = "if (defined(%seen)) { print keys %seen; }";
        let diagnostic = make_diagnostic(
            (4, 18),
            DiagnosticSeverity::Warning,
            "native.common.deprecated_defined",
            "Use of 'defined %seen' is deprecated",
        );

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Replace with '%seen'");
        assert_eq!(actions[0].edit.range, (4, 18));
        assert_eq!(actions[0].edit.new_text, "%seen");
    }

    #[test]
    fn test_native_critic_undef_comparison_quick_fix() {
        let source = "if ($value == undef) { print $value; }";
        let diagnostic = make_diagnostic(
            (4, 19),
            DiagnosticSeverity::Warning,
            "native.common.undef_comparison",
            "Using '==' with undef -- use defined() to check first",
        );

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Use defined() check");
        assert_eq!(actions[0].edit.range, (4, 19));
        assert_eq!(actions[0].edit.new_text, "!defined($value)");
    }

    #[test]
    fn test_native_critic_bareword_filehandle_quick_fix() {
        let diagnostic = make_diagnostic(
            (5, 7),
            DiagnosticSeverity::Warning,
            "native.io.bareword_filehandle",
            "Bareword filehandle 'FH' should be lexical",
        );

        let provider = CodeActionsProvider::new("open FH, $path;\n".to_string());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Replace bareword filehandle 'FH' with lexical '$fh_fh'");
        assert_eq!(actions[0].edit.range, (5, 7));
        assert_eq!(actions[0].edit.new_text, "my $fh_fh");
    }

    #[test]
    fn test_native_critic_two_arg_open_quick_fix() {
        let diagnostic = make_diagnostic(
            (0, 19),
            DiagnosticSeverity::Warning,
            "native.io.two_arg_open",
            "Two-argument open should use an explicit mode",
        );

        let provider = CodeActionsProvider::new("open(my $fh, $path);\n".to_string());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Convert to three-argument open() for safety");
        assert_eq!(actions[0].edit.range, (0, 19));
        assert_eq!(actions[0].edit.new_text, "open(my $fh, '<', $path)");
    }

    #[test]
    fn test_legacy_two_arg_open_range_only_open_edits_whole_call() {
        let diagnostic = make_diagnostic(
            (0, 4),
            DiagnosticSeverity::Warning,
            "two-arg-open",
            "Two-argument open should use an explicit mode",
        );

        let provider = CodeActionsProvider::new("open FH, $path;\n".to_string());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Convert to three-argument open() for safety");
        assert_eq!(actions[0].edit.range, (0, "open FH, $path".len()));
        assert_eq!(actions[0].edit.new_text, "open(FH, '<', $path)");
    }

    #[test]
    fn test_legacy_two_arg_open_range_only_open_rejects_ambiguous_line_fallback() {
        let diagnostic = make_diagnostic(
            (0, 4),
            DiagnosticSeverity::Warning,
            "two-arg-open",
            "Two-argument open should use an explicit mode",
        );

        for source in ["open FH, $path; # legacy\n", "open FH, $path; close FH;\n"] {
            let provider = CodeActionsProvider::new(source.to_string());
            let actions = provider.get_actions_for_diagnostic(&diagnostic);

            assert!(
                actions.is_empty(),
                "ambiguous fallback should not produce a fix for {source:?}"
            );
        }
    }

    // ── Quick-fix: unused parameter ─────────────────────────────────────

    #[test]
    fn test_unused_parameter_fix_offers_safe_rename_only() {
        let diagnostic = make_diagnostic(
            (20, 25),
            DiagnosticSeverity::Warning,
            "unused-parameter",
            "Parameter '$self' is unused",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert!(actions[0].title.contains("$_self"));
        assert!(actions[0].title.contains("mark as intentionally unused"));
    }

    #[test]
    fn test_native_critic_unused_parameter_quick_fix() {
        let diagnostic = make_diagnostic(
            (20, 27),
            DiagnosticSeverity::Warning,
            "native.variables.unused_parameter",
            "Parameter '$unused' is never used",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Rename to '$_unused' (mark as intentionally unused)");
        assert_eq!(actions[0].edit.range, (20, 27));
        assert_eq!(actions[0].edit.new_text, "$_unused");
    }

    #[test]
    fn test_unused_parameter_rename_stays_within_parameter_range() {
        let diagnostic = make_diagnostic(
            (20, 25),
            DiagnosticSeverity::Warning,
            "unused-parameter",
            "Parameter '$ctx' is unused",
        );

        let provider = provider_covering(diagnostic.range);
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].edit.range, (20, 25));
        assert_eq!(actions[0].edit.new_text, "$_ctx");
    }

    // ── Quick-fix: unquoted bareword ────────────────────────────────────

    #[test]
    fn test_unquoted_bareword_fix_offers_quoting() {
        let source = "my %h = (foo => 1);\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (9, 12),
            DiagnosticSeverity::Error,
            "unquoted-bareword",
            "Bareword 'foo' used in expression",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert!(actions.len() >= 2);
        assert_eq!(actions[0].title, "Quote bareword as 'foo'");
        assert_eq!(actions[1].title, "Quote bareword as \"foo\"");
        assert_eq!(actions[0].edit.new_text, "'foo'");
        assert_eq!(actions[1].edit.new_text, "\"foo\"");
    }

    #[test]
    fn test_unquoted_bareword_uppercase_offers_filehandle_declaration() {
        let source = "print LOGFILE 'hello';\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (6, 13),
            DiagnosticSeverity::Error,
            "unquoted-bareword",
            "Bareword 'LOGFILE' used in expression",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        // 2 quoting options + 1 filehandle declaration
        assert_eq!(actions.len(), 3);
        assert!(actions[2].title.contains("filehandle"));
        assert!(actions[2].edit.new_text.contains("open my $logfile"));
    }

    #[test]
    fn test_unquoted_bareword_lowercase_no_filehandle_action() {
        let source = "print hello;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (6, 11),
            DiagnosticSeverity::Error,
            "unquoted-bareword",
            "Bareword 'hello' used in expression",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        // Only 2 quoting options, no filehandle
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_unquoted_bareword_underscore_in_name_offers_filehandle() {
        let source = "print LOG_FILE 'msg';\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (6, 14),
            DiagnosticSeverity::Error,
            "unquoted-bareword",
            "Bareword 'LOG_FILE' used in expression",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        // uppercase + underscore = still qualifies as filehandle
        assert_eq!(actions.len(), 3);
    }

    // ── Quick-fix: parse errors ─────────────────────────────────────────

    #[test]
    fn test_parse_error_semicolon_fix() {
        let source = "print 'hello'\nprint 'world';".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = Diagnostic {
            range: (13, 14),
            severity: DiagnosticSeverity::Error,
            code: Some("parse-error-missingsemicolon".to_string()),
            message: "Missing semicolon".to_string(),
            related_information: vec![],
            tags: vec![],
            suggestion: Some("Add a ';' at the end of the statement".to_string()),
        };

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add missing semicolon");
    }

    #[test]
    fn test_parse_error_unclosed_string_fix_single_quote() {
        let source = "my $x = 'hello;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (8, 15),
            DiagnosticSeverity::Error,
            "parse-error-unclosedstring",
            "Unclosed string literal",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert!(actions[0].title.contains("closing quote"));
        assert_eq!(actions[0].edit.range, (15, 15));
    }

    #[test]
    fn test_parse_error_unclosed_string_fix_double_quote() {
        // No single quote near the position, so detect_quote_char defaults to double
        let source = "my $x = \"hello;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (8, 15),
            DiagnosticSeverity::Error,
            "parse-error-unclosedstring",
            "Unclosed string literal",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].edit.new_text, "\"");
    }

    #[test]
    fn test_parse_error_unclosed_paren_fix() {
        let source = "my @a = (1, 2, 3\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (8, 17),
            DiagnosticSeverity::Error,
            "parse-error-unclosedparen",
            "Unclosed parenthesis",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add closing parenthesis");
        assert_eq!(actions[0].edit.new_text, ")");
        assert_eq!(actions[0].edit.range, (17, 17));
    }

    #[test]
    fn test_parse_error_unclosed_brace_fix() {
        let source = "if ($x) {\n    print 1;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (8, 22),
            DiagnosticSeverity::Error,
            "parse-error-unclosedbrace",
            "Unclosed brace",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add closing brace");
        assert_eq!(actions[0].edit.new_text, "}");
    }

    #[test]
    fn test_parse_error_unknown_code_returns_empty() {
        let source = "broken code".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (0, 6),
            DiagnosticSeverity::Error,
            "parse-error-unknownthing",
            "Unknown parse error",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert!(actions.is_empty());
    }

    // ── get_code_actions: diagnostic context / range filtering ──────────

    #[test]
    fn test_get_code_actions_filters_by_range_overlap() {
        let source = "my $a = 1;\nmy $b = 2;\nprint $c;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diag_a = make_diagnostic(
            (3, 5),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$a' is declared but never used",
        );
        let diag_c = make_diagnostic(
            (27, 29),
            DiagnosticSeverity::Error,
            "undefined-variable",
            "Variable '$c' is undefined",
        );

        let diagnostics = vec![diag_a, diag_c];

        // Query a range that only overlaps with the first diagnostic
        let actions = provider.get_code_actions((0, 10), &diagnostics);
        assert!(!actions.is_empty());
        // All returned actions should relate to the unused-variable diagnostic
        for action in &actions {
            assert_eq!(action.diagnostic_id.as_deref(), Some("unused-variable"));
        }
    }

    #[test]
    fn test_get_code_actions_returns_empty_when_no_overlap() {
        let source = "my $a = 1;\nprint $b;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (0, 5),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$a' is declared but never used",
        );

        // Query range that doesn't overlap with the diagnostic
        let actions = provider.get_code_actions((15, 20), &[diagnostic]);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_get_code_actions_with_empty_diagnostics() {
        let source = "print 'hello';\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let actions = provider.get_code_actions((0, 15), &[]);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_get_code_actions_multiple_diagnostics_overlap() {
        let source = "my $x = $y;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diag_unused = make_diagnostic(
            (3, 5),
            DiagnosticSeverity::Warning,
            "unused-variable",
            "Variable '$x' is declared but never used",
        );
        let diag_undef = make_diagnostic(
            (8, 10),
            DiagnosticSeverity::Error,
            "undefined-variable",
            "Variable '$y' is undefined",
        );

        // Query the whole line -- both diagnostics overlap
        let actions = provider.get_code_actions((0, 12), &[diag_unused, diag_undef]);
        // Should have actions from both diagnostics
        let has_unused =
            actions.iter().any(|a| a.diagnostic_id.as_deref() == Some("unused-variable"));
        let has_undef =
            actions.iter().any(|a| a.diagnostic_id.as_deref() == Some("undefined-variable"));
        assert!(has_unused);
        assert!(has_undef);
    }

    // ── Unknown / no diagnostic code ────────────────────────────────────

    #[test]
    fn test_unknown_diagnostic_code_returns_empty() {
        let source = "print 1;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (0, 7),
            DiagnosticSeverity::Warning,
            "unknown-code-xyz",
            "Some unknown diagnostic",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_diagnostic_with_no_code_returns_empty() {
        let source = "print 1;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = Diagnostic {
            range: (0, 7),
            severity: DiagnosticSeverity::Warning,
            code: None,
            message: "No code diagnostic".to_string(),
            related_information: vec![],
            tags: vec![],
            suggestion: None,
        };

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert!(actions.is_empty());
    }

    // ── CodeAction struct field verification ─────────────────────────────

    #[test]
    fn test_code_action_carries_diagnostic_id() {
        let source = "use strict;\nprint $z;".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (18, 20),
            DiagnosticSeverity::Error,
            "undefined-variable",
            "Variable '$z' is undefined",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        for action in &actions {
            assert_eq!(action.diagnostic_id.as_deref(), Some("undefined-variable"));
            assert_eq!(action.diagnostic_range, Some((18, 20)));
        }
    }

    // ── source_utils unit tests ─────────────────────────────────────────

    #[test]
    fn test_ranges_overlap_full_overlap() {
        assert!(source_utils::ranges_overlap((0, 10), (5, 15)));
    }

    #[test]
    fn test_ranges_overlap_contained() {
        assert!(source_utils::ranges_overlap((2, 8), (0, 10)));
    }

    #[test]
    fn test_ranges_overlap_no_overlap() {
        assert!(!source_utils::ranges_overlap((0, 5), (5, 10)));
    }

    #[test]
    fn test_ranges_overlap_adjacent_no_overlap() {
        assert!(!source_utils::ranges_overlap((0, 5), (5, 10)));
        assert!(!source_utils::ranges_overlap((5, 10), (0, 5)));
    }

    #[test]
    fn test_ranges_overlap_identical() {
        assert!(source_utils::ranges_overlap((3, 7), (3, 7)));
    }

    #[test]
    fn test_ranges_overlap_single_point_overlap() {
        assert!(source_utils::ranges_overlap((0, 6), (5, 10)));
    }

    #[test]
    fn test_extract_quoted_value_single_quotes() {
        let result = source_utils::extract_quoted_value("Variable '$foo' is undefined");
        assert_eq!(result, Some("$foo".to_string()));
    }

    #[test]
    fn test_extract_quoted_value_backticks() {
        let result = source_utils::extract_quoted_value("Variable `$bar` is undefined");
        assert_eq!(result, Some("$bar".to_string()));
    }

    #[test]
    fn test_extract_quoted_value_no_quotes() {
        let result = source_utils::extract_quoted_value("Variable $baz is undefined");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_quoted_value_double_quotes() {
        let result = source_utils::extract_quoted_value("Variable \"$baz\" is undefined");
        assert_eq!(result, Some("$baz".to_string()));
    }

    #[test]
    fn test_extract_quoted_value_single_quote_preferred_over_backtick() {
        // Single quotes appear first in the message, so they should be extracted
        let result = source_utils::extract_quoted_value("'first' then `second`");
        assert_eq!(result, Some("first".to_string()));
    }

    #[test]
    fn test_split_sigil_scalar() {
        let (sigil, name) = source_utils::split_sigil("$foo");
        assert_eq!(sigil, "$");
        assert_eq!(name, "foo");
    }

    #[test]
    fn test_split_sigil_array() {
        let (sigil, name) = source_utils::split_sigil("@items");
        assert_eq!(sigil, "@");
        assert_eq!(name, "items");
    }

    #[test]
    fn test_split_sigil_hash() {
        let (sigil, name) = source_utils::split_sigil("%config");
        assert_eq!(sigil, "%");
        assert_eq!(name, "config");
    }

    #[test]
    fn test_split_sigil_no_sigil() {
        let (sigil, name) = source_utils::split_sigil("bareword");
        assert_eq!(sigil, "");
        assert_eq!(name, "bareword");
    }

    #[test]
    fn test_make_unused_name_scalar() {
        assert_eq!(source_utils::make_unused_name("$foo"), "$_foo");
    }

    #[test]
    fn test_make_unused_name_array() {
        assert_eq!(source_utils::make_unused_name("@items"), "@_items");
    }

    #[test]
    fn test_make_unused_name_hash() {
        assert_eq!(source_utils::make_unused_name("%config"), "%_config");
    }

    #[test]
    fn test_make_unused_name_no_sigil() {
        assert_eq!(source_utils::make_unused_name("plain"), "_plain");
    }

    #[test]
    fn test_find_declaration_position_at_line_start() {
        let source = "line1\nline2\nline3".to_string();
        let provider = CodeActionsProvider::new(source);

        // Position 8 is in "line2"; line start is at 6 (after first '\n')
        let pos = source_utils::find_declaration_position(&provider, 8);
        assert_eq!(pos, 6);
    }

    #[test]
    fn test_find_declaration_range_prefers_same_line_binding() {
        let source = "my $name = 'global';\nmy $name = 'local';\nprint $name;\n".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let near = source.find("print $name").unwrap_or(0);

        let (start, end) =
            must_some(source_utils::find_declaration_range(&provider, "$name", near));

        assert_eq!(&source[start..end], "my $name = 'local';\n");
    }

    #[test]
    fn test_find_declaration_range_without_semicolon_falls_back_to_pattern_length() {
        let source = "my $unterminated\nprint $unterminated\n".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let near = source.find("print $unterminated").unwrap_or(0);

        let (start, end) =
            must_some(source_utils::find_declaration_range(&provider, "$unterminated", near));

        assert_eq!(&source[start..end], "my $unterminated");
    }

    #[test]
    fn test_find_declaration_position_first_line() {
        let source = "print $x;".to_string();
        let provider = CodeActionsProvider::new(source);

        // No newline before this, so declaration position is 0
        let pos = source_utils::find_declaration_position(&provider, 6);
        assert_eq!(pos, 0);
    }

    #[test]
    fn test_find_line_end_middle_of_source() {
        let source = "line1\nline2\nline3".to_string();
        let provider = CodeActionsProvider::new(source);

        // Starting from offset 6 ("line2\n"), line end is at offset 11
        let end = source_utils::find_line_end(&provider, 6);
        assert_eq!(end, 11);
    }

    #[test]
    fn test_find_line_end_last_line_no_newline() {
        let source = "only line".to_string();
        let provider = CodeActionsProvider::new(source);

        // No newline, so line end is at source length
        let end = source_utils::find_line_end(&provider, 0);
        assert_eq!(end, 9);
    }

    #[test]
    fn test_detect_quote_char_single_quote_nearby() {
        let source = "my $x = 'hello".to_string();
        let provider = CodeActionsProvider::new(source);

        // Position 9 is inside the string; single quote at position 8
        let ch = source_utils::detect_quote_char(&provider, 9);
        assert_eq!(ch, '\'');
    }

    #[test]
    fn test_detect_quote_char_defaults_to_double() {
        let source = "my $x = hello".to_string();
        let provider = CodeActionsProvider::new(source);

        // No single quote nearby
        let ch = source_utils::detect_quote_char(&provider, 10);
        assert_eq!(ch, '"');
    }

    #[test]
    fn test_find_declaration_range_finds_my_declaration() {
        let source = "my $x = 42;\nprint $x;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        // near=18 ("$x" at offset 18 in "print $x")
        let range = source_utils::find_declaration_range(&provider, "$x", 18);
        // Should find "my $x = 42;\n" starting at offset 0, ending after semicolon+newline
        assert_eq!(range, Some((0, 12))); // "my $x = 42;\n" is 12 bytes
    }

    #[test]
    fn test_find_declaration_range_when_near_is_inside_declaration() {
        let source = "my $unused = 42;\nprint 1;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let range = source_utils::find_declaration_range(&provider, "$unused", 3);
        assert_eq!(range, Some((0, 17)));
    }

    #[test]
    fn test_find_declaration_range_uses_nearest_same_line_match() {
        let source = "my $x = 1; { my $x = 2; }\n".to_string();
        let provider = CodeActionsProvider::new(source.clone());
        let inner_decl = must_some(source.rfind("my $x"));

        let range =
            must_some(source_utils::find_declaration_range(&provider, "$x", inner_decl + 3));
        assert_eq!(range.0, inner_decl);
        assert_eq!(&provider.source()[range.0..range.1], "my $x = 2;");
    }

    #[test]
    fn test_find_declaration_range_no_declaration_returns_near() {
        let source = "print $y;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let range = source_utils::find_declaration_range(&provider, "$y", 6);
        assert_eq!(range, None);
    }

    // ── Quick-fix: PL001 / PL002 missing-semicolon via message text ─────

    #[test]
    fn test_pl001_missing_semicolon_message_triggers_fix() {
        let source = "my $x = 1\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (0, 9),
            DiagnosticSeverity::Error,
            "PL001",
            "Missing semicolon after statement. Add `;` here (found `my`)",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1, "Expected 1 action, got: {:?}", actions);
        assert_eq!(actions[0].title, "Add missing semicolon");
        assert_eq!(actions[0].kind, CodeActionKind::QuickFix);
    }

    #[test]
    fn test_pl002_missing_semicolon_message_triggers_fix() {
        let source = "my $x = 1\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (0, 9),
            DiagnosticSeverity::Error,
            "PL002",
            "Missing semicolon after statement. Add `;` here",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add missing semicolon");
    }

    #[test]
    fn test_pl001_generic_message_returns_no_semicolon_fix() {
        let source = "my $x = 1;\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (0, 9),
            DiagnosticSeverity::Error,
            "PL001",
            "Unexpected token at line 1",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert!(actions.is_empty(), "PL001 with unrelated message must not produce actions");
    }

    #[test]
    fn test_pl002_unclosed_brace_message_triggers_fix() {
        let source = "sub x { print 'ok';\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (6, 7),
            DiagnosticSeverity::Error,
            "PL002",
            "Unclosed `{` -- check for a missing `}` to close the block",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add closing brace");
        assert_eq!(actions[0].edit.new_text, "}");
    }

    #[test]
    fn test_pl001_semicolon_inserted_at_line_end() {
        // "my $x = 1\n" — semicolon should be inserted after "1" (before \n)
        let source = "my $x = 1\n".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = make_diagnostic(
            (0, 9),
            DiagnosticSeverity::Error,
            "PL001",
            "Missing semicolon after statement. Add `;` here",
        );

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        // find_line_end from diagnostic.range.1=9 -> finds '\n' at offset 0 from pos 9 -> returns 9
        assert_eq!(actions[0].edit.range, (9, 9));
        assert_eq!(actions[0].edit.new_text, ";");
    }
}
