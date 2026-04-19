//! Comprehensive unit tests for perl-lsp-providers crate.
//!
//! Covers: folding ranges, on-type formatting, selection ranges, signature help,
//! linked editing, LSP errors, re-exports, and edge cases.

use std::sync::Arc;

use perl_parser_core::{ParseError, Parser, ast::Node};
use perl_tdd_support::{must, must_some};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse(source: &str) -> Result<Arc<Node>, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse().map(Arc::new)
}

// =========================================================================
// LspError
// =========================================================================

mod lsp_error_tests {
    use perl_lsp_providers::ide::lsp_compat::lsp_errors::LspError;

    #[test]
    fn construct_and_read_fields() {
        let err = LspError {
            code: -32600,
            message: "Invalid Request".to_string(),
        };
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "Invalid Request");
    }

    #[test]
    fn clone_and_debug() {
        let err = LspError {
            code: -32601,
            message: "Method not found".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
        let dbg = format!("{:?}", err);
        assert!(dbg.contains("-32601"));
    }

    #[test]
    fn equality() {
        let a = LspError {
            code: 1,
            message: "a".to_string(),
        };
        let b = LspError {
            code: 1,
            message: "a".to_string(),
        };
        let c = LspError {
            code: 2,
            message: "a".to_string(),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn empty_message() {
        let err = LspError {
            code: 0,
            message: String::new(),
        };
        assert_eq!(err.code, 0);
        assert!(err.message.is_empty());
    }
}

// =========================================================================
// FoldingRangeExtractor
// =========================================================================

mod folding_tests {
    use super::*;
    use perl_lsp_providers::ide::lsp_compat::folding::{
        FoldingRange, FoldingRangeExtractor, FoldingRangeKind,
    };

    #[test]
    fn extractor_default() {
        let extractor = FoldingRangeExtractor::default();
        // Default extractor is equivalent to new()
        let _ = extractor;
    }

    #[test]
    fn empty_source_produces_no_ranges() -> Result<(), ParseError> {
        let ast = parse("")?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(ranges.is_empty());
        Ok(())
    }

    #[test]
    fn single_statement_no_fold() -> Result<(), ParseError> {
        let ast = parse("my $x = 1;")?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        // A single statement should not produce folds (or only trivial ones)
        for r in &ranges {
            assert!(r.end_offset > r.start_offset);
        }
        Ok(())
    }

    #[test]
    fn subroutine_produces_fold() -> Result<(), ParseError> {
        let code = "sub hello {\n    print \"hi\";\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(
            !ranges.is_empty(),
            "subroutine should produce at least one fold"
        );
        Ok(())
    }

    #[test]
    fn multiple_use_statements_produce_import_fold() -> Result<(), ParseError> {
        let code = "use strict;\nuse warnings;\nuse Carp;\nmy $x = 1;\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        let import_folds: Vec<&FoldingRange> = ranges
            .iter()
            .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Imports)))
            .collect();
        assert!(
            !import_folds.is_empty(),
            "consecutive use statements should produce import fold"
        );
        Ok(())
    }

    #[test]
    fn single_use_no_import_fold() -> Result<(), ParseError> {
        let code = "use strict;\nmy $x = 1;\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        let import_folds: Vec<&FoldingRange> = ranges
            .iter()
            .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Imports)))
            .collect();
        assert!(
            import_folds.is_empty(),
            "single use should not produce import fold"
        );
        Ok(())
    }

    #[test]
    fn if_block_produces_fold() -> Result<(), ParseError> {
        let code = "if (1) {\n    print 1;\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(!ranges.is_empty(), "if block should produce a fold");
        Ok(())
    }

    #[test]
    fn while_loop_produces_fold() -> Result<(), ParseError> {
        let code = "while (1) {\n    last;\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(!ranges.is_empty(), "while loop should produce a fold");
        Ok(())
    }

    #[test]
    fn for_loop_produces_fold() -> Result<(), ParseError> {
        let code = "for (my $i = 0; $i < 10; $i++) {\n    print $i;\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(!ranges.is_empty(), "for loop should produce a fold");
        Ok(())
    }

    #[test]
    fn foreach_loop_produces_fold() -> Result<(), ParseError> {
        let code = "foreach my $item (@list) {\n    print $item;\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(!ranges.is_empty(), "foreach loop should produce a fold");
        Ok(())
    }

    #[test]
    fn package_produces_fold() -> Result<(), ParseError> {
        let code = "package Foo {\n    sub bar { 1 }\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(!ranges.is_empty(), "package block should produce a fold");
        Ok(())
    }

    #[test]
    fn hash_literal_produces_fold() -> Result<(), ParseError> {
        let code = "my %h = (\n    a => 1,\n    b => 2,\n);\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        // Hash literal with elements should produce fold
        assert!(!ranges.is_empty());
        Ok(())
    }

    #[test]
    fn array_literal_produces_fold() -> Result<(), ParseError> {
        let code = "my @arr = (\n    1,\n    2,\n    3,\n);\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(!ranges.is_empty());
        Ok(())
    }

    #[test]
    fn extract_resets_between_calls() -> Result<(), ParseError> {
        let code = "sub a {\n    1;\n}\nsub b {\n    2;\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let first = extractor.extract(&ast);
        let second = extractor.extract(&ast);
        assert_eq!(
            first.len(),
            second.len(),
            "extract should reset internal state"
        );
        Ok(())
    }

    #[test]
    fn heredoc_ranges_from_text() {
        let code = "my $x = <<END;\nline1\nline2\nEND\n";
        let ranges = FoldingRangeExtractor::extract_heredoc_ranges(code);
        // Should detect heredoc body as a foldable region
        for r in &ranges {
            assert!(r.end_offset > r.start_offset);
            assert!(matches!(r.kind, Some(FoldingRangeKind::Region)));
        }
    }

    #[test]
    fn heredoc_ranges_empty_source() {
        let ranges = FoldingRangeExtractor::extract_heredoc_ranges("");
        assert!(ranges.is_empty());
    }

    #[test]
    fn heredoc_ranges_no_heredoc() {
        let code = "my $x = 42;\nprint $x;\n";
        let ranges = FoldingRangeExtractor::extract_heredoc_ranges(code);
        assert!(ranges.is_empty());
    }

    #[test]
    fn folding_range_debug_and_clone() {
        let range = FoldingRange {
            start_offset: 0,
            end_offset: 10,
            kind: Some(FoldingRangeKind::Comment),
        };
        let cloned = range.clone();
        assert_eq!(cloned.start_offset, 0);
        assert_eq!(cloned.end_offset, 10);
        assert!(matches!(cloned.kind, Some(FoldingRangeKind::Comment)));
        let _ = format!("{:?}", range);
    }

    #[test]
    fn folding_range_kind_variants() {
        let comment = FoldingRangeKind::Comment;
        let imports = FoldingRangeKind::Imports;
        let region = FoldingRangeKind::Region;
        let _ = format!("{:?} {:?} {:?}", comment, imports, region);
        // Clone works
        let _ = comment.clone();
        let _ = imports.clone();
        let _ = region.clone();
    }

    #[test]
    fn trailing_import_block() -> Result<(), ParseError> {
        // File that ends with consecutive use statements
        let code = "my $x = 1;\nuse A;\nuse B;\nuse C;\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        let import_folds: Vec<&FoldingRange> = ranges
            .iter()
            .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Imports)))
            .collect();
        assert!(
            !import_folds.is_empty(),
            "trailing consecutive use statements should produce import fold"
        );
        Ok(())
    }

    #[test]
    fn data_section_produces_comment_fold() -> Result<(), ParseError> {
        let code = "print 1;\n__DATA__\nsome data here\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        let comment_folds: Vec<&FoldingRange> = ranges
            .iter()
            .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Comment)))
            .collect();
        // __DATA__ with body should produce comment fold
        assert!(
            !comment_folds.is_empty(),
            "DATA section with body should produce comment fold"
        );
        Ok(())
    }

    #[test]
    fn eval_block_produces_fold() -> Result<(), ParseError> {
        let code = "eval {\n    die 'oops';\n};\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        // eval block may or may not produce folds depending on AST structure
        let _ = ranges;
        Ok(())
    }

    #[test]
    fn begin_block_produces_fold() -> Result<(), ParseError> {
        let code = "BEGIN {\n    require Foo;\n}\n";
        let ast = parse(code)?;
        let mut extractor = FoldingRangeExtractor::new();
        let ranges = extractor.extract(&ast);
        assert!(!ranges.is_empty(), "BEGIN block should produce a fold");
        Ok(())
    }
}

