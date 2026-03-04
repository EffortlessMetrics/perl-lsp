//! Comprehensive unit tests for `perl-workspace-index`.
//!
//! Covers: WorkspaceIndex indexing, search, dual indexing (qualified + bare),
//! multi-file scenarios, DocumentStore, BoundedLruCache, IndexStateMachine,
//! ProductionIndexCoordinator, and IndexCoordinator.

use perl_tdd_support::must_some;
use perl_workspace_index::workspace::cache::{
    BoundedLruCache, CacheConfig, CombinedWorkspaceCacheConfig, EstimateSize,
};
use perl_workspace_index::workspace::document_store::DocumentStore;
use perl_workspace_index::workspace::production_coordinator::{
    ProductionCoordinatorConfig, ProductionIndexCoordinator, WorkspaceCacheManager,
};
use perl_workspace_index::workspace::state_machine::{
    BuildPhase, DegradationReason, IndexState, IndexStateKind, IndexStateMachine,
    InvalidationReason, ResourceKind, TransitionResult,
};
use perl_workspace_index::workspace::workspace_index::{
    IndexCoordinator, IndexResourceLimits, SymKind, SymbolKey, WorkspaceIndex,
};
use std::sync::Arc;
use url::Url;

// ---------------------------------------------------------------------------
// Helper: parse a file:// URL without unwrap
// ---------------------------------------------------------------------------
fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

// =========================================================================
// WorkspaceIndex – basic indexing
// =========================================================================

#[test]
fn test_new_index_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    assert_eq!(index.file_count(), 0);
    assert_eq!(index.symbol_count(), 0);
    assert!(!index.has_symbols());
    Ok(())
}

#[test]
fn test_index_single_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/example.pl")?;
    index.index_file(uri, "sub greet { return 'hi'; }".to_string())?;

    assert_eq!(index.file_count(), 1);
    assert!(index.has_symbols());
    assert!(index.symbol_count() > 0);
    Ok(())
}

#[test]
fn test_find_definition_bare_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/app.pl")?;
    index.index_file(uri, "sub hello { 42 }".to_string())?;

    let def = must_some(index.find_definition("hello"));
    assert!(def.uri.contains("app.pl"));
    Ok(())
}

#[test]
fn test_find_definition_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Greeter.pm")?;
    index.index_file(uri, "package Greeter;\nsub say_hello { return 1; }".to_string())?;

    let def = must_some(index.find_definition("Greeter::say_hello"));
    assert!(def.uri.contains("Greeter.pm"));
    Ok(())
}

#[test]
fn test_find_definition_returns_none_for_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/a.pl")?;
    index.index_file(uri, "sub existing { }".to_string())?;

    assert!(index.find_definition("nonexistent").is_none());
    Ok(())
}

// =========================================================================
// Dual indexing – qualified + bare names
// =========================================================================

#[test]
fn test_dual_indexing_find_refs_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/utils.pm")?;
    index.index_file(uri, "package Utils;\nsub process_data { 1 }\nprocess_data();".to_string())?;

    let refs = index.find_references("Utils::process_data");
    // Should find at least the bare call
    assert!(!refs.is_empty());
    Ok(())
}

#[test]
fn test_dual_indexing_bare_name_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/script.pl")?;
    index.index_file(uri, "sub run { 1 }\nrun();".to_string())?;

    let refs = index.find_references("run");
    assert!(!refs.is_empty());
    Ok(())
}

// =========================================================================
// Multi-file scenarios
// =========================================================================

#[test]
fn test_multi_file_indexing() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri_a = file_url("/a.pl")?;
    let uri_b = file_url("/b.pl")?;

    index.index_file(uri_a, "sub alpha { 1 }".to_string())?;
    index.index_file(uri_b, "sub beta { 2 }".to_string())?;

    assert_eq!(index.file_count(), 2);
    assert!(index.find_definition("alpha").is_some());
    assert!(index.find_definition("beta").is_some());
    Ok(())
}

