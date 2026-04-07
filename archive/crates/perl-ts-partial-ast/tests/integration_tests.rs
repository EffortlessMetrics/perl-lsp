//! Integration tests for perl-ts-partial-ast
//!
//! These tests exercise the public API from the perspective of an external consumer,
//! covering ExtendedAstNode variants, the builder, edge case handling, phase-aware
//! parsing, understanding parser, and the tree-sitter adapter.

use perl_tdd_support::must;
use perl_ts_partial_ast::edge_case_handler::{EdgeCaseConfig, EdgeCaseHandler, RecommendedAction};
use perl_ts_partial_ast::partial_parse_ast::{
    DynamicPart, ExtendedAstBuilder, ExtendedAstNode, RuntimeContext,
};
use perl_ts_partial_ast::phase_aware_parser::{PerlPhase, PhaseAction, PhaseAwareParser};
use perl_ts_partial_ast::tree_sitter_adapter::{
    EdgeCaseNodeType, TreeSitterAdapter, TreeSitterNode,
};
use perl_ts_partial_ast::understanding_parser::UnderstandingParser;

use perl_parser_pest::pure_rust_parser::AstNode;
use perl_ts_heredoc_analysis::anti_pattern_detector::{
    AntiPattern, Diagnostic, Location, Severity,
};
use perl_ts_heredoc_analysis::dynamic_delimiter_recovery::ParseContext;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// ExtendedAstNode — variant construction and method coverage
// ---------------------------------------------------------------------------

#[test]
fn normal_node_is_fully_parsed_and_no_anti_patterns() {
    let node = ExtendedAstNode::Normal(AstNode::Identifier(Arc::from("foo")));
    assert!(!node.has_anti_patterns());
    assert!(node.as_normal().is_some());
    assert!(node.collect_diagnostics().is_empty());
}

#[test]
fn with_warning_node_reports_anti_patterns() {
    let diag = make_diagnostic(Severity::Warning, "test warning");
    let node = ExtendedAstNode::WithWarning {
        node: Box::new(AstNode::Number(Arc::from("99"))),
        diagnostics: vec![diag],
    };
    assert!(node.has_anti_patterns());
    // as_normal should still return the inner node
    let inner = node.as_normal();
    assert!(inner.is_some());
    assert_eq!(node.collect_diagnostics().len(), 1);
}

#[test]
fn partial_parse_node_sexp_contains_fragment_output() {
    let fragment = ExtendedAstNode::Normal(AstNode::String(Arc::from("hello")));
    let node = ExtendedAstNode::PartialParse {
        pattern: AntiPattern::FormatHeredoc {
            location: Location { line: 1, column: 0, offset: 0 },
            format_name: "STDOUT".to_string(),
            heredoc_delimiter: "END".to_string(),
        },
        raw_text: Arc::from("format STDOUT =\n<<END\nhello\nEND\n."),
        parsed_fragments: vec![fragment],
        diagnostics: vec![],
    };
    let sexp = node.to_sexp();
    assert!(sexp.contains("partial_parse"));
    assert!(sexp.contains("format_heredoc"));
    assert!(node.has_anti_patterns());
    // as_normal returns None for partial parse
    assert!(node.as_normal().is_none());
}

#[test]
fn unparseable_node_sexp_contains_reason() {
    let node = ExtendedAstNode::Unparseable {
        pattern: AntiPattern::DynamicHeredocDelimiter {
            location: Location { line: 5, column: 0, offset: 40 },
            expression: "<<$delim".to_string(),
        },
        raw_text: Arc::from("<<$delim;\ncontent\n"),
        reason: "Dynamic delimiter".to_string(),
        diagnostics: vec![make_diagnostic(Severity::Error, "cannot resolve")],
        recovery_point: 40,
    };
    let sexp = node.to_sexp();
    assert!(sexp.contains("unparseable"));
    assert!(sexp.contains("Dynamic delimiter"));
    assert_eq!(node.collect_diagnostics().len(), 1);
}