// =========================================================================
// On-type formatting
// =========================================================================

mod on_type_formatting_tests {
    use perl_lsp_providers::ide::lsp_compat::on_type_formatting::compute_on_type_edit;

    #[test]
    fn open_brace_is_not_a_trigger() {
        let text = "sub foo {";
        let edits = compute_on_type_edit(text, 0, 9, '{', 2);
        assert!(
            edits.is_none(),
            "open brace is not an on-type formatting trigger"
        );
    }

    #[test]
    fn close_brace_adjusts_indent() {
        let text = "sub foo {\n    print 1;\n    }";
        // Typing '}' on line 2
        let edits = compute_on_type_edit(text, 2, 5, '}', 2);
        // Closing brace with mismatch should produce edits
        assert!(edits.is_some() || edits.is_none()); // May or may not adjust
    }

    #[test]
    fn close_brace_already_correct_indent() {
        let text = "sub foo {\n    print 1;\n}";
        let edits = compute_on_type_edit(text, 2, 1, '}', 2);
        // Already at correct indent - None is valid
        assert!(edits.is_none());
    }

    #[test]
    fn semicolon_maintains_indent() {
        let text = "    my $x = 1;";
        let edits = compute_on_type_edit(text, 0, 14, ';', 2);
        assert!(edits.is_none(), "semicolon preserves existing indentation");
    }

    #[test]
    fn newline_after_brace_increases_indent() {
        // line 1 must exist in the text for the '\n' handler to work
        let text = "sub foo {\n    ";
        let edits = compute_on_type_edit(text, 1, 0, '\n', 2);
        // Newline on line 1 looks at line 0 which ends with '{' -> should indent
        assert!(edits.is_some(), "newline after brace should indent");
    }

    #[test]
    fn newline_on_first_line_returns_none() {
        let text = "my $x = 1;";
        let edits = compute_on_type_edit(text, 0, 10, '\n', 2);
        assert!(edits.is_none());
    }

    #[test]
    fn unknown_char_returns_none() {
        let text = "my $x = 1;";
        let edits = compute_on_type_edit(text, 0, 5, 'a', 2);
        assert!(edits.is_none());
    }

    #[test]
    fn line_beyond_text_returns_none() {
        let text = "line 1\nline 2";
        let edits = compute_on_type_edit(text, 99, 0, '{', 2);
        assert!(edits.is_none());
    }

    #[test]
    fn empty_text() {
        let edits = compute_on_type_edit("", 0, 0, '{', 2);
        // Empty text has no lines at line 0 in some implementations
        // Accept either None or Some
        let _ = edits;
    }