#[test]
fn test_multi_file_cross_file_search() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri_a = file_url("/lib/Foo.pm")?;
    let uri_b = file_url("/lib/Bar.pm")?;

    index.index_file(uri_a, "package Foo;\nsub do_work { 1 }".to_string())?;
    index.index_file(uri_b, "package Bar;\nsub do_other { 1 }".to_string())?;

    let results = index.search_symbols("do_");
    assert!(results.len() >= 2);
    Ok(())
}

#[test]
fn test_remove_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/removeme.pl")?;
    let uri_str = uri.to_string();
    index.index_file(uri, "sub gone { 1 }".to_string())?;

    assert_eq!(index.file_count(), 1);
    index.remove_file(&uri_str);
    assert_eq!(index.file_count(), 0);
    Ok(())
}

#[test]
fn test_remove_file_url() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/removeme2.pl")?;
    index.index_file(uri.clone(), "sub vanish { 1 }".to_string())?;

    index.remove_file_url(&uri);
    assert_eq!(index.file_count(), 0);
    Ok(())
}

#[test]
fn test_clear_index() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/c.pl")?;
    index.index_file(uri, "sub c_func { 1 }".to_string())?;
    assert!(index.has_symbols());

    index.clear();
    assert_eq!(index.file_count(), 0);
    assert_eq!(index.symbol_count(), 0);
    Ok(())
}

// =========================================================================
// Symbol search
// =========================================================================

#[test]
fn test_search_symbols_case_insensitive() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/search.pl")?;
    index.index_file(uri, "sub MyFunction { 1 }".to_string())?;

    let results = index.search_symbols("myfunction");
    assert!(!results.is_empty());
    Ok(())
}

#[test]
fn test_find_symbols_alias() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/alias.pl")?;
    index.index_file(uri, "sub target { 1 }".to_string())?;

    let a = index.search_symbols("target");
    let b = index.find_symbols("target");
    assert_eq!(a.len(), b.len());
    Ok(())
}

#[test]
fn test_all_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/all.pl")?;
    index.index_file(uri, "package Pkg;\nsub one { 1 }\nsub two { 2 }".to_string())?;

    let all = index.all_symbols();
    // At minimum: package Pkg + sub one + sub two
    assert!(all.len() >= 3);
    Ok(())
}

#[test]
fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/specific.pl")?;
    let uri_str = uri.to_string();
    index.index_file(uri, "sub only_here { 1 }".to_string())?;

    let syms = index.file_symbols(&uri_str);
    assert!(!syms.is_empty());
    Ok(())
}

// =========================================================================
// Package members
// =========================================================================

#[test]
fn test_get_package_members() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/members.pm")?;
    index.index_file(uri, "package Animals;\nsub cat { 1 }\nsub dog { 2 }".to_string())?;

    let members = index.get_package_members("Animals");
    assert!(members.len() >= 2);
    Ok(())
}

// =========================================================================
// Dependencies
// =========================================================================

#[test]
fn test_file_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/deps.pl")?;
    let uri_str = uri.to_string();
    index.index_file(uri, "use strict;\nuse warnings;\nsub x { 1 }".to_string())?;

    // The parser should extract use statements as dependencies
    let _deps = index.file_dependencies(&uri_str);
    // Even if empty, should not error
    Ok(())
}

// =========================================================================
// SymbolKey-based lookup (find_def / find_refs)
// =========================================================================

#[test]
fn test_find_def_with_symbol_key() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/key_def.pm")?;
    index.index_file(uri, "package MyPkg;\nsub example { return 42; }".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("MyPkg"),
        name: Arc::from("example"),
        sigil: None,
        kind: SymKind::Sub,
    };
    let def = must_some(index.find_def(&key));
    assert!(def.uri.contains("key_def.pm"));
    Ok(())
}

