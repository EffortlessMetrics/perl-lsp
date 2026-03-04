//! Comprehensive unit tests for perl-lsp-tooling crate.
//!
//! Tests cover:
//! - Performance module: AstCache, IncrementalParser, SymbolIndex, parallel processing
//! - Perl::Critic module: Severity, CriticConfig, CriticAnalyzer, BuiltInAnalyzer, QuickFix
//! - Perltidy module: PerlTidyConfig, PerlTidyFormatter, BuiltInFormatter, FormatSuggestion

use perl_lsp_tooling::mock::{CommandInvocation, MockResponse, MockSubprocessRuntime};
use perl_lsp_tooling::performance::parallel::process_files_parallel;
use perl_lsp_tooling::performance::{AstCache, IncrementalParser, SymbolIndex};
use perl_lsp_tooling::perl_critic::{
    BuiltInAnalyzer, CriticAnalyzer, CriticConfig, Policy, QuickFix, Severity, TextEdit, Violation,
};
use perl_lsp_tooling::perltidy::{
    BuiltInFormatter, FormatSuggestion, PerlTidyConfig, PerlTidyFormatter,
};
use perl_parser_core::position::{Position, Range};
use perl_parser_core::{Node, NodeKind, SourceLocation};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ─── Helper ───────────────────────────────────────────────────────────────────

fn make_program_node() -> Node {
    Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 })
}

fn make_error_node() -> Node {
    Node::new(
        NodeKind::Error {
            message: "test".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        SourceLocation { start: 0, end: 10 },
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance module: AstCache
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ast_cache_miss_for_unknown_uri() {
    let cache = AstCache::new(10, 60);
    assert!(cache.get("nonexistent.pl", "anything").is_none());
}

#[test]
fn ast_cache_hit_with_matching_content() {
    let cache = AstCache::new(10, 60);
    let ast = Arc::new(make_program_node());
    cache.put("file.pl".to_string(), "my $x = 1;", ast.clone());
    assert!(cache.get("file.pl", "my $x = 1;").is_some());
}

#[test]
fn ast_cache_invalidated_on_content_change() {
    let cache = AstCache::new(10, 60);
    let ast = Arc::new(make_program_node());
    cache.put("file.pl".to_string(), "original", ast);

    // Different content should miss
    assert!(cache.get("file.pl", "modified").is_none());

    // Stale entry should have been removed, so even original content misses now
    assert!(cache.get("file.pl", "original").is_none());
}

#[test]
fn ast_cache_overwrite_same_key() {
    let cache = AstCache::new(10, 60);
    let ast1 = Arc::new(make_program_node());
    let ast2 = Arc::new(Node::new(
        NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 5 },
    ));

    cache.put("file.pl".to_string(), "v1", ast1);
    // Overwrite with new content hash; old content should miss
    cache.put("file.pl".to_string(), "v2", ast2);

    // v1 content hash no longer matches the stored entry
    assert!(cache.get("file.pl", "v1").is_none());
    // v2 should be retrievable (may require brief sync for moka)
    // We mainly verify v1 is invalidated when content changes
}

#[test]
fn ast_cache_cleanup_runs_without_error() {
    let cache = AstCache::new(10, 60);
    let ast = Arc::new(make_program_node());
    cache.put("file.pl".to_string(), "content", ast);
    cache.cleanup(); // Should not panic
    assert!(cache.get("file.pl", "content").is_some());
}

#[test]
fn ast_cache_concurrent_readers_and_writers() {
    let cache = Arc::new(AstCache::new(200, 60));
    let mut handles = vec![];

    for i in 0..4 {
        let c = Arc::clone(&cache);
        let handle = std::thread::spawn(move || {
            for j in 0..20 {
                let key = format!("t{}_{}.pl", i, j);
                let content = format!("content_{}_{}", i, j);
                let ast = Arc::new(make_program_node());
                c.put(key.clone(), &content, ast);
                let _ = c.get(&key, &content);
            }
        });
        handles.push(handle);
    }

    for h in handles {
        assert!(h.join().is_ok(), "Thread should complete without panic");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance module: IncrementalParser
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn incremental_parser_default_no_changes() {
    let parser = IncrementalParser::default();
    assert!(!parser.needs_reparse(0, 100));
}

#[test]
fn incremental_parser_mark_single_region() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);

    assert!(parser.needs_reparse(15, 25)); // overlaps
    assert!(parser.needs_reparse(5, 15)); // overlaps start
    assert!(parser.needs_reparse(10, 20)); // exact match
    assert!(!parser.needs_reparse(0, 10)); // adjacent before (end == start, no overlap)
    assert!(!parser.needs_reparse(20, 30)); // adjacent after
    assert!(!parser.needs_reparse(25, 35)); // disjoint
}

#[test]
fn incremental_parser_merge_overlapping() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.mark_changed(15, 30);

    // Should have merged into (10, 30)
    assert!(parser.needs_reparse(10, 30));
    assert!(parser.needs_reparse(25, 35));
    assert!(!parser.needs_reparse(0, 10));
}

#[test]
fn incremental_parser_merge_adjacent() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.mark_changed(20, 30);

    // Adjacent regions merge: (10, 30)
    assert!(parser.needs_reparse(15, 25));
}

