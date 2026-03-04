//! Extended unit tests for perl-lsp-tooling crate.
//!
//! These tests supplement comprehensive_unit_tests.rs with additional coverage for:
//! - Boundary conditions and edge cases in tool configuration
//! - Subprocess mock interaction patterns
//! - Perltidy/perlcritic argument generation corner cases
//! - Error propagation and recovery paths
//! - Incremental parser merge algorithms
//! - Symbol index tokenization edge cases
//! - Parallel processing with various workloads

use perl_lsp_tooling::mock::{MockResponse, MockSubprocessRuntime};
use perl_lsp_tooling::performance::parallel::process_files_parallel;
use perl_lsp_tooling::performance::{AstCache, IncrementalParser, SymbolIndex};
use perl_lsp_tooling::perl_critic::{
    BuiltInAnalyzer, CriticAnalyzer, CriticConfig, QuickFix, Severity, TextEdit, Violation,
};
use perl_lsp_tooling::perltidy::{BuiltInFormatter, PerlTidyConfig, PerlTidyFormatter};
use perl_parser_core::position::{Position, Range};
use perl_parser_core::{Node, NodeKind, SourceLocation};
use std::path::Path;
use std::sync::Arc;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn program_node() -> Node {
    Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 })
}

fn mock_runtime_with_success(stdout: &[u8]) -> Arc<MockSubprocessRuntime> {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(stdout.to_vec()));
    rt
}

fn mock_runtime_with_failure(stderr: &[u8], code: i32) -> Arc<MockSubprocessRuntime> {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::failure(stderr.to_vec(), code));
    rt
}

fn default_formatter(rt: Arc<MockSubprocessRuntime>) -> PerlTidyFormatter {
    PerlTidyFormatter::new(PerlTidyConfig::default(), rt)
}

fn default_analyzer(rt: Arc<MockSubprocessRuntime>) -> CriticAnalyzer {
    CriticAnalyzer::new(CriticConfig::default(), rt)
}

// ═══════════════════════════════════════════════════════════════════════════════
// PerlTidyConfig — argument generation edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn config_block_comment_indentation_nonzero() {
    let config = PerlTidyConfig { block_comment_indentation: Some(3), ..PerlTidyConfig::default() };
    let rt = mock_runtime_with_success(b"ok\n");
    let mut fmt = PerlTidyFormatter::new(config, rt.clone());
    let _ = fmt.format("x");
    let inv = rt.invocations();
    assert!(inv[0].args.iter().any(|a| a == "--block-comment-indentation=3"));
}

#[test]
fn config_profile_suppresses_all_other_args() {
    let config = PerlTidyConfig {
        maximum_line_length: Some(120),
        indent_columns: Some(8),
        tabs: Some(true),
        profile: Some("/my/rc".to_string()),
        ..PerlTidyConfig::default()
    };
    let rt = mock_runtime_with_success(b"ok\n");
    let mut fmt = PerlTidyFormatter::new(config, rt.clone());
    let _ = fmt.format("x");
    let args = &rt.invocations()[0].args;
    // Only profile and -st should be present, no --maximum-line-length etc.
    assert!(args.iter().any(|a| a == "--profile=/my/rc"));
    assert!(!args.iter().any(|a| a.starts_with("--maximum-line-length")));
    assert!(!args.iter().any(|a| a == "--tabs"));
    assert!(!args.iter().any(|a| a.starts_with("--indent-columns")));
}

#[test]
fn config_all_booleans_true_generates_positive_flags() {
    let config = PerlTidyConfig {
        tabs: Some(true),
        opening_brace_on_new_line: Some(true),
        cuddled_else: Some(true),
        space_after_keyword: Some(true),
        add_trailing_commas: Some(true),
        vertical_alignment: Some(true),
        profile: None,
        ..PerlTidyConfig::default()
    };
    let rt = mock_runtime_with_success(b"ok\n");
    let mut fmt = PerlTidyFormatter::new(config, rt.clone());
    let _ = fmt.format("x");
    let args = &rt.invocations()[0].args;
    assert!(args.contains(&"--tabs".to_string()));
    assert!(args.contains(&"--opening-brace-on-new-line".to_string()));
    assert!(args.contains(&"--cuddled-else".to_string()));
    assert!(args.contains(&"--space-after-keyword".to_string()));
    assert!(args.contains(&"--add-trailing-commas".to_string()));
    assert!(args.contains(&"--vertical-alignment".to_string()));
}

#[test]
fn config_all_booleans_false_generates_negative_flags() {
    let config = PerlTidyConfig {
        tabs: Some(false),
        opening_brace_on_new_line: Some(false),
        cuddled_else: Some(false),
        space_after_keyword: Some(false),
        add_trailing_commas: Some(false),
        vertical_alignment: Some(false),
        profile: None,
        ..PerlTidyConfig::default()
    };
    let rt = mock_runtime_with_success(b"ok\n");
    let mut fmt = PerlTidyFormatter::new(config, rt.clone());
    let _ = fmt.format("x");
    let args = &rt.invocations()[0].args;
    assert!(args.contains(&"--notabs".to_string()));
    assert!(args.contains(&"--opening-brace-always-on-right".to_string()));
    assert!(args.contains(&"--nocuddled-else".to_string()));
    assert!(args.contains(&"--nospace-after-keyword".to_string()));
    assert!(args.contains(&"--no-add-trailing-commas".to_string()));
    assert!(args.contains(&"--no-vertical-alignment".to_string()));
}