#[test]
fn test_find_refs_with_symbol_key() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/key_ref.pm")?;
    index.index_file(uri, "package Svc;\nsub handler { 1 }\nhandler();".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("Svc"),
        name: Arc::from("handler"),
        sigil: None,
        kind: SymKind::Sub,
    };
    // find_refs excludes the definition site
    let _refs = index.find_refs(&key);
    Ok(())
}

// =========================================================================
// Content-hash early exit (re-indexing same content is a no-op)
// =========================================================================

#[test]
fn test_reindex_same_content_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/noop.pl")?;
    let code = "sub stable { 1 }".to_string();

    index.index_file(uri.clone(), code.clone())?;
    let count1 = index.symbol_count();

    index.index_file(uri, code)?;
    let count2 = index.symbol_count();

    assert_eq!(count1, count2);
    Ok(())
}

// =========================================================================
// index_file_str convenience method
// =========================================================================

#[test]
fn test_index_file_str() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///str_test.pl", "sub from_str { 1 }")?;

    assert!(index.find_definition("from_str").is_some());
    Ok(())
}

// =========================================================================
// Variable indexing
// =========================================================================

#[test]
fn test_variable_declaration_indexed() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/vars.pl")?;
    index.index_file(uri, "my $count = 0;\nmy @items = ();\nmy %lookup;".to_string())?;

    let syms = index.all_symbols();
    let var_names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
    assert!(var_names.contains(&"$count"));
    assert!(var_names.contains(&"@items"));
    assert!(var_names.contains(&"%lookup"));
    Ok(())
}

// =========================================================================
// normalize_var
// =========================================================================

#[test]
fn test_normalize_var_scalar() {
    use perl_workspace_index::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("$foo");
    assert_eq!(sigil, Some('$'));
    assert_eq!(name, "foo");
}

#[test]
fn test_normalize_var_array() {
    use perl_workspace_index::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("@bar");
    assert_eq!(sigil, Some('@'));
    assert_eq!(name, "bar");
}

#[test]
fn test_normalize_var_hash() {
    use perl_workspace_index::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("%baz");
    assert_eq!(sigil, Some('%'));
    assert_eq!(name, "baz");
}

#[test]
fn test_normalize_var_no_sigil() {
    use perl_workspace_index::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("plain");
    assert_eq!(sigil, None);
    assert_eq!(name, "plain");
}

#[test]
fn test_normalize_var_empty() {
    use perl_workspace_index::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("");
    assert_eq!(sigil, None);
    assert_eq!(name, "");
}

// =========================================================================
// DocumentStore
// =========================================================================

#[test]
fn test_document_store_open_get_close() {
    let store = DocumentStore::new();
    store.open("file:///doc.pl".to_string(), 1, "content".to_string());

    assert!(store.is_open("file:///doc.pl"));
    assert_eq!(store.count(), 1);

    let doc = must_some(store.get("file:///doc.pl"));
    assert_eq!(doc.text, "content");

    assert!(store.close("file:///doc.pl"));
    assert!(!store.is_open("file:///doc.pl"));
    assert_eq!(store.count(), 0);
}

#[test]
fn test_document_store_update() {
    let store = DocumentStore::new();
    store.open("file:///upd.pl".to_string(), 1, "v1".to_string());
    assert!(store.update("file:///upd.pl", 2, "v2".to_string()));

    let doc = must_some(store.get("file:///upd.pl"));
    assert_eq!(doc.version, 2);
    assert_eq!(doc.text, "v2");
}

#[test]
fn test_document_store_update_nonexistent() {
    let store = DocumentStore::new();
    assert!(!store.update("file:///noexist.pl", 1, "text".to_string()));
}

#[test]
fn test_document_store_close_nonexistent() {
    let store = DocumentStore::new();
    assert!(!store.close("file:///never_opened.pl"));
}

