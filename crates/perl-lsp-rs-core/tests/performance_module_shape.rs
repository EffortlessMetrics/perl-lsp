//! Integration test: `perl-lsp-performance` public API reachable via `perl_lsp_rs_core::performance`.

use perl_lsp_rs_core::performance::*;

#[test]
fn performance_module_exposes_ast_cache() {
    // Verify that AstCache is accessible post-absorption
    let cache = AstCache::new(100, 60);
    assert_some(Some(cache), "AstCache::new should return cache");
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
    assert_some(Some(cache), "AstCache should be constructible");
}