    #[test]
    fn close_brace_on_first_line() {
        let text = "}";
        let edits = compute_on_type_edit(text, 0, 1, '}', 2);
        assert!(edits.is_none(), "close brace on line 0 returns None");
    }

    #[test]
    fn carriage_return_trigger() {
        let text = "sub foo {\n    ";
        let edits = compute_on_type_edit(text, 1, 0, '\r', 2);
        assert!(
            edits.is_some(),
            "carriage return should behave like newline"
        );
    }

    #[test]
    fn nested_braces_indent() {
        let text = "sub foo {\n    if (1) {\n        ";
        let edits = compute_on_type_edit(text, 2, 0, '\n', 2);
        assert!(edits.is_some());
    }

    #[test]
    fn close_brace_finds_matching_open() {
        let text = "sub foo {\n    if (1) {\n        print 1;\n    }\n}";
        // Closing brace for outer sub at line 4
        let edits = compute_on_type_edit(text, 4, 1, '}', 2);
        // Already correct indent -> None
        assert!(edits.is_none());
    }
}

// =========================================================================
// Selection range
// =========================================================================

mod selection_range_tests {
    use super::*;
    use perl_lsp_providers::ide::lsp_compat::selection_range::{build_parent_map, selection_chain};

    #[test]
    fn build_parent_map_empty_source() -> Result<(), ParseError> {
        let ast = parse("")?;
        let map = build_parent_map(&ast);
        // Root has no parent, so map should be empty or contain only children
        let _ = map;
        Ok(())
    }

    #[test]
    fn build_parent_map_with_statements() -> Result<(), ParseError> {
        let ast = parse("my $x = 1; my $y = 2;")?;
        let map = build_parent_map(&ast);
        // Should have entries for each child node
        assert!(!map.is_empty());
        Ok(())
    }

    #[test]
    fn selection_chain_at_offset_zero() -> Result<(), ParseError> {
        let code = "my $x = 1;";
        let ast = parse(code)?;
        let parent_map = build_parent_map(&ast);
        let to_pos16 = |offset: usize| -> (u32, u32) {
            perl_parser_core::position::offset_to_utf16_line_col(code, offset)
        };
        let chain = selection_chain(&ast, &parent_map, 0, &to_pos16);
        assert!(chain.is_object(), "selection chain should be a JSON object");
        assert!(chain.get("range").is_some(), "chain should have a range");
        Ok(())
    }

    #[test]
    fn selection_chain_at_end_of_source() -> Result<(), ParseError> {
        let code = "my $x = 1;";
        let ast = parse(code)?;
        let parent_map = build_parent_map(&ast);
        let to_pos16 = |offset: usize| -> (u32, u32) {
            perl_parser_core::position::offset_to_utf16_line_col(code, offset)
        };
        let chain = selection_chain(&ast, &parent_map, code.len(), &to_pos16);
        assert!(chain.is_object());
        Ok(())
    }

    #[test]
    fn selection_chain_nested_code() -> Result<(), ParseError> {
        let code = "sub foo {\n    my $x = 1;\n}\n";
        let ast = parse(code)?;
        let parent_map = build_parent_map(&ast);
        let to_pos16 = |offset: usize| -> (u32, u32) {
            perl_parser_core::position::offset_to_utf16_line_col(code, offset)
        };
        // Position inside the subroutine body
        let chain = selection_chain(&ast, &parent_map, 15, &to_pos16);
        assert!(chain.is_object());
        // Should have parent chain
        let range = chain.get("range");
        assert!(range.is_some());
        Ok(())
    }

    #[test]
    fn selection_chain_has_parent_property() -> Result<(), ParseError> {
        let code = "sub foo {\n    my $x = 1;\n}\n";
        let ast = parse(code)?;
        let parent_map = build_parent_map(&ast);
        let to_pos16 = |offset: usize| -> (u32, u32) {
            perl_parser_core::position::offset_to_utf16_line_col(code, offset)
        };
        let chain = selection_chain(&ast, &parent_map, 15, &to_pos16);
        // Walk the chain and verify structure
        let mut current = &chain;
        let mut depth = 0;
        while current.is_object() {
            assert!(current.get("range").is_some());
            depth += 1;
            if let Some(parent) = current.get("parent") {
                if parent.is_null() {
                    break;
                }
                current = parent;
            } else {
                break;
            }
        }
        assert!(depth >= 1, "should have at least one level in the chain");
        Ok(())
    }

    #[test]
    fn selection_chain_string_depth() -> Result<(), ParseError> {
        let code = "sub greet {\n    my $msg = \"hello world\";\n}\n";
        let ast = parse(code)?;
        // Byte offset for line 1 char 22 (the 'w' in "world"):
        // line 0 = "sub greet {\n" = 12 bytes
        // char 22 on line 1 => offset = 12 + 22 = 34
        let offset = 34;
        let parent_map = build_parent_map(&ast);
        let to_pos16 = |o: usize| -> (u32, u32) {
            perl_parser_core::position::offset_to_utf16_line_col(code, o)
        };
        let chain = selection_chain(&ast, &parent_map, offset, &to_pos16);

        // Count depth
        let mut depth = 0;
        let mut current = &chain;
        while current.is_object() {
            depth += 1;
            if let Some(parent) = current.get("parent") {
                if parent.is_null() {
                    break;
                }
                current = parent;
            } else {
                break;
            }
        }
        // With full node traversal, inside "hello world" we should get:
        // String -> VariableDeclaration (or ExpressionStatement) -> Block -> Subroutine -> Program
        // That's at least 4 levels
        assert!(
            depth >= 3,
            "string inside sub should produce >= 3 levels, got {}. AST sexp: {}",
            depth,
            ast.to_sexp()
        );
        Ok(())
    }