#[test]
fn config_pbp_cuddled_else_is_false() {
    let config = PerlTidyConfig::pbp();
    assert_eq!(config.cuddled_else, Some(false));
    assert_eq!(config.add_trailing_commas, Some(true));
}

#[test]
fn config_gnu_indent_columns_is_two() {
    let config = PerlTidyConfig::gnu();
    assert_eq!(config.indent_columns, Some(2));
    assert_eq!(config.opening_brace_on_new_line, Some(true));
    assert_eq!(config.vertical_alignment, Some(false));
}

#[test]
fn config_gnu_has_gnu_style_extra_arg() {
    let config = PerlTidyConfig::gnu();
    assert!(config.extra_args.contains(&"--gnu-style".to_string()));
}

#[test]
fn config_extra_args_appended_after_flags() {
    let config = PerlTidyConfig {
        extra_args: vec!["--custom-flag".to_string(), "--another".to_string()],
        profile: None,
        ..PerlTidyConfig::default()
    };
    let rt = mock_runtime_with_success(b"ok\n");
    let mut fmt = PerlTidyFormatter::new(config, rt.clone());
    let _ = fmt.format("x");
    let args = &rt.invocations()[0].args;
    assert!(args.contains(&"--custom-flag".to_string()));
    assert!(args.contains(&"--another".to_string()));
}

#[test]
fn config_maximum_line_length_zero() {
    let config =
        PerlTidyConfig { maximum_line_length: Some(0), profile: None, ..PerlTidyConfig::default() };
    let rt = mock_runtime_with_success(b"ok\n");
    let mut fmt = PerlTidyFormatter::new(config, rt.clone());
    let _ = fmt.format("x");
    let args = &rt.invocations()[0].args;
    assert!(args.contains(&"--maximum-line-length=0".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// PerlTidyFormatter — format, format_file, format_range, get_suggestions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn formatter_stdin_contains_input_code() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"formatted\n");
    let mut fmt = default_formatter(rt.clone());
    fmt.format("my $input = 42;")?;
    let inv = &rt.invocations()[0];
    assert_eq!(inv.stdin.as_deref(), Some(b"my $input = 42;" as &[u8]));
    Ok(())
}

#[test]
fn formatter_returns_exact_stdout() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"exact output");
    let mut fmt = default_formatter(rt);
    let result = fmt.format("code")?;
    assert_eq!(result, "exact output");
    Ok(())
}

#[test]
fn formatter_cache_hit_does_not_invoke_runtime() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"out\n");
    let mut fmt = default_formatter(rt.clone());
    fmt.format("code")?;
    fmt.format("code")?;
    fmt.format("code")?;
    assert_eq!(rt.invocations().len(), 1);
    Ok(())
}

#[test]
fn formatter_different_code_not_cached() -> Result<(), String> {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(b"a\n".to_vec()));
    rt.add_response(MockResponse::success(b"b\n".to_vec()));
    let mut fmt = default_formatter(rt.clone());
    let r1 = fmt.format("code_a")?;
    let r2 = fmt.format("code_b")?;
    assert_ne!(r1, r2);
    assert_eq!(rt.invocations().len(), 2);
    Ok(())
}

#[test]
fn formatter_clear_cache_allows_reinvocation() -> Result<(), String> {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(b"first\n".to_vec()));
    rt.add_response(MockResponse::success(b"second\n".to_vec()));
    let mut fmt = default_formatter(rt.clone());
    let r1 = fmt.format("code")?;
    fmt.clear_cache();
    let r2 = fmt.format("code")?;
    assert_eq!(r1, "first\n");
    assert_eq!(r2, "second\n");
    assert_eq!(rt.invocations().len(), 2);
    Ok(())
}

#[test]
fn formatter_error_on_runtime_failure() {
    let rt = mock_runtime_with_failure(b"parse error near line 5", 2);
    let mut fmt = default_formatter(rt);
    let result = fmt.format("bad code");
    assert!(result.is_err());
}

#[test]
fn formatter_error_message_contains_stderr() {
    let rt = mock_runtime_with_failure(b"unexpected token", 1);
    let mut fmt = default_formatter(rt);
    let err = fmt.format("bad").err();
    assert!(err.is_some());
    let msg = err.unwrap_or_default();
    assert!(msg.contains("unexpected token"));
}

#[test]
fn formatter_format_file_passes_filename() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let fmt = PerlTidyFormatter::new(PerlTidyConfig::default(), rt.clone());
    fmt.format_file(Path::new("/tmp/my_script.pl"))?;
    let args = &rt.invocations()[0].args;
    assert!(args.iter().any(|a| a == "/tmp/my_script.pl"));
    Ok(())
}

#[test]
fn formatter_format_file_no_stdin_sent() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let fmt = PerlTidyFormatter::new(PerlTidyConfig::default(), rt.clone());
    fmt.format_file(Path::new("test.pl"))?;
    assert!(rt.invocations()[0].stdin.is_none());
    Ok(())
}

#[test]
fn formatter_format_file_includes_dash_dash_separator() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let fmt = PerlTidyFormatter::new(PerlTidyConfig::default(), rt.clone());
    fmt.format_file(Path::new("-dangerous-name.pl"))?;
    let args = &rt.invocations()[0].args;
    let sep_idx = args.iter().position(|a| a == "--");
    let file_idx = args.iter().position(|a| a == "-dangerous-name.pl");
    assert!(sep_idx.is_some());
    assert!(file_idx.is_some());
    // separator must precede file path
    assert!(sep_idx < file_idx);
    Ok(())
}