#[test]
fn incremental_parser_disjoint_regions_kept_separate() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.mark_changed(40, 50);

    assert!(parser.needs_reparse(15, 25));
    assert!(parser.needs_reparse(45, 55));
    assert!(!parser.needs_reparse(25, 35)); // gap between regions
}

#[test]
fn incremental_parser_clear_resets() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    assert!(parser.needs_reparse(15, 25));

    parser.clear();
    assert!(!parser.needs_reparse(15, 25));
}

#[test]
fn incremental_parser_merge_three_overlapping() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.mark_changed(30, 40);
    // This should merge all three into one
    parser.mark_changed(15, 35);

    assert!(parser.needs_reparse(10, 40));
    assert!(!parser.needs_reparse(0, 10));
    assert!(!parser.needs_reparse(40, 50));
}

#[test]
fn incremental_parser_zero_width_region() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(5, 5);
    // Zero-width region: node_start < 5 && node_end > 5 → needs overlap
    assert!(!parser.needs_reparse(5, 10)); // start == end of region
    assert!(!parser.needs_reparse(0, 5)); // end == start of region
    assert!(parser.needs_reparse(4, 6)); // spans the point
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance module: SymbolIndex
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn symbol_index_default_is_empty() {
    let index = SymbolIndex::default();
    assert!(index.search_prefix("anything").is_empty());
    assert!(index.search_fuzzy("anything").is_empty());
}

#[test]
fn symbol_index_prefix_search_exact() {
    let mut index = SymbolIndex::new();
    index.add_symbol("foo_bar".to_string());
    index.add_symbol("foo_baz".to_string());
    index.add_symbol("bar_qux".to_string());

    let results = index.search_prefix("foo_");
    assert_eq!(results.len(), 2);
    assert!(results.contains(&"foo_bar".to_string()));
    assert!(results.contains(&"foo_baz".to_string()));
}

#[test]
fn symbol_index_prefix_no_match() {
    let mut index = SymbolIndex::new();
    index.add_symbol("alpha".to_string());

    assert!(index.search_prefix("beta").is_empty());
}

#[test]
fn symbol_index_prefix_full_match() {
    let mut index = SymbolIndex::new();
    index.add_symbol("exact".to_string());

    let results = index.search_prefix("exact");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], "exact");
}

#[test]
fn symbol_index_fuzzy_search_by_token() {
    let mut index = SymbolIndex::new();
    index.add_symbol("get_user_name".to_string());
    index.add_symbol("set_user_email".to_string());
    index.add_symbol("delete_order".to_string());

    let results = index.search_fuzzy("user");
    assert!(results.contains(&"get_user_name".to_string()));
    assert!(results.contains(&"set_user_email".to_string()));
    assert!(!results.contains(&"delete_order".to_string()));
}

#[test]
fn symbol_index_fuzzy_search_multi_token_ranking() {
    let mut index = SymbolIndex::new();
    index.add_symbol("get_user_name".to_string());
    index.add_symbol("get_user_email".to_string());
    index.add_symbol("set_name".to_string());

    // "user name" matches two tokens in get_user_name, one token each in the others
    let results = index.search_fuzzy("user name");
    assert!(!results.is_empty());
    // get_user_name should rank highest (matches both "user" and "name")
    assert_eq!(results[0], "get_user_name");
}

#[test]
fn symbol_index_camel_case_tokenization() {
    let mut index = SymbolIndex::new();
    index.add_symbol("MyClassName".to_string());
    index.add_symbol("MyOtherClass".to_string());

    let results = index.search_fuzzy("class");
    assert!(results.contains(&"MyClassName".to_string()));
    assert!(results.contains(&"MyOtherClass".to_string()));
}