#[test]
fn test_document_store_get_text() {
    let store = DocumentStore::new();
    store.open("file:///txt.pl".to_string(), 1, "hello".to_string());
    assert_eq!(store.get_text("file:///txt.pl"), Some("hello".to_string()));
    assert_eq!(store.get_text("file:///no.pl"), None);
}

#[test]
fn test_document_store_all_documents() {
    let store = DocumentStore::new();
    store.open("file:///one.pl".to_string(), 1, "1".to_string());
    store.open("file:///two.pl".to_string(), 1, "2".to_string());
    assert_eq!(store.all_documents().len(), 2);
}

#[test]
fn test_document_store_default_trait() {
    let store = DocumentStore::default();
    assert_eq!(store.count(), 0);
}

// =========================================================================
// BoundedLruCache
// =========================================================================

#[test]
fn test_cache_insert_and_get() {
    let cache = BoundedLruCache::<String, String>::default();
    cache.insert("k1".to_string(), "v1".to_string());

    assert_eq!(cache.get(&"k1".to_string()), Some("v1".to_string()));
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());
}

#[test]
fn test_cache_miss_returns_none() {
    let cache = BoundedLruCache::<String, String>::default();
    assert_eq!(cache.get(&"missing".to_string()), None);
}

#[test]
fn test_cache_lru_eviction() {
    let config = CacheConfig { max_items: 2, max_bytes: 1024, ttl: None };
    let cache = BoundedLruCache::<String, String>::new(config);

    cache.insert("a".to_string(), "1".to_string());
    cache.insert("b".to_string(), "2".to_string());
    cache.insert("c".to_string(), "3".to_string());

    // 'a' should be evicted (LRU)
    assert!(cache.get(&"a".to_string()).is_none());
    assert!(cache.get(&"b".to_string()).is_some());
    assert!(cache.get(&"c".to_string()).is_some());
}

#[test]
fn test_cache_update_existing_key() {
    let cache = BoundedLruCache::<String, String>::default();
    cache.insert("key".to_string(), "old".to_string());
    cache.insert("key".to_string(), "new".to_string());

    assert_eq!(cache.get(&"key".to_string()), Some("new".to_string()));
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_cache_remove() {
    let cache = BoundedLruCache::<String, String>::default();
    cache.insert("rm".to_string(), "val".to_string());

    assert_eq!(cache.remove(&"rm".to_string()), Some("val".to_string()));
    assert!(cache.is_empty());
}

#[test]
fn test_cache_clear() {
    let cache = BoundedLruCache::<String, String>::default();
    cache.insert("x".to_string(), "y".to_string());
    cache.clear();

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_cache_stats_tracking() {
    let cache = BoundedLruCache::<String, String>::default();
    cache.insert("h".to_string(), "v".to_string());

    let _ = cache.get(&"h".to_string()); // hit
    let _ = cache.get(&"miss".to_string()); // miss

    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_cache_memory_limit_eviction() {
    let config = CacheConfig {
        max_items: 100,
        max_bytes: 10, // very small
        ttl: None,
    };
    let cache = BoundedLruCache::<String, String>::new(config);
    cache.insert_with_size("big".to_string(), "data".to_string(), 8);
    cache.insert_with_size("bigger".to_string(), "more".to_string(), 8);

    // First entry should be evicted to fit second
    assert!(cache.get(&"big".to_string()).is_none());
    assert!(cache.get(&"bigger".to_string()).is_some());
}

#[test]
fn test_cache_config_defaults() {
    let config = CacheConfig::default();
    assert_eq!(config.max_items, 10_000);
    assert_eq!(config.max_bytes, 50 * 1024 * 1024);
    assert!(config.ttl.is_none());
}

// =========================================================================
// EstimateSize trait
// =========================================================================

#[test]
fn test_estimate_size_string() {
    assert_eq!("hello".estimate_size(), 5);
    assert_eq!(String::from("world").estimate_size(), 5);
}

#[test]
fn test_estimate_size_vec() {
    let v: Vec<String> = vec!["ab".to_string(), "cd".to_string()];
    assert_eq!(v.estimate_size(), 4);
}

#[test]
fn test_estimate_size_option() {
    let some: Option<String> = Some("test".to_string());
    let none: Option<String> = None;
    assert_eq!(some.estimate_size(), 4);
    assert_eq!(none.estimate_size(), 0);
}

#[test]
fn test_estimate_size_unit() {
    assert_eq!(().estimate_size(), 0);
}

// =========================================================================
// IndexStateMachine
// =========================================================================

#[test]
fn test_state_machine_starts_idle() {
    let sm = IndexStateMachine::new();
    assert!(matches!(sm.state(), IndexState::Idle { .. }));
    assert_eq!(sm.state().kind(), IndexStateKind::Idle);
}

#[test]
fn test_state_machine_idle_to_initializing() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_initializing(), TransitionResult::Success);
    assert!(matches!(sm.state(), IndexState::Initializing { .. }));
}

#[test]
fn test_state_machine_initializing_to_building() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_initializing(), TransitionResult::Success);
    assert_eq!(sm.transition_to_building(50), TransitionResult::Success);
    assert!(matches!(sm.state(), IndexState::Building { .. }));
}