#[test]
fn formatter_format_file_error_on_failure() {
    let rt = mock_runtime_with_failure(b"file not found", 1);
    let fmt = PerlTidyFormatter::new(PerlTidyConfig::default(), rt);
    let result = fmt.format_file(Path::new("nonexistent.pl"));
    assert!(result.is_err());
}

#[test]
fn formatter_format_range_single_line() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"    formatted_line");
    let mut fmt = default_formatter(rt);
    let code = "line0\nline1\nline2\nline3";
    let result = fmt.format_range(code, 1, 1)?;
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "line0");
    assert_eq!(lines[1], "    formatted_line");
    assert_eq!(lines[2], "line2");
    assert_eq!(lines[3], "line3");
    Ok(())
}

#[test]
fn formatter_format_range_start_equals_zero() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"new_first\nnew_second");
    let mut fmt = default_formatter(rt);
    let code = "first\nsecond\nthird";
    let result = fmt.format_range(code, 0, 1)?;
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "new_first");
    assert_eq!(lines[1], "new_second");
    assert_eq!(lines[2], "third");
    Ok(())
}

#[test]
fn formatter_format_range_end_at_last_line() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"new_last");
    let mut fmt = default_formatter(rt);
    let code = "first\nsecond\nthird";
    let result = fmt.format_range(code, 2, 2)?;
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "first");
    assert_eq!(lines[1], "second");
    assert_eq!(lines[2], "new_last");
    Ok(())
}

#[test]
fn formatter_format_range_start_out_of_bounds() {
    let rt = mock_runtime_with_success(b"x");
    let mut fmt = default_formatter(rt);
    let result = fmt.format_range("one\ntwo", 5, 6);
    assert!(result.is_err());
}

#[test]
fn formatter_format_range_end_out_of_bounds() {
    let rt = mock_runtime_with_success(b"x");
    let mut fmt = default_formatter(rt);
    let result = fmt.format_range("one\ntwo", 0, 10);
    assert!(result.is_err());
}

#[test]
fn formatter_get_suggestions_identical_code() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"my $x = 1;\n");
    let mut fmt = default_formatter(rt);
    let suggestions = fmt.get_suggestions("my $x = 1;\n")?;
    assert!(suggestions.is_empty());
    Ok(())
}

#[test]
fn formatter_get_suggestions_detects_line_change() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"my $x = 1;\nmy $y = 2;\n");
    let mut fmt = default_formatter(rt);
    let suggestions = fmt.get_suggestions("my $x=1;\nmy $y=2;\n")?;
    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0].line, 0);
    assert_eq!(suggestions[0].original, "my $x=1;");
    assert_eq!(suggestions[0].formatted, "my $x = 1;");
    assert_eq!(suggestions[1].line, 1);
    Ok(())
}

#[test]
fn formatter_get_suggestions_description_populated() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"changed\n");
    let mut fmt = default_formatter(rt);
    let suggestions = fmt.get_suggestions("original\n")?;
    assert_eq!(suggestions.len(), 1);
    assert!(!suggestions[0].description.is_empty());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// BuiltInFormatter — formatting behavior
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn builtin_formatter_indentation_after_open_brace() {
    let fmt = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "sub foo {\nreturn 1;\n}\n";
    let result = fmt.format(code);
    assert!(result.contains("    return 1;"));
}

#[test]
fn builtin_formatter_tab_mode() {
    let config = PerlTidyConfig { tabs: Some(true), ..PerlTidyConfig::default() };
    let fmt = BuiltInFormatter::new(config);
    let code = "if ($x) {\nprint;\n}\n";
    let result = fmt.format(code);
    assert!(result.contains("\tprint;"));
}

#[test]
fn builtin_formatter_custom_indent_two_spaces() {
    let config = PerlTidyConfig { indent_columns: Some(2), ..PerlTidyConfig::default() };
    let fmt = BuiltInFormatter::new(config);
    let code = "if ($x) {\nreturn;\n}\n";
    let result = fmt.format(code);
    assert!(result.contains("  return;"));
    assert!(!result.contains("    return;"));
}

#[test]
fn builtin_formatter_indent_saturates_at_zero() {
    let fmt = BuiltInFormatter::new(PerlTidyConfig::default());
    // Start with closing brace — indent should not go negative
    let code = "}\n}\nprint;\n";
    let result = fmt.format(code);
    // "print;" should be at indent level 0
    for line in result.lines() {
        let trimmed = line.trim();
        if trimmed == "print;" {
            assert_eq!(line, "print;");
        }
    }
}

#[test]
fn builtin_formatter_preserves_empty_lines() {
    let fmt = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "line1\n\nline2\n";
    let result = fmt.format(code);
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1], "");
}

#[test]
fn builtin_formatter_nested_two_levels() {
    let fmt = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "if ($a) {\nif ($b) {\ndeep;\n}\n}\n";
    let result = fmt.format(code);
    assert!(result.contains("        deep;"));
}

#[test]
fn builtin_formatter_paren_indentation() {
    let fmt = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "my @arr = (\n1,\n2,\n);\n";
    let result = fmt.format(code);
    assert!(result.contains("    1,"));
    assert!(result.contains("    2,"));
}

#[test]
fn builtin_formatter_empty_string() {
    let fmt = BuiltInFormatter::new(PerlTidyConfig::default());
    let result = fmt.format("");
    assert!(result.is_empty());
}

#[test]
fn builtin_formatter_single_line_no_newline() {
    let fmt = BuiltInFormatter::new(PerlTidyConfig::default());
    let result = fmt.format("print 1;");
    assert_eq!(result, "print 1;\n");
}