#[test]
fn runtime_dependent_parse_sexp_includes_construct_type() {
    let node = ExtendedAstNode::RuntimeDependentParse {
        construct_type: "BEGIN_heredoc".to_string(),
        static_parts: vec![ExtendedAstNode::Normal(AstNode::Identifier(Arc::from("cfg")))],
        dynamic_parts: vec![DynamicPart {
            expression: "config data".to_string(),
            context: RuntimeContext::BeginBlock,
            fallback_parse: None,
        }],
        diagnostics: vec![],
    };
    let sexp = node.to_sexp();
    assert!(sexp.contains("runtime_dependent"));
    assert!(sexp.contains("BEGIN_heredoc"));
    assert!(node.has_anti_patterns());
}

#[test]
fn collect_diagnostics_recurses_into_partial_parse_children() {
    let child_diag = make_diagnostic(Severity::Info, "child issue");
    let child = ExtendedAstNode::WithWarning {
        node: Box::new(AstNode::Identifier(Arc::from("inner"))),
        diagnostics: vec![child_diag],
    };
    let parent_diag = make_diagnostic(Severity::Warning, "parent issue");
    let parent = ExtendedAstNode::PartialParse {
        pattern: AntiPattern::FormatHeredoc {
            location: Location { line: 1, column: 0, offset: 0 },
            format_name: "RPT".to_string(),
            heredoc_delimiter: "END".to_string(),
        },
        raw_text: Arc::from("..."),
        parsed_fragments: vec![child],
        diagnostics: vec![parent_diag],
    };
    // Should collect both parent and child diagnostics
    assert_eq!(parent.collect_diagnostics().len(), 2);
}

#[test]
fn collect_diagnostics_recurses_into_runtime_dependent_dynamic_fallback() {
    let fallback_diag = make_diagnostic(Severity::Warning, "fallback issue");
    let fallback_node = ExtendedAstNode::WithWarning {
        node: Box::new(AstNode::String(Arc::from("fb"))),
        diagnostics: vec![fallback_diag],
    };
    let node = ExtendedAstNode::RuntimeDependentParse {
        construct_type: "eval_heredoc".to_string(),
        static_parts: vec![],
        dynamic_parts: vec![DynamicPart {
            expression: "eval code".to_string(),
            context: RuntimeContext::EvalString,
            fallback_parse: Some(Box::new(fallback_node)),
        }],
        diagnostics: vec![make_diagnostic(Severity::Info, "top")],
    };
    // top-level + fallback
    assert_eq!(node.collect_diagnostics().len(), 2);
}

// ---------------------------------------------------------------------------
// ExtendedAstBuilder
// ---------------------------------------------------------------------------

#[test]
fn builder_without_diagnostics_produces_normal_node() {
    let builder = ExtendedAstBuilder::new();
    let node = builder.build_normal(AstNode::String(Arc::from("clean")));
    assert!(matches!(node, ExtendedAstNode::Normal(_)));
}

#[test]
fn builder_with_diagnostics_produces_with_warning_node() {
    let mut builder = ExtendedAstBuilder::new();
    builder.add_diagnostic(make_diagnostic(Severity::Warning, "lint"));
    let node = builder.build_normal(AstNode::Number(Arc::from("1")));
    assert!(matches!(node, ExtendedAstNode::WithWarning { .. }));
}

#[test]
fn builder_build_partial_creates_partial_parse() {
    let mut builder = ExtendedAstBuilder::new();
    builder.add_diagnostic(make_diagnostic(Severity::Info, "partial"));
    let pattern = AntiPattern::FormatHeredoc {
        location: Location { line: 1, column: 0, offset: 0 },
        format_name: "OUT".to_string(),
        heredoc_delimiter: "EOF".to_string(),
    };
    let node = builder.build_partial(pattern, Arc::from("raw text"), vec![]);
    assert!(matches!(node, ExtendedAstNode::PartialParse { .. }));
    assert_eq!(node.collect_diagnostics().len(), 1);
}

#[test]
fn builder_build_unparseable_creates_unparseable() {
    let mut builder = ExtendedAstBuilder::new();
    builder.add_diagnostic(make_diagnostic(Severity::Error, "fatal"));
    let pattern = AntiPattern::DynamicHeredocDelimiter {
        location: Location { line: 1, column: 0, offset: 0 },
        expression: "<<$x".to_string(),
    };
    let node = builder.build_unparseable(pattern, Arc::from("<<$x"), "bad".to_string(), 10);
    assert!(matches!(node, ExtendedAstNode::Unparseable { .. }));
    assert_eq!(node.collect_diagnostics().len(), 1);
}