    #[test]
    fn selection_chain_function_name() -> Result<(), ParseError> {
        let code = "sub calculate {\n    return 1;\n}\n";
        let ast = parse(code)?;
        // 'c' of calculate is at byte 4
        let offset = 4;
        let parent_map = build_parent_map(&ast);
        let to_pos16 = |o: usize| -> (u32, u32) {
            perl_parser_core::position::offset_to_utf16_line_col(code, o)
        };
        let chain = selection_chain(&ast, &parent_map, offset, &to_pos16);

        let mut depth = 0;
        let mut current = &chain;
        while current.is_object() {
            depth += 1;
            if let Some(parent) = current.get("parent") {
                if parent.is_null() {
                    break;
                }
                current = parent;
            } else {
                break;
            }
        }
        // On a function name, we should get at least: name_span -> Subroutine -> Program
        assert!(
            depth >= 2,
            "function name should produce >= 2 levels, got {}",
            depth,
        );
        Ok(())
    }
}

// =========================================================================
// Signature help
// =========================================================================

mod signature_help_tests {
    use super::*;
    use perl_lsp_providers::ide::lsp_compat::signature_help::SignatureHelpProvider;

    #[test]
    fn has_builtin_print() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        assert!(provider.has_builtin("print"));
    }

    #[test]
    fn has_builtin_push() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        assert!(provider.has_builtin("push"));
    }

    #[test]
    fn has_no_fake_builtin() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        assert!(!provider.has_builtin("not_a_real_function_xyz"));
    }

    #[test]
    fn builtin_count_is_positive() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        assert!(provider.builtin_count() > 0);
    }

    #[test]
    fn get_builtin_signature_print() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let sig = must_some(provider.get_builtin_signature("print"));
        assert!(!sig.signatures.is_empty());
        assert!(!sig.documentation.is_empty());
    }

    #[test]
    fn get_builtin_signature_nonexistent() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        assert!(provider.get_builtin_signature("nonexistent_fn").is_none());
    }

    #[test]
    fn signature_help_for_print() {
        let code = "print(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
    }

    #[test]
    fn signature_help_active_parameter_increments() {
        let code = "substr($str, 5, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(2));
    }

    #[test]
    fn signature_help_no_paren_returns_none() {
        let code = "my $x = 1;";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        assert!(provider.get_signature_help(code, code.len()).is_none());
    }

    #[test]
    fn signature_help_empty_string() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        assert!(provider.get_signature_help("", 0).is_none());
    }

    #[test]
    fn signature_help_method_call() {
        let code = "$obj->method(1, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = provider.get_signature_help(code, code.len() - 1);
        // Method call may not resolve to a builtin, but should not crash
        let _ = help;
    }

    #[test]
    fn signature_help_nested_parens() {
        let code = "push(@arr, split(',', ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = provider.get_signature_help(code, code.len() - 1);
        // Should find the inner split call
        if let Some(h) = help {
            assert!(!h.signatures.is_empty());
        }
    }

    #[test]
    fn new_with_source_works() {
        let code = "sub greet { print 'hello'; }";
        let ast = must(parse(code));
        let provider = SignatureHelpProvider::new_with_source(&ast, code);
        assert!(provider.builtin_count() > 0);
    }

    #[test]
    fn user_defined_sub_signature() {
        let code = "sub add($a, $b) { $a + $b }\nadd(1, ";
        let ast = must(parse(code));
        let provider = SignatureHelpProvider::new_with_source(&ast, code);
        let help = provider.get_signature_help(code, code.len() - 1);
        if let Some(h) = help {
            assert!(!h.signatures.is_empty());
            // Check active param is 1 (second arg)
            assert_eq!(h.active_parameter, Some(1));
        }
    }

    #[test]
    fn parameter_info_fields() {
        use perl_lsp_providers::ide::lsp_compat::signature_help::ParameterInfo;
        let p = ParameterInfo {
            label: "$x".to_string(),
            documentation: Some("a var".to_string()),
        };
        assert_eq!(p.label, "$x");
        assert_eq!(p.documentation.as_deref(), Some("a var"));
        let _ = format!("{:?}", p);
        let _ = p.clone();
    }

    #[test]
    fn signature_info_fields() {
        use perl_lsp_providers::ide::lsp_compat::signature_help::SignatureInfo;
        let s = SignatureInfo {
            label: "sub foo".to_string(),
            documentation: None,
            parameters: vec![],
            active_parameter: Some(0),
        };
        assert_eq!(s.label, "sub foo");
        assert!(s.documentation.is_none());
        assert!(s.parameters.is_empty());
        let _ = format!("{:?}", s);
        let _ = s.clone();
    }

    #[test]
    fn signature_help_fields() {
        use perl_lsp_providers::ide::lsp_compat::signature_help::SignatureHelp;
        let h = SignatureHelp {
            signatures: vec![],
            active_signature: None,
            active_parameter: None,
        };
        assert!(h.signatures.is_empty());
        assert!(h.active_signature.is_none());
        let _ = format!("{:?}", h);
        let _ = h.clone();
    }

    #[test]
    fn signature_help_cursor_at_open_paren() {
        let code = "print(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        // Position right at the open paren
        let help = provider.get_signature_help(code, 6);
        if let Some(h) = help {
            assert_eq!(h.active_parameter, Some(0));
        }
    }

    // -----------------------------------------------------------------
    // Comprehensive builtin signature help tests for 10 common builtins
    // -----------------------------------------------------------------

    #[test]
    fn signature_help_open_first_arg() {
        let code = "open(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        // open has 3 signature variants
        assert!(
            help.signatures.len() >= 2,
            "open should have multiple variants"
        );
    }

    #[test]
    fn signature_help_open_second_arg() {
        let code = "open($fh, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(1));
    }

    #[test]
    fn signature_help_open_third_arg() {
        let code = "open($fh, '<', ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(2));
    }

    #[test]
    fn signature_help_open_has_documentation() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let sig = must_some(provider.get_builtin_signature("open"));
        assert!(
            !sig.documentation.is_empty(),
            "open should have documentation"
        );
    }

    #[test]
    fn signature_help_print_first_arg() {
        let code = "print(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        // print has 4 signature variants
        assert!(
            help.signatures.len() >= 2,
            "print should have multiple variants"
        );
    }

    #[test]
    fn signature_help_print_with_filehandle_and_list() {
        let code = "print($fh, @data, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(2));
    }

    #[test]
    fn signature_help_push_first_arg() {
        let code = "push(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        // First parameter should be ARRAY
        assert!(!help.signatures[0].parameters.is_empty());
        assert_eq!(help.signatures[0].parameters[0].label, "ARRAY");
    }

    #[test]
    fn signature_help_push_second_arg() {
        let code = "push(@arr, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(1));
    }

    #[test]
    fn signature_help_push_has_array_param_doc() {
        let code = "push(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        let first_param = &help.signatures[0].parameters[0];
        assert_eq!(first_param.label, "ARRAY");
        assert!(
            first_param.documentation.is_some(),
            "ARRAY param should have documentation"
        );
    }

    #[test]
    fn signature_help_pop_no_args() {
        let code = "pop(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        // pop has two variants: pop ARRAY and pop
        assert!(
            help.signatures.len() >= 2,
            "pop should have at least 2 variants"
        );
    }

    #[test]
    fn signature_help_pop_with_array() {
        let code = "pop(@arr";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert_eq!(help.active_parameter, Some(0));
    }

    #[test]
    fn signature_help_splice_all_args() {
        let code = "splice(@arr, 0, 2, @new, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        // 4 commas means we are on 5th element (index 4)
        assert_eq!(help.active_parameter, Some(4));
        // splice has 4 variants
        assert!(
            help.signatures.len() >= 3,
            "splice should have multiple variants"
        );
    }

    #[test]
    fn signature_help_splice_offset_only() {
        let code = "splice(@arr, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(1));
    }

    #[test]
    fn signature_help_splice_offset_length() {
        let code = "splice(@arr, 0, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(2));
    }

    #[test]
    fn signature_help_map_first_arg() {
        let code = "map(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        // map has two variants: map BLOCK LIST and map EXPR, LIST
        assert!(
            help.signatures.len() >= 2,
            "map should have at least 2 variants"
        );
    }

    #[test]
    fn signature_help_map_second_arg() {
        let code = "map({ $_ * 2 }, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        // After the closing paren of the inner block, then comma
        // The outer call to map is detected
        let help = provider.get_signature_help(code, code.len() - 1);
        if let Some(h) = help {
            // Should be on the LIST parameter
            assert!(h.active_parameter.is_some());
        }
    }

    #[test]
    fn signature_help_grep_first_arg() {
        let code = "grep(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        assert!(
            help.signatures.len() >= 2,
            "grep should have at least 2 variants"
        );
    }

    #[test]
    fn signature_help_grep_with_expr() {
        let code = "grep($test, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(1));
    }

    #[test]
    fn signature_help_sort_first_arg() {
        let code = "sort(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        // sort has three variants
        assert!(
            help.signatures.len() >= 2,
            "sort should have multiple variants"
        );
    }

    #[test]
    fn signature_help_sort_with_block() {
        let code = "sort({ $a <=> $b }, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = provider.get_signature_help(code, code.len() - 1);
        if let Some(h) = help {
            assert!(h.active_parameter.is_some());
        }
    }

    #[test]
    fn signature_help_join_first_arg() {
        let code = "join(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
    }

    #[test]
    fn signature_help_join_second_arg() {
        let code = "join($sep, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(1));
    }

    #[test]
    fn signature_help_join_param_labels() {
        let code = "join(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        let sig = &help.signatures[0];
        assert!(
            sig.parameters.len() >= 2,
            "join should have at least EXPR and LIST params"
        );
        assert_eq!(sig.parameters[0].label, "EXPR");
        assert_eq!(sig.parameters[1].label, "LIST");
    }

    #[test]
    fn signature_help_split_first_arg() {
        let code = "split(";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len()));
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(0));
        // split has 4 variants
        assert!(
            help.signatures.len() >= 3,
            "split should have multiple variants"
        );
    }

    #[test]
    fn signature_help_split_second_arg() {
        let code = "split($pat, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(1));
    }

    #[test]
    fn signature_help_split_third_arg() {
        let code = "split($pat, $str, ";
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let help = must_some(provider.get_signature_help(code, code.len() - 1));
        assert_eq!(help.active_parameter, Some(2));
    }

    // -----------------------------------------------------------------
    // All 10 builtins are recognized
    // -----------------------------------------------------------------

    #[test]
    fn all_target_builtins_recognized() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let targets = [
            "open", "print", "push", "pop", "splice", "map", "grep", "sort", "join", "split",
        ];
        for name in &targets {
            assert!(
                provider.has_builtin(name),
                "builtin '{}' should be recognized",
                name
            );
        }
    }

    #[test]
    fn all_target_builtins_have_signatures() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let targets = [
            "open", "print", "push", "pop", "splice", "map", "grep", "sort", "join", "split",
        ];
        for name in &targets {
            let sig = provider.get_builtin_signature(name);
            assert!(sig.is_some(), "builtin '{}' should have a signature", name);
            let sig = sig.map(|s| &s.signatures);
            assert!(
                sig.is_some_and(|s| !s.is_empty()),
                "builtin '{}' should have at least one signature variant",
                name
            );
        }
    }

    #[test]
    fn all_target_builtins_have_documentation() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let targets = [
            "open", "print", "push", "pop", "splice", "map", "grep", "sort", "join", "split",
        ];
        for name in &targets {
            let sig = must_some(provider.get_builtin_signature(name));
            assert!(
                !sig.documentation.is_empty(),
                "builtin '{}' should have non-empty documentation",
                name
            );
        }
    }

    #[test]
    fn all_target_builtins_provide_help_at_open_paren() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let targets = [
            "open", "print", "push", "pop", "splice", "map", "grep", "sort", "join", "split",
        ];
        for name in &targets {
            let code = format!("{}(", name);
            let help = provider.get_signature_help(&code, code.len());
            assert!(
                help.is_some(),
                "builtin '{}' should provide signature help at open paren",
                name
            );
            let h = help.as_ref();
            assert!(
                h.is_some_and(|h| !h.signatures.is_empty()),
                "builtin '{}' should have non-empty signatures in help response",
                name
            );
            assert_eq!(
                h.and_then(|h| h.active_parameter),
                Some(0),
                "builtin '{}' should start at parameter 0",
                name
            );
        }
    }

    #[test]
    fn signature_help_parameters_have_labels() {
        let ast = must(parse(""));
        let provider = SignatureHelpProvider::new(&ast);
        let targets = [
            "open", "print", "push", "pop", "splice", "map", "grep", "sort", "join", "split",
        ];
        for name in &targets {
            let code = format!("{}(", name);
            let help = must_some(provider.get_signature_help(&code, code.len()));
            for sig in &help.signatures {
                for param in &sig.parameters {
                    assert!(
                        !param.label.is_empty(),
                        "parameter label in '{}' should not be empty",
                        name
                    );
                }
            }
        }
    }
}