#[test]
fn builtin_formatter_defaults_to_four_space_indent() {
    let config = PerlTidyConfig { tabs: None, indent_columns: None, ..PerlTidyConfig::default() };
    let fmt = BuiltInFormatter::new(config);
    let code = "sub f {\nbody;\n}\n";
    let result = fmt.format(code);
    assert!(result.contains("    body;"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic — Severity
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn severity_from_number_boundary_values() {
    assert_eq!(Severity::from_number(0), Severity::Harsh);
    assert_eq!(Severity::from_number(6), Severity::Harsh);
    assert_eq!(Severity::from_number(255), Severity::Harsh);
}

#[test]
fn severity_eq_reflexive() {
    assert_eq!(Severity::Brutal, Severity::Brutal);
    assert_eq!(Severity::Cruel, Severity::Cruel);
    assert_eq!(Severity::Harsh, Severity::Harsh);
    assert_eq!(Severity::Stern, Severity::Stern);
    assert_eq!(Severity::Gentle, Severity::Gentle);
}

#[test]
fn severity_ne_across_variants() {
    assert_ne!(Severity::Brutal, Severity::Gentle);
    assert_ne!(Severity::Cruel, Severity::Stern);
}

#[cfg(feature = "lsp-compat")]
#[test]
fn severity_diagnostic_mapping_brutal_is_error() {
    assert_eq!(Severity::Brutal.to_diagnostic_severity(), lsp_types::DiagnosticSeverity::ERROR);
}

#[cfg(feature = "lsp-compat")]
#[test]
fn severity_diagnostic_mapping_harsh_is_warning() {
    assert_eq!(Severity::Harsh.to_diagnostic_severity(), lsp_types::DiagnosticSeverity::WARNING);
}

#[cfg(feature = "lsp-compat")]
#[test]
fn severity_diagnostic_mapping_gentle_is_information() {
    assert_eq!(
        Severity::Gentle.to_diagnostic_severity(),
        lsp_types::DiagnosticSeverity::INFORMATION
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic — CriticConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn critic_config_default_profile_is_none() {
    let config = CriticConfig::default();
    assert!(config.profile.is_none());
    assert!(config.theme.is_none());
    assert!(config.include.is_empty());
    assert!(config.exclude.is_empty());
    assert!(!config.verbose);
    assert!(!config.color);
}

#[test]
fn critic_config_severity_range() {
    let config = CriticConfig { severity: 1, ..CriticConfig::default() };
    assert_eq!(config.severity, 1);
    let config = CriticConfig { severity: 5, ..CriticConfig::default() };
    assert_eq!(config.severity, 5);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic — CriticAnalyzer
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn analyzer_empty_output_yields_no_violations() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let mut analyzer = default_analyzer(rt);
    let violations = analyzer.analyze_file(Path::new("clean.pl"))?;
    assert!(violations.is_empty());
    Ok(())
}

#[test]
fn analyzer_whitespace_only_output_yields_no_violations() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"  \n\n  \n");
    let mut analyzer = default_analyzer(rt);
    let violations = analyzer.analyze_file(Path::new("clean.pl"))?;
    assert!(violations.is_empty());
    Ok(())
}

#[test]
fn analyzer_malformed_line_skipped() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"not:enough:fields\n");
    let mut analyzer = default_analyzer(rt);
    let violations = analyzer.analyze_file(Path::new("test.pl"))?;
    assert!(violations.is_empty());
    Ok(())
}

#[test]
fn analyzer_parses_multiple_violations() -> Result<(), String> {
    let output = b"t.pl:1:1:3:Pol1:msg1\nt.pl:5:3:1:Pol2:msg2\n";
    let rt = mock_runtime_with_success(output);
    let mut analyzer = default_analyzer(rt);
    let violations = analyzer.analyze_file(Path::new("t.pl"))?;
    assert_eq!(violations.len(), 2);
    assert_eq!(violations[0].policy, "Pol1");
    assert_eq!(violations[0].range.start.line, 0);
    assert_eq!(violations[1].policy, "Pol2");
    assert_eq!(violations[1].range.start.line, 4);
    assert_eq!(violations[1].range.start.column, 2);
    Ok(())
}

#[test]
fn analyzer_violation_severity_parsed_correctly() -> Result<(), String> {
    let output = b"f.pl:1:1:1:P:brutal\nf.pl:2:1:5:P:gentle\n";
    let rt = mock_runtime_with_success(output);
    let mut analyzer = default_analyzer(rt);
    let violations = analyzer.analyze_file(Path::new("f.pl"))?;
    assert_eq!(violations[0].severity, Severity::Brutal);
    assert_eq!(violations[1].severity, Severity::Gentle);
    Ok(())
}

#[test]
fn analyzer_caching_returns_same_violations() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"f.pl:1:1:3:P:msg\n");
    let mut analyzer = default_analyzer(rt.clone());
    let v1 = analyzer.analyze_file(Path::new("f.pl"))?;
    let v2 = analyzer.analyze_file(Path::new("f.pl"))?;
    assert_eq!(v1.len(), v2.len());
    assert_eq!(v1[0].policy, v2[0].policy);
    assert_eq!(rt.invocations().len(), 1);
    Ok(())
}

#[test]
fn analyzer_invalidate_cache_forces_rerun() -> Result<(), String> {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(b"f.pl:1:1:3:P:msg\n".to_vec()));
    rt.add_response(MockResponse::success(b"".to_vec()));
    let mut analyzer = default_analyzer(rt.clone());
    let v1 = analyzer.analyze_file(Path::new("f.pl"))?;
    assert_eq!(v1.len(), 1);
    analyzer.invalidate_cache("f.pl");
    let v2 = analyzer.analyze_file(Path::new("f.pl"))?;
    assert!(v2.is_empty());
    assert_eq!(rt.invocations().len(), 2);
    Ok(())
}

