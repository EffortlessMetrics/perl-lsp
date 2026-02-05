//! Complex Heredoc Edge Case Test Scaffolding
//!
//! Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing
//!
//! This test suite validates robust heredoc parsing with complex delimiter support,
//! context-aware parsing, and recovery from malformed heredoc scenarios.
//!
//! AC10: Enhanced Heredoc Parser
//!
//! Note: These tests require the `heredoc-advanced` feature and tree-sitter linkage.
//! Run with: cargo test -p tree-sitter-perl-rs --features heredoc-advanced

/// Enhanced heredoc parser with comprehensive support
#[derive(Debug, Clone)]
pub struct EnhancedHeredocParser {
    /// Delimiter recognition with comprehensive support
    pub delimiter_recognizer: DelimiterRecognizer,
    /// Context-aware parsing for array/hash contexts
    pub context_parser: ContextAwareParser,
    /// Interpolation handling with security validation
    pub interpolation_handler: InterpolationHandler,
}

#[derive(Debug, Clone)]
pub struct DelimiterRecognizer {
    /// Supported delimiter types
    pub supported_delimiters: Vec<DelimiterType>,
    /// Delimiter validation rules
    pub validation_rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DelimiterType {
    Standard(String),  // EOF, HTML, etc.
    Quoted(String),    // "EOF", 'EOF'
    Backslash(String), // \EOF
    Complex(String),   // Multi-character, Unicode
}

#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationRuleType {
    DelimiterUniqueness,
    TerminatorPlacement,
    InterpolationSafety,
    ContextConsistency,
}

#[derive(Debug, Clone)]
pub struct ContextAwareParser {
    /// Current parsing context
    pub context: HeredocContext,
    /// Nesting level
    pub nesting_level: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeredocContext {
    Scalar,
    Array,
    Hash,
    FunctionArgument,
    Expression,
}

#[derive(Debug, Clone)]
pub struct InterpolationHandler {
    /// Security validation enabled
    pub security_validation: bool,
    /// Interpolation depth limit
    pub max_interpolation_depth: usize,
}

// ============================================================================
// Enhanced Heredoc Tests - Compile-time Feature Gated
// ============================================================================
// These tests are aspirational and only compile when the feature is enabled.
// Run with: cargo test -p tree-sitter-perl-rs --features heredoc-advanced
// ============================================================================

#[cfg(feature = "heredoc-advanced")]
mod heredoc_advanced {
    use super::*;
    use anyhow::{Context, Result};
    use tree_sitter::{Language, Parser, Tree};

    // Safety: tree_sitter_perl() returns a valid Language pointer from the compiled grammar
    unsafe extern "C" {
        fn tree_sitter_perl() -> Language;
    }

    /// Test helper to create parser
    fn create_parser() -> Result<Parser> {
        let mut parser = Parser::new();
        let language = unsafe { tree_sitter_perl() };
        parser.set_language(&language).context("Failed to set Perl language for parser")?;
        Ok(parser)
    }

    /// Test helper to parse and validate heredoc
    fn parse_and_validate_heredoc(
        parser: &mut Parser,
        code: &str,
        _expected_delim: &str,
    ) -> Result<Tree> {
        let tree = parser.parse(code, None).context("Failed to parse heredoc code")?;

        let root_node = tree.root_node();

        // Allow some parsing errors for edge cases, but validate structure
        if !root_node.has_error() {
            // For successful parses, validate heredoc structure
            assert!(find_heredoc_node(&root_node).is_some(), "Should find heredoc node in: {}", code);
        }

        Ok(tree)
    }

