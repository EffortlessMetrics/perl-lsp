use crate::features::diagnostics::Diagnostic;
use perl_diagnostics_codes::DiagnosticCode;

mod fixes;
mod source_utils;

/// Represents a code action (quick-fix) that can be applied to resolve a diagnostic
///
/// Code actions provide automated fixes and refactoring operations for Perl code.
#[derive(Debug, Clone)]
pub struct CodeAction {
    /// Human-readable title describing the action
    pub title: String,
    /// The kind/category of code action
    pub kind: CodeActionKind,
    /// The text edit to apply
    pub edit: TextEdit,
    /// ID of the diagnostic this action fixes
    pub diagnostic_id: Option<String>,
    /// Exact diagnostic range this action was derived from
    pub diagnostic_range: Option<(usize, usize)>,
}

/// Kind of code action
///
/// Categorizes the type of code action to help editors organize actions.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic issue
    QuickFix,
    /// General refactoring operation
    Refactor,
    /// Extract code into a new construct
    RefactorExtract,
    /// Inline a construct into its usage sites
    RefactorInline,
    /// Rewrite code using a different pattern
    RefactorRewrite,
}

/// Text edit operation
///
/// Represents a change to be made to source code.
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// The range of text to replace (start, end)
    pub range: (usize, usize),
    /// The new text to insert
    pub new_text: String,
}

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
        match diagnostic.code.as_deref() {
            Some(c)
                if c == DiagnosticCode::UndefinedVariable.as_str()
                    || c == "undefined-variable"
                    || c == "undeclared-variable" =>
            {
                fixes::fix_undefined_variable(self, diagnostic)
            }
            Some(c) if c == DiagnosticCode::UnusedVariable.as_str() || c == "unused-variable" => {
                fixes::fix_unused_variable(self, diagnostic)
            }
            Some(c)
                if c == DiagnosticCode::VariableShadowing.as_str() || c == "variable-shadowing" =>
            {
                fixes::fix_variable_shadowing(diagnostic)
            }
            Some(c)
                if c == DiagnosticCode::VariableRedeclaration.as_str()
                    || c == "variable-redeclaration" =>
            {
                fixes::fix_variable_redeclaration(self, diagnostic)
            }
            Some(c)
                if c == DiagnosticCode::DuplicateParameter.as_str()
                    || c == "duplicate-parameter" =>
            {
                fixes::fix_duplicate_parameter(diagnostic)
            }
            Some(c)
                if c == DiagnosticCode::ParameterShadowsGlobal.as_str()
                    || c == "parameter-shadows-global" =>
            {
                fixes::fix_parameter_shadowing(diagnostic)
            }
            Some(c) if c == DiagnosticCode::UnusedParameter.as_str() || c == "unused-parameter" => {
                fixes::fix_unused_parameter(diagnostic)
            }
            Some(c)
                if c == DiagnosticCode::UnquotedBareword.as_str() || c == "unquoted-bareword" =>
            {
                fixes::fix_unquoted_bareword(self, diagnostic)
            }
            Some(code) if code.starts_with("parse-error-") => {
                fixes::fix_parse_error(self, diagnostic, code)
            }
            // PL001 / PL002 are general parse error codes. When the diagnostic message
            // indicates a missing semicolon, route through the same fix as
            // "parse-error-missingsemicolon" so the quick-fix fires correctly.
            Some("PL001") | Some("PL002")
                if diagnostic.message.to_ascii_lowercase().contains("missing semicolon") =>
            {
                fixes::fix_parse_error(self, diagnostic, "parse-error-missingsemicolon")
            }
            Some(code) if code.starts_with("Perl::Critic::Policy::") => {
                fixes::fix_perlcritic_policy(self, diagnostic, code)
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn source(&self) -> &str {
        &self.source
    }
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

    #[test]
    fn test_perlcritic_require_use_strict_fix() {
        let source = "print 'hello';\n".to_string();
        let provider = CodeActionsProvider::new(source);
        let diagnostic = make_diagnostic(
            (0, 5),
            DiagnosticSeverity::Warning,
            "Perl::Critic::Policy::TestingAndDebugging::RequireUseStrict",
            "Code does not use strict",
        );
        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add `use strict;` (Perl::Critic)");
        assert_eq!(actions[0].edit.new_text, "use strict;\n");
    }

    #[test]
    fn test_perlcritic_require_use_warnings_fix() {
        let source = "use strict;\nprint 'hello';\n".to_string();
        let provider = CodeActionsProvider::new(source);
        let diagnostic = make_diagnostic(
            (12, 17),
            DiagnosticSeverity::Warning,
            "Perl::Critic::Policy::TestingAndDebugging::RequireUseWarnings",
            "Code does not use warnings",
        );
        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add `use warnings;` (Perl::Critic)");
        assert_eq!(actions[0].edit.new_text, "use warnings;\n");
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

    // ── Quick-fix: variable shadowing ───────────────────────────────────

    #[test]
    fn test_variable_shadowing_fix_offers_three_alternatives() {
        let diagnostic = make_diagnostic(
            (20, 24),
            DiagnosticSeverity::Warning,
            "variable-shadowing",
            "Variable '$foo' shadows outer variable",
        );

        let provider = CodeActionsProvider::new(String::new());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].title, "Rename shadowing variable to '$inner_foo'");
        assert_eq!(actions[1].title, "Rename shadowing variable to '$local_foo'");
        assert_eq!(actions[2].title, "Rename shadowing variable to '$foo_2'");
    }

    #[test]
    fn test_variable_shadowing_fix_preserves_sigil() {
        let diagnostic = make_diagnostic(
            (10, 15),
            DiagnosticSeverity::Warning,
            "variable-shadowing",
            "Variable '@items' shadows outer variable",
        );

        let provider = CodeActionsProvider::new(String::new());
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

        let provider = CodeActionsProvider::new(String::new());
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

        let provider = CodeActionsProvider::new(String::new());
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

        let provider = CodeActionsProvider::new(String::new());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions[1].edit.new_text, "@vals_2");
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

        let provider = CodeActionsProvider::new(String::new());
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

        let provider = CodeActionsProvider::new(String::new());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions[0].edit.new_text, "%p_opts");
        assert_eq!(actions[1].edit.new_text, "%opts_param");
        assert_eq!(actions[2].edit.new_text, "%opts_arg");
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

        let provider = CodeActionsProvider::new(String::new());
        let actions = provider.get_actions_for_diagnostic(&diagnostic);

        assert_eq!(actions.len(), 1);
        assert!(actions[0].title.contains("$_self"));
        assert!(actions[0].title.contains("mark as intentionally unused"));
    }

    #[test]
    fn test_unused_parameter_rename_stays_within_parameter_range() {
        let diagnostic = make_diagnostic(
            (20, 25),
            DiagnosticSeverity::Warning,
            "unused-parameter",
            "Parameter '$ctx' is unused",
        );

        let provider = CodeActionsProvider::new(String::new());
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