#[test]
fn analyzer_different_files_cached_independently() -> Result<(), String> {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(b"a.pl:1:1:3:P:msg_a\n".to_vec()));
    rt.add_response(MockResponse::success(b"b.pl:2:1:1:Q:msg_b\n".to_vec()));
    let mut analyzer = default_analyzer(rt.clone());
    let va = analyzer.analyze_file(Path::new("a.pl"))?;
    let vb = analyzer.analyze_file(Path::new("b.pl"))?;
    assert_eq!(va[0].policy, "P");
    assert_eq!(vb[0].policy, "Q");
    assert_eq!(rt.invocations().len(), 2);
    Ok(())
}

#[test]
fn analyzer_passes_severity_arg() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let config = CriticConfig { severity: 1, ..CriticConfig::default() };
    let mut analyzer = CriticAnalyzer::new(config, rt.clone());
    analyzer.analyze_file(Path::new("x.pl"))?;
    assert!(rt.invocations()[0].args.contains(&"--severity=1".to_string()));
    Ok(())
}

#[test]
fn analyzer_passes_theme_arg() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let config = CriticConfig { theme: Some("bugs".to_string()), ..CriticConfig::default() };
    let mut analyzer = CriticAnalyzer::new(config, rt.clone());
    analyzer.analyze_file(Path::new("x.pl"))?;
    assert!(rt.invocations()[0].args.contains(&"--theme=bugs".to_string()));
    Ok(())
}

#[test]
fn analyzer_passes_include_and_exclude() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let config = CriticConfig {
        include: vec!["PolicyA".to_string(), "PolicyB".to_string()],
        exclude: vec!["PolicyC".to_string()],
        ..CriticConfig::default()
    };
    let mut analyzer = CriticAnalyzer::new(config, rt.clone());
    analyzer.analyze_file(Path::new("x.pl"))?;
    let args = &rt.invocations()[0].args;
    assert!(args.contains(&"--include=PolicyA".to_string()));
    assert!(args.contains(&"--include=PolicyB".to_string()));
    assert!(args.contains(&"--exclude=PolicyC".to_string()));
    Ok(())
}

#[test]
fn analyzer_includes_verbose_format() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let mut analyzer = default_analyzer(rt.clone());
    analyzer.analyze_file(Path::new("x.pl"))?;
    let args = &rt.invocations()[0].args;
    assert!(args.iter().any(|a| a.starts_with("--verbose=")));
    Ok(())
}

#[test]
fn analyzer_argument_injection_protection() -> Result<(), String> {
    let rt = mock_runtime_with_success(b"");
    let mut analyzer = default_analyzer(rt.clone());
    analyzer.analyze_file(Path::new("--evil-flag"))?;
    let args = &rt.invocations()[0].args;
    let sep_pos = args.iter().position(|a| a == "--");
    let file_pos = args.iter().position(|a| a == "--evil-flag");
    assert!(sep_pos.is_some());
    assert!(file_pos.is_some());
    assert!(sep_pos < file_pos);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic — BuiltInAnalyzer
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn builtin_analyzer_no_strict_no_warnings() {
    let analyzer = BuiltInAnalyzer::new();
    let ast = program_node();
    let violations = analyzer.analyze(&ast, "print 1;");
    assert_eq!(violations.len(), 2);
    let policies: Vec<&str> = violations.iter().map(|v| v.policy.as_str()).collect();
    assert!(policies.contains(&"TestingAndDebugging::RequireUseStrict"));
    assert!(policies.contains(&"TestingAndDebugging::RequireUseWarnings"));
}

#[test]
fn builtin_analyzer_strict_only() {
    let analyzer = BuiltInAnalyzer::new();
    let ast = program_node();
    let violations = analyzer.analyze(&ast, "use strict;\nprint 1;");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].policy, "TestingAndDebugging::RequireUseWarnings");
}

#[test]
fn builtin_analyzer_warnings_only() {
    let analyzer = BuiltInAnalyzer::new();
    let ast = program_node();
    let violations = analyzer.analyze(&ast, "use warnings;\nprint 1;");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].policy, "TestingAndDebugging::RequireUseStrict");
}

#[test]
fn builtin_analyzer_both_pragmas_no_violations() {
    let analyzer = BuiltInAnalyzer::new();
    let ast = program_node();
    let violations = analyzer.analyze(&ast, "use strict;\nuse warnings;\nprint 1;");
    assert!(violations.is_empty());
}

#[test]
fn builtin_analyzer_quick_fix_for_strict() {
    let analyzer = BuiltInAnalyzer::new();
    let violation = Violation {
        policy: "TestingAndDebugging::RequireUseStrict".to_string(),
        description: String::new(),
        explanation: String::new(),
        severity: Severity::Harsh,
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        file: String::new(),
    };
    let fix = analyzer.get_quick_fix(&violation, "");
    assert!(fix.is_some());
    let fix = fix.unwrap_or_else(|| QuickFix {
        title: String::new(),
        edit: TextEdit {
            range: Range {
                start: Position { byte: 0, line: 0, column: 0 },
                end: Position { byte: 0, line: 0, column: 0 },
            },
            new_text: String::new(),
        },
    });
    assert!(fix.edit.new_text.contains("use strict"));
}