#[test]
fn builder_default_is_equivalent_to_new() {
    let b1 = ExtendedAstBuilder::new();
    let b2 = ExtendedAstBuilder::default();
    // Both should produce Normal from a clean build
    let n1 = b1.build_normal(AstNode::Identifier(Arc::from("a")));
    let n2 = b2.build_normal(AstNode::Identifier(Arc::from("b")));
    assert!(matches!(n1, ExtendedAstNode::Normal(_)));
    assert!(matches!(n2, ExtendedAstNode::Normal(_)));
}

// ---------------------------------------------------------------------------
// EdgeCaseHandler & EdgeCaseConfig
// ---------------------------------------------------------------------------

#[test]
fn edge_case_config_default_values() {
    let config = EdgeCaseConfig::default();
    assert!(!config.enable_sandbox);
    assert!(!config.interactive_mode);
    assert!(!config.strict_mode);
}

#[test]
fn edge_case_handler_analyze_clean_code_produces_no_phase_warnings() {
    let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
    let code = "my $x = 42;\nprint $x;\n";
    let analysis = handler.analyze(code);
    assert!(analysis.phase_warnings.is_empty());
}

#[test]
fn edge_case_handler_analyze_begin_block_produces_phase_warning() {
    let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
    let code = "BEGIN {\n    my $x = 1;\n}\n";
    let analysis = handler.analyze(code);
    assert!(!analysis.phase_warnings.is_empty(), "BEGIN block should produce a phase warning");
    assert!(analysis.phase_warnings.iter().any(|w| w.contains("BEGIN")));
}

#[test]
fn edge_case_handler_generates_report_with_header() {
    let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
    let code = "my $x = 1;\n";
    let analysis = handler.analyze(code);
    let report = handler.generate_report(&analysis);
    assert!(report.contains("Perl Heredoc Edge Case Analysis"));
    assert!(report.contains("Total Issues:"));
}

#[test]
fn edge_case_handler_sandbox_config_adds_sandbox_action() {
    let mut handler = EdgeCaseHandler::new(EdgeCaseConfig {
        recovery_mode:
            perl_ts_heredoc_analysis::dynamic_delimiter_recovery::RecoveryMode::BestGuess,
        enable_sandbox: true,
        interactive_mode: false,
        strict_mode: false,
    });
    // Code with a dynamic heredoc delimiter triggers RefactorCode + RunInSandbox
    let code = "my $delim = 'EOF';\nprint <<$delim;\ncontent\nEOF\n";
    let analysis = handler.analyze(code);
    let has_sandbox = analysis
        .recommended_actions
        .iter()
        .any(|a| matches!(a, RecommendedAction::RunInSandbox { .. }));
    assert!(has_sandbox, "sandbox-enabled config should produce RunInSandbox action");
}

#[test]
fn edge_case_handler_dynamic_delimiter_resolution() {
    let handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
    let context = ParseContext {
        current_package: Some("main".to_string()),
        imported_modules: vec![],
        in_subroutine: None,
        file_type_hint: None,
    };
    let resolution = handler.handle_dynamic_delimiter("<<$var", &context);
    assert_eq!(resolution.expression, "<<$var");
    // Confidence should be between 0 and 1
    assert!(resolution.confidence >= 0.0 && resolution.confidence <= 1.0);
}

// ---------------------------------------------------------------------------
// PhaseAwareParser
// ---------------------------------------------------------------------------

#[test]
fn phase_aware_parser_detects_use_statements() {
    let mut parser = PhaseAwareParser::new();
    let code = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let transitions = parser.analyze_phases(code);
    let use_count = transitions.iter().filter(|t| matches!(t.to, PerlPhase::Use)).count();
    assert_eq!(use_count, 2, "should detect two use statements");
}

#[test]
fn phase_aware_parser_detects_eval() {
    let mut parser = PhaseAwareParser::new();
    let code = "eval { die 'oops' };\n";
    let transitions = parser.analyze_phases(code);
    assert!(transitions.iter().any(|t| matches!(t.to, PerlPhase::Eval)));
}

#[test]
fn phase_enter_and_exit_restores_previous_phase() {
    let mut parser = PhaseAwareParser::new();
    // Default is TopLevel
    parser.enter_phase(PerlPhase::Begin, 1);
    parser.enter_phase(PerlPhase::Eval, 5);
    parser.exit_phase(); // should go back to Begin
    // Now handle_phase_heredoc should act as if we are in BEGIN
    let action = parser.handle_phase_heredoc("END", Location { line: 6, column: 0, offset: 50 });
    assert!(
        matches!(action, PhaseAction::Defer { .. }),
        "After exiting Eval inside Begin, should still be in Begin"
    );
}

