use crate::features::diagnostics::Diagnostic;

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
            // Check if diagnostic overlaps with the requested range
            if Self::ranges_overlap(diagnostic.range, range) {
                actions.extend(self.get_actions_for_diagnostic(diagnostic));
            }
        }

        actions
    }

    /// Get code actions for a specific diagnostic
    fn get_actions_for_diagnostic(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Match on diagnostic code to provide appropriate fixes
        if let Some(code) = &diagnostic.code {
            match code.as_str() {
                "undefined-variable" | "undeclared-variable" => {
                    actions.extend(self.fix_undefined_variable(diagnostic));
                }
                "unused-variable" => {
                    actions.extend(self.fix_unused_variable(diagnostic));
                }
                "variable-shadowing" => {
                    actions.extend(self.fix_variable_shadowing(diagnostic));
                }
                "variable-redeclaration" => {
                    actions.extend(self.fix_variable_redeclaration(diagnostic));
                }
                "duplicate-parameter" => {
                    actions.extend(self.fix_duplicate_parameter(diagnostic));
                }
                "parameter-shadows-global" => {
                    actions.extend(self.fix_parameter_shadowing(diagnostic));
                }
                "unused-parameter" => {
                    actions.extend(self.fix_unused_parameter(diagnostic));
                }
                "unquoted-bareword" => {
                    actions.extend(self.fix_unquoted_bareword(diagnostic));
                }
                code if code.starts_with("parse-error-") => {
                    actions.extend(self.fix_parse_error(diagnostic, code));
                }
                _ => {}
            }
        }

        actions
    }

    /// Generate fix for undefined variable (add 'my' declaration)
    fn fix_undefined_variable(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Extract variable name from diagnostic message
        if let Some(var_name) = Self::extract_variable_name(&diagnostic.message) {
            // Find the best location to insert the declaration
            let insert_pos = self.find_declaration_position(diagnostic.range.0);

            // Generate the declaration
            let declaration = format!("my {};\n", var_name);

            actions.push(CodeAction {
                title: format!("Declare '{}' with 'my'", var_name),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit { range: (insert_pos, insert_pos), new_text: declaration },
                diagnostic_id: diagnostic.code.clone(),
            });

            // Alternative: Add 'our' for package variable
            actions.push(CodeAction {
                title: format!("Declare '{}' with 'our'", var_name),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit {
                    range: (insert_pos, insert_pos),
                    new_text: format!("our {};\n", var_name),
                },
                diagnostic_id: diagnostic.code.clone(),
            });
        }

        actions
    }

    /// Generate fix for unused variable
    fn fix_unused_variable(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        if let Some(var_name) = Self::extract_variable_name(&diagnostic.message) {
            // Option 1: Remove the declaration
            actions.push(CodeAction {
                title: format!("Remove unused variable '{}'", var_name),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit {
                    range: self.find_declaration_range(&var_name, diagnostic.range.0),
                    new_text: String::new(),
                },
                diagnostic_id: diagnostic.code.clone(),
            });

            // Option 2: Prefix with underscore to indicate intentionally unused
            let underscore_name = if let Some(stripped) = var_name.strip_prefix('$') {
                format!("$_{}", stripped)
            } else if let Some(stripped) = var_name.strip_prefix('@') {
                format!("@_{}", stripped)
            } else if let Some(stripped) = var_name.strip_prefix('%') {
                format!("%_{}", stripped)
            } else {
                format!("_{}", var_name)
            };

            actions.push(CodeAction {
                title: format!("Rename to '{}' (mark as intentionally unused)", underscore_name),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit { range: diagnostic.range, new_text: underscore_name },
                diagnostic_id: diagnostic.code.clone(),
            });
        }

        actions
    }

    /// Generate fix for variable shadowing
    fn fix_variable_shadowing(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        if let Some(var_name) = Self::extract_variable_name(&diagnostic.message) {
            // Suggest renaming the inner variable
            let base_name = var_name.trim_start_matches(['$', '@', '%']);
            let sigil = &var_name[..var_name.len() - base_name.len()];

            // Generate alternative names
            let alternatives = vec![
                format!("{}inner_{}", sigil, base_name),
                format!("{}local_{}", sigil, base_name),
                format!("{}{}_2", sigil, base_name),
            ];

            for alt_name in alternatives {
                actions.push(CodeAction {
                    title: format!("Rename shadowing variable to '{}'", alt_name),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit { range: diagnostic.range, new_text: alt_name.clone() },
                    diagnostic_id: diagnostic.code.clone(),
                });
            }
        }

        actions
    }

    /// Generate fix for variable redeclaration
    fn fix_variable_redeclaration(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Remove the 'my' keyword from the redeclaration
        let range = diagnostic.range;
        let text = &self.source[range.0..range.1];

        if text.starts_with("my ") {
            actions.push(CodeAction {
                title: "Remove redundant 'my'".to_string(),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit {
                    range: (range.0, range.0 + 3), // Remove "my "
                    new_text: String::new(),
                },
                diagnostic_id: diagnostic.code.clone(),
            });
        }

        actions
    }

    /// Generate fixes for parse errors
    fn fix_parse_error(&self, diagnostic: &Diagnostic, error_code: &str) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        match error_code {
            "parse-error-missingsemicolon" => {
                // Add semicolon at the end of the line
                let line_end = self.find_line_end(diagnostic.range.1);
                actions.push(CodeAction {
                    title: "Add missing semicolon".to_string(),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit { range: (line_end, line_end), new_text: ";".to_string() },
                    diagnostic_id: diagnostic.code.clone(),
                });
            }
            "parse-error-unclosedstring" => {
                // Add closing quote
                let quote_char = self.detect_quote_char(diagnostic.range.0);
                actions.push(CodeAction {
                    title: format!("Add closing quote '{}'", quote_char),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit {
                        range: (diagnostic.range.1, diagnostic.range.1),
                        new_text: quote_char.to_string(),
                    },
                    diagnostic_id: diagnostic.code.clone(),
                });
            }
            "parse-error-unclosedparen" => {
                // Add closing parenthesis
                actions.push(CodeAction {
                    title: "Add closing parenthesis".to_string(),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit {
                        range: (diagnostic.range.1, diagnostic.range.1),
                        new_text: ")".to_string(),
                    },
                    diagnostic_id: diagnostic.code.clone(),
                });
            }
            "parse-error-unclosedbrace" => {
                // Add closing brace
                actions.push(CodeAction {
                    title: "Add closing brace".to_string(),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit {
                        range: (diagnostic.range.1, diagnostic.range.1),
                        new_text: "}".to_string(),
                    },
                    diagnostic_id: diagnostic.code.clone(),
                });
            }
            _ => {}
        }

        actions
    }

    // Helper methods

    /// Generate fix for duplicate parameter
    fn fix_duplicate_parameter(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        if let Some(param_name) = Self::extract_variable_name(&diagnostic.message) {
            // Remove the duplicate parameter
            actions.push(CodeAction {
                title: format!("Remove duplicate parameter '{}'", param_name),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit { range: diagnostic.range, new_text: String::new() },
                diagnostic_id: diagnostic.code.clone(),
            });

            // Rename the duplicate to a different name
            let base_name = param_name.trim_start_matches(['$', '@', '%']);
            let sigil = &param_name[..param_name.len() - base_name.len()];
            let new_name = format!("{}{}_2", sigil, base_name);

            actions.push(CodeAction {
                title: format!("Rename duplicate to '{}'", new_name),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit { range: diagnostic.range, new_text: new_name },
                diagnostic_id: diagnostic.code.clone(),
            });
        }

        actions
    }

    /// Generate fix for parameter shadowing
    fn fix_parameter_shadowing(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        if let Some(param_name) = Self::extract_variable_name(&diagnostic.message) {
            let base_name = param_name.trim_start_matches(['$', '@', '%']);
            let sigil = &param_name[..param_name.len() - base_name.len()];

            // Suggest renaming the parameter
            let alternatives = vec![
                format!("{}p_{}", sigil, base_name), // p_ prefix for parameter
                format!("{}{}_param", sigil, base_name),
                format!("{}{}_arg", sigil, base_name),
            ];

            for alt_name in alternatives {
                actions.push(CodeAction {
                    title: format!("Rename parameter to '{}'", alt_name),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit { range: diagnostic.range, new_text: alt_name.clone() },
                    diagnostic_id: diagnostic.code.clone(),
                });
            }
        }

        actions
    }

    /// Generate fix for unused parameter
    fn fix_unused_parameter(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        if let Some(param_name) = Self::extract_variable_name(&diagnostic.message) {
            // Prefix with underscore to indicate intentionally unused
            let underscore_name = if let Some(stripped) = param_name.strip_prefix('$') {
                format!("$_{}", stripped)
            } else if let Some(stripped) = param_name.strip_prefix('@') {
                format!("@_{}", stripped)
            } else if let Some(stripped) = param_name.strip_prefix('%') {
                format!("%_{}", stripped)
            } else {
                format!("_{}", param_name)
            };

            actions.push(CodeAction {
                title: format!("Rename to '{}' (mark as intentionally unused)", underscore_name),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit { range: diagnostic.range, new_text: underscore_name },
                diagnostic_id: diagnostic.code.clone(),
            });

            // Add a comment to document why it's unused
            actions.push(CodeAction {
                title: "Add comment explaining unused parameter".to_string(),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit {
                    range: (diagnostic.range.0, diagnostic.range.0),
                    new_text: "# unused ".to_string(),
                },
                diagnostic_id: diagnostic.code.clone(),
            });
        }

        actions
    }

    /// Generate fix for unquoted bareword
    fn fix_unquoted_bareword(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Extract bareword from diagnostic message
        if let Some(start) = diagnostic.message.find('\'') {
            if let Some(end) = diagnostic.message[start + 1..].find('\'') {
                let bareword = &diagnostic.message[start + 1..start + 1 + end];

                // Quote with single quotes
                actions.push(CodeAction {
                    title: format!("Quote bareword as '{}'", bareword),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit { range: diagnostic.range, new_text: format!("'{}'", bareword) },
                    diagnostic_id: diagnostic.code.clone(),
                });

                // Quote with double quotes
                actions.push(CodeAction {
                    title: format!("Quote bareword as \"{}\"", bareword),
                    kind: CodeActionKind::QuickFix,
                    edit: TextEdit {
                        range: diagnostic.range,
                        new_text: format!("\"{}\"", bareword),
                    },
                    diagnostic_id: diagnostic.code.clone(),
                });

                // If it looks like a filehandle, suggest declaring it
                if bareword.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    let insert_pos = self.find_declaration_position(diagnostic.range.0);
                    actions.push(CodeAction {
                        title: format!("Declare {} as filehandle", bareword),
                        kind: CodeActionKind::QuickFix,
                        edit: TextEdit {
                            range: (insert_pos, insert_pos),
                            new_text: format!(
                                "open my ${}, '<', 'filename.txt' or die $!;\n",
                                bareword.to_lowercase()
                            ),
                        },
                        diagnostic_id: diagnostic.code.clone(),
                    });
                }
            }
        }

        actions
    }

    /// Check if two ranges overlap
    fn ranges_overlap(r1: (usize, usize), r2: (usize, usize)) -> bool {
        r1.0 < r2.1 && r2.0 < r1.1
    }

    /// Extract variable name from diagnostic message
    fn extract_variable_name(message: &str) -> Option<String> {
        // Look for patterns like "Variable '$foo' is undefined"
        if let Some(start) = message.find('\'') {
            if let Some(end) = message[start + 1..].find('\'') {
                return Some(message[start + 1..start + 1 + end].to_string());
            }
        }
        // Also try with backticks
        if let Some(start) = message.find('`') {
            if let Some(end) = message[start + 1..].find('`') {
                return Some(message[start + 1..start + 1 + end].to_string());
            }
        }
        None
    }

    /// Find the best position to insert a variable declaration
    fn find_declaration_position(&self, near: usize) -> usize {
        // Find the start of the current line
        let line_start = self.source[..near].rfind('\n').map(|i| i + 1).unwrap_or(0);

        // Check if we're after 'use strict' or 'use warnings'
        let before_line = if line_start > 0 {
            let prev_line_start =
                self.source[..line_start - 1].rfind('\n').map(|i| i + 1).unwrap_or(0);
            &self.source[prev_line_start..line_start]
        } else {
            ""
        };

        // If previous line is 'use' statement, insert after it
        if before_line.trim_start().starts_with("use ") {
            line_start
        } else {
            // Otherwise, insert at the beginning of the current line
            line_start
        }
    }

    /// Find the range of a variable declaration
    fn find_declaration_range(&self, var_name: &str, near: usize) -> (usize, usize) {
        // Simple search for "my $var" pattern
        let search_pattern = format!("my {}", var_name);

        // Search backwards from the diagnostic position
        if let Some(pos) = self.source[..near].rfind(&search_pattern) {
            // Find the end of the declaration (usually semicolon + newline)
            let end = self.source[pos..]
                .find(';')
                .map(|i| {
                    let semi_pos = pos + i + 1;
                    // Include the newline if present
                    if semi_pos < self.source.len() && self.source.as_bytes()[semi_pos] == b'\n' {
                        semi_pos + 1
                    } else {
                        semi_pos
                    }
                })
                .unwrap_or(pos + search_pattern.len());

            return (pos, end);
        }

        // Fallback to the diagnostic range
        (near, near)
    }

    /// Find the end of the current line
    fn find_line_end(&self, pos: usize) -> usize {
        self.source[pos..].find('\n').map(|i| pos + i).unwrap_or(self.source.len())
    }

    /// Detect the quote character used at the given position
    fn detect_quote_char(&self, pos: usize) -> char {
        // Look for the opening quote before the position
        let before = &self.source[pos.saturating_sub(10)..pos];
        if before.contains('\'') {
            '\''
        } else {
            '"' // Default to double quote
        }
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
            range: (18, 20), // Position of $x
            severity: DiagnosticSeverity::Error,
            code: Some("undefined-variable".to_string()),
            message: "Variable '$x' is undefined".to_string(),
            related_information: vec![],
            tags: vec![],
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
            range: (3, 10), // Position of $unused
            severity: DiagnosticSeverity::Warning,
            code: Some("unused-variable".to_string()),
            message: "Variable '$unused' is declared but never used".to_string(),
            related_information: vec![],
            tags: vec![],
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
            range: (13, 14), // End of first line
            severity: DiagnosticSeverity::Error,
            code: Some("parse-error-missingsemicolon".to_string()),
            message: "Missing semicolon".to_string(),
            related_information: vec![],
            tags: vec![],
        };

        let actions = provider.get_actions_for_diagnostic(&diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add missing semicolon");
    }
}