#[test]
fn builtin_analyzer_quick_fix_for_warnings() {
    let analyzer = BuiltInAnalyzer::new();
    let violation = Violation {
        policy: "TestingAndDebugging::RequireUseWarnings".to_string(),
        description: String::new(),
        explanation: String::new(),
        severity: Severity::Harsh,
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        file: String::new(),
    };
    let fix = analyzer.get_quick_fix(&violation, "");
    assert!(fix.is_some());
}

#[test]
fn builtin_analyzer_no_quick_fix_for_unknown() {
    let analyzer = BuiltInAnalyzer::new();
    let violation = Violation {
        policy: "Unknown::Policy".to_string(),
        description: String::new(),
        explanation: String::new(),
        severity: Severity::Harsh,
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        file: String::new(),
    };
    assert!(analyzer.get_quick_fix(&violation, "").is_none());
}

#[test]
fn builtin_analyzer_default_equals_new() {
    let a = BuiltInAnalyzer::default();
    let b = BuiltInAnalyzer::new();
    let ast = program_node();
    let va = a.analyze(&ast, "print 1;");
    let vb = b.analyze(&ast, "print 1;");
    assert_eq!(va.len(), vb.len());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic — Diagnostics (lsp-compat)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "lsp-compat")]
#[test]
fn analyzer_to_diagnostics_maps_range() {
    let rt = mock_runtime_with_success(b"");
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), rt);
    let violations = vec![Violation {
        policy: "P".to_string(),
        description: "desc".to_string(),
        explanation: String::new(),
        severity: Severity::Harsh,
        range: Range {
            start: Position { byte: 0, line: 5, column: 10 },
            end: Position { byte: 0, line: 5, column: 15 },
        },
        file: "f.pl".to_string(),
    }];
    let diags = analyzer.to_diagnostics(&violations);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].range.start.line, 5);
    assert_eq!(diags[0].range.start.character, 10);
    assert_eq!(diags[0].range.end.character, 15);
}

#[cfg(feature = "lsp-compat")]
#[test]
fn analyzer_diagnostics_source_is_perlcritic() {
    let rt = mock_runtime_with_success(b"");
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), rt);
    let violations = vec![Violation {
        policy: "P".to_string(),
        description: "d".to_string(),
        explanation: String::new(),
        severity: Severity::Gentle,
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        file: String::new(),
    }];
    let diags = analyzer.to_diagnostics(&violations);
    assert_eq!(diags[0].source.as_deref(), Some("perlcritic"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance — AstCache
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ast_cache_get_nonexistent_key_returns_none() {
    let cache = AstCache::new(100, 3600);
    assert!(cache.get("never_stored.pl", "content").is_none());
}

#[test]
fn ast_cache_put_and_get_roundtrip() {
    let cache = AstCache::new(100, 3600);
    let ast = Arc::new(program_node());
    cache.put("test.pl".to_string(), "hello", ast);
    assert!(cache.get("test.pl", "hello").is_some());
}

#[test]
fn ast_cache_content_change_invalidates() {
    let cache = AstCache::new(100, 3600);
    let ast = Arc::new(program_node());
    cache.put("test.pl".to_string(), "v1", ast);
    // Content changed
    assert!(cache.get("test.pl", "v2").is_none());
    // Stale entry removed — even v1 should miss now
    assert!(cache.get("test.pl", "v1").is_none());
}

#[test]
fn ast_cache_overwrite_same_uri() {
    let cache = AstCache::new(100, 3600);
    let ast1 = Arc::new(program_node());
    let ast2 = Arc::new(Node::new(
        NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 5 },
    ));
    cache.put("f.pl".to_string(), "c1", ast1);
    // Overwrite with new content hash
    cache.put("f.pl".to_string(), "c2", ast2);
    // Old content hash no longer matches
    assert!(cache.get("f.pl", "c1").is_none());
}

#[test]
fn ast_cache_cleanup_is_safe() {
    let cache = AstCache::new(10, 60);
    cache.cleanup();
    let ast = Arc::new(program_node());
    cache.put("f.pl".to_string(), "c", ast);
    cache.cleanup();
    assert!(cache.get("f.pl", "c").is_some());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance — IncrementalParser
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn incremental_parser_new_has_no_changes() {
    let parser = IncrementalParser::new();
    assert!(!parser.needs_reparse(0, 100));
}

#[test]
fn incremental_parser_default_same_as_new() {
    let p1 = IncrementalParser::new();
    let p2 = IncrementalParser::default();
    assert!(!p1.needs_reparse(0, 100));
    assert!(!p2.needs_reparse(0, 100));
}

#[test]
fn incremental_parser_single_region_overlap() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    // Overlapping
    assert!(parser.needs_reparse(5, 15));
    assert!(parser.needs_reparse(15, 25));
    assert!(parser.needs_reparse(10, 20));
    assert!(parser.needs_reparse(12, 18));
    // Not overlapping
    assert!(!parser.needs_reparse(0, 10));
    assert!(!parser.needs_reparse(20, 30));
}

#[test]
fn incremental_parser_adjacent_boundary_not_overlap() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    // Exactly touching — node_end == start means no overlap
    assert!(!parser.needs_reparse(0, 10));
    assert!(!parser.needs_reparse(20, 30));
}

#[test]
fn incremental_parser_merge_two_overlapping() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.mark_changed(15, 30);
    // Should be merged into (10, 30)
    assert!(parser.needs_reparse(10, 30));
    assert!(parser.needs_reparse(25, 35));
    assert!(!parser.needs_reparse(0, 10));
}