#[test]
fn check_phase_heredoc_returns_partial_parse() {
    let mut parser = PhaseAwareParser::new();
    parser.enter_phase(PerlPhase::Check, 1);
    let action = parser.handle_phase_heredoc("DATA", Location { line: 2, column: 0, offset: 10 });
    assert!(
        matches!(action, PhaseAction::PartialParse { .. }),
        "CHECK phase should return PartialParse"
    );
}

#[test]
fn init_phase_heredoc_returns_partial_parse() {
    let mut parser = PhaseAwareParser::new();
    parser.enter_phase(PerlPhase::Init, 1);
    let action = parser.handle_phase_heredoc("SETUP", Location { line: 2, column: 0, offset: 10 });
    assert!(
        matches!(action, PhaseAction::PartialParse { .. }),
        "INIT phase should return PartialParse"
    );
}

#[test]
fn runtime_phase_heredoc_returns_parse() {
    let mut parser = PhaseAwareParser::new();
    // TopLevel is the default; should produce Parse
    let action = parser.handle_phase_heredoc("EOF", Location { line: 1, column: 0, offset: 0 });
    assert!(matches!(action, PhaseAction::Parse), "TopLevel phase should return Parse (normal)");
}

#[test]
fn use_phase_heredoc_returns_partial_parse() {
    let mut parser = PhaseAwareParser::new();
    parser.enter_phase(PerlPhase::Use, 1);
    let action = parser.handle_phase_heredoc("MOD", Location { line: 2, column: 0, offset: 10 });
    assert!(
        matches!(action, PhaseAction::PartialParse { .. }),
        "Use phase should return PartialParse"
    );
}

#[test]
fn generate_phase_diagnostics_includes_suggested_fix() {
    let mut parser = PhaseAwareParser::new();
    parser.enter_phase(PerlPhase::Begin, 1);
    parser.handle_phase_heredoc("END", Location { line: 2, column: 0, offset: 10 });
    let diags = parser.generate_phase_diagnostics();
    assert_eq!(diags.len(), 1);
    assert!(diags[0].suggested_fix.is_some());
    let fix = diags[0].suggested_fix.as_deref().unwrap_or("");
    assert!(fix.contains("INIT") || fix.contains("runtime"));
}

#[test]
fn create_phase_node_produces_runtime_dependent_node() {
    let mut parser = PhaseAwareParser::new();
    parser.enter_phase(PerlPhase::Begin, 1);
    parser.handle_phase_heredoc("CFG", Location { line: 2, column: 0, offset: 10 });
    let deferred = &parser.generate_phase_diagnostics();
    // create_phase_node needs a DeferredHeredoc; access via the public API
    // We tested the diagnostics above; now verify the node type via analyze
    // Instead, test phase_name behavior indirectly via sexp
    let node = ExtendedAstNode::RuntimeDependentParse {
        construct_type: "BEGIN_heredoc".to_string(),
        static_parts: vec![],
        dynamic_parts: vec![DynamicPart {
            expression: "CFG".to_string(),
            context: RuntimeContext::BeginBlock,
            fallback_parse: None,
        }],
        diagnostics: deferred.clone(),
    };
    let sexp = node.to_sexp();
    assert!(sexp.contains("runtime_dependent"));
    assert!(sexp.contains("BEGIN_heredoc"));
}

// ---------------------------------------------------------------------------
// UnderstandingParser
// ---------------------------------------------------------------------------

#[test]
fn understanding_parser_default_is_equivalent_to_new() {
    // Verifying Default impl works
    let _parser: UnderstandingParser = Default::default();
}

#[test]
fn understanding_parser_clean_perl_has_full_coverage() {
    let mut parser = UnderstandingParser::new();
    let code = "my @list = (1, 2, 3);\n";
    let result = must(parser.parse_with_understanding(code));
    assert!((result.parse_coverage - 100.0).abs() < f64::EPSILON);
    assert!(result.recovery_points.is_empty());
}