#[test]
fn symbol_index_empty_query() {
    let mut index = SymbolIndex::new();
    index.add_symbol("something".to_string());

    assert!(index.search_prefix("").len() >= 1); // empty prefix matches all
    assert!(index.search_fuzzy("").is_empty()); // no tokens to match
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance module: Parallel processing
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parallel_process_empty_list() {
    let results: Vec<i32> = process_files_parallel(vec![], 4, |_| 0);
    assert!(results.is_empty());
}

#[test]
fn parallel_process_single_worker() {
    let files = vec!["a.pl".to_string(), "b.pl".to_string()];
    let results = process_files_parallel(files, 1, |f| f.len());
    assert_eq!(results.len(), 2);
}

#[test]
fn parallel_process_more_workers_than_files() {
    let files = vec!["x.pl".to_string()];
    let results = process_files_parallel(files, 8, |f| f.to_uppercase());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], "X.PL");
}

#[test]
fn parallel_process_all_files_processed() {
    let counter = Arc::new(AtomicUsize::new(0));
    let files: Vec<String> = (0..20).map(|i| format!("file{}.pl", i)).collect();

    let c = Arc::clone(&counter);
    let results = process_files_parallel(files, 4, move |_| {
        c.fetch_add(1, Ordering::SeqCst);
        true
    });

    assert_eq!(results.len(), 20);
    assert_eq!(counter.load(Ordering::SeqCst), 20);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic module: Severity
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn severity_from_number_all_values() {
    assert_eq!(Severity::from_number(1), Severity::Brutal);
    assert_eq!(Severity::from_number(2), Severity::Cruel);
    assert_eq!(Severity::from_number(3), Severity::Harsh);
    assert_eq!(Severity::from_number(4), Severity::Stern);
    assert_eq!(Severity::from_number(5), Severity::Gentle);
}

#[test]
fn severity_from_number_out_of_range_defaults_to_harsh() {
    assert_eq!(Severity::from_number(0), Severity::Harsh);
    assert_eq!(Severity::from_number(6), Severity::Harsh);
    assert_eq!(Severity::from_number(255), Severity::Harsh);
}

#[cfg(feature = "lsp-compat")]
#[test]
fn severity_to_diagnostic_severity() {
    use lsp_types::DiagnosticSeverity;

    assert_eq!(Severity::Brutal.to_diagnostic_severity(), DiagnosticSeverity::ERROR);
    assert_eq!(Severity::Cruel.to_diagnostic_severity(), DiagnosticSeverity::ERROR);
    assert_eq!(Severity::Harsh.to_diagnostic_severity(), DiagnosticSeverity::WARNING);
    assert_eq!(Severity::Stern.to_diagnostic_severity(), DiagnosticSeverity::INFORMATION);
    assert_eq!(Severity::Gentle.to_diagnostic_severity(), DiagnosticSeverity::INFORMATION);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic module: CriticConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn critic_config_default_values() {
    let config = CriticConfig::default();
    assert_eq!(config.severity, 3);
    assert!(config.profile.is_none());
    assert!(config.include.is_empty());
    assert!(config.exclude.is_empty());
    assert!(config.theme.is_none());
    assert!(!config.verbose);
    assert!(!config.color);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic module: CriticAnalyzer with mock
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn critic_analyzer_parses_output_format() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let output = b"test.pl:10:5:2:SomePolicy:Some violation message\n";
    runtime.add_response(MockResponse::success(output.to_vec()));

    let mut analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);
    let violations = analyzer.analyze_file(Path::new("test.pl"))?;

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].policy, "SomePolicy");
    assert_eq!(violations[0].description, "Some violation message");
    assert_eq!(violations[0].severity, Severity::Cruel); // severity 2
    assert_eq!(violations[0].range.start.line, 9); // 0-indexed
    assert_eq!(violations[0].range.start.column, 4); // 0-indexed
    assert_eq!(violations[0].file, "test.pl");
    Ok(())
}

#[test]
fn critic_analyzer_handles_empty_output() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));

    let mut analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);
    let violations = analyzer.analyze_file(Path::new("clean.pl"))?;

    assert!(violations.is_empty());
    Ok(())
}