#[test]
fn incremental_parser_merge_fully_contained() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 50);
    parser.mark_changed(20, 30);
    // Inner region is contained; should still be one merged region
    assert!(parser.needs_reparse(10, 50));
}

#[test]
fn incremental_parser_clear_resets_all() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 100);
    parser.clear();
    assert!(!parser.needs_reparse(0, 100));
}

#[test]
fn incremental_parser_mark_after_clear() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 50);
    parser.clear();
    parser.mark_changed(60, 70);
    assert!(!parser.needs_reparse(0, 50));
    assert!(parser.needs_reparse(60, 70));
}

#[test]
fn incremental_parser_multiple_disjoint_regions() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 10);
    parser.mark_changed(50, 60);
    parser.mark_changed(100, 110);
    assert!(parser.needs_reparse(5, 15));
    assert!(!parser.needs_reparse(20, 40));
    assert!(parser.needs_reparse(55, 65));
    assert!(parser.needs_reparse(105, 115));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance — SymbolIndex
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn symbol_index_new_prefix_search_empty() {
    let idx = SymbolIndex::new();
    assert!(idx.search_prefix("anything").is_empty());
}

#[test]
fn symbol_index_new_fuzzy_search_empty() {
    let idx = SymbolIndex::new();
    assert!(idx.search_fuzzy("anything").is_empty());
}

#[test]
fn symbol_index_add_and_prefix_search() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("foo_bar".to_string());
    idx.add_symbol("foo_baz".to_string());
    idx.add_symbol("qux".to_string());
    let results = idx.search_prefix("foo");
    assert_eq!(results.len(), 2);
    assert!(results.contains(&"foo_bar".to_string()));
    assert!(results.contains(&"foo_baz".to_string()));
}

#[test]
fn symbol_index_prefix_exact_match() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("hello".to_string());
    let results = idx.search_prefix("hello");
    assert_eq!(results.len(), 1);
}

#[test]
fn symbol_index_prefix_no_match() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("hello".to_string());
    assert!(idx.search_prefix("world").is_empty());
}

#[test]
fn symbol_index_fuzzy_search_by_token() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("get_user_name".to_string());
    idx.add_symbol("set_user_age".to_string());
    let results = idx.search_fuzzy("user");
    assert!(results.contains(&"get_user_name".to_string()));
    assert!(results.contains(&"set_user_age".to_string()));
}

#[test]
fn symbol_index_fuzzy_ranking_by_token_count() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("calculate_total_amount".to_string());
    idx.add_symbol("total_count".to_string());
    // "calculate total" has 2 matching tokens for calculate_total_amount, 1 for total_count
    let results = idx.search_fuzzy("calculate total");
    assert!(!results.is_empty());
    assert_eq!(results[0], "calculate_total_amount");
}

#[test]
fn symbol_index_camel_case_tokenized() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("getUserName".to_string());
    let results = idx.search_fuzzy("user");
    assert!(results.contains(&"getUserName".to_string()));
}

#[test]
fn symbol_index_empty_prefix_returns_all() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("abc".to_string());
    idx.add_symbol("def".to_string());
    let results = idx.search_prefix("");
    assert_eq!(results.len(), 2);
}

#[test]
fn symbol_index_default_same_as_new() {
    let i1 = SymbolIndex::new();
    let i2 = SymbolIndex::default();
    assert!(i1.search_prefix("x").is_empty());
    assert!(i2.search_prefix("x").is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance — Parallel processing
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parallel_empty_input() {
    let results: Vec<i32> = process_files_parallel(vec![], 4, |_| 0);
    assert!(results.is_empty());
}

#[test]
fn parallel_single_item() {
    let results = process_files_parallel(vec!["only.pl".to_string()], 2, |f| f.len());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 7);
}

#[test]
fn parallel_many_items_all_processed() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let counter = Arc::new(AtomicUsize::new(0));
    let files: Vec<String> = (0..50).map(|i| format!("file_{}.pl", i)).collect();
    let c = Arc::clone(&counter);
    let results = process_files_parallel(files, 4, move |_| {
        c.fetch_add(1, Ordering::SeqCst);
        true
    });
    assert_eq!(results.len(), 50);
    assert_eq!(counter.load(Ordering::SeqCst), 50);
}

#[test]
fn parallel_worker_count_one() {
    let files = vec!["a.pl".to_string(), "b.pl".to_string()];
    let results = process_files_parallel(files, 1, |f| f.to_uppercase());
    assert_eq!(results.len(), 2);
}

#[test]
fn parallel_returns_complex_results() {
    let files = vec!["hello".to_string(), "world".to_string()];
    let results: Vec<(String, usize)> = process_files_parallel(files, 2, |f| (f.clone(), f.len()));
    assert_eq!(results.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Mock runtime behavior
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn mock_runtime_records_invocations() {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(b"".to_vec()));
    rt.add_response(MockResponse::success(b"".to_vec()));
    let mut fmt = default_formatter(rt.clone());
    let _ = fmt.format("first");
    let _ = fmt.format("second");
    assert_eq!(rt.invocations().len(), 2);
    assert_eq!(rt.invocations()[0].program, "perltidy");
    assert_eq!(rt.invocations()[1].program, "perltidy");
}

#[test]
fn mock_runtime_queued_responses_consumed_in_order() {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(b"first\n".to_vec()));
    rt.add_response(MockResponse::success(b"second\n".to_vec()));
    let mut fmt = default_formatter(rt);
    let r1 = fmt.format("code1");
    let r2 = fmt.format("code2");
    assert_eq!(r1.unwrap_or_default(), "first\n");
    assert_eq!(r2.unwrap_or_default(), "second\n");
}