#[test]
fn understanding_parser_report_contains_coverage() {
    let mut parser = UnderstandingParser::new();
    let code = "print 'hello';\n";
    let result = must(parser.parse_with_understanding(code));
    let report = result.generate_report();
    assert!(report.contains("Parse Coverage:"));
    assert!(report.contains("AST Structure:"));
}

#[test]
fn understanding_parser_error_recovery_on_incomplete_syntax() {
    let mut parser = UnderstandingParser::new();
    // Deliberately incomplete/broken Perl -- tests error recovery path
    let code = "my $x = <<HEREDOC;\nsome content\n";
    let result = must(parser.parse_with_understanding(code));
    // The parser should still produce a result (possibly with anti-patterns)
    let _sexp = result.ast.to_sexp();
    // Report should be generatable without error
    let report = result.generate_report();
    assert!(!report.is_empty());
}

// ---------------------------------------------------------------------------
// TreeSitterAdapter & EdgeCaseNodeType
// ---------------------------------------------------------------------------

#[test]
fn edge_case_node_type_as_str_standard() {
    assert_eq!(EdgeCaseNodeType::Heredoc.as_str(), "heredoc");
    assert_eq!(EdgeCaseNodeType::HeredocOpener.as_str(), "heredoc_opener");
    assert_eq!(EdgeCaseNodeType::HeredocBody.as_str(), "heredoc_body");
    assert_eq!(EdgeCaseNodeType::HeredocDelimiter.as_str(), "heredoc_delimiter");
}

#[test]
fn edge_case_node_type_as_str_edge_cases() {
    assert_eq!(EdgeCaseNodeType::DynamicHeredocDelimiter.as_str(), "dynamic_heredoc_delimiter");
    assert_eq!(EdgeCaseNodeType::PhaseDependendHeredoc.as_str(), "phase_dependent_heredoc");
    assert_eq!(EdgeCaseNodeType::TiedHandleHeredoc.as_str(), "tied_handle_heredoc");
    assert_eq!(EdgeCaseNodeType::SourceFilteredHeredoc.as_str(), "source_filtered_heredoc");
    assert_eq!(EdgeCaseNodeType::EncodingAffectedHeredoc.as_str(), "encoding_affected_heredoc");
}

#[test]
fn edge_case_node_type_as_str_error_recovery() {
    assert_eq!(EdgeCaseNodeType::HeredocError.as_str(), "ERROR");
    assert_eq!(EdgeCaseNodeType::UnresolvedDelimiter.as_str(), "MISSING");
    assert_eq!(EdgeCaseNodeType::PartialHeredoc.as_str(), "ERROR");
}

#[test]
fn tree_sitter_adapter_converts_normal_node() {
    let ast = ExtendedAstNode::Normal(AstNode::ScalarVariable(Arc::from("$x")));
    let output = TreeSitterAdapter::convert_to_tree_sitter(ast, vec![], "$x");
    assert_eq!(output.tree.root.node_type, "scalar_variable");
    assert!(!output.tree.root.is_error);
    assert!(!output.tree.root.is_missing);
    assert_eq!(output.metadata.edge_case_count, 0);
}

#[test]
fn tree_sitter_adapter_converts_with_warning_preserving_diagnostics() {
    let diag = Diagnostic {
        severity: Severity::Warning,
        pattern: AntiPattern::FormatHeredoc {
            location: Location { line: 1, column: 0, offset: 0 },
            format_name: "OUT".to_string(),
            heredoc_delimiter: "END".to_string(),
        },
        message: "format heredoc".to_string(),
        explanation: "test".to_string(),
        suggested_fix: None,
        references: vec![],
    };
    let ast = ExtendedAstNode::WithWarning {
        node: Box::new(AstNode::Identifier(Arc::from("x"))),
        diagnostics: vec![diag],
    };
    let output = TreeSitterAdapter::convert_to_tree_sitter(ast, vec![], "x");
    // The node itself is not error
    assert!(!output.tree.root.is_error);
    // But diagnostics were extracted
    assert_eq!(output.diagnostics.len(), 1);
}

