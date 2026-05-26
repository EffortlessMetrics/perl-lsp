//! Code actions and quick fixes for Perl
//!
//! This module provides automated fixes for common issues and refactoring actions.
//!
//! # LSP Workflow Integration
//!
//! Code actions integrate with the Parse → Index → Navigate → Complete → Analyze workflow:
//!
//! - **Parse**: AST analysis identifies code patterns requiring fixes or refactoring
//! - **Index**: Symbol tables provide context for variable and function renaming actions
//! - **Navigate**: Cross-file analysis enables workspace-wide refactoring operations
//! - **Complete**: Code action suggestions are refined based on completion context
//! - **Analyze**: Diagnostic analysis drives automated fix generation and prioritization
//!
//! This integration ensures code actions are contextually appropriate and maintain
//! code correctness across the entire Perl workspace.
//!
//! # LSP Client Capabilities
//!
//! Requires client support for `textDocument/codeAction` capabilities and
//! `workspace/workspaceEdit` to apply edits across files.
//!
//! # Protocol Compliance
//!
//! Implements LSP code action protocol semantics (LSP 3.17+) including
//! range-based requests, diagnostic filtering, and edit application rules.
//!
//! # Performance Characteristics
//!
//! - **Action generation**: <50ms for typical code action requests
//! - **Edit application**: <100ms for complex workspace refactoring
//! - **Memory usage**: <5MB for action metadata and edit operations
//! - **Incremental analysis**: Leverages ≤1ms parsing SLO for real-time suggestions
//!
//! # Related Modules
//!
//! This module integrates with diagnostics and import optimization modules
//! for import-related code actions.
//!
//! # See also
//!
//! - [`DiagnosticsProvider`](crate::ide::lsp_compat::diagnostics::DiagnosticsProvider)
//! - [`crate::ide::lsp_compat::references`]
//!
//! # Usage Examples
//!
//! ```ignore
//! use perl_lsp_providers::ide::lsp_compat::code_actions::{CodeActionsProvider, CodeActionKind};
//! use perl_lsp_providers::ide::lsp_compat::diagnostics::Diagnostic;
//! use perl_parser_core::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "my $unused_var = 42;";
//! let provider = CodeActionsProvider::new(code.to_string());
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//! let diagnostics = vec![]; // Would contain actual diagnostics
//!
//! // Generate code actions for diagnostics
//! let actions = provider.get_code_actions(&ast, (0, code.len()), &diagnostics);
//! for action in actions {
//!     println!("Available action: {} ({:?})", action.title, action.kind);
//! }
//! # Ok(())
//! # }
//! ```

use super::{diagnostic_routes, source_actions};

pub use super::types::{CodeAction, CodeActionKind};

use crate::providers::diagnostics::Diagnostic;
use perl_parser_core::Node;

/// Code actions provider
///
/// Analyzes Perl source code and provides automated fixes and refactoring
/// actions for common issues and improvement opportunities.
pub struct CodeActionsProvider {
    source: String,
}

impl CodeActionsProvider {
    /// Create a new code actions provider
    pub fn new(source: String) -> Self {
        Self { source }
    }

    /// Get code actions for a range
    pub fn get_code_actions(
        &self,
        ast: &Node,
        range: (usize, usize),
        diagnostics: &[Diagnostic],
    ) -> Vec<CodeAction> {
        let mut actions = diagnostic_routes::quick_fixes_for_diagnostics(&self.source, diagnostics);

        actions.extend(source_actions::get_source_actions(&self.source, range));
        actions.extend(super::refactors::get_refactoring_actions(&self.source, ast, range));
        actions.extend(super::modernize::get_modernize_actions(&self.source));

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::diagnostics::DiagnosticSeverity;
    use perl_parser_core::Parser;
    use perl_tdd_support::{must, must_some};

    /// Create a diagnostic with byte offsets
    fn make_diagnostic(start: usize, end: usize, code: &str, msg: &str) -> Diagnostic {
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

    fn apply_action(source: &str, action: &CodeAction) -> String {
        let mut edits = action.edit.changes.clone();
        edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));

        let mut output = source.to_string();
        for edit in edits {
            output.replace_range(edit.location.start..edit.location.end, &edit.new_text);
        }
        output
    }