#[test]
fn mock_runtime_clear_invocations() {
    let rt = Arc::new(MockSubprocessRuntime::new());
    rt.add_response(MockResponse::success(b"".to_vec()));
    let mut fmt = default_formatter(rt.clone());
    let _ = fmt.format("x");
    assert_eq!(rt.invocations().len(), 1);
    rt.clear_invocations();
    assert!(rt.invocations().is_empty());
}

#[test]
fn mock_response_success_exit_code_zero() {
    let resp = MockResponse::success(b"ok".to_vec());
    assert_eq!(resp.status_code, 0);
}

#[test]
fn mock_response_failure_nonzero_exit_code() {
    let resp = MockResponse::failure(b"err".to_vec(), 42);
    assert_eq!(resp.status_code, 42);
    assert_eq!(resp.stderr, b"err");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Serialization round-trips
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn perltidy_config_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let config = PerlTidyConfig::pbp();
    let json = serde_json::to_string(&config)?;
    let deser: PerlTidyConfig = serde_json::from_str(&json)?;
    assert_eq!(deser.maximum_line_length, config.maximum_line_length);
    assert_eq!(deser.indent_columns, config.indent_columns);
    assert_eq!(deser.extra_args, config.extra_args);
    Ok(())
}

#[test]
fn critic_config_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let config = CriticConfig {
        severity: 2,
        profile: Some("myrc".to_string()),
        include: vec!["A".to_string()],
        exclude: vec!["B".to_string()],
        theme: Some("core".to_string()),
        verbose: true,
        color: true,
    };
    let json = serde_json::to_string(&config)?;
    let deser: CriticConfig = serde_json::from_str(&json)?;
    assert_eq!(deser.severity, 2);
    assert_eq!(deser.profile.as_deref(), Some("myrc"));
    assert_eq!(deser.theme.as_deref(), Some("core"));
    assert!(deser.verbose);
    assert!(deser.color);
    Ok(())
}

#[test]
fn severity_serde_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    for sev in
        [Severity::Brutal, Severity::Cruel, Severity::Harsh, Severity::Stern, Severity::Gentle]
    {
        let json = serde_json::to_string(&sev)?;
        let deser: Severity = serde_json::from_str(&json)?;
        assert_eq!(deser, sev);
    }
    Ok(())
}

#[test]
fn violation_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let v = Violation {
        policy: "TestPolicy".to_string(),
        description: "Test description".to_string(),
        explanation: "Test explanation".to_string(),
        severity: Severity::Cruel,
        range: Range {
            start: Position { byte: 0, line: 10, column: 5 },
            end: Position { byte: 0, line: 10, column: 20 },
        },
        file: "test.pl".to_string(),
    };
    let json = serde_json::to_string(&v)?;
    let deser: Violation = serde_json::from_str(&json)?;
    assert_eq!(deser.policy, "TestPolicy");
    assert_eq!(deser.severity, Severity::Cruel);
    assert_eq!(deser.range.start.line, 10);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Re-exports and type availability
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn subprocess_error_type_available() {
    let err = perl_lsp_tooling::SubprocessError { message: "boom".to_string() };
    assert_eq!(err.message, "boom");
}

#[test]
fn subprocess_output_success_method() {
    let output =
        perl_lsp_tooling::SubprocessOutput { stdout: vec![], stderr: vec![], status_code: 0 };
    assert!(output.success());
}

#[test]
fn subprocess_output_failure_method() {
    let output = perl_lsp_tooling::SubprocessOutput {
        stdout: vec![],
        stderr: b"err".to_vec(),
        status_code: 1,
    };
    assert!(!output.success());
    assert_eq!(output.stderr_lossy(), "err");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Struct trait implementations
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn violation_clone_preserves_fields() {
    let v = Violation {
        policy: "P".to_string(),
        description: "D".to_string(),
        explanation: "E".to_string(),
        severity: Severity::Brutal,
        range: Range {
            start: Position { byte: 0, line: 1, column: 2 },
            end: Position { byte: 0, line: 3, column: 4 },
        },
        file: "f.pl".to_string(),
    };
    let v2 = v.clone();
    assert_eq!(v2.policy, "P");
    assert_eq!(v2.severity, Severity::Brutal);
    assert_eq!(v2.range.start.line, 1);
    assert_eq!(v2.file, "f.pl");
}

#[test]
fn quick_fix_clone_preserves_fields() {
    let fix = QuickFix {
        title: "Fix it".to_string(),
        edit: TextEdit {
            range: Range {
                start: Position { byte: 0, line: 0, column: 0 },
                end: Position { byte: 0, line: 0, column: 5 },
            },
            new_text: "replacement".to_string(),
        },
    };
    let fix2 = fix.clone();
    assert_eq!(fix2.title, "Fix it");
    assert_eq!(fix2.edit.new_text, "replacement");
    assert_eq!(fix2.edit.range.end.column, 5);
}

#[test]
fn format_suggestion_has_debug() {
    use perl_lsp_tooling::perltidy::FormatSuggestion;
    let s = FormatSuggestion {
        line: 0,
        original: "a".to_string(),
        formatted: "b".to_string(),
        description: "change".to_string(),
    };
    let debug = format!("{:?}", s);
    assert!(debug.contains("FormatSuggestion"));
}

#[test]
fn text_edit_debug() {
    let edit = TextEdit {
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        new_text: "hello".to_string(),
    };
    let debug = format!("{:?}", edit);
    assert!(debug.contains("TextEdit"));
}
