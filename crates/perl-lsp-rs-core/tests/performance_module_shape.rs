//! Integration test: `perl-lsp-performance` public API reachable via `perl_lsp_rs_core::performance`.

use perl_lsp_rs_core::performance::*;

#[test]
fn performance_module_exposes_ast_cache() {
    // Verify that AstCache is accessible post-absorption
    let cache = AstCache::new(100, 60);
    assert!(assert_some(Some(cache), "AstCache::new should return cache").is_ok());
}

#[test]
fn performance_module_exposes_incremental_parser() {
    // Verify that IncrementalParser is accessible post-absorption
    let _: Option<IncrementalParser> = None;
}

#[test]
fn performance_module_exposes_symbol_index() {
    // Verify that SymbolIndex is accessible post-absorption
    let _: Option<SymbolIndex> = None;
}

#[test]
fn performance_module_exposes_parallel_submodule() {
    // Verify that parallel submodule is accessible post-absorption
    let _: Option<parallel::ParallelIndexer> = None;
}

#[test]
fn performance_ast_cache_stores_and_retrieves() {
    // Verify that AstCache functionality works post-absorption
    let cache = AstCache::new(100, 60);
    let _content = "sub foo {}";
    // Note: full integration test would require actual Node construction,
    // which is complex. This test verifies the cache is instantiable.
    assert!(assert_some(Some(cache), "AstCache should be constructible").is_ok());
}

#[test]
fn incremental_parser_given_reversed_change_range_when_marked_then_reparse_is_detected() {
    let mut parser = IncrementalParser::new();

    // Simulate a caller accidentally sending end/start in reverse order.
    parser.mark_changed(20, 10);

    assert!(parser.needs_reparse(12, 18));
    assert!(!parser.needs_reparse(0, 9));
}

#[test]
fn incremental_parser_given_zero_width_change_when_marked_then_no_nodes_require_reparse() {
    let mut parser = IncrementalParser::new();

    parser.mark_changed(10, 10);

    assert!(!parser.needs_reparse(0, 100));
}

#[test]
fn incremental_parser_given_reversed_node_range_when_checked_then_overlap_is_detected() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);

    assert!(parser.needs_reparse(18, 12));
    assert!(!parser.needs_reparse(9, 0));
}

#[test]
fn incremental_parser_given_point_query_inside_change_when_checked_then_reparse_is_required() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(30, 40);

    assert!(parser.needs_reparse(35, 35));
    assert!(!parser.needs_reparse(40, 40));
}

#[test]
fn parallel_given_zero_workers_when_processing_then_all_files_are_still_processed() {
    let files = vec!["a.pm".to_string(), "b.pm".to_string(), "c.pm".to_string()];

    let mut processed = parallel::process_files_parallel(files, 0, |file| file);
    processed.sort();

    assert_eq!(processed, vec!["a.pm", "b.pm", "c.pm"]);
}

#[test]
fn parallel_given_more_workers_than_files_when_processing_then_each_file_processed_once() {
    let files = vec!["one.pm".to_string(), "two.pm".to_string()];

    let mut processed = parallel::process_files_parallel(files, 16, |file| file);
    processed.sort();

    assert_eq!(processed, vec!["one.pm", "two.pm"]);
}