#[test]
fn critic_analyzer_skips_malformed_lines() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let output = b"malformed line without colons\ntest.pl:1:1:3:Policy:msg\n\n";
    runtime.add_response(MockResponse::success(output.to_vec()));

    let mut analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);
    let violations = analyzer.analyze_file(Path::new("test.pl"))?;

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].policy, "Policy");
    Ok(())
}

#[test]
fn critic_analyzer_caching_and_invalidation() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"f.pl:1:1:3:P:msg\n".to_vec()));
    runtime.add_response(MockResponse::success(b"".to_vec()));

    let mut analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime.clone());

    // First call populates cache
    let v1 = analyzer.analyze_file(Path::new("f.pl"))?;
    assert_eq!(v1.len(), 1);

    // Second call uses cache (only 1 invocation)
    let v2 = analyzer.analyze_file(Path::new("f.pl"))?;
    assert_eq!(v2.len(), 1);
    assert_eq!(runtime.invocations().len(), 1);

    // Invalidate and re-analyze
    analyzer.invalidate_cache("f.pl");
    let v3 = analyzer.analyze_file(Path::new("f.pl"))?;
    assert!(v3.is_empty());
    assert_eq!(runtime.invocations().len(), 2);
    Ok(())
}

#[test]
fn critic_analyzer_passes_config_args() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));

    let config = CriticConfig {
        severity: 1,
        profile: Some("/etc/perlcriticrc".to_string()),
        theme: Some("core".to_string()),
        include: vec!["IncludeMe".to_string()],
        exclude: vec!["ExcludeMe".to_string()],
        verbose: false,
        color: false,
    };
    let mut analyzer = CriticAnalyzer::new(config, runtime.clone());
    analyzer.analyze_file(Path::new("x.pl"))?;

    let inv = &runtime.invocations()[0];
    assert_eq!(inv.program, "perlcritic");
    assert!(inv.args.contains(&"--severity=1".to_string()));
    assert!(inv.args.contains(&"--profile=/etc/perlcriticrc".to_string()));
    assert!(inv.args.contains(&"--theme=core".to_string()));
    assert!(inv.args.contains(&"--include=IncludeMe".to_string()));
    assert!(inv.args.contains(&"--exclude=ExcludeMe".to_string()));
    // Security: -- separator before file path
    assert!(inv.args.contains(&"--".to_string()));
    Ok(())
}

#[test]
fn critic_analyzer_argument_injection_protection() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));

    let mut analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime.clone());
    analyzer.analyze_file(Path::new("-rf"))?;

    let inv = &runtime.invocations()[0];
    let sep_pos = inv.args.iter().position(|a| a == "--");
    let file_pos = inv.args.iter().position(|a| a == "-rf");
    assert!(sep_pos.is_some() && file_pos.is_some());
    assert!(sep_pos < file_pos, "-- must precede file argument");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic module: BuiltInAnalyzer
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn builtin_analyzer_detects_missing_strict() {
    let analyzer = BuiltInAnalyzer::new();
    let ast = make_error_node();
    let violations = analyzer.analyze(&ast, "print 'hello';\n");

    let strict_violation = violations.iter().find(|v| v.policy.contains("RequireUseStrict"));
    assert!(strict_violation.is_some());
}

#[test]
fn builtin_analyzer_detects_missing_warnings() {
    let analyzer = BuiltInAnalyzer::new();
    let ast = make_error_node();
    let violations = analyzer.analyze(&ast, "print 'hello';\n");

    let warn_violation = violations.iter().find(|v| v.policy.contains("RequireUseWarnings"));
    assert!(warn_violation.is_some());
}

#[test]
fn builtin_analyzer_passes_with_strict_and_warnings() {
    let analyzer = BuiltInAnalyzer::new();
    let ast = make_error_node();
    let violations = analyzer.analyze(&ast, "use strict;\nuse warnings;\nprint 'hello';\n");

    assert!(violations.is_empty());
}