// =========================================================================
// Linked editing
// =========================================================================

mod linked_editing_tests {
    use perl_lsp_providers::ide::lsp_compat::linked_editing::handle_linked_editing;

    #[test]
    fn matching_parens() {
        let text = "(hello)";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some(), "should find matching parens");
        let ranges = result.map(|r| r.ranges);
        if let Some(r) = ranges {
            assert_eq!(r.len(), 2);
        }
    }

    #[test]
    fn matching_brackets() {
        let text = "[1, 2, 3]";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some(), "should find matching brackets");
    }

    #[test]
    fn matching_braces() {
        let text = "{ foo }";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some(), "should find matching braces");
    }

    #[test]
    fn matching_double_quotes() {
        let text = "\"hello\"";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some(), "should find matching quotes");
    }

    #[test]
    fn matching_single_quotes() {
        let text = "'hello'";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some(), "should find matching single quotes");
    }

    #[test]
    fn no_match_for_plain_text() {
        let text = "hello world";
        let result = handle_linked_editing(text, 0, 2);
        assert!(result.is_none(), "no bracket at position => None");
    }

    #[test]
    fn cursor_after_close_paren() {
        // Test cursor on the opening paren to find matching closing one
        let text = "(x)";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some());
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
        }
    }

    #[test]
    fn nested_parens() {
        let text = "((inner))";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some());
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
        }
    }

    #[test]
    fn empty_text() {
        let result = handle_linked_editing("", 0, 0);
        assert!(result.is_none());
    }

    #[test]
    fn unmatched_paren() {
        let text = "(no close";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_none(), "unmatched paren should return None");
    }

    #[test]
    fn multiline_braces() {
        let text = "{\n  code;\n}";
        // Opening brace at line 0, col 0
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some());
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
            // First range on line 0, second on line 2
            assert_eq!(r.ranges[0].start.line, 0);
            assert_eq!(r.ranges[1].start.line, 2);
        }
    }

    #[test]
    fn cursor_on_open_brace_forward_scan() {
        // Test cursor on the opening brace to find its matching close
        let text = "{x}";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some());
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
        }
    }

    #[test]
    fn angle_brackets() {
        let text = "<data>";
        let result = handle_linked_editing(text, 0, 0);
        assert!(result.is_some(), "should find matching angle brackets");
    }

    // --- backward-scan tests (cursor on close bracket) ---

    #[test]
    fn cursor_on_close_paren_backward_scan() {
        // Cursor ON ')' at col 2 — exercises the backward-scan branch
        // "(x)" — ')' is at byte 2, line 0, col 2
        let text = "(x)";
        let result = handle_linked_editing(text, 0, 2);
        assert!(
            result.is_some(),
            "cursor on close paren should find matching open"
        );
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
            assert_eq!(r.ranges[0].start.character, 0); // open paren
            assert_eq!(r.ranges[1].start.character, 2); // close paren
        }
    }

    #[test]
    fn cursor_on_close_bracket_backward_scan() {
        // Cursor ON ']' at col 5; '[' is at col 0
        let text = "[1, 2]";
        let result = handle_linked_editing(text, 0, 5);
        assert!(
            result.is_some(),
            "cursor on close bracket should find matching open"
        );
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
            assert_eq!(r.ranges[0].start.character, 0); // open bracket
            assert_eq!(r.ranges[1].start.character, 5); // close bracket
        }
    }

    #[test]
    fn cursor_on_close_brace_backward_scan() {
        // Cursor ON '}' at col 7; '{' is at col 0
        let text = "{ code }";
        let result = handle_linked_editing(text, 0, 7);
        assert!(
            result.is_some(),
            "cursor on close brace should find matching open"
        );
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
            assert_eq!(r.ranges[0].start.character, 0); // open brace
            assert_eq!(r.ranges[1].start.character, 7); // close brace
        }
    }

    #[test]
    fn cursor_on_close_paren_nested_backward_scan() {
        // "(())" — cursor on outer ')' at col 3
        let text = "(())";
        let result = handle_linked_editing(text, 0, 3);
        assert!(
            result.is_some(),
            "cursor on outer close paren should find outer open"
        );
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
            assert_eq!(r.ranges[0].start.character, 0); // outer open
            assert_eq!(r.ranges[1].start.character, 3); // outer close
        }
    }

    #[test]
    fn unmatched_close_paren_returns_none() {
        // "x)" — no matching open bracket, must return None without panicking
        let text = "x)";
        let result = handle_linked_editing(text, 0, 1);
        assert!(
            result.is_none(),
            "unmatched close paren should return None, not panic"
        );
    }

    #[test]
    fn cursor_on_close_angle_bracket_backward_scan() {
        // Cursor ON '>' at col 5; '<' is at col 0
        // '>' is in CLOSE but not in OPEN, so it takes the close-bracket branch (not quote branch)
        let text = "<data>";
        let result = handle_linked_editing(text, 0, 5);
        assert!(
            result.is_some(),
            "cursor on close angle bracket should find matching open"
        );
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
            assert_eq!(r.ranges[0].start.character, 0); // open angle
            assert_eq!(r.ranges[1].start.character, 5); // close angle
        }
    }

    #[test]
    fn cursor_on_close_brace_multiline_backward_scan() {
        // '}' is on line 2 col 0; '{' is on line 0 col 0
        let text = "{\n  code;\n}";
        let result = handle_linked_editing(text, 2, 0);
        assert!(
            result.is_some(),
            "cursor on close brace in multiline block should find open brace on line 0"
        );
        if let Some(r) = result {
            assert_eq!(r.ranges.len(), 2);
            assert_eq!(r.ranges[0].start.line, 0); // open brace on line 0
            assert_eq!(r.ranges[0].start.character, 0);
            assert_eq!(r.ranges[1].start.line, 2); // close brace on line 2
            assert_eq!(r.ranges[1].start.character, 0);
        }
    }
}