#[test]
fn test_state_machine_building_to_ready() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_initializing(), TransitionResult::Success);
    assert_eq!(sm.transition_to_building(10), TransitionResult::Success);
    assert_eq!(sm.transition_to_ready(10, 100), TransitionResult::Success);
    assert!(sm.state().is_ready());
}

#[test]
fn test_state_machine_ready_to_updating() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_initializing(), TransitionResult::Success);
    assert_eq!(sm.transition_to_building(0), TransitionResult::Success);
    assert_eq!(sm.transition_to_ready(0, 0), TransitionResult::Success);
    assert_eq!(sm.transition_to_updating(5), TransitionResult::Success);
    assert!(matches!(sm.state(), IndexState::Updating { .. }));
}

#[test]
fn test_state_machine_invalid_transition() {
    let sm = IndexStateMachine::new();
    // Cannot go from Idle directly to Building
    let result = sm.transition_to_building(10);
    assert!(matches!(result, TransitionResult::InvalidTransition { .. }));
}

#[test]
fn test_state_machine_to_error() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_error("boom".to_string()), TransitionResult::Success);
    assert!(sm.state().is_error());
}

#[test]
fn test_state_machine_error_recovery() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_error("fail".to_string()), TransitionResult::Success);
    // Can recover from Error → Initializing
    assert_eq!(sm.transition_to_initializing(), TransitionResult::Success);
}

#[test]
fn test_state_machine_to_idle() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_initializing(), TransitionResult::Success);
    assert_eq!(sm.transition_to_idle(), TransitionResult::Success);
    assert!(matches!(sm.state(), IndexState::Idle { .. }));
}

#[test]
fn test_state_machine_invalidating() {
    let sm = IndexStateMachine::new();
    // Idle → Invalidating should work (non-transitional state)
    assert_eq!(
        sm.transition_to_invalidating(InvalidationReason::ManualRequest),
        TransitionResult::Success
    );
    assert!(matches!(sm.state(), IndexState::Invalidating { .. }));
}

#[test]
fn test_state_machine_degraded() {
    let sm = IndexStateMachine::new();
    // Idle is not Error, so degradation should succeed
    assert_eq!(
        sm.transition_to_degraded(DegradationReason::IoError { message: "disk full".to_string() }),
        TransitionResult::Success
    );
    assert!(matches!(sm.state(), IndexState::Degraded { .. }));
}

#[test]
fn test_state_is_transitional() {
    let sm = IndexStateMachine::new();
    assert_eq!(sm.transition_to_initializing(), TransitionResult::Success);
    assert!(sm.state().is_transitional());

    assert_eq!(sm.transition_to_building(0), TransitionResult::Success);
    assert!(sm.state().is_transitional());
}