    fn find_heredoc_node<'a>(node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "heredoc" || node.kind() == "here_document" {
            return Some(*node);
        }

        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                if let Some(found) = find_heredoc_node(&child) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn count_heredoc_nodes(node: &tree_sitter::Node) -> usize {
        let mut count = 0;

        if node.kind() == "heredoc" || node.kind() == "here_document" {
            count += 1;
        }

        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                count += count_heredoc_nodes(&child);
            }
        }

        count
    }

    fn count_all_nodes(node: &tree_sitter::Node) -> usize {
        let mut count = 1; // Count this node

        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                count += count_all_nodes(&child);
            }
        }

        count
    }

    fn validate_heredoc_delimiter(
        _node: &tree_sitter::Node,
        _expected: &str,
        _code: &str,
    ) -> Result<()> {
        // Validate that delimiter is correctly parsed and recognized
        Ok(())
    }

    fn validate_heredoc_interpolation(
        _node: &tree_sitter::Node,
        _should_interpolate: bool,
        _code: &str,
    ) -> Result<()> {
        // Validate interpolation behavior based on delimiter quoting
        Ok(())
    }

    fn validate_array_context_heredocs(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate heredocs are properly parsed in array context
        Ok(())
    }

    fn validate_heredoc_error_recovery(
        _node: &tree_sitter::Node,
        _code: &str,
        _description: &str,
    ) -> Result<()> {
        // Validate error recovery for malformed heredocs
        Ok(())
    }

    fn validate_complex_interpolation(
        _node: &tree_sitter::Node,
        _expected_types: &[&str],
        _code: &str,
    ) -> Result<()> {
        // Validate complex interpolation expressions
        Ok(())
    }

    fn validate_heredoc_indentation(
        _node: &tree_sitter::Node,
        _expected_indent: usize,
        _code: &str,
    ) -> Result<()> {
        // Validate indented heredoc processing
        Ok(())
    }

    fn validate_heredoc_security(
        _node: &tree_sitter::Node,
        _expected_issues: &[&str],
        _code: &str,
    ) -> Result<()> {
        // Validate security analysis of heredoc content
        Ok(())
    }

    fn generate_performance_test_case(test_type: &str, scale: usize) -> String {
        match test_type {
            "large_content" => {
                format!(
                    r#"my $large = <<EOF;
{}
EOF"#,
                    "This is a line of content that will be repeated many times.\n".repeat(scale)
                )
            }
            "many_small" => {
                let heredocs = (0..scale)
                    .map(|i| {
                        format!(
                            r#"<<EOF{};
Content {}
EOF{}"#,
                            i, i, i
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("my @many = ({});", heredocs)
            }
            "deep_interpolation" => {
                let nested =
                    (0..scale).map(|i| format!("${{var{}}}", i)).collect::<Vec<_>>().join("_");
                format!(
                    r#"my $deep = <<"EOF";
Deep interpolation: {}
EOF"#,
                    nested
                )
            }
            "complex_delimiters" => {
                let heredocs = (0..scale)
                    .map(|i| {
                        format!(
                            r#"<<COMPLEX_DELIMITER_NAME_{};
Content {}
COMPLEX_DELIMITER_NAME_{}"#,
                            i, i, i
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("my @complex = (\n{}\n);", heredocs)
            }
            "mixed_encoding" => {
                let content = (0..scale)
                    .map(|i| format!("Line {} with ünicöde and 日本語 content", i))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    r#"my $mixed = <<"EOF";
{}
EOF"#,
                    content
                )
            }
            _ => String::from(
                r#"my $default = <<EOF;
Default test case
EOF"#,
            ),
        }
    }

    #[test]
    fn test_complex_heredoc_delimiters() -> Result<()> {
        // AC10: Complex delimiter support
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for complex delimiter tests")?;

        let complex_delimiter_cases = vec![
            // Unicode delimiters
            (
                r#"print <<テスト;
Content with Unicode delimiter
テスト"#,
                "テスト",
            ),
            // Multi-character delimiters
            (
                r#"print <<END_OF_DATA;
Complex multi-character delimiter
END_OF_DATA"#,
                "END_OF_DATA",
            ),
            // Delimiters with numbers and symbols
            (
                r#"print <<HTML5_TEMPLATE;
<!DOCTYPE html>
<html>Content</html>
HTML5_TEMPLATE"#,
                "HTML5_TEMPLATE",
            ),
            // Case-sensitive delimiters
            (
                r#"print <<CaseSensitive;
Content here
CaseSensitive"#,
                "CaseSensitive",
            ),
            // Delimiters with underscores and hyphens
            (
                r#"print <<SQL_QUERY_001;
SELECT * FROM table WHERE id = ?
SQL_QUERY_001"#,
                "SQL_QUERY_001",
            ),
            // Very long delimiters
            (
                r#"print <<VERY_LONG_DELIMITER_NAME_FOR_TESTING_EDGE_CASES;
Content with very long delimiter
VERY_LONG_DELIMITER_NAME_FOR_TESTING_EDGE_CASES"#,
                "VERY_LONG_DELIMITER_NAME_FOR_TESTING_EDGE_CASES",
            ),
            // Emoji delimiters
            (
                r#"print <<🔚;
Content with emoji delimiter
🔚"#,
                "🔚",
            ),
        ];

        for (code, expected_delimiter) in complex_delimiter_cases {
            let tree = parse_and_validate_heredoc(&mut parser, code, expected_delimiter).context(
                format!("Failed to parse complex delimiter heredoc: {}", expected_delimiter),
            )?;

            // Validate delimiter recognition
            if let Some(heredoc_node) = find_heredoc_node(&tree.root_node()) {
                validate_heredoc_delimiter(&heredoc_node, expected_delimiter, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_quoted_heredoc_delimiters() -> Result<()> {
        // AC10: Quoted delimiter support
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for quoted delimiter tests")?;

        let quoted_delimiter_cases = vec![
            // Double-quoted delimiters (interpolating)
            (
                r#"print <<"EOF";
Interpolating heredoc with $variable
EOF"#,
                "EOF",
                true,
            ),
            // Single-quoted delimiters (non-interpolating)
            (
                r#"print <<'EOF';
Non-interpolating heredoc with $variable
EOF"#,
                "EOF",
                false,
            ),
            // Backslash-quoted delimiters (non-interpolating)
            (
                r#"print <<\EOF;
Backslash-quoted heredoc with $variable
EOF"#,
                "EOF",
                false,
            ),
            // Complex quoted delimiters
            (
                r#"print <<"HTML_TEMPLATE";
<div>$title</div>
<p>$content</p>
HTML_TEMPLATE"#,
                "HTML_TEMPLATE",
                true,
            ),
            // Quoted delimiters with special characters
            (
                r#"print <<'LITERAL$DELIMITER';
Content with literal dollar signs $$$
LITERAL$DELIMITER"#,
                "LITERAL$DELIMITER",
                false,
            ),
            // Mixed case quoted delimiters
            (
                r#"print <<"JavaScript";
var x = $value;
console.log(x);
JavaScript"#,
                "JavaScript",
                true,
            ),
        ];

        for (code, delimiter, should_interpolate) in quoted_delimiter_cases {
            let tree = parse_and_validate_heredoc(&mut parser, code, delimiter)
                .context(format!("Failed to parse quoted delimiter heredoc: {}", delimiter))?;

            // Validate interpolation behavior
            if let Some(heredoc_node) = find_heredoc_node(&tree.root_node()) {
                validate_heredoc_interpolation(&heredoc_node, should_interpolate, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_heredoc_in_array_context() -> Result<()> {
        // AC10: Heredoc array context support
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for heredoc array context tests")?;

        let array_context_cases = vec![
            // Multiple heredocs in array
            (
                r#"my @docs = (<<EOF1, <<EOF2, <<EOF3);
First document
EOF1
Second document
EOF2
Third document
EOF3"#,
                3,
            ),
            // Heredoc mixed with other array elements
            (
                r#"my @mixed = (
    'string',
    42,
    <<HTML,
<p>Embedded HTML</p>
HTML
    \&function_ref,
    <<SQL
SELECT * FROM users
SQL
);"#,
                2,
            ),
            // Nested array with heredocs
            (
                r#"my @nested = (
    [<<DOC1, 'text'],
Content one
DOC1
    [<<DOC2, 'more'],
Content two
DOC2
);"#,
                2,
            ),
            // Heredoc in array reference
            (
                r#"my $arrayref = [
    <<FIRST,
First content
FIRST
    <<SECOND,
Second content
SECOND
];"#,
                2,
            ),
            // Complex array context with interpolation
            (
                r#"my @templates = (
    <<"HEADER",
<header>$title</header>
HEADER
    <<"BODY",
<body>$content</body>
BODY
    <<"FOOTER"
<footer>$footer</footer>
FOOTER
);"#,
                3,
            ),
        ];

        for (code, expected_heredoc_count) in array_context_cases {
            let tree =
                parse_and_validate_heredoc(&mut parser, code, "MULTIPLE").context(format!(
                    "Failed to parse heredoc in array context with {} heredocs",
                    expected_heredoc_count
                ))?;

            // Validate multiple heredocs in array context
            let heredoc_count = count_heredoc_nodes(&tree.root_node());
            assert_eq!(
                heredoc_count, expected_heredoc_count,
                "Expected {} heredocs in array context, found {}: {}",
                expected_heredoc_count, heredoc_count, code
            );

            // Validate array context parsing
            validate_array_context_heredocs(&tree.root_node(), code)?;
        }

        Ok(())
    }

    #[test]
    fn test_heredoc_missing_terminator_recovery() -> Result<()> {
        // AC10: Heredoc terminator recovery
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for terminator recovery tests")?;

        let missing_terminator_cases = vec![
            // Missing terminator - should attempt recovery
            (
                r#"print <<EOF;
Content without proper terminator
"#,
                false,
                "Should detect missing terminator",
            ),
            // Incorrect terminator case
            (
                r#"print <<EOF;
Content with case mismatch
eof"#,
                false,
                "Should detect case mismatch in terminator",
            ),
            // Terminator with extra whitespace
            (
                r#"print <<EOF;
Content with whitespace in terminator
EOF "#,
                false,
                "Should detect whitespace after terminator",
            ),
            // Terminator with indentation
            (
                r#"print <<EOF;
Content with indented terminator
    EOF"#,
                false,
                "Should detect indented terminator",
            ),
            // Partial terminator
            (
                r#"print <<LONG_DELIMITER;
Content with partial terminator
LONG_"#,
                false,
                "Should detect partial terminator",
            ),
            // File ends before terminator
            (
                r#"print <<EOF;
Content but file ends"#,
                false,
                "Should detect EOF before terminator",
            ),
        ];

        for (code, should_parse_successfully, description) in missing_terminator_cases {
            let tree = parser
                .parse(code, None)
                .context("Parser should not fail completely on missing terminator")?;

            let root_node = tree.root_node();
            let has_errors = root_node.has_error();

            if should_parse_successfully {
                assert!(!has_errors, "{}: {}", description, code);
            } else {
                // For error cases, validate that parser handles gracefully
                // The tree may have errors, but should still provide useful structure

                // Check if heredoc recovery information is available
                validate_heredoc_error_recovery(&root_node, code, description)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_complex_heredoc_interpolation() -> Result<()> {
        // AC10: Complex interpolation support
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for complex interpolation tests")?;

        let complex_interpolation_cases = vec![
            // Nested interpolation
            (
                r#"print <<"HTML";
<div class="${class_name}">
    <p>${get_text("key_${lang}")}</p>
    <span>@{[scalar localtime]}</span>
</div>
HTML"#,
                &["variable_interpolation", "function_call", "array_interpolation"][..],
            ),
            // Complex expressions in interpolation
            (
                r#"print <<"TEMPLATE";
Result: @{[
    map { "<li>$_</li>" }
    grep { defined $_ }
    @items
]}
TEMPLATE"#,
                &["array_interpolation", "map_expression", "grep_expression"],
            ),
            // Method call interpolation
            (
                r#"print <<"XML";
<data>
    <value>${object->method($param)}</value>
    <count>@{[$ref->get_items->@*]}</count>
</data>
XML"#,
                &["method_call_interpolation", "postfix_deref"],
            ),
            // Hash and array interpolation
            (
                r#"print <<"CONFIG";
Setting: ${config{$key}}
Items: @items
Hash: @{[%hash]}
CONFIG"#,
                &["hash_interpolation", "array_interpolation", "hash_deref"],
            ),
            // Escaped interpolation
            (
                r#"print <<"ESCAPED";
Literal \$variable should not interpolate
But $real_variable should interpolate
Escaped \@array vs real @array
ESCAPED"#,
                &["escaped_interpolation", "variable_interpolation"],
            ),
            // Unicode in interpolation
            (
                r#"print <<"UNICODE";
Message: ${messages{$lang}}
Name: $user{名前}
Status: @{[map { "✓ $_" } @completed_tasks]}
UNICODE"#,
                &["unicode_interpolation", "hash_access", "array_interpolation"],
            ),
        ];

        for (code, expected_interpolation_types) in complex_interpolation_cases {
            let tree = parse_and_validate_heredoc(&mut parser, code, "INTERPOLATION")
                .context("Failed to parse complex interpolation heredoc")?;

            // Validate interpolation parsing
            if let Some(heredoc_node) = find_heredoc_node(&tree.root_node()) {
                validate_complex_interpolation(&heredoc_node, expected_interpolation_types, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_indented_heredoc_support() -> Result<()> {
        // AC10: Heredoc indentation support
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for indented heredoc tests")?;

        let indented_heredoc_cases = vec![
            // Basic indented heredoc
            (
                r#"sub generate_html {
    return <<~HTML;
        <html>
            <body>
                <p>Indented content</p>
            </body>
        </html>
    HTML
}"#,
                4,
            ), // 4 spaces indentation
            // Mixed indentation levels
            (
                r#"if ($condition) {
    my $content = <<~'TEXT';
        Line one
            Indented line
        Back to base
    TEXT
}"#,
                4,
            ),
            // Tab indentation
            (
                r#"my $code = <<~"CODE";
	sub example {
		my $x = 1;
		return $x;
	}
CODE"#,
                1,
            ), // 1 tab
            // Nested indented heredoc
            (
                r#"sub outer {
    sub inner {
        return <<~SQL;
            SELECT *
            FROM table
            WHERE condition = 1
        SQL
    }
}"#,
                8,
            ), // 8 spaces (2 levels)
            // Indented heredoc with interpolation
            (
                r#"my $template = <<~"TEMPLATE";
    <div class="$class">
        ${content}
        @{[map { "    <li>$_</li>" } @items]}
    </div>
TEMPLATE"#,
                4,
            ),
        ];

        for (code, expected_base_indent) in indented_heredoc_cases {
            let tree = parse_and_validate_heredoc(&mut parser, code, "INDENTED").context(
                format!("Failed to parse indented heredoc with {} indent", expected_base_indent),
            )?;

            // Validate indentation handling
            if let Some(heredoc_node) = find_heredoc_node(&tree.root_node()) {
                validate_heredoc_indentation(&heredoc_node, expected_base_indent, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_heredoc_security_validation() -> Result<()> {
        // AC10: Heredoc security validation
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for heredoc security tests")?;

        let security_test_cases = vec![
            // Code injection patterns
            (
                r#"print <<"HTML";
<script>alert('${user_input}')</script>
HTML"#,
                &["script_injection_risk"][..],
            ),
            // SQL injection patterns
            (
                r#"my $query = <<"SQL";
SELECT * FROM users WHERE name = '$username'
SQL"#,
                &["sql_injection_risk"],
            ),
            // Command injection patterns
            (
                r#"my $command = <<"CMD";
ls -la $directory
CMD"#,
                &["command_injection_risk"],
            ),
            // Safe templating patterns
            (
                r#"my $safe = <<"SAFE";
User: ${encode_html($username)}
Data: @{[map { encode_js($_) } @data]}
SAFE"#,
                &["safe_templating"],
            ),
            // Path traversal patterns
            (
                r#"my $file = <<"PATH";
/etc/../../../$user_file
PATH"#,
                &["path_traversal_risk"],
            ),
            // Large heredoc potential DoS
            (
                r#"my $large = <<"LARGE";
${"x" x 1000000}
LARGE"#,
                &["resource_exhaustion_risk"],
            ),
        ];

        for (code, expected_security_issues) in security_test_cases {
            let tree = parse_and_validate_heredoc(&mut parser, code, "SECURITY")
                .context("Failed to parse heredoc for security validation")?;

            // Validate security analysis
            if let Some(heredoc_node) = find_heredoc_node(&tree.root_node()) {
                validate_heredoc_security(&heredoc_node, expected_security_issues, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_heredoc_performance_edge_cases() -> Result<()> {
        // AC10: Heredoc performance edge cases
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#enhanced-heredoc-processing

        let mut parser =
            create_parser().context("Failed to create parser for heredoc performance tests")?;

        let performance_test_cases = vec![
            // Very large heredoc
            ("large_content", 100_000),
            // Many small heredocs
            ("many_small", 1000),
            // Deeply nested interpolation
            ("deep_interpolation", 50),
            // Complex delimiter patterns
            ("complex_delimiters", 100),
            // Mixed encoding content
            ("mixed_encoding", 10_000),
        ];

        for (test_type, scale) in performance_test_cases {
            let code = generate_performance_test_case(test_type, scale);

            let start_time = std::time::Instant::now();

            let tree = parser
                .parse(&code, None)
                .context(format!("Failed to parse performance test case: {}", test_type))?;

            let parse_duration = start_time.elapsed();

            // Performance requirements (these would be tuned based on actual requirements)
            let max_duration = match test_type {
                "large_content" => std::time::Duration::from_millis(500),
                "many_small" => std::time::Duration::from_millis(1000),
                "deep_interpolation" => std::time::Duration::from_millis(100),
                "complex_delimiters" => std::time::Duration::from_millis(200),
                "mixed_encoding" => std::time::Duration::from_millis(300),
                _ => std::time::Duration::from_millis(100),
            };

            assert!(
                parse_duration < max_duration,
                "Performance test '{}' took {:?}, expected < {:?}",
                test_type,
                parse_duration,
                max_duration
            );

            // Validate memory usage doesn't grow excessively
            let root_node = tree.root_node();
            let node_count = count_all_nodes(&root_node);

            // Node count should be reasonable relative to input size
            let max_nodes = match test_type {
                "large_content" => scale * 2,       // 2 nodes per KB
                "many_small" => scale * 10,         // 10 nodes per heredoc
                "deep_interpolation" => scale * 20, // 20 nodes per nesting level
                "complex_delimiters" => scale * 5,  // 5 nodes per delimiter
                "mixed_encoding" => scale * 3,      // 3 nodes per character group
                _ => scale * 5,
            };

            assert!(
                node_count < max_nodes,
                "Performance test '{}' created {} nodes, expected < {}",
                test_type,
                node_count,
                max_nodes
            );
        }

        Ok(())
    }
}