    #[test]
    fn test_undefined_variable_fix() {
        let source = "use strict;\nprint $undefined;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        // Create a synthetic diagnostic for undefined-variable (stable code PL103)
        // "$undefined" starts at byte offset 18 (after "use strict;\nprint ")
        let diagnostics = vec![make_diagnostic(
            18, // start of "$undefined"
            28, // end of "$undefined"
            "PL103",
            "Undefined variable '$undefined'",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title.contains("Declare") || a.title.contains("my")),
            "Expected action to declare variable, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_assignment_in_condition_fix() {
        let source = "if ($x = 5) { }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        // Create a synthetic diagnostic for assignment-in-condition (stable code PL403)
        // "$x = 5" is at bytes 4-10
        let diagnostics = vec![make_diagnostic(
            4,  // start of "$x = 5"
            10, // end of "$x = 5"
            "PL403",
            "Assignment in condition",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title.contains("==")),
            "Expected action to change to comparison, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_critic_policy_alias_for_assignment_in_condition() {
        let source = "if ($x = 5) { }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            10,
            "native.common.assignment_in_condition",
            "Assignment in condition - did you mean '=='?",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Change to comparison (==)"
                && a.edit.changes.iter().any(|edit| edit.new_text == "==")),
            "Expected native critic alias to offer comparison fix, got: {:?}",
            actions
        );
        assert!(
            actions.iter().any(|a| a.title == "Keep assignment (add parentheses)"),
            "Expected native critic alias to offer intentional-assignment fix, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_unreachable_code_alias_produces_quick_fix() {
        let source = "sub f {\nreturn 1;\nmy $dead = 2;\n}\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.find("my $dead"));
        let end = start + "my $dead = 2;".len();
        let diagnostics = vec![make_diagnostic(
            start,
            end,
            "native.common.unreachable_code",
            "Unreachable code: this statement cannot be executed",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|action| action.title == "Remove unreachable code"
                && action.edit.changes.iter().any(|edit| edit.new_text.is_empty()
                    && &source[edit.location.start..edit.location.end] == "my $dead = 2;\n")),
            "Expected native unreachable-code alias to remove dead line, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_deprecated_defined_alias_produces_quick_fix() {
        let source = "if (defined @items) { print @items; }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            18,
            "native.common.deprecated_defined",
            "Use of 'defined @items' is deprecated",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Replace with '@items'"
                && a.edit.changes.iter().any(|edit| edit.new_text == "@items")),
            "Expected native deprecated-defined alias to offer defined() removal, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_deprecated_defined_alias_normalizes_parenthesized_quick_fix() {
        let source = "if (defined(%seen)) { print keys %seen; }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            18,
            "native.common.deprecated_defined",
            "Use of 'defined %seen' is deprecated",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Replace with '%seen'"
                && a.edit.changes.iter().any(|edit| edit.new_text == "%seen")),
            "Expected native deprecated-defined alias to normalize parenthesized defined() removal, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_undef_comparison_alias_produces_defined_quick_fix() {
        let source = "if ($value == undef) { print $value; }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            19,
            "native.common.undef_comparison",
            "Using '==' with undef -- use defined() to check first",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Use defined() check"
                && a.edit.changes.iter().any(|edit| edit.new_text == "!defined($value)")),
            "Expected native undef-comparison alias to offer defined() fix, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_hardcoded_shebang_suggests_portable() {
        let source = "#!/usr/bin/perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1, "Expected one shebang action");
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl");
    }

    #[test]
    fn test_hardcoded_shebang_preserves_flags() {
        let source = "#!/usr/bin/perl -w\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1);
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl -w");
    }

    #[test]
    fn test_env_perl_shebang_not_flagged() {
        let source = "#!/usr/bin/env perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert!(shebang_actions.is_empty(), "env perl should not be flagged");
    }

    #[test]
    fn test_no_shebang_not_flagged() {
        let source = "use strict;\nuse warnings;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert!(shebang_actions.is_empty(), "No shebang should not be flagged");
    }

    #[test]
    fn test_local_bin_perl_shebang() {
        let source = "#!/usr/local/bin/perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1, "Local bin perl should be flagged");
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl");
    }

    #[test]
    fn test_shebang_with_taint_flag() {
        let source = "#!/usr/bin/perl -T\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1);
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl -T");
    }

    #[test]
    fn test_bash_shebang_not_flagged() {
        let source = "#!/bin/bash\necho hello\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert!(shebang_actions.is_empty(), "Non-perl shebang should not be flagged");
    }

    #[test]
    fn test_shebang_fix_not_suggested_when_range_starts_after_first_line() {
        let source = "#!/usr/bin/perl\nmy $x = 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];
        let range_start = must_some(source.find("my $x"));

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (range_start, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();
        assert!(
            shebang_actions.is_empty(),
            "Shebang fix should only appear when requested range includes line 1"
        );
    }

    #[test]
    fn test_perlcritic_policy_aliases_produce_quick_fixes() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("InputOutput::ProhibitBarewordFileHandles".to_string()),
                message: "Bareword filehandle 'FH'".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("InputOutput::RequireThreeArgOpen".to_string()),
                message: "Use 3-arg open".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        assert!(actions.iter().any(|a| a.title.contains("bareword filehandle")));
        assert!(actions.iter().any(|a| a.title.contains("three-argument open() for safety")));
    }

    #[test]
    fn test_fully_qualified_perlcritic_policy_aliases_produce_quick_fixes() {
        let source = "open FH, $path;
";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some(
                    "Perl::Critic::Policy::InputOutput::ProhibitBarewordFileHandles".to_string(),
                ),
                message: "Bareword filehandle 'FH'".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("Perl::Critic::Policy::InputOutput::RequireThreeArgOpen".to_string()),
                message: "Use 3-arg open".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        assert!(actions.iter().any(|a| a.title.contains("bareword filehandle")));
        assert!(actions.iter().any(|a| a.title.contains("three-argument open() for safety")));
    }

    #[test]
    fn test_perlcritic_require_brief_open_alias_produces_quick_fix() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (0, 4),
            severity: DiagnosticSeverity::Warning,
            code: Some("InputOutput::RequireBriefOpen".to_string()),
            message: "Use 3-arg open".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title.contains("three-argument open() for safety")));
    }

    #[test]
    fn test_native_bareword_filehandle_alias_produces_quick_fix() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (5, 7),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.io.bareword_filehandle".to_string()),
            message: "Bareword filehandle 'FH' should be lexical".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let fix =
            must_some(actions.iter().find(|action| action.title.contains("bareword filehandle")));
        assert_eq!(fix.edit.changes[0].new_text, "my $fh_fh");
    }

    #[test]
    fn test_native_two_arg_open_alias_produces_quick_fix() {
        let source = "open(my $fh, $path);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (0, 19),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.io.two_arg_open".to_string()),
            message: "Two-argument open should use an explicit mode".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let fix =
            must_some(actions.iter().find(|action| action.title.contains("three-argument open()")));
        assert_eq!(fix.edit.changes[0].new_text, "open(my $fh, '<', $path)");
    }

    #[test]
    fn test_legacy_two_arg_open_alias_range_only_open_edits_whole_call() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (0, 4),
            severity: DiagnosticSeverity::Warning,
            code: Some("InputOutput::RequireThreeArgOpen".to_string()),
            message: "Two-argument open should use an explicit mode".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let fix =
            must_some(actions.iter().find(|action| action.title.contains("three-argument open()")));
        assert_eq!(fix.edit.changes[0].location.start, 0);
        assert_eq!(fix.edit.changes[0].location.end, "open FH, $path".len());
        assert_eq!(fix.edit.changes[0].new_text, "open(FH, '<', $path)");
    }

    #[test]
    fn test_legacy_two_arg_open_alias_range_only_open_rejects_ambiguous_line_fallback() {
        for source in ["open FH, $path; # legacy\n", "open FH, $path; close FH;\n"] {
            let mut parser = Parser::new(source);
            let ast = must(parser.parse());
            let diagnostics = vec![Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("InputOutput::RequireThreeArgOpen".to_string()),
                message: "Two-argument open should use an explicit mode".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            }];

            let provider = CodeActionsProvider::new(source.to_string());
            let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

            assert!(
                actions.iter().all(|action| !action.title.contains("three-argument open()")),
                "ambiguous fallback should not produce a two-arg open fix for {source:?}"
            );
        }
    }

    #[test]
    fn test_perlcritic_policy_aliases_for_strict_warnings_and_unused_variable() {
        let source = "print $unused;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, source.len()),
                severity: DiagnosticSeverity::Warning,
                code: Some("TestingAndDebugging::RequireUseStrict".to_string()),
                message: "Code does not use strict".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, source.len()),
                severity: DiagnosticSeverity::Warning,
                code: Some("TestingAndDebugging::RequireUseWarnings".to_string()),
                message: "Code does not use warnings".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (6, 13),
                severity: DiagnosticSeverity::Warning,
                code: Some("Variables::ProhibitUnusedVariables".to_string()),
                message: "Unused variable '$unused'".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title == "Add 'use strict'"));
        assert!(actions.iter().any(|a| a.title == "Add 'use warnings'"));
        assert!(actions.iter().any(|a| a.title.contains("Remove unused variable")));
    }

    #[test]
    fn test_native_critic_policy_aliases_for_strict_and_warnings() {
        let source = "print 'hello';\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Warning,
                code: Some("native.testing.require_use_strict".to_string()),
                message: "Code does not use strict".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Warning,
                code: Some("native.testing.require_use_warnings".to_string()),
                message: "Code does not use warnings".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title == "Add 'use strict'"));
        assert!(actions.iter().any(|a| a.title == "Add 'use warnings'"));
    }

    #[test]
    fn test_native_critic_policy_alias_for_unused_lexical() {
        let source = "use strict;\nuse warnings;\nmy $unused = 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.find("$unused"));
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$unused".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.unused_lexical".to_string()),
            message: "Lexical variable '$unused' is declared but never used".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title == "Remove unused variable"));
        assert!(actions.iter().any(|a| {
            a.title == "Rename to '$_unused'"
                && a.edit.changes.iter().any(|edit| edit.new_text == "$_unused")
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_unused_parameter() {
        let source = "use strict;\nuse warnings;\nsub helper($used, $unused) { return $used; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.find("$unused"));
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$unused".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.unused_parameter".to_string()),
            message: "Parameter '$unused' is never used".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Rename to '_$unused'"
                && action.edit.changes.iter().any(|edit| {
                    edit.location.start == start
                        && edit.location.end == start + "$unused".len()
                        && edit.new_text == "_$unused"
                })
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_duplicate_parameter() {
        let source = "use strict;\nuse warnings;\nsub helper($arg, $arg) { return $arg; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.find(", $arg")) + ", ".len();
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$arg".len()),
            severity: DiagnosticSeverity::Error,
            code: Some("native.variables.duplicate_parameter".to_string()),
            message: "Parameter '$arg' appears more than once in this signature".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Remove duplicate parameter '$arg'"
                && action.edit.changes.iter().any(|edit| {
                    edit.location.start == start
                        && edit.location.end == start + "$arg".len()
                        && edit.new_text.is_empty()
                })
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Rename duplicate to '$arg_2'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$arg_2")
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_parameter_shadows_global() {
        let source = "use strict;\nuse warnings;\nmy $name = 'outer';\nsub helper($name) { return $name; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.find("($name")) + 1;
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$name".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.parameter_shadows_global".to_string()),
            message: "Parameter '$name' shadows an outer declaration".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Rename parameter to '$p_name'"
                && action.edit.changes.iter().any(|edit| {
                    edit.location.start == start
                        && edit.location.end == start + "$name".len()
                        && edit.new_text == "$p_name"
                })
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Rename parameter to '$name_param'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$name_param")
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_duplicate_lexical() {
        let source = "use strict;\nuse warnings;\nmy $dup = 1;\nmy $dup = 2;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.rfind("$dup"));
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$dup".len()),
            severity: DiagnosticSeverity::Error,
            code: Some("native.variables.duplicate_lexical".to_string()),
            message: "Lexical variable '$dup' is declared more than once in the same scope"
                .to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let action = must_some(
            actions.iter().find(|action| action.title == "Remove duplicate 'my' declaration"),
        );
        assert!(action.edit.changes.iter().any(|edit| {
            edit.location.start == must_some(source.rfind("my $dup"))
                && edit.location.end == start
                && edit.new_text.is_empty()
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_shadowed_lexical() {
        let source = "use strict;\nuse warnings;\nmy $value = 1;\n{ my $value = 2; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.rfind("$value"));
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$value".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.shadowed_lexical".to_string()),
            message: "Lexical variable '$value' shadows an outer declaration".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Rename to '$value_inner'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$value_inner")
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Rename to '$value_local'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$value_local")
        }));
    }

    #[test]
    fn test_phase_scoped_strict_quick_fix_moves_pragma_to_file_scope() {
        let source = "BEGIN { use strict; }\nmy $x = 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.find("use strict;"));
        let end = start + "use strict;".len();
        let diagnostics = vec![make_diagnostic(
            start,
            end,
            "PL502",
            "`use strict` inside a BEGIN block does not enable strict for the rest of the file",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        let action = must_some(
            actions.iter().find(|action| action.title == "Move 'use strict' to file scope"),
        );

        let rewritten = apply_action(source, action);
        assert!(rewritten.starts_with("use strict;\nBEGIN { "));
        assert!(rewritten.contains("BEGIN {  }"));
    }

    #[test]
    fn test_phase_scoped_warnings_quick_fix_preserves_shebang() {
        let source = "#!/usr/bin/perl\nBEGIN { use warnings; }\nprint 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = must_some(source.find("use warnings;"));
        let end = start + "use warnings;".len();
        let diagnostics = vec![make_diagnostic(
            start,
            end,
            "PL503",
            "`use warnings` inside a BEGIN block does not enable warnings for the rest of the file",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        let action = must_some(
            actions.iter().find(|action| action.title == "Move 'use warnings' to file scope"),
        );

        let rewritten = apply_action(source, action);
        assert!(rewritten.starts_with("#!/usr/bin/perl\nuse warnings;\n"));
        assert!(rewritten.contains("BEGIN {  }"));
    }

    #[test]
    fn test_parse_error_code_variants_route_to_same_quick_fix() {
        let source = "my $x =\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        for code in ["PL001", "PL002", "parse-error-missing-expression"] {
            let diagnostics = vec![make_diagnostic(
                source.len() - 1,
                source.len(),
                code,
                "Parse error near newline",
            )];
            let provider = CodeActionsProvider::new(source.to_string());
            let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
            assert!(
                !actions.is_empty(),
                "Expected parse error code {code} to produce at least one quick fix"
            );
        }
    }

    #[test]
    fn test_pl408_duplicate_hash_key_rename_action() {
        // PL408: duplicate hash key 'host' on a multiline hash offers rename and delete.
        let source = "my %cfg = (\n    host => 'db1',\n    host => 'db2',\n);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        // Second 'host' key.
        let dup_start = must_some(source.rfind("host"));
        let dup_end = dup_start + "host".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'host' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let rename = must_some(
            actions.iter().find(|a| a.title.contains("Rename") && a.title.contains("host")),
        );
        assert_eq!(rename.edit.changes[0].new_text, "host_2");
        assert_eq!(rename.edit.changes[0].location.start, dup_start);
        assert_eq!(rename.edit.changes[0].location.end, dup_end);
    }

    #[test]
    fn test_pl408_duplicate_hash_key_delete_preferred_for_multiline() {
        // PL408: delete action is preferred and removes only the duplicate line.
        let source = "my %cfg = (\n    host => 'db1',\n    host => 'db2',\n);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let dup_start = must_some(source.rfind("host"));
        let dup_end = dup_start + "host".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'host' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let delete = must_some(
            actions.iter().find(|a| a.title.contains("Remove") && a.title.contains("host")),
        );
        assert!(delete.is_preferred, "remove action should be preferred");

        let rewritten = apply_action(source, delete);
        assert_eq!(rewritten, "my %cfg = (\n    host => 'db1',\n);\n");
    }

    #[test]
    fn test_pl408_inline_hash_only_rename_no_delete() {
        // PL408: inline hash suppresses delete; only rename is offered.
        let source = "my %h = (foo => 1, foo => 2);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let dup_start = must_some(source.rfind("foo"));
        let dup_end = dup_start + "foo".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'foo' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            !actions.iter().any(|a| a.title.contains("Remove")),
            "should not offer delete for inline hash"
        );
        assert!(
            actions.iter().any(|a| a.title.contains("Rename") && a.title.contains("foo")),
            "should still offer rename for inline hash"
        );
    }

    #[test]
    fn test_pl408_single_quoted_key_rename_preserves_quotes() {
        let source = "my %h = (\n    'key' => 1,\n    'key' => 2,\n);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let dup_start = must_some(source.rfind("'key'"));
        let dup_end = dup_start + "'key'".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'key' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let rename = must_some(actions.iter().find(|a| a.title.contains("Rename")));
        assert_eq!(rename.edit.changes[0].new_text, "'key_2'");
    }

    #[test]
    fn test_native_printf_format_arity_listop_inserts_undef() {
        // Route: native.common.printf_format_arity → fix_printf_format_arity
        // Listop form: printf "fmt", $a — insert undef before the semicolon.
        let source = "use strict;\nuse warnings;\nprintf \"%s %s\", $name;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let call_start = must_some(source.find("printf"));
        // Call node range ends before the ';'
        let call_end = call_start + "printf \"%s %s\", $name".len();
        let diagnostics = vec![make_diagnostic(
            call_start,
            call_end,
            "native.common.printf_format_arity",
            "`printf` format string has 2 specifiers but 1 argument supplied",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let action =
            must_some(actions.iter().find(|a| a.title == "Add 1 missing argument as undef"));
        let rewritten = apply_action(source, action);
        assert!(
            rewritten.contains("printf \"%s %s\", $name, undef;"),
            "expected undef appended before semicolon, got: {rewritten:?}",
        );
    }

    #[test]
    fn test_pl405_parens_printf_inserts_multiple_undef() {
        // Route: PL405 → fix_printf_format_arity
        // Parens form: printf("fmt", $a) — insert undef before closing ')'.
        let source = "use strict;\nuse warnings;\nprintf(\"%s %s %s\", $a);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let call_start = must_some(source.find("printf("));
        // Call node includes the closing ')'
        let call_end = call_start + "printf(\"%s %s %s\", $a)".len();
        let diagnostics = vec![make_diagnostic(
            call_start,
            call_end,
            "PL405",
            "`printf` format string has 3 specifiers but 1 argument supplied",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let action =
            must_some(actions.iter().find(|a| a.title == "Add 2 missing arguments as undef"));
        let rewritten = apply_action(source, action);
        assert!(
            rewritten.contains("printf(\"%s %s %s\", $a, undef, undef);"),
            "expected two undef args inserted before closing paren, got: {rewritten:?}",
        );
    }

    #[test]
    fn test_pl405_too_many_args_no_quick_fix() {
        // When args > specifiers the fix is skipped (removing args is too destructive).
        let source = "use strict;\nuse warnings;\nprintf \"%s\", $a, $b;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let call_start = must_some(source.find("printf"));
        let call_end = call_start + "printf \"%s\", $a, $b".len();
        let diagnostics = vec![make_diagnostic(
            call_start,
            call_end,
            "native.common.printf_format_arity",
            "`printf` format string has 1 specifier but 2 arguments supplied",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            !actions.iter().any(|a| a.title.contains("missing argument")),
            "should not offer undef insertion when args exceed specifiers, got: {actions:?}",
        );
    }
}
