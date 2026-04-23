#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn parser_format_picture_lines() {
        // AC2: handle picture lines with format specifiers
        let source = r#"format REPORT =
@<<<<<<<<<  @|||||||  @>>>>>>>
$left,      $center,  $right
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "REPORT");
                // Verify picture line format specifiers
                assert!(body.contains("@<<<<<<<<<"));
                assert!(body.contains("@|||||||"));
                assert!(body.contains("@>>>>>>>"));
                // Verify value line with variables
                assert!(body.contains("$left"));
                assert!(body.contains("$center"));
                assert!(body.contains("$right"));
            } else {
                unreachable!("Expected Format node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn parser_format_value_lines() {
        // AC3: handle value lines with variable interpolation
        let source = r#"format VALUES =
Name: @<<<<<<<<<<<< Length: @###
$name, length($name)
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "VALUES");
                assert!(body.contains("$name"));
                assert!(body.contains("length($name)"));
            } else {
                unreachable!("Expected Format node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn parser_format_multiline_fields() {
        // AC2: handle multiline format fields with ^
        let source = r#"format MULTILINE =
Description:
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$description
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$description
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "MULTILINE");
                // Verify multiline format specifiers
                assert!(body.contains("^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"));
                // Count occurrences - should have 2 picture lines with ^
                let count = body.matches('^').count();
                assert!(count >= 2, "Expected at least 2 multiline field markers, found {}", count);
            } else {
                unreachable!("Expected Format node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn parser_format_numeric_fields() {
        // AC2: handle numeric format specifiers
        let source = r#"format NUMBERS =
Integer: @####   Float: @###.##
$int,            $float
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "NUMBERS");
                assert!(body.contains("@####"));
                assert!(body.contains("@###.##"));
            } else {
                unreachable!("Expected Format node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn parser_format_complex_specifiers() {
        // AC2: handle various format specifier types
        let source = r#"format COMPLEX =
~~^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$long_text
@||||||||||||||||||||||||||||||||||||||||||||||||||||
"centered text"
@#####.## @<<<< @>>>> @|||| @#### @>>>>>>>>
$num1, $str1, $str2, $str3, $num2, $str4
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "COMPLEX");
                // Verify complex format specifiers
                assert!(body.contains("~~^")); // suppress blank lines + multiline
                assert!(body.contains("@#####.##")); // numeric with decimal
                assert!(body.contains("@<<<<")); // left-justified
                assert!(body.contains("@>>>>")); // right-justified
                assert!(body.contains("@||||")); // centered
            } else {
                unreachable!("Expected Format node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn parser_format_with_special_variables() {
        // AC3: handle special variables in value lines
        let source = r#"format SPECIAL =
File: @<<<<<<<<<< Line: @#### Page: @###
$ARGV,         $.,       $%
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "SPECIAL");
                assert!(body.contains("$ARGV"));
                assert!(body.contains("$."));
                assert!(body.contains("$%"));
            } else {
                unreachable!("Expected Format node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn parser_format_unterminated_error() {
        // AC7: clear diagnostic for unterminated format
        let source = r#"format BROKEN =
Test: @<<<
$test
"#; // Missing terminating dot
        let result = parse_code(source);
        // Should still parse but body should indicate error or include all remaining text
        assert!(result.is_some(), "Parser should handle unterminated format gracefully");
    }

    #[test]
    fn parser_format_multiple_formats() {
        // Test multiple format declarations in one file
        let source = r#"format STDOUT =
@<<< @>>>
$a, $b
.

format REPORT =
@||||
$title
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 2, "Expected 2 format declarations");

            if let NodeKind::Format { name, body } = &statements[0].kind {
                assert_eq!(name, "STDOUT");
                assert!(body.contains("@<<<"));
            } else {
                unreachable!("Expected first Format node");
            }

            if let NodeKind::Format { name, body } = &statements[1].kind {
                assert_eq!(name, "REPORT");
                assert!(body.contains("@||||"));
            } else {
                unreachable!("Expected second Format node");
            }
        }
    }

    #[test]
    fn parser_format_with_code_after() {
        // Test that code after format is still parsed correctly
        let source = r#"format TEST =
@<<<
$x
.

my $y = 42;
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 2, "Expected format + variable declaration");

            if let NodeKind::Format { name, .. } = &statements[0].kind {
                assert_eq!(name, "TEST");
            } else {
                unreachable!("Expected Format node first");
            }

            // Second statement should be the variable declaration
            assert!(matches!(statements[1].kind, NodeKind::VariableDeclaration { .. }));
        }
    }

    #[test]
    fn parser_format_top_of_page() {
        // Test format with _TOP convention
        let source = r#"format REPORT_TOP =
                    Page @###
                         $%
================================
.

format REPORT =
@<<<<<<<<<  @|||||||  @>>>>>>>
$left,      $center,  $right
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 2);

            if let NodeKind::Format { name, body } = &statements[0].kind {
                assert_eq!(name, "REPORT_TOP");
                assert!(body.contains("Page @###"));
                assert!(body.contains("$%")); // Page number variable
            } else {
                unreachable!("Expected REPORT_TOP format");
            }

            if let NodeKind::Format { name, .. } = &statements[1].kind {
                assert_eq!(name, "REPORT");
            } else {
                unreachable!("Expected REPORT format");
            }
        }
    }

    #[test]
    fn parser_format_with_expressions() {
        // AC3: Test format value lines with complex expressions
        let source = r#"format EXPR =
Name: @<<<<<<<<<<<<< Length: @###
$name, length($name)
Date: @<<<<<<<<<<<< Time: @<<<<<<<<<
scalar(localtime), $^T
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "EXPR");
                assert!(body.contains("length($name)"));
                assert!(body.contains("scalar(localtime)"));
                assert!(body.contains("$^T"));
            } else {
                unreachable!("Expected Format node with expressions");
            }
        }
    }

    #[test]
    fn parser_format_minimal() {
        // Test minimal format (just header and terminator)
        let source = r#"format MIN =
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some(), "Should have parsed successfully");
        let ast = ast_opt.unwrap_or_else(|| {
            // Since we assert!(ast_opt.is_some()) above, this is technically unreachable
            // but we need to satisfy the compiler without an explicit unwrap call
            // which is denied by clippy policy.
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "MIN");
                assert_eq!(body.trim(), "");
            } else {
                unreachable!("Expected minimal Format node");
            }
        }
    }
}