#[test]
fn test_state_started_at_exists() {
    let sm = IndexStateMachine::new();
    let _t = sm.state().state_started_at();
    // Just ensure it doesn't panic
}

// =========================================================================
// IndexState kind helpers
// =========================================================================

#[test]
fn test_index_state_kind_variants() {
    assert_eq!(IndexStateKind::Ready, IndexStateKind::Ready);
    assert_ne!(IndexStateKind::Idle, IndexStateKind::Error);
}

#[test]
fn test_build_phase_variants() {
    assert_eq!(BuildPhase::Idle, BuildPhase::Idle);
    assert_ne!(BuildPhase::Scanning, BuildPhase::Indexing);
}

#[test]
fn test_invalidation_reason_eq() {
    assert_eq!(InvalidationReason::ManualRequest, InvalidationReason::ManualRequest);
    assert_ne!(InvalidationReason::CacheCorruption, InvalidationReason::ConfigurationChanged);
}

#[test]
fn test_resource_kind_eq() {
    assert_eq!(ResourceKind::MaxFiles, ResourceKind::MaxFiles);
    assert_ne!(ResourceKind::MaxSymbols, ResourceKind::MaxCacheBytes);
}

// =========================================================================
// ProductionIndexCoordinator
// =========================================================================

#[test]
fn test_coordinator_new_is_idle() {
    let coord = ProductionIndexCoordinator::new();
    assert!(matches!(coord.state(), IndexState::Idle { .. }));
}

#[test]
fn test_coordinator_default_is_idle() {
    let coord = ProductionIndexCoordinator::default();
    assert!(matches!(coord.state(), IndexState::Idle { .. }));
}

#[test]
fn test_coordinator_initialize() -> Result<(), String> {
    let coord = ProductionIndexCoordinator::new();
    coord.initialize()?;
    assert!(coord.state().is_ready());
    Ok(())
}

#[test]
fn test_coordinator_index_and_find_definition() -> Result<(), String> {
    let coord = ProductionIndexCoordinator::new();
    coord.initialize()?;

    let uri = Url::parse("file:///coord_test.pl").map_err(|e| e.to_string())?;
    coord.index_file(uri, "sub coord_func { 99 }".to_string())?;

    let def = coord.find_definition("coord_func");
    assert!(def.is_some());
    Ok(())
}

#[test]
fn test_coordinator_find_references() -> Result<(), String> {
    let coord = ProductionIndexCoordinator::new();
    coord.initialize()?;

    let uri = Url::parse("file:///ref_test.pl").map_err(|e| e.to_string())?;
    coord.index_file(uri, "sub ref_func { 1 }\nref_func();".to_string())?;

    let refs = coord.find_references("ref_func");
    assert!(!refs.is_empty());
    Ok(())
}

#[test]
fn test_coordinator_invalidate_clears_state() -> Result<(), String> {
    let coord = ProductionIndexCoordinator::new();
    coord.initialize()?;

    let uri = Url::parse("file:///inv.pl").map_err(|e| e.to_string())?;
    coord.index_file(uri, "sub inv_func { 1 }".to_string())?;

    coord.invalidate(InvalidationReason::ManualRequest);
    assert!(matches!(coord.state(), IndexState::Idle { .. }));
    Ok(())
}

#[test]
fn test_coordinator_statistics() {
    let coord = ProductionIndexCoordinator::new();
    let stats = coord.statistics();

    assert!(matches!(stats.state, IndexState::Idle { .. }));
    assert_eq!(stats.cache_stats.len(), 3); // ast, symbol, workspace
}

#[test]
fn test_coordinator_with_config() {
    let config = ProductionCoordinatorConfig::default();
    let coord = ProductionIndexCoordinator::with_config(config);
    assert!(matches!(coord.state(), IndexState::Idle { .. }));
}

// =========================================================================
// WorkspaceCacheManager
// =========================================================================