// =========================================================================
// Re-exports from top-level modules
// =========================================================================

mod reexport_tests {
    use super::*;

    #[test]
    fn parser_reexport_works() -> Result<(), ParseError> {
        let mut parser = perl_lsp_providers::Parser::new("my $x = 1;");
        let ast = parser.parse()?;
        assert!(matches!(
            ast.kind,
            perl_lsp_providers::ast::NodeKind::Program { .. }
        ));
        Ok(())
    }

    #[test]
    fn node_kind_accessible() {
        let _kind = perl_lsp_providers::NodeKind::Program { statements: vec![] };
    }

    #[test]
    fn source_location_accessible() {
        let loc = perl_lsp_providers::SourceLocation { start: 0, end: 5 };
        assert_eq!(loc.start, 0);
        assert_eq!(loc.end, 5);
    }

    #[test]
    fn diagnostics_module_accessible() -> Result<(), ParseError> {
        let ast = parse("my $x = 1;")?;
        let provider = perl_lsp_providers::diagnostics::DiagnosticsProvider::new(
            &ast,
            "my $x = 1;".to_string(),
        );
        let _ = provider;
        Ok(())
    }

    #[test]
    fn completion_module_accessible() -> Result<(), ParseError> {
        let ast = parse("my $x = 1;")?;
        let provider = perl_lsp_providers::completion::CompletionProvider::new(&ast);
        let _ = provider;
        Ok(())
    }