#[test]
fn builtin_analyzer_quick_fix_strict() {
    let analyzer = BuiltInAnalyzer::new();
    let violation = Violation {
        policy: "TestingAndDebugging::RequireUseStrict".to_string(),
        description: "missing strict".to_string(),
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
    let fix = fix.map(|f| f.title);
    assert_eq!(fix.as_deref(), Some("Add 'use strict'"));
}

#[test]
fn builtin_analyzer_quick_fix_warnings() {
    let analyzer = BuiltInAnalyzer::new();
    let violation = Violation {
        policy: "TestingAndDebugging::RequireUseWarnings".to_string(),
        description: "missing warnings".to_string(),
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
    let fix = fix.map(|f| f.title);
    assert_eq!(fix.as_deref(), Some("Add 'use warnings'"));
}

#[test]
fn builtin_analyzer_no_quick_fix_for_unknown_policy() {
    let analyzer = BuiltInAnalyzer::new();
    let violation = Violation {
        policy: "Unknown::Policy".to_string(),
        description: "something".to_string(),
        explanation: String::new(),
        severity: Severity::Gentle,
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        file: String::new(),
    };

    assert!(analyzer.get_quick_fix(&violation, "").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic module: LSP diagnostics (lsp-compat feature)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "lsp-compat")]
#[test]
fn critic_analyzer_to_diagnostics() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);

    let violations = vec![Violation {
        policy: "TestPolicy".to_string(),
        description: "test message".to_string(),
        explanation: "test explanation".to_string(),
        severity: Severity::Cruel,
        range: Range {
            start: Position { byte: 0, line: 5, column: 3 },
            end: Position { byte: 0, line: 5, column: 10 },
        },
        file: "test.pl".to_string(),
    }];

    let diagnostics = analyzer.to_diagnostics(&violations);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, "test message");
    assert_eq!(diagnostics[0].source.as_deref(), Some("perlcritic"));
    assert_eq!(diagnostics[0].range.start.line, 5);
    assert_eq!(diagnostics[0].range.start.character, 3);
}

#[cfg(feature = "lsp-compat")]
#[test]
fn critic_analyzer_to_diagnostics_empty() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);

    let diagnostics = analyzer.to_diagnostics(&[]);
    assert!(diagnostics.is_empty());
}

#[cfg(feature = "lsp-compat")]
#[test]
fn critic_analyzer_quick_fix_strict_lsp() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);

    let violation = Violation {
        policy: "TestingAndDebugging::RequireUseStrict".to_string(),
        description: "no strict".to_string(),
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
    assert_eq!(fix.map(|f| f.edit.new_text), Some("use strict;\n".to_string()));
}

#[cfg(feature = "lsp-compat")]
#[test]
fn critic_analyzer_quick_fix_unused_variable() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);

    let violation = Violation {
        policy: "Variables::ProhibitUnusedVariables".to_string(),
        description: "unused var".to_string(),
        explanation: String::new(),
        severity: Severity::Stern,
        range: Range {
            start: Position { byte: 0, line: 3, column: 0 },
            end: Position { byte: 0, line: 3, column: 10 },
        },
        file: String::new(),
    };

    let fix = analyzer.get_quick_fix(&violation, "");
    assert!(fix.is_some());
    assert_eq!(fix.map(|f| f.title), Some("Remove unused variable".to_string()));
}

#[cfg(feature = "lsp-compat")]
#[test]
fn critic_analyzer_quick_fix_unused_subroutine() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);

    let violation = Violation {
        policy: "Subroutines::ProhibitUnusedPrivateSubroutines".to_string(),
        description: "unused sub".to_string(),
        explanation: String::new(),
        severity: Severity::Stern,
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        file: String::new(),
    };

    let fix = analyzer.get_quick_fix(&violation, "");
    assert!(fix.is_some());
    assert_eq!(fix.map(|f| f.title), Some("Remove unused subroutine".to_string()));
}

