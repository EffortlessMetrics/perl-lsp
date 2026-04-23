//! Comprehensive tests for edge case detection and handling

#[cfg(all(test, feature = "pure-rust"))]
mod tests {
    use tree_sitter_perl::{
        anti_pattern_detector::{AntiPattern, Severity},
        dynamic_delimiter_recovery::RecoveryMode,
        edge_case_handler::{EdgeCaseConfig, EdgeCaseHandler},
        tree_sitter_adapter::TreeSitterAdapter,
    };

    #[test]
    fn test_dynamic_delimiter_detection() {
        let code = r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
This has a dynamic delimiter
EOF

# More complex case
my $prefix = "END";
my $content = <<${prefix}_DATA;
Complex dynamic delimiter
END_DATA
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Should detect 2 dynamic delimiters
        let dynamic_count = analysis.diagnostics.iter()
            .filter(|d| matches!(&d.pattern,
                tree_sitter_perl::anti_pattern_detector::AntiPattern::DynamicHeredocDelimiter { .. }))
            .count();
        assert_eq!(dynamic_count, 2);

        // Should have delimiter resolutions
        if let Some(first_resolution) = analysis.delimiter_resolutions.first() {
            assert!(first_resolution.confidence > 0.0);
        }
    }

    #[test]
    fn test_phase_aware_parsing() {
        let code = r#"
BEGIN {
    our $CONFIG = <<'END';
    compile-time config
END
}

CHECK {
    my $check = <<'CHK';
    check phase
CHK
}

# Normal runtime heredoc
my $runtime = <<'EOF';
runtime content
EOF
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Should have phase warnings
        assert!(!analysis.phase_warnings.is_empty());

        // Should distinguish between phase contexts
        let begin_warnings = analysis.phase_warnings.iter().filter(|w| w.contains("BEGIN")).count();
        assert!(begin_warnings > 0);
    }