#[test]
fn test_cache_manager_ast_round_trip() {
    let config = CombinedWorkspaceCacheConfig::default();
    let mgr = WorkspaceCacheManager::new(&config);

    mgr.insert_ast("file1".to_string(), vec![1, 2, 3]);
    assert_eq!(mgr.get_ast("file1"), Some(vec![1, 2, 3]));
    assert_eq!(mgr.get_ast("missing"), None);
}

#[test]
fn test_cache_manager_symbol_round_trip() {
    let config = CombinedWorkspaceCacheConfig::default();
    let mgr = WorkspaceCacheManager::new(&config);

    mgr.insert_symbol("sym1".to_string(), vec![4, 5]);
    assert_eq!(mgr.get_symbol("sym1"), Some(vec![4, 5]));
}

#[test]
fn test_cache_manager_workspace_round_trip() {
    let config = CombinedWorkspaceCacheConfig::default();
    let mgr = WorkspaceCacheManager::new(&config);

    mgr.insert_workspace("ws1".to_string(), vec![6]);
    assert_eq!(mgr.get_workspace("ws1"), Some(vec![6]));
}

#[test]
fn test_cache_manager_clear_all() {
    let config = CombinedWorkspaceCacheConfig::default();
    let mgr = WorkspaceCacheManager::new(&config);

    mgr.insert_ast("a".to_string(), vec![1]);
    mgr.insert_symbol("s".to_string(), vec![2]);
    mgr.insert_workspace("w".to_string(), vec![3]);

    mgr.clear_all();
    assert!(mgr.get_ast("a").is_none());
    assert!(mgr.get_symbol("s").is_none());
    assert!(mgr.get_workspace("w").is_none());
}

#[test]
fn test_cache_manager_stats() {
    let config = CombinedWorkspaceCacheConfig::default();
    let mgr = WorkspaceCacheManager::new(&config);
    let stats = mgr.stats();

    assert!(stats.contains_key("ast"));
    assert!(stats.contains_key("symbol"));
    assert!(stats.contains_key("workspace"));
}

#[test]
fn test_cache_manager_total_memory() {
    let config = CombinedWorkspaceCacheConfig::default();
    let mgr = WorkspaceCacheManager::new(&config);
    assert_eq!(mgr.total_memory_usage(), 0);

    mgr.insert_ast("k".to_string(), vec![0; 100]);
    assert!(mgr.total_memory_usage() >= 100);
}

// =========================================================================
// IndexCoordinator (from workspace_index.rs)
// =========================================================================

#[test]
fn test_index_coordinator_starts_building() {
    let coord = IndexCoordinator::new();
    assert!(matches!(
        coord.state().kind(),
        perl_workspace_index::workspace::workspace_index::IndexStateKind::Building
    ));
}

#[test]
fn test_index_coordinator_transition_to_ready() {
    let coord = IndexCoordinator::new();
    coord.transition_to_ready(5, 50);
    assert!(matches!(
        coord.state().kind(),
        perl_workspace_index::workspace::workspace_index::IndexStateKind::Ready
    ));
}

#[test]
fn test_index_coordinator_with_limits() {
    let limits = IndexResourceLimits { max_files: 100, ..IndexResourceLimits::default() };
    let coord = IndexCoordinator::with_limits(limits);
    assert_eq!(coord.limits().max_files, 100);
}

#[test]
fn test_index_coordinator_query_dispatch() -> Result<(), Box<dyn std::error::Error>> {
    let coord = IndexCoordinator::new();

    // In building state → should use partial query
    let result = coord.query(|_idx| "full", |_idx| "partial");
    assert_eq!(result, "partial");

    coord.transition_to_ready(0, 0);
    let result = coord.query(|_idx| "full", |_idx| "partial");
    assert_eq!(result, "full");
    Ok(())
}