#[cfg(feature = "lsp-compat")]
#[test]
fn critic_analyzer_no_quick_fix_for_unknown_policy_lsp() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let analyzer = CriticAnalyzer::new(CriticConfig::default(), runtime);

    let violation = Violation {
        policy: "SomeOther::Policy".to_string(),
        description: "x".to_string(),
        explanation: String::new(),
        severity: Severity::Gentle,
        range: Range {
            start: Position { byte: 0, line: 0, column: 0 },
            end: Position { byte: 0, line: 0, column: 0 },
        },
        file: String::new(),
    };

    assert!(analyzer.get_quick_fix(&violation, "").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perltidy module: PerlTidyConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn perltidy_config_default_values() {
    let config = PerlTidyConfig::default();
    assert_eq!(config.maximum_line_length, Some(80));
    assert_eq!(config.indent_columns, Some(4));
    assert_eq!(config.tabs, Some(false));
    assert_eq!(config.opening_brace_on_new_line, Some(false));
    assert_eq!(config.cuddled_else, Some(true));
    assert_eq!(config.space_after_keyword, Some(true));
    assert_eq!(config.add_trailing_commas, Some(false));
    assert_eq!(config.vertical_alignment, Some(true));
    assert_eq!(config.block_comment_indentation, Some(0));
    assert!(config.profile.is_none());
    assert!(config.extra_args.is_empty());
}

#[test]
fn perltidy_config_pbp_style() {
    let config = PerlTidyConfig::pbp();
    assert_eq!(config.maximum_line_length, Some(78));
    assert_eq!(config.cuddled_else, Some(false));
    assert_eq!(config.add_trailing_commas, Some(true));
    assert!(config.extra_args.contains(&"--perl-best-practices".to_string()));
}

#[test]
fn perltidy_config_gnu_style() {
    let config = PerlTidyConfig::gnu();
    assert_eq!(config.maximum_line_length, Some(79));
    assert_eq!(config.indent_columns, Some(2));
    assert_eq!(config.opening_brace_on_new_line, Some(true));
    assert!(config.extra_args.contains(&"--gnu-style".to_string()));
}

#[test]
fn perltidy_config_profile_overrides_other_args() {
    let config = PerlTidyConfig {
        profile: Some("/home/user/.perltidyrc".to_string()),
        ..PerlTidyConfig::default()
    };

    // When a profile is set, to_args should only return --profile=...
    // (to_args is private, so we test via formatter invocation)
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"formatted\n".to_vec()));

    let mut formatter = PerlTidyFormatter::new(config, runtime.clone());
    let _ = formatter.format("code");

    let inv = &runtime.invocations()[0];
    // Should contain profile arg
    assert!(inv.args.iter().any(|a| a.starts_with("--profile=")));
    // Should NOT contain --maximum-line-length (profile overrides)
    assert!(!inv.args.iter().any(|a| a.starts_with("--maximum-line-length")));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perltidy module: PerlTidyFormatter with mock
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn formatter_format_returns_stdout() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"my $x = 1;\n".to_vec()));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let result = formatter.format("my $x=1;")?;

    assert_eq!(result, "my $x = 1;\n");
    Ok(())
}

#[test]
fn formatter_caches_result() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"out\n".to_vec()));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());
    let r1 = formatter.format("in")?;
    let r2 = formatter.format("in")?;

    assert_eq!(r1, r2);
    assert_eq!(runtime.invocations().len(), 1); // only one actual call
    Ok(())
}

#[test]
fn formatter_clear_cache_forces_rerun() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"first\n".to_vec()));
    runtime.add_response(MockResponse::success(b"second\n".to_vec()));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());
    let r1 = formatter.format("code")?;
    formatter.clear_cache();
    let r2 = formatter.format("code")?;

    assert_eq!(r1, "first\n");
    assert_eq!(r2, "second\n");
    assert_eq!(runtime.invocations().len(), 2);
    Ok(())
}

#[test]
fn formatter_error_on_nonzero_exit() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::failure(b"perltidy error".to_vec(), 2));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let result = formatter.format("bad code");

    assert!(result.is_err());
    let err = result.err().map(|e| e.contains("Perltidy failed"));
    assert_eq!(err, Some(true));
}

#[test]
fn formatter_passes_st_flag() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"ok\n".to_vec()));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());
    formatter.format("x")?;

    let inv = &runtime.invocations()[0];
    assert!(inv.args.contains(&"-st".to_string()));
    Ok(())
}

#[test]
fn formatter_sends_code_as_stdin() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"ok\n".to_vec()));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());
    formatter.format("my $code;")?;

    let inv = &runtime.invocations()[0];
    assert_eq!(inv.stdin.as_deref(), Some(b"my $code;" as &[u8]));
    Ok(())
}

#[test]
fn formatter_format_file_uses_argument_separator() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));

    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());
    formatter.format_file(Path::new("-dangerous_name.pl"))?;

    let inv = &runtime.invocations()[0];
    let sep_idx = inv.args.iter().position(|a| a == "--");
    let file_idx = inv.args.iter().position(|a| a == "-dangerous_name.pl");
    assert!(sep_idx.is_some() && file_idx.is_some());
    assert!(sep_idx < file_idx, "-- must precede file path");
    Ok(())
}

#[test]
fn formatter_format_file_no_stdin() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));

    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());
    formatter.format_file(Path::new("script.pl"))?;

    let inv = &runtime.invocations()[0];
    assert!(inv.stdin.is_none());
    Ok(())
}

