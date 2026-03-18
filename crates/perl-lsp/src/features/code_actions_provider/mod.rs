use crate::features::diagnostics::Diagnostic;

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
            Some("undefined-variable" | "undeclared-variable") => {
                fixes::fix_undefined_variable(self, diagnostic)
            }
            Some("unused-variable") => fixes::fix_unused_variable(self, diagnostic),
            Some("variable-shadowing") => fixes::fix_variable_shadowing(diagnostic),
            Some("variable-redeclaration") => fixes::fix_variable_redeclaration(self, diagnostic),
            Some("duplicate-parameter") => fixes::fix_duplicate_parameter(diagnostic),
            Some("parameter-shadows-global") => fixes::fix_parameter_shadowing(diagnostic),
            Some("unused-parameter") => fixes::fix_unused_parameter(diagnostic),
            Some("unquoted-bareword") => fixes::fix_unquoted_bareword(self, diagnostic),
            Some(code) if code.starts_with("parse-error-") => {
                fixes::fix_parse_error(self, diagnostic, code)
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

    #[test]
    fn test_undefined_variable_fix() {
        let source = "use strict;\nprint $x;".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = Diagnostic {
            range: (18, 20),
            severity: DiagnosticSeverity::Error,
            code: Some("undefined-variable".to_string()),
            message: "Variable '$x' is undefined".to_string(),
            related_information: vec![],
            tags: vec![],
            suggestion: None,
        };

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].title, "Declare '$x' with 'my'");
        assert_eq!(actions[1].title, "Declare '$x' with 'our'");
    }

    #[test]
    fn test_unused_variable_fix() {
        let source = "my $unused = 42;".to_string();
        let provider = CodeActionsProvider::new(source);

        let diagnostic = Diagnostic {
            range: (3, 10),
            severity: DiagnosticSeverity::Warning,
            code: Some("unused-variable".to_string()),
            message: "Variable '$unused' is declared but never used".to_string(),
            related_information: vec![],
            tags: vec![],
            suggestion: None,
        };

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 2);
        assert!(actions[0].title.contains("Remove"));
        assert!(actions[1].title.contains("$_unused"));
    }

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
}