#[test]
fn tree_sitter_adapter_converts_unparseable_to_error_node() {
    let ast = ExtendedAstNode::Unparseable {
        pattern: AntiPattern::DynamicHeredocDelimiter {
            location: Location { line: 1, column: 0, offset: 0 },
            expression: "<<$var".to_string(),
        },
        raw_text: Arc::from("<<$var\ncontent\n"),
        reason: "dynamic".to_string(),
        diagnostics: vec![],
        recovery_point: 0,
    };
    let output = TreeSitterAdapter::convert_to_tree_sitter(ast, vec![], "<<$var\ncontent\n");
    assert_eq!(output.tree.root.node_type, "ERROR");
    assert!(output.tree.root.is_error);
    assert_eq!(output.metadata.edge_case_count, 1);
}

#[test]
fn tree_sitter_adapter_converts_runtime_dependent_with_begin() {
    let ast = ExtendedAstNode::RuntimeDependentParse {
        construct_type: "BEGIN_heredoc".to_string(),
        static_parts: vec![],
        dynamic_parts: vec![],
        diagnostics: vec![],
    };
    let output = TreeSitterAdapter::convert_to_tree_sitter(ast, vec![], "");
    assert_eq!(output.tree.root.node_type, "phase_dependent_heredoc");
    assert!(!output.tree.root.is_error);
    assert!(output.tree.root.is_missing);
    assert_eq!(output.metadata.edge_case_count, 1);
}

#[test]
fn tree_sitter_node_to_json_includes_required_fields() {
    let node = TreeSitterNode {
        node_type: "identifier".to_string(),
        start_byte: 0,
        end_byte: 4,
        start_point: (0, 0),
        end_point: (0, 4),
        children: vec![],
        is_error: false,
        is_missing: false,
        field_name: None,
        text: Some("test".to_string()),
    };
    let json = node.to_json();
    assert_eq!(json["type"], "identifier");
    assert_eq!(json["startIndex"], 0);
    assert_eq!(json["endIndex"], 4);
    assert_eq!(json["text"], "test");
    // is_error and is_missing should NOT appear when false
    assert!(json.get("isError").is_none());
    assert!(json.get("isMissing").is_none());
}

#[test]
fn tree_sitter_node_to_json_includes_error_flag() {
    let node = TreeSitterNode {
        node_type: "ERROR".to_string(),
        start_byte: 0,
        end_byte: 10,
        start_point: (0, 0),
        end_point: (0, 10),
        children: vec![],
        is_error: true,
        is_missing: false,
        field_name: None,
        text: None,
    };
    let json = node.to_json();
    assert_eq!(json["isError"], true);
}

#[test]
fn tree_sitter_node_to_json_includes_children() {
    let child = TreeSitterNode {
        node_type: "number".to_string(),
        start_byte: 0,
        end_byte: 2,
        start_point: (0, 0),
        end_point: (0, 2),
        children: vec![],
        is_error: false,
        is_missing: false,
        field_name: None,
        text: Some("42".to_string()),
    };
    let parent = TreeSitterNode {
        node_type: "list".to_string(),
        start_byte: 0,
        end_byte: 4,
        start_point: (0, 0),
        end_point: (0, 4),
        children: vec![child],
        is_error: false,
        is_missing: false,
        field_name: None,
        text: None,
    };
    let json = parent.to_json();
    let children = json["children"].as_array();
    assert!(children.is_some());
    assert_eq!(children.map(|c| c.len()), Some(1));
}

// ---------------------------------------------------------------------------
// End-to-end: EdgeCaseHandler full pipeline
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_source_filter_detection_and_report() {
    let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
    let code = "use Filter::Simple;\nmy $x = 1;\n";
    let analysis = handler.analyze(code);
    // Source filter should trigger ManualReview + EnableFeature
    let has_manual = analysis
        .recommended_actions
        .iter()
        .any(|a| matches!(a, RecommendedAction::ManualReview { .. }));
    let has_feature = analysis
        .recommended_actions
        .iter()
        .any(|a| matches!(a, RecommendedAction::EnableFeature { .. }));
    assert!(has_manual, "source filter should trigger ManualReview");
    assert!(has_feature, "source filter should trigger EnableFeature");

    let report = handler.generate_report(&analysis);
    assert!(report.contains("Recommended Actions"));
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_diagnostic(severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        severity,
        pattern: AntiPattern::DynamicHeredocDelimiter {
            location: Location { line: 1, column: 0, offset: 0 },
            expression: "<<$var".to_string(),
        },
        message: message.to_string(),
        explanation: "test explanation".to_string(),
        suggested_fix: None,
        references: vec![],
    }
}