#[test]
fn formatter_format_file_error() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::failure(b"file not found".to_vec(), 1));

    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let result = formatter.format_file(Path::new("missing.pl"));

    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perltidy module: format_range
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn formatter_format_range_middle() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    // The range "print $x;" gets formatted to "print $x ;"
    runtime.add_response(MockResponse::success(b"print $x ;".to_vec()));

    let code = "line0\nprint $x;\nline2\n";
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let result = formatter.format_range(code, 1, 1)?;

    // Should have line0, formatted range, and line2
    assert!(result.contains("line0"));
    assert!(result.contains("print $x ;"));
    assert!(result.contains("line2"));
    Ok(())
}

#[test]
fn formatter_format_range_out_of_bounds() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format_range("one line", 5, 10);
    assert!(result.is_err());
    assert!(result.err().map(|e| e.contains("out of bounds")) == Some(true));
}

#[test]
fn formatter_format_range_first_line() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"FORMATTED".to_vec()));

    let code = "line0\nline1\nline2";
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let result = formatter.format_range(code, 0, 0)?;

    assert!(result.starts_with("FORMATTED"));
    assert!(result.contains("line1"));
    assert!(result.contains("line2"));
    Ok(())
}

#[test]
fn formatter_format_range_last_line() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"FORMATTED".to_vec()));

    let code = "line0\nline1\nline2";
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let result = formatter.format_range(code, 2, 2)?;

    assert!(result.contains("line0"));
    assert!(result.contains("line1"));
    assert!(result.ends_with("FORMATTED"));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perltidy module: get_suggestions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn formatter_get_suggestions_no_changes() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    // Return same code as input
    runtime.add_response(MockResponse::success(b"my $x = 1;".to_vec()));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let suggestions = formatter.get_suggestions("my $x = 1;")?;

    assert!(suggestions.is_empty());
    Ok(())
}

#[test]
fn formatter_get_suggestions_with_changes() -> Result<(), String> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"my $x = 1;\nmy $y = 2;".to_vec()));

    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);
    let suggestions = formatter.get_suggestions("my $x=1;\nmy $y=2;")?;

    assert!(!suggestions.is_empty());
    // Each suggestion should have a line number and description
    for s in &suggestions {
        assert!(!s.description.is_empty());
        assert_ne!(s.original, s.formatted);
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perltidy module: BuiltInFormatter
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn builtin_formatter_indents_block() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "if ($x) {\nprint $x;\n}\n";
    let formatted = formatter.format(code);

    assert!(formatted.contains("    print $x;"));
}

#[test]
fn builtin_formatter_handles_nested_blocks() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "if ($x) {\nif ($y) {\nprint;\n}\n}\n";
    let formatted = formatter.format(code);

    assert!(formatted.contains("        print;"));
}

#[test]
fn builtin_formatter_uses_tabs_when_configured() {
    let config = PerlTidyConfig { tabs: Some(true), ..PerlTidyConfig::default() };
    let formatter = BuiltInFormatter::new(config);
    let code = "if ($x) {\nprint;\n}\n";
    let formatted = formatter.format(code);

    assert!(formatted.contains("\tprint;"));
}

#[test]
fn builtin_formatter_uses_custom_indent_size() {
    let config = PerlTidyConfig { indent_columns: Some(2), ..PerlTidyConfig::default() };
    let formatter = BuiltInFormatter::new(config);
    let code = "if ($x) {\nprint;\n}\n";
    let formatted = formatter.format(code);

    assert!(formatted.contains("  print;"));
    assert!(!formatted.contains("    print;")); // not 4 spaces
}

#[test]
fn builtin_formatter_empty_lines_preserved() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "line1\n\nline2\n";
    let formatted = formatter.format(code);

    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1], ""); // empty line preserved
}

#[test]
fn builtin_formatter_closing_braces() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "sub foo {\nreturn 1;\n}\n";
    let formatted = formatter.format(code);

    // Closing brace should be at indent level 0
    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[2], "}");
}