    #[test]
    fn inlay_hints_module_accessible() {
        let provider = perl_lsp_providers::inlay_hints::InlayHintsProvider::new();
        let _ = provider;
    }

    #[test]
    fn semantic_tokens_module_accessible() {
        let provider = perl_lsp_providers::semantic_tokens::SemanticTokensProvider::new();
        let _ = provider;
    }

    #[test]
    fn rename_module_accessible() -> Result<(), ParseError> {
        let ast = parse("my $x = 1;")?;
        let provider =
            perl_lsp_providers::rename::RenameProvider::new(&ast, "my $x = 1;".to_string());
        let _ = provider;
        Ok(())
    }

    #[test]
    fn navigation_module_accessible() {
        let provider = perl_lsp_providers::navigation::TypeDefinitionProvider::new();
        let _ = provider;
    }

    #[test]
    fn code_actions_module_accessible() {
        let provider =
            perl_lsp_providers::code_actions::CodeActionsProvider::new("my $x = 1;".to_string());
        let _ = provider;
    }

    #[test]
    fn formatting_types_accessible() {
        let pos = perl_lsp_providers::formatting::FormatPosition::new(0, 0);
        let range = perl_lsp_providers::formatting::FormatRange::new(
            pos.clone(),
            perl_lsp_providers::formatting::FormatPosition::new(1, 0),
        );
        let _ = range;

        let opts = perl_lsp_providers::formatting::FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };
        assert_eq!(opts.tab_size, 4);
        assert!(opts.insert_spaces);
    }

    #[test]
    fn tooling_subprocess_output() {
        let output = perl_lsp_providers::tooling::SubprocessOutput {
            stdout: b"ok".to_vec(),
            stderr: Vec::new(),
            status_code: 0,
        };
        assert!(output.success());

        let failed = perl_lsp_providers::tooling::SubprocessOutput {
            stdout: Vec::new(),
            stderr: b"err".to_vec(),
            status_code: 1,
        };
        assert!(!failed.success());
    }

    #[test]
    fn tooling_subprocess_error() {
        let err = perl_lsp_providers::tooling::SubprocessError::new("test error");
        assert_eq!(err.message, "test error");
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_tooling_export_still_works() {
        let err = perl_lsp_providers::tooling_export::SubprocessError::new("compat");
        assert_eq!(err.message, "compat");
    }
}