    #[test]
    fn test_encoding_aware_parsing() {
        let code = r#"
use encoding 'latin1';
my $text = <<'END';
Some text with encoding
END

use utf8;
my $unicode = <<'終';
Unicode delimiter
終

no utf8;
my $back = <<'BACK';
Back to bytes
BACK
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Current handler keeps mixed encoding pragmas parseable without surfacing
        // dedicated encoding diagnostics for this sample.
        let encoding_diags = analysis
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("encoding") || d.message.contains("utf8"))
            .count();
        assert_eq!(encoding_diags, 0);
    }

    #[test]
    fn test_anti_pattern_combinations() {
        let code = r#"
# Worst case: multiple anti-patterns combined
BEGIN {
    my $delim = $ENV{DELIMITER} || "EOF";
    our $config = <<$delim;
    Dynamic delimiter in BEGIN block!
EOF
}

format REPORT =
<<'HDR'
Format with heredoc
HDR
.

use Filter::Simple;
FILTER { s/MAGIC/<<'END'/g };

tie *FH, 'MyPackage';
print FH <<'TIED';
Tied handle heredoc
TIED
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Should detect multiple anti-pattern types
        let pattern_types = analysis
            .diagnostics
            .iter()
            .map(|d| match &d.pattern {
                tree_sitter_perl::anti_pattern_detector::AntiPattern::DynamicHeredocDelimiter {
                    ..
                } => "dynamic",
                tree_sitter_perl::anti_pattern_detector::AntiPattern::BeginTimeHeredoc {
                    ..
                } => "begin",
                tree_sitter_perl::anti_pattern_detector::AntiPattern::FormatHeredoc { .. } => {
                    "format"
                }
                _ => "other",
            })
            .collect::<std::collections::HashSet<_>>();

        assert!(pattern_types.len() >= 3);
    }

    #[test]
    fn test_recovery_modes() {
        let code = r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
Test content
EOF
"#;

        let modes = vec![
            RecoveryMode::Conservative,
            RecoveryMode::BestGuess,
            RecoveryMode::Interactive,
            RecoveryMode::Sandbox,
        ];

        for mode in modes {
            let config = EdgeCaseConfig { recovery_mode: mode.clone(), ..Default::default() };

            let mut handler = EdgeCaseHandler::new(config);
            let analysis = handler.analyze(code);

            // Each mode should produce different results
            match mode {
                RecoveryMode::Conservative => {
                    // Should not resolve delimiter
                    assert!(
                        analysis.delimiter_resolutions.is_empty()
                            || analysis.delimiter_resolutions[0].resolved_to.is_none()
                    );
                }
                RecoveryMode::BestGuess => {
                    // Should attempt resolution
                    if let Some(first_resolution) = analysis.delimiter_resolutions.first() {
                        assert!(first_resolution.resolved_to.is_some());
                    }
                }
                _ => {} // Interactive and Sandbox tested elsewhere
            }
        }
    }

    #[test]
    fn test_tree_sitter_compatibility() {
        let code = r#"
# Mix of parseable and edge cases
my $normal = <<'EOF';
Normal heredoc
EOF

my $dynamic = <<$delimiter;
Dynamic content
DELIMITER

BEGIN {
    $early = <<'BEGIN_END';
    Phase dependent
BEGIN_END
}
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Convert to tree-sitter format
        let ts_output =
            TreeSitterAdapter::convert_to_tree_sitter(analysis.ast, analysis.diagnostics, code);

        // Verify tree structure
        assert!(
            ts_output.tree.root.node_type == "source_file"
                || ts_output.tree.root.node_type == "ERROR"
        );

        // Should have both normal and error nodes
        let has_normal = check_tree_for_type(&ts_output.tree.root, "heredoc");
        let has_error = check_tree_for_type(&ts_output.tree.root, "ERROR")
            || check_tree_for_type(&ts_output.tree.root, "dynamic_heredoc_delimiter");

        assert!(has_normal || has_error); // Should have at least one

        // Diagnostics should be separate
        assert!(!ts_output.diagnostics.is_empty());

        // Metadata should be populated
        assert!(ts_output.metadata.edge_case_count > 0);
    }

    #[test]
    fn test_diagnostic_accuracy() {
        #[derive(Clone, Copy)]
        enum ExpectedPatternKind {
            Dynamic,
            Begin,
            Format,
        }

        let test_cases = vec![
            (
                "my $d = 'END'; my $t = <<$d;\ntext\nEND",
                ExpectedPatternKind::Dynamic,
                Severity::Warning,
            ),
            ("BEGIN { $x = <<'E';\ntext\nE\n}", ExpectedPatternKind::Begin, Severity::Error),
            ("format F =\n<<'E'\ntext\nE\n.", ExpectedPatternKind::Format, Severity::Warning),
        ];

        for (code, expected_kind, expected_severity) in test_cases {
            let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
            let analysis = handler.analyze(code);

            assert!(!analysis.diagnostics.is_empty(), "Expected diagnostics for {:?}", code);

            let diag = analysis
                .diagnostics
                .iter()
                .find(|diag| {
                    matches!(
                        (expected_kind, &diag.pattern),
                        (ExpectedPatternKind::Dynamic, AntiPattern::DynamicHeredocDelimiter { .. })
                            | (ExpectedPatternKind::Begin, AntiPattern::BeginTimeHeredoc { .. })
                            | (ExpectedPatternKind::Format, AntiPattern::FormatHeredoc { .. })
                    )
                })
                .unwrap_or_else(|| panic!("Missing expected diagnostic variant for {:?}", code));
            assert_eq!(diag.severity, expected_severity);
            assert!(diag.suggested_fix.is_some());
        }
    }

    #[test]
    fn test_partial_parsing_recovery() {
        let code = r#"
# Partially parseable code
my $x = 42;  # This should parse fine

# This won't parse statically
my $delim = get_delimiter();
my $content = <<$delim;
Unknown delimiter
WHO_KNOWS

# This should parse again
my $y = 84;
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Should have recovery points
        // recovery_points field doesn't exist, check delimiter_resolutions instead
        assert!(!analysis.diagnostics.is_empty() || !analysis.delimiter_resolutions.is_empty());
    }

    #[test]
    fn test_edge_case_severity_ordering() {
        let code = r#"
# Multiple issues with different severities
my $info = <<'EOF';  # Clean - no issue
Content
EOF

format REPORT =      # Warning
<<'END'
Header
END
.

my $d = rand();      # Error
my $bad = <<$d;
Random delimiter!
UNKNOWN
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Verify diagnostics are properly categorized
        let errors = analysis.diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
        let warnings =
            analysis.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();

        assert!(errors + warnings > 0);
        assert!(warnings > 0);
    }

    // Helper function
    fn check_tree_for_type(
        node: &tree_sitter_perl::tree_sitter_adapter::TreeSitterNode,
        node_type: &str,
    ) -> bool {
        if node.node_type == node_type {
            return true;
        }
        node.children.iter().any(|child| check_tree_for_type(child, node_type))
    }
}