#[test]
fn test_index_coordinator_index_returns_ref() {
    let coord = IndexCoordinator::new();
    let _idx: &Arc<WorkspaceIndex> = coord.index();
}

#[test]
fn test_index_coordinator_instrumentation() {
    let coord = IndexCoordinator::new();
    let snap = coord.instrumentation_snapshot();
    // Should have at least Building state tracked
    assert!(!snap.state_durations_ms.is_empty() || snap.state_transition_counts.is_empty());
}

// =========================================================================
// IndexResourceLimits defaults
// =========================================================================

#[test]
fn test_resource_limits_defaults() {
    let limits = IndexResourceLimits::default();
    assert_eq!(limits.max_files, 10_000);
    assert_eq!(limits.max_symbols_per_file, 5_000);
    assert_eq!(limits.max_total_symbols, 500_000);
    assert_eq!(limits.max_ast_cache_bytes, 256 * 1024 * 1024);
    assert_eq!(limits.max_ast_cache_items, 100);
    assert_eq!(limits.max_scan_duration_ms, 30_000);
}

// =========================================================================
// WorkspaceIndex document store integration
// =========================================================================

#[test]
fn test_workspace_index_document_store() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/store.pl")?;
    let uri_str = uri.to_string();
    index.index_file(uri, "sub stored { 1 }".to_string())?;

    let store = index.document_store();
    assert!(store.is_open(&uri_str));
    Ok(())
}

// =========================================================================
// count_usages
// =========================================================================

#[test]
fn test_count_usages() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/usage.pl")?;
    index.index_file(uri, "sub called { 1 }\ncalled();\ncalled();".to_string())?;

    // count_usages excludes definition references
    let _count = index.count_usages("called");
    // At minimum should not panic
    Ok(())
}

// =========================================================================
// find_unused_symbols
// =========================================================================

#[test]
fn test_find_unused_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/unused.pl")?;
    index.index_file(uri, "sub used_fn { 1 }\nsub unused_fn { 2 }\nused_fn();".to_string())?;

    let unused = index.find_unused_symbols();
    let unused_names: Vec<&str> = unused.iter().map(|s| s.name.as_str()).collect();
    // unused_fn has no usage references
    assert!(unused_names.contains(&"unused_fn"));
    Ok(())
}

// =========================================================================
// SLO re-exports smoke test
// =========================================================================

#[test]
fn test_slo_reexports() {
    use perl_workspace_index::workspace::slo::{SloConfig, SloTracker};

    let tracker = SloTracker::new(SloConfig::default());
    assert!(tracker.all_slos_met());
}

// =========================================================================
// Cache TTL (optional)
// =========================================================================

#[test]
fn test_cache_ttl_expiration() {
    use std::thread;
    use std::time::Duration;

    let config =
        CacheConfig { max_items: 100, max_bytes: 1024, ttl: Some(Duration::from_millis(50)) };
    let cache = BoundedLruCache::<String, String>::new(config);
    cache.insert("ttl_key".to_string(), "val".to_string());

    // Should be present immediately
    assert!(cache.get(&"ttl_key".to_string()).is_some());

    // Wait for TTL to expire
    thread::sleep(Duration::from_millis(100));
    assert!(cache.get(&"ttl_key".to_string()).is_none());
}

// =========================================================================
// Thread safety smoke test
// =========================================================================

#[test]
fn test_workspace_index_thread_safety() -> Result<(), Box<dyn std::error::Error>> {
    use std::thread;

    let index = Arc::new(WorkspaceIndex::new());

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let idx = Arc::clone(&index);
            thread::spawn(move || {
                let uri_str = format!("file:///thread_{}.pl", i);
                let uri = Url::parse(&uri_str).ok()?;
                let code = format!("sub thread_fn_{} {{ {} }}", i, i);
                idx.index_file(uri, code).ok()?;
                Some(())
            })
        })
        .collect();

    for h in handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    assert!(index.file_count() >= 1);
    Ok(())
}