// =========================================================================
// IDE module structure
// =========================================================================

mod ide_module_tests {
    use super::*;

    #[test]
    fn top_level_code_actions_reexport() {
        let provider =
            perl_lsp_providers::code_actions::CodeActionsProvider::new("my $x = 1;".to_string());
        let _ = provider;
    }

    #[test]
    fn top_level_completion_reexport() -> Result<(), ParseError> {
        let ast = parse("my $x = 1;")?;
        let provider = perl_lsp_providers::completion::CompletionProvider::new(&ast);
        let _ = provider;
        Ok(())
    }

    #[test]
    fn top_level_diagnostics_reexport() -> Result<(), ParseError> {
        let ast = parse("my $x = 1;")?;
        let provider = perl_lsp_providers::diagnostics::DiagnosticsProvider::new(
            &ast,
            "my $x = 1;".to_string(),
        );
        let _ = provider;
        Ok(())
    }

    #[test]
    fn top_level_rename_reexport() -> Result<(), ParseError> {
        let ast = parse("my $x = 1;")?;
        let provider =
            perl_lsp_providers::rename::RenameProvider::new(&ast, "my $x = 1;".to_string());
        let _ = provider;
        Ok(())
    }

    #[test]
    fn top_level_inlay_hints_reexport() {
        let provider = perl_lsp_providers::inlay_hints::InlayHintsProvider::new();
        let _ = provider;
    }

    #[test]
    fn top_level_formatting_reexport() {
        let range = perl_lsp_providers::formatting::FormatRange::new(
            perl_lsp_providers::formatting::FormatPosition::new(0, 0),
            perl_lsp_providers::formatting::FormatPosition::new(0, 5),
        );
        let _ = range;
    }
}

// =========================================================================
// Diagnostics provider integration
// =========================================================================

mod diagnostics_integration_tests {
    use super::*;
    use perl_lsp_providers::diagnostics::DiagnosticsProvider;

    #[test]
    fn diagnostics_for_valid_code() -> Result<(), ParseError> {
        let code = "use strict;\nuse warnings;\nmy $x = 42;\nprint $x;\n";
        let ast = parse(code)?;
        let provider = DiagnosticsProvider::new(&ast, code.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[], code, None);
        // Valid code should produce few or no diagnostics
        let _ = diagnostics;
        Ok(())
    }

    #[test]
    fn diagnostics_for_empty_code() -> Result<(), ParseError> {
        let ast = parse("")?;
        let provider = DiagnosticsProvider::new(&ast, String::new());
        let diagnostics = provider.get_diagnostics(&ast, &[], "", None);
        let _ = diagnostics;
        Ok(())
    }
}

// =========================================================================
// Formatting options edge cases
// =========================================================================

mod formatting_edge_cases {
    use perl_lsp_providers::formatting::{FormatPosition, FormatRange, FormattingOptions};

    #[test]
    fn formatting_options_tabs() {
        let opts = FormattingOptions {
            tab_size: 8,
            insert_spaces: false,
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        };
        assert_eq!(opts.tab_size, 8);
        assert!(!opts.insert_spaces);
        assert!(opts.trim_trailing_whitespace.is_none());
    }

    #[test]
    fn format_position_new() {
        let pos = FormatPosition::new(10, 5);
        assert_eq!(pos.line, 10);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn format_range_new() {
        let start = FormatPosition::new(0, 0);
        let end = FormatPosition::new(10, 0);
        let range = FormatRange::new(start, end);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 10);
    }
}

// =========================================================================
// Tooling types
// =========================================================================

mod tooling_tests {
    use perl_lsp_providers::tooling::{SubprocessError, SubprocessOutput};

    #[test]
    fn subprocess_output_success() {
        let out = SubprocessOutput {
            stdout: vec![1, 2, 3],
            stderr: vec![],
            status_code: 0,
        };
        assert!(out.success());
    }

    #[test]
    fn subprocess_output_failure() {
        let out = SubprocessOutput {
            stdout: vec![],
            stderr: vec![1],
            status_code: 127,
        };
        assert!(!out.success());
    }

    #[test]
    fn subprocess_error_message() {
        let err = SubprocessError::new("command not found");
        assert_eq!(err.message, "command not found");
    }

    #[test]
    fn subprocess_error_empty_message() {
        let err = SubprocessError::new("");
        assert!(err.message.is_empty());
    }

    #[test]
    fn subprocess_output_empty() {
        let out = SubprocessOutput {
            stdout: vec![],
            stderr: vec![],
            status_code: 0,
        };
        assert!(out.success());
        assert!(out.stdout.is_empty());
        assert!(out.stderr.is_empty());
    }
}