#[test]
fn builtin_formatter_parens_and_brackets() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let code = "my @arr = (\n1,\n2,\n);\n";
    let formatted = formatter.format(code);

    assert!(formatted.contains("    1,"));
    assert!(formatted.contains("    2,"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perltidy module: FormatSuggestion struct
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn format_suggestion_debug_trait() {
    let suggestion = FormatSuggestion {
        line: 5,
        original: "  foo".to_string(),
        formatted: "    foo".to_string(),
        description: "indent fix".to_string(),
    };
    let debug = format!("{:?}", suggestion);
    assert!(debug.contains("FormatSuggestion"));
    assert!(debug.contains("indent fix"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Perl::Critic module: Violation and QuickFix structs
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn violation_debug_and_clone() {
    let v = Violation {
        policy: "Test::Policy".to_string(),
        description: "desc".to_string(),
        explanation: "explain".to_string(),
        severity: Severity::Stern,
        range: Range {
            start: Position { byte: 0, line: 1, column: 2 },
            end: Position { byte: 0, line: 1, column: 5 },
        },
        file: "test.pl".to_string(),
    };

    let v2 = v.clone();
    assert_eq!(v.policy, v2.policy);
    assert_eq!(v.severity, v2.severity);

    let debug = format!("{:?}", v);
    assert!(debug.contains("Test::Policy"));
}

#[test]
fn quick_fix_debug_and_clone() {
    let fix = QuickFix {
        title: "Fix it".to_string(),
        edit: TextEdit {
            range: Range {
                start: Position { byte: 0, line: 0, column: 0 },
                end: Position { byte: 0, line: 0, column: 0 },
            },
            new_text: "use strict;\n".to_string(),
        },
    };

    let fix2 = fix.clone();
    assert_eq!(fix.title, fix2.title);
    assert_eq!(fix.edit.new_text, fix2.edit.new_text);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Severity serialization
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn severity_serialize_deserialize() -> Result<(), Box<dyn std::error::Error>> {
    let s = Severity::Brutal;
    let json = serde_json::to_string(&s)?;
    let deserialized: Severity = serde_json::from_str(&json)?;
    assert_eq!(deserialized, Severity::Brutal);
    Ok(())
}

#[test]
fn violation_serialize_deserialize() -> Result<(), Box<dyn std::error::Error>> {
    let v = Violation {
        policy: "TestPolicy".to_string(),
        description: "desc".to_string(),
        explanation: "explain".to_string(),
        severity: Severity::Gentle,
        range: Range {
            start: Position { byte: 0, line: 1, column: 0 },
            end: Position { byte: 0, line: 1, column: 5 },
        },
        file: "test.pl".to_string(),
    };

    let json = serde_json::to_string(&v)?;
    let deserialized: Violation = serde_json::from_str(&json)?;
    assert_eq!(deserialized.policy, "TestPolicy");
    assert_eq!(deserialized.severity, Severity::Gentle);
    Ok(())
}

#[test]
fn critic_config_serialize_deserialize() -> Result<(), Box<dyn std::error::Error>> {
    let config = CriticConfig {
        severity: 1,
        profile: Some("path".to_string()),
        include: vec!["Policy1".to_string()],
        exclude: vec!["Policy2".to_string()],
        theme: Some("core".to_string()),
        verbose: true,
        color: true,
    };

    let json = serde_json::to_string(&config)?;
    let deserialized: CriticConfig = serde_json::from_str(&json)?;
    assert_eq!(deserialized.severity, 1);
    assert_eq!(deserialized.profile.as_deref(), Some("path"));
    assert_eq!(deserialized.theme.as_deref(), Some("core"));
    Ok(())
}

#[test]
fn perltidy_config_serialize_deserialize() -> Result<(), Box<dyn std::error::Error>> {
    let config = PerlTidyConfig::pbp();
    let json = serde_json::to_string(&config)?;
    let deserialized: PerlTidyConfig = serde_json::from_str(&json)?;
    assert_eq!(deserialized.maximum_line_length, Some(78));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Re-exports from lib.rs
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn reexports_subprocess_types() {
    // Verify that SubprocessError, SubprocessOutput, SubprocessRuntime are accessible
    let _: fn() -> perl_lsp_tooling::SubprocessError =
        || perl_lsp_tooling::SubprocessError::new("test");
    // MockSubprocessRuntime implements SubprocessRuntime
    let runtime = MockSubprocessRuntime::new();
    let _: &dyn perl_lsp_tooling::SubprocessRuntime = &runtime;
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn reexports_os_subprocess_runtime() {
    let _runtime = perl_lsp_tooling::OsSubprocessRuntime::new();
}

#[test]
fn mock_module_reexports() {
    // Verify mock types are accessible via perl_lsp_tooling::mock
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"hello".to_vec()));
    let invocations: Vec<CommandInvocation> = runtime.invocations();
    assert!(invocations.is_empty());
}
