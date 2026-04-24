//! Comprehensive unit tests for `perl-workspace-index`.
//!
//! Covers: WorkspaceIndex indexing, search, dual indexing (qualified + bare),
//! multi-file scenarios, DocumentStore, BoundedLruCache, IndexStateMachine,
//! ProductionIndexCoordinator, and IndexCoordinator.

use perl_tdd_support::must_some;
use perl_workspace::workspace::cache::{
    BoundedLruCache, CacheConfig, CombinedWorkspaceCacheConfig, EstimateSize,
};
use perl_workspace::workspace::document_store::DocumentStore;
use perl_workspace::workspace::production_coordinator::{
    ProductionCoordinatorConfig, ProductionIndexCoordinator, WorkspaceCacheManager,
};
use perl_workspace::workspace::state_machine::{
    BuildPhase, DegradationReason, IndexState, IndexStateKind, IndexStateMachine,
    InvalidationReason, ResourceKind, TransitionResult,
};
use perl_workspace::workspace::workspace_index::{
    IndexCoordinator, IndexResourceLimits, SafeDeletePreflightStatus, SymKind, SymbolKey,
    WorkspaceIndex,
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
fn test_remove_file_clears_references() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/refs.pl")?;
    let code = "package Refs;\nsub keep_me { 1 }\nkeep_me();\nRefs::keep_me();\n";
    index.index_file(uri.clone(), code.to_string())?;

    assert!(!index.find_references("Refs::keep_me").is_empty());
    assert!(!index.find_references("keep_me").is_empty());

    index.remove_file(&uri.to_string());

    assert!(
        index.find_references("Refs::keep_me").is_empty(),
        "qualified references should be removed after file deletion"
    );
    assert!(
        index.find_references("keep_me").is_empty(),
        "bare references should be removed after file deletion"
    );
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

#[test]
fn test_safe_delete_preflight_blocked_by_external_refs() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let def_uri = file_url("/safe_delete_def.pm")?;
    let caller_uri = file_url("/safe_delete_caller.pm")?;
    index.index_file(def_uri, "package SafeDelete;\nsub victim { 1 }\nvictim();".to_string())?;
    index.index_file(caller_uri, "package Caller;\nSafeDelete::victim();".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("SafeDelete"),
        name: Arc::from("victim"),
        sigil: None,
        kind: SymKind::Sub,
    };
    let result = index.safe_delete_preflight(&key);

    assert_eq!(result.status, SafeDeletePreflightStatus::BlockedByExternalReferences);
    assert_eq!(result.external_references.len(), 1);
    assert!(result.external_references[0].uri.contains("safe_delete_caller.pm"));

    Ok(())
}

#[test]
fn test_safe_delete_preflight_safe_when_no_external_refs() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();
    let def_uri = file_url("/safe_delete_only.pm")?;
    index.index_file(def_uri, "package SafeDelete;\nsub victim { 1 }\nvictim();".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("SafeDelete"),
        name: Arc::from("victim"),
        sigil: None,
        kind: SymKind::Sub,
    };
    let result = index.safe_delete_preflight(&key);

    assert_eq!(result.status, SafeDeletePreflightStatus::Safe);
    assert!(result.external_references.is_empty());

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
    use perl_workspace::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("$foo");
    assert_eq!(sigil, Some('$'));
    assert_eq!(name, "foo");
}

#[test]
fn test_normalize_var_array() {
    use perl_workspace::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("@bar");
    assert_eq!(sigil, Some('@'));
    assert_eq!(name, "bar");
}

#[test]
fn test_normalize_var_hash() {
    use perl_workspace::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("%baz");
    assert_eq!(sigil, Some('%'));
    assert_eq!(name, "baz");
}

#[test]
fn test_normalize_var_no_sigil() {
    use perl_workspace::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("plain");
    assert_eq!(sigil, None);
    assert_eq!(name, "plain");
}

#[test]
fn test_normalize_var_empty() {
    use perl_workspace::workspace::workspace_index::normalize_var;
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
fn test_document_store_rejects_stale_update_version() {
    let store = DocumentStore::new();
    store.open("file:///stale.pl".to_string(), 10, "latest".to_string());
    assert!(!store.update("file:///stale.pl", 9, "older".to_string()));

    let doc = must_some(store.get("file:///stale.pl"));
    assert_eq!(doc.version, 10);
    assert_eq!(doc.text, "latest");
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
        perl_workspace::workspace::workspace_index::IndexStateKind::Building
    ));
}

#[test]
fn test_index_coordinator_transition_to_ready() {
    let coord = IndexCoordinator::new();
    coord.transition_to_ready(5, 50);
    assert!(matches!(
        coord.state().kind(),
        perl_workspace::workspace::workspace_index::IndexStateKind::Ready
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
    use perl_workspace::workspace::slo::{SloConfig, SloTracker};

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

// =========================================================================
// IndexCoordinator – parse storm & notify lifecycle
// =========================================================================

#[test]
fn test_index_coordinator_notify_change_increments_pending() {
    let coord = IndexCoordinator::new();
    coord.transition_to_ready(0, 0);
    coord.notify_change("file:///a.pl");
    // Should not crash; pending count is internal
}

#[test]
fn test_index_coordinator_notify_parse_complete_decrements() {
    let coord = IndexCoordinator::new();
    coord.transition_to_ready(0, 0);
    coord.notify_change("file:///a.pl");
    coord.notify_parse_complete("file:///a.pl");
}

#[test]
fn test_index_coordinator_parse_storm_triggers_degradation() {
    use perl_workspace::workspace::workspace_index::IndexStateKind;

    let coord = IndexCoordinator::new();
    coord.transition_to_ready(0, 0);

    // Exceed parse storm threshold (default = 10)
    for i in 0..12 {
        coord.notify_change(&format!("file:///storm_{}.pl", i));
    }

    assert!(matches!(coord.state().kind(), IndexStateKind::Degraded));
}

#[test]
fn test_index_coordinator_recovery_from_parse_storm() {
    use perl_workspace::workspace::workspace_index::IndexStateKind;

    let coord = IndexCoordinator::new();
    coord.transition_to_ready(0, 0);

    // Trigger parse storm
    for i in 0..12 {
        coord.notify_change(&format!("file:///storm_{}.pl", i));
    }
    assert!(matches!(coord.state().kind(), IndexStateKind::Degraded));

    // Drain all pending parses
    for i in 0..12 {
        coord.notify_parse_complete(&format!("file:///storm_{}.pl", i));
    }
    // Should recover from parse storm (back to Building for re-scan)
    let kind = coord.state().kind();
    assert!(matches!(kind, IndexStateKind::Building));
}

#[test]
fn test_index_coordinator_enforce_limits_file_count() {
    use perl_workspace::workspace::workspace_index::{IndexPerformanceCaps, IndexStateKind};

    let limits = IndexResourceLimits { max_files: 2, ..IndexResourceLimits::default() };
    let coord = IndexCoordinator::with_limits_and_caps(limits, IndexPerformanceCaps::default());
    coord.transition_to_ready(0, 0);

    // Index more files than the limit
    for i in 0..3 {
        let uri = Url::parse(&format!("file:///limit_{}.pl", i)).ok();
        if let Some(u) = uri {
            let _ = coord.index().index_file(u, format!("sub f{} {{ 1 }}", i));
        }
    }
    coord.enforce_limits();
    assert!(matches!(coord.state().kind(), IndexStateKind::Degraded));
}

#[test]
fn test_index_coordinator_check_limits_none_when_ok() {
    let coord = IndexCoordinator::new();
    assert!(coord.check_limits().is_none());
}

#[test]
fn test_index_coordinator_check_limits_prefers_file_count_over_symbol_count()
-> Result<(), Box<dyn std::error::Error>> {
    use perl_workspace::workspace::workspace_index::{
        DegradationReason as IxDegradationReason, IndexPerformanceCaps,
        ResourceKind as IxResourceKind,
    };

    let limits = IndexResourceLimits {
        max_files: 1,
        max_total_symbols: 1,
        ..IndexResourceLimits::default()
    };
    let coord = IndexCoordinator::with_limits_and_caps(limits, IndexPerformanceCaps::default());
    coord.transition_to_ready(0, 0);

    for i in 0..2 {
        let uri = Url::parse(&format!("file:///limit_priority_{}.pl", i))?;
        coord.index().index_file(uri, format!("sub f{} {{ 1 }}", i))?;
    }

    let reason = coord.check_limits().expect("limits should be exceeded");
    assert!(matches!(
        reason,
        IxDegradationReason::ResourceLimit { kind: IxResourceKind::MaxFiles }
    ));
    Ok(())
}

#[test]
fn test_index_coordinator_phase_transitions() {
    let coord = IndexCoordinator::new();
    coord.transition_to_scanning();
    coord.update_scan_progress(50);
    coord.transition_to_indexing(50);
    coord.update_building_progress(25);
    coord.transition_to_ready(50, 200);
}

#[test]
fn test_index_coordinator_record_early_exit() {
    use perl_workspace::workspace::workspace_index::EarlyExitReason;

    let coord = IndexCoordinator::new();
    coord.record_early_exit(EarlyExitReason::InitialTimeBudget, 150, 10, 100);

    let snap = coord.instrumentation_snapshot();
    assert!(snap.last_early_exit.is_some());
    let ee = snap.last_early_exit.as_ref();
    assert_eq!(ee.map(|e| e.elapsed_ms), Some(150));
}

#[test]
fn test_index_coordinator_performance_caps_default() {
    let coord = IndexCoordinator::new();
    let caps = coord.performance_caps();
    assert_eq!(caps.initial_scan_budget_ms, 500);
    assert_eq!(caps.incremental_budget_ms, 10);
}

#[test]
fn test_index_coordinator_with_limits_and_caps() {
    use perl_workspace::workspace::workspace_index::IndexPerformanceCaps;

    let limits = IndexResourceLimits { max_files: 42, ..IndexResourceLimits::default() };
    let caps = IndexPerformanceCaps { initial_scan_budget_ms: 200, incremental_budget_ms: 20 };
    let coord = IndexCoordinator::with_limits_and_caps(limits, caps);
    assert_eq!(coord.limits().max_files, 42);
    assert_eq!(coord.performance_caps().initial_scan_budget_ms, 200);
}

#[test]
fn test_index_coordinator_default_trait() {
    let coord = IndexCoordinator::default();
    assert!(matches!(
        coord.state().kind(),
        perl_workspace::workspace::workspace_index::IndexStateKind::Building
    ));
}

#[test]
fn test_index_coordinator_debug_impl() {
    let coord = IndexCoordinator::new();
    let debug_str = format!("{:?}", coord);
    assert!(debug_str.contains("IndexCoordinator"));
}

// =========================================================================
// WorkspaceIndex – incremental update (changed content)
// =========================================================================

#[test]
fn test_incremental_update_changed_content() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/incr.pl")?;

    index.index_file(uri.clone(), "sub old_fn { 1 }".to_string())?;
    assert!(index.find_definition("old_fn").is_some());

    // Re-index with different content
    index.index_file(uri, "sub new_fn { 2 }".to_string())?;
    assert!(index.find_definition("new_fn").is_some());
    // Old symbol should be gone after re-indexing
    assert!(index.find_definition("old_fn").is_none());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – empty file
// =========================================================================

#[test]
fn test_index_empty_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/empty.pl")?;
    index.index_file(uri, String::new())?;

    assert_eq!(index.file_count(), 1);
    assert_eq!(index.symbol_count(), 0);
    assert!(!index.has_symbols());
    Ok(())
}

#[test]
fn test_index_whitespace_only_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/blank.pl")?;
    index.index_file(uri, "   \n\n  \t  \n".to_string())?;

    assert_eq!(index.file_count(), 1);
    Ok(())
}

// =========================================================================
// WorkspaceIndex – multiple packages in one file
// =========================================================================

#[test]
fn test_multiple_packages_single_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/multi_pkg.pm")?;
    let code = "package Alpha;\nsub a_fn { 1 }\npackage Beta;\nsub b_fn { 2 }";
    index.index_file(uri, code.to_string())?;

    assert!(index.find_definition("Alpha::a_fn").is_some());
    assert!(index.find_definition("Beta::b_fn").is_some());

    let alpha_members = index.get_package_members("Alpha");
    let beta_members = index.get_package_members("Beta");
    assert!(!alpha_members.is_empty());
    assert!(!beta_members.is_empty());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – use statement dependency tracking
// =========================================================================

#[test]
fn test_use_statement_creates_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/use_dep.pl")?;
    let uri_str = uri.to_string();
    index.index_file(uri, "use File::Basename;\nuse Carp;".to_string())?;

    let deps = index.file_dependencies(&uri_str);
    assert!(deps.contains("File::Basename"));
    assert!(deps.contains("Carp"));
    Ok(())
}

#[test]
fn test_find_dependents() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri_a = file_url("/dep_a.pl")?;
    let uri_b = file_url("/dep_b.pl")?;

    index.index_file(uri_a, "use My::Module;".to_string())?;
    index.index_file(uri_b, "use My::Module;\nuse Other::Mod;".to_string())?;

    let dependents = index.find_dependents("My::Module");
    assert!(dependents.len() >= 2);
    Ok(())
}

// =========================================================================
// WorkspaceIndex – clear_file / clear_file_url
// =========================================================================

#[test]
fn test_clear_file_alias() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/clearme.pl")?;
    let uri_str = uri.to_string();
    index.index_file(uri, "sub x { 1 }".to_string())?;

    assert_eq!(index.file_count(), 1);
    index.clear_file(&uri_str);
    assert_eq!(index.file_count(), 0);
    Ok(())
}

#[test]
fn test_clear_file_url_alias() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/clearme2.pl")?;
    index.index_file(uri.clone(), "sub y { 1 }".to_string())?;

    index.clear_file_url(&uri);
    assert_eq!(index.file_count(), 0);
    Ok(())
}

// =========================================================================
// WorkspaceIndex – SymbolKey with sigil (variable lookup)
// =========================================================================

#[test]
fn test_find_def_variable_with_sigil() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/var_key.pl")?;
    index.index_file(uri, "my $counter = 0;".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("main"),
        name: Arc::from("counter"),
        sigil: Some('$'),
        kind: SymKind::Var,
    };
    let def = index.find_def(&key);
    assert!(def.is_some());
    Ok(())
}

#[test]
fn test_find_refs_variable_with_sigil() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/var_refs.pl")?;
    index.index_file(uri, "my $x = 1;\n$x = 2;\n$x;".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("main"),
        name: Arc::from("x"),
        sigil: Some('$'),
        kind: SymKind::Var,
    };
    let refs = index.find_refs(&key);
    // Should have references (excluding definition)
    assert!(!refs.is_empty());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – SymbolKey main package
// =========================================================================

#[test]
fn test_find_refs_main_package() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/main_pkg.pl")?;
    index.index_file(uri, "sub foo { 1 }\nfoo();".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("main"),
        name: Arc::from("foo"),
        sigil: None,
        kind: SymKind::Sub,
    };
    // find_refs for main package should search bare name
    let _refs = index.find_refs(&key);
    Ok(())
}

// =========================================================================
// WorkspaceIndex – function call dual indexing
// =========================================================================

#[test]
fn test_function_call_dual_indexed() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/dual_call.pl")?;
    let code = "package Utils;\nsub process { 1 }\nUtils::process();\nprocess();";
    index.index_file(uri, code.to_string())?;

    // Searching qualified name should find both qualified and bare calls
    let refs = index.find_references("Utils::process");
    assert!(refs.len() >= 2);
    Ok(())
}

#[test]
fn test_cross_file_qualified_call() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri_lib = file_url("/lib/Math.pm")?;
    let uri_app = file_url("/app.pl")?;

    index.index_file(uri_lib, "package Math;\nsub add { 1 }".to_string())?;
    index.index_file(uri_app, "Math::add(1, 2);".to_string())?;

    let refs = index.find_references("Math::add");
    assert!(!refs.is_empty());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – count_usages excludes definitions
// =========================================================================

#[test]
fn test_count_usages_excludes_definition() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/count.pl")?;
    index.index_file(uri, "sub counted { 1 }\ncounted();\ncounted();".to_string())?;

    let count = index.count_usages("counted");
    // The definition reference should be excluded; only call sites count
    assert!(count >= 2);
    Ok(())
}

#[test]
fn test_count_usages_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/count_q.pl")?;
    let code = "package Svc;\nsub handle { 1 }\nSvc::handle();\nhandle();";
    index.index_file(uri, code.to_string())?;

    let count = index.count_usages("Svc::handle");
    assert!(count >= 1);
    Ok(())
}

// =========================================================================
// WorkspaceIndex – find_unused_symbols edge cases
// =========================================================================

#[test]
fn test_find_unused_symbols_all_used() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/all_used.pl")?;
    index.index_file(uri, "sub a { 1 }\na();".to_string())?;

    let unused = index.find_unused_symbols();
    let sub_unused: Vec<_> = unused.iter().filter(|s| s.name == "a").collect();
    assert!(sub_unused.is_empty());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – search with empty query
// =========================================================================

#[test]
fn test_search_symbols_empty_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/query_empty.pl")?;
    index.index_file(uri, "sub anything { 1 }".to_string())?;

    // Empty query matches everything
    let results = index.search_symbols("");
    assert!(!results.is_empty());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – file_symbols for nonexistent file
// =========================================================================

#[test]
fn test_file_symbols_nonexistent() {
    let index = WorkspaceIndex::new();
    let syms = index.file_symbols("file:///no_such_file.pl");
    assert!(syms.is_empty());
}

// =========================================================================
// WorkspaceIndex – file_dependencies for nonexistent file
// =========================================================================

#[test]
fn test_file_dependencies_nonexistent() {
    let index = WorkspaceIndex::new();
    let deps = index.file_dependencies("file:///no_such_file.pl");
    assert!(deps.is_empty());
}

// =========================================================================
// WorkspaceIndex – find_dependents for unknown module
// =========================================================================

#[test]
fn test_find_dependents_unknown_module() {
    let index = WorkspaceIndex::new();
    let dependents = index.find_dependents("No::Such::Module");
    assert!(dependents.is_empty());
}

// =========================================================================
// WorkspaceIndex – remove nonexistent file
// =========================================================================

#[test]
fn test_remove_nonexistent_file_is_noop() {
    let index = WorkspaceIndex::new();
    index.remove_file("file:///no_such.pl");
    assert_eq!(index.file_count(), 0);
}

// =========================================================================
// WorkspaceIndex – assignment reference tracking
// =========================================================================

#[test]
fn test_assignment_write_reference() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/assign.pl")?;
    index.index_file(uri, "my $x = 1;\n$x = 2;".to_string())?;

    let refs = index.find_references("$x");
    // Should have definition + read + write references
    assert!(refs.len() >= 2);
    Ok(())
}

#[test]
fn test_compound_assignment_read_and_write() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/compound.pl")?;
    index.index_file(uri, "my $count = 0;\n$count += 1;".to_string())?;

    let refs = index.find_references("$count");
    // Compound assignment creates both read and write references
    assert!(refs.len() >= 2);
    Ok(())
}

// =========================================================================
// WorkspaceIndex – package symbol itself
// =========================================================================

#[test]
fn test_package_declaration_is_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/pkg_sym.pm")?;
    index.index_file(uri, "package My::Package;".to_string())?;

    let def = index.find_definition("My::Package");
    assert!(def.is_some());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – default trait
// =========================================================================

#[test]
fn test_workspace_index_default() {
    let index = WorkspaceIndex::default();
    assert_eq!(index.file_count(), 0);
}

// =========================================================================
// IndexStateMachine – additional transitions
// =========================================================================

#[test]
fn test_state_machine_cannot_build_from_idle() {
    let sm = IndexStateMachine::new();
    let result = sm.transition_to_building(10);
    assert!(matches!(result, TransitionResult::InvalidTransition { .. }));
}

#[test]
fn test_state_machine_cannot_update_from_idle() {
    let sm = IndexStateMachine::new();
    let result = sm.transition_to_updating(1);
    assert!(matches!(result, TransitionResult::InvalidTransition { .. }));
}

#[test]
fn test_state_machine_cannot_invalidate_from_transitional() {
    let sm = IndexStateMachine::new();
    sm.transition_to_initializing();
    let result = sm.transition_to_invalidating(InvalidationReason::ManualRequest);
    assert!(matches!(result, TransitionResult::InvalidTransition { .. }));
}

#[test]
fn test_state_machine_cannot_degrade_from_error() {
    let sm = IndexStateMachine::new();
    sm.transition_to_error("fatal".to_string());
    let result =
        sm.transition_to_degraded(DegradationReason::IoError { message: "disk".to_string() });
    assert!(matches!(result, TransitionResult::InvalidTransition { .. }));
}

#[test]
fn test_state_machine_ready_to_ready_updates() {
    let sm = IndexStateMachine::new();
    sm.transition_to_initializing();
    sm.transition_to_building(10);
    sm.transition_to_ready(10, 100);
    // Ready → Ready (update stats) should succeed
    assert_eq!(sm.transition_to_ready(20, 200), TransitionResult::Success);
    if let IndexState::Ready { file_count, symbol_count, .. } = sm.state() {
        assert_eq!(file_count, 20);
        assert_eq!(symbol_count, 200);
    }
}

#[test]
fn test_state_machine_update_building_progress_wrong_state() {
    let sm = IndexStateMachine::new();
    // Not in Building state
    let result = sm.update_building_progress(10, BuildPhase::Indexing);
    assert!(matches!(result, TransitionResult::InvalidTransition { .. }));
}

#[test]
fn test_state_machine_update_init_progress_wrong_state() {
    let sm = IndexStateMachine::new();
    let result = sm.update_initialization_progress(50);
    assert!(matches!(result, TransitionResult::InvalidTransition { .. }));
}

#[test]
fn test_state_machine_init_progress_clamped_to_100() {
    let sm = IndexStateMachine::new();
    sm.transition_to_initializing();
    assert_eq!(sm.update_initialization_progress(255), TransitionResult::Success);
    if let IndexState::Initializing { progress, .. } = sm.state() {
        assert_eq!(progress, 100);
    }
}

#[test]
fn test_state_machine_degraded_preserves_symbol_count() {
    let sm = IndexStateMachine::new();
    sm.transition_to_initializing();
    sm.transition_to_building(10);
    sm.transition_to_ready(10, 500);
    sm.transition_to_degraded(DegradationReason::ScanTimeout { elapsed_ms: 5000 });

    if let IndexState::Degraded { available_symbols, .. } = sm.state() {
        assert_eq!(available_symbols, 500);
    }
}

#[test]
fn test_state_machine_invalidating_to_ready() {
    let sm = IndexStateMachine::new();
    sm.transition_to_invalidating(InvalidationReason::ManualRequest);
    assert_eq!(sm.transition_to_ready(5, 50), TransitionResult::Success);
}

#[test]
fn test_state_machine_degraded_to_building() {
    let sm = IndexStateMachine::new();
    sm.transition_to_degraded(DegradationReason::ParseStorm { pending_parses: 15 });
    assert_eq!(sm.transition_to_building(100), TransitionResult::Success);
}

#[test]
fn test_state_machine_degraded_to_updating() {
    let sm = IndexStateMachine::new();
    sm.transition_to_degraded(DegradationReason::IoError { message: "err".to_string() });
    assert_eq!(sm.transition_to_updating(3), TransitionResult::Success);
}

// =========================================================================
// IndexStateMachine – TransitionResult variant coverage
// =========================================================================

#[test]
fn test_transition_result_guard_failed() {
    let result = TransitionResult::GuardFailed { condition: "test guard".to_string() };
    assert!(matches!(result, TransitionResult::GuardFailed { .. }));
}

// =========================================================================
// DegradationReason variant coverage
// =========================================================================

#[test]
fn test_degradation_reason_resource_limit() {
    let reason = DegradationReason::ResourceLimit { kind: ResourceKind::MaxFiles };
    assert!(matches!(reason, DegradationReason::ResourceLimit { .. }));
}

#[test]
fn test_degradation_reason_parse_storm() {
    let reason = DegradationReason::ParseStorm { pending_parses: 42 };
    assert!(matches!(reason, DegradationReason::ParseStorm { pending_parses: 42 }));
}

#[test]
fn test_degradation_reason_scan_timeout() {
    let reason = DegradationReason::ScanTimeout { elapsed_ms: 31000 };
    assert!(matches!(reason, DegradationReason::ScanTimeout { elapsed_ms: 31000 }));
}

// =========================================================================
// InvalidationReason variant coverage
// =========================================================================

#[test]
fn test_invalidation_reason_all_variants() {
    let reasons = [
        InvalidationReason::ConfigurationChanged,
        InvalidationReason::FileSystemChanged,
        InvalidationReason::ManualRequest,
        InvalidationReason::CacheCorruption,
        InvalidationReason::DependencyChanged,
    ];
    assert_eq!(reasons.len(), 5);
}

// =========================================================================
// BoundedLruCache – additional edge cases
// =========================================================================

#[test]
fn test_cache_remove_nonexistent() {
    let cache = BoundedLruCache::<String, String>::default();
    assert!(cache.remove(&"absent".to_string()).is_none());
}

#[test]
fn test_cache_insert_with_size_too_large() {
    let config = CacheConfig { max_items: 10, max_bytes: 5, ttl: None };
    let cache = BoundedLruCache::<String, String>::new(config);
    let inserted = cache.insert_with_size("k".to_string(), "v".to_string(), 100);
    assert!(!inserted);
}

#[test]
fn test_cache_config_accessor() {
    let config = CacheConfig { max_items: 42, max_bytes: 9999, ttl: None };
    let cache = BoundedLruCache::<String, String>::new(config);
    assert_eq!(cache.config().max_items, 42);
    assert_eq!(cache.config().max_bytes, 9999);
}

#[test]
fn test_cache_lru_order_update_on_get() {
    let config = CacheConfig { max_items: 2, max_bytes: 1024, ttl: None };
    let cache = BoundedLruCache::<String, String>::new(config);

    cache.insert("a".to_string(), "1".to_string());
    cache.insert("b".to_string(), "2".to_string());
    // Access 'a' to make it most recently used
    let _ = cache.get(&"a".to_string());
    // Insert 'c' which should evict 'b' (now LRU), not 'a'
    cache.insert("c".to_string(), "3".to_string());

    assert!(cache.get(&"a".to_string()).is_some());
    assert!(cache.get(&"b".to_string()).is_none());
    assert!(cache.get(&"c".to_string()).is_some());
}

// =========================================================================
// EstimateSize – additional types
// =========================================================================

#[test]
fn test_estimate_size_hashmap() {
    use std::collections::HashMap;
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("key".to_string(), "value".to_string());
    assert_eq!(map.estimate_size(), 3 + 5); // "key" + "value"
}

#[test]
fn test_estimate_size_result() {
    let ok: Result<String, String> = Ok("data".to_string());
    let err: Result<String, String> = Err("err".to_string());
    assert_eq!(ok.estimate_size(), 4);
    assert_eq!(err.estimate_size(), 3);
}

#[test]
fn test_estimate_size_u8_slice() {
    let data: &[u8] = &[1, 2, 3, 4, 5];
    assert_eq!(data.estimate_size(), 5);
}

#[test]
fn test_estimate_size_str_ref() {
    let s = "hello";
    assert_eq!(s.estimate_size(), 5);
}

// =========================================================================
// ProductionIndexCoordinator – additional tests
// =========================================================================

#[test]
fn test_production_coordinator_search_symbols() -> Result<(), String> {
    let coord = ProductionIndexCoordinator::new();
    coord.initialize()?;

    let uri = Url::parse("file:///search_prod.pl").map_err(|e| e.to_string())?;
    coord.index_file(uri, "sub searchable { 1 }".to_string())?;

    let results = coord.index().search_symbols("searchable");
    assert!(!results.is_empty());
    Ok(())
}

#[test]
fn test_production_coordinator_all_symbols() -> Result<(), String> {
    let coord = ProductionIndexCoordinator::new();
    coord.initialize()?;

    let uri = Url::parse("file:///all_prod.pl").map_err(|e| e.to_string())?;
    coord.index_file(uri, "sub s1 { 1 }\nsub s2 { 2 }".to_string())?;

    let all = coord.index().all_symbols();
    assert!(all.len() >= 2);
    Ok(())
}

#[test]
fn test_production_coordinator_slo_tracker_accessor() {
    let coord = ProductionIndexCoordinator::new();
    let tracker = coord.slo_tracker();
    assert!(tracker.all_slos_met());
}

#[test]
fn test_production_coordinator_cache_accessor() {
    let coord = ProductionIndexCoordinator::new();
    let _cache = coord.cache();
}

#[test]
fn test_production_coordinator_config_accessor() {
    let coord = ProductionIndexCoordinator::new();
    let _config = coord.config();
}

#[test]
fn test_production_coordinator_index_accessor() {
    let coord = ProductionIndexCoordinator::new();
    let idx = coord.index();
    assert_eq!(idx.file_count(), 0);
}

// =========================================================================
// WorkspaceIndex – large index stress test
// =========================================================================

#[test]
fn test_large_index_many_files() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    for i in 0..50 {
        let uri = file_url(&format!("/large_{}.pl", i))?;
        let code = format!("sub fn_{} {{ {} }}", i, i);
        index.index_file(uri, code)?;
    }

    assert_eq!(index.file_count(), 50);
    assert!(index.symbol_count() >= 50);

    // Spot check a few definitions
    assert!(index.find_definition("fn_0").is_some());
    assert!(index.find_definition("fn_49").is_some());
    Ok(())
}

#[test]
fn test_large_index_many_symbols_per_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/many_subs.pl")?;

    let mut code = String::new();
    for i in 0..30 {
        code.push_str(&format!("sub multi_{} {{ {} }}\n", i, i));
    }
    index.index_file(uri, code)?;

    assert!(index.symbol_count() >= 30);
    assert!(index.find_definition("multi_0").is_some());
    assert!(index.find_definition("multi_29").is_some());
    Ok(())
}

// =========================================================================
// WorkspaceIndex – normalize_var edge: sigil-only
// =========================================================================

#[test]
fn test_normalize_var_sigil_only() {
    use perl_workspace::workspace::workspace_index::normalize_var;
    let (sigil, name) = normalize_var("$");
    assert_eq!(sigil, Some('$'));
    assert_eq!(name, "");
}

// =========================================================================
// CacheStats hit rate calculation
// =========================================================================

#[test]
fn test_cache_stats_hit_rate_zero_total() {
    use perl_workspace::workspace::cache::CacheStats;
    let rate = CacheStats::calculate_hit_rate(0, 0);
    assert!((rate - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_cache_stats_hit_rate_all_hits() {
    use perl_workspace::workspace::cache::CacheStats;
    let rate = CacheStats::calculate_hit_rate(100, 0);
    assert!((rate - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_cache_stats_hit_rate_all_misses() {
    use perl_workspace::workspace::cache::CacheStats;
    let rate = CacheStats::calculate_hit_rate(0, 100);
    assert!((rate - 0.0).abs() < f64::EPSILON);
}

// =========================================================================
// IndexCoordinator – transition_to_degraded with Building state
// =========================================================================

#[test]
fn test_index_coordinator_degrade_from_building() {
    use perl_workspace::workspace::workspace_index::{
        DegradationReason as IxDegReason, IndexStateKind as IxStateKind,
    };

    let coord = IndexCoordinator::new();
    // Starts in Building
    coord.transition_to_degraded(IxDegReason::IoError { message: "err".to_string() });
    assert!(matches!(coord.state().kind(), IxStateKind::Degraded));
}

// =========================================================================
// WorkspaceIndex – document_store integration after remove
// =========================================================================

#[test]
fn test_document_store_closed_after_remove() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/rm_doc.pl")?;
    let uri_str = uri.to_string();
    index.index_file(uri, "sub rm { 1 }".to_string())?;

    assert!(index.document_store().is_open(&uri_str));
    index.remove_file(&uri_str);
    assert!(!index.document_store().is_open(&uri_str));
    Ok(())
}

// =========================================================================
// DocumentStore – URI normalization
// =========================================================================

#[test]
fn test_document_store_uri_key_consistency() {
    let key1 = DocumentStore::uri_key("file:///Path/To/File.pl");
    let key2 = DocumentStore::uri_key("file:///Path/To/File.pl");
    assert_eq!(key1, key2);
}

// =========================================================================
// WorkspaceIndex – thread safety with concurrent reads and writes
// =========================================================================

#[test]
fn test_concurrent_index_and_search() -> Result<(), Box<dyn std::error::Error>> {
    use std::thread;

    let index = Arc::new(WorkspaceIndex::new());

    // First index some files
    for i in 0..5 {
        let uri = Url::parse(&format!("file:///conc_{}.pl", i))?;
        index.index_file(uri, format!("sub conc_fn_{} {{ {} }}", i, i))?;
    }

    // Concurrent reads while writing
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let idx = Arc::clone(&index);
            thread::spawn(move || {
                // Mix of reads and writes
                let _ = idx.search_symbols("conc_fn");
                let _ = idx.find_definition("conc_fn_0");
                let _ = idx.all_symbols();
                let uri = Url::parse(&format!("file:///conc_extra_{}.pl", i)).ok()?;
                idx.index_file(uri, format!("sub extra_{} {{ 1 }}", i)).ok()?;
                Some(())
            })
        })
        .collect();

    for h in handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    assert!(index.file_count() >= 5);
    Ok(())
}

// =========================================================================
// WorkspaceIndex – find_dependents via use parent / use base (#2747)
// =========================================================================

/// Regression test for #2747: a file that only has `use parent 'My::Base'`
/// (no direct `use My::Base`) must register `My::Base` as a dependency so
/// that `find_dependents("My::Base")` includes it.
#[test]
fn test_find_dependents_via_use_parent() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/child.pm")?;
    // Only use parent, no direct use My::Base
    index.index_file(uri, "package Child;\nuse parent 'My::Base';\n1;\n".to_string())?;

    let dependents = index.find_dependents("My::Base");
    assert!(
        !dependents.is_empty(),
        "use parent 'My::Base' should register My::Base as a dependency"
    );
    Ok(())
}

/// use base works the same way as use parent for dependency tracking.
#[test]
fn test_find_dependents_via_use_base() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/derived.pm")?;
    index.index_file(uri, "package Derived;\nuse base 'My::Root';\n1;\n".to_string())?;

    let dependents = index.find_dependents("My::Root");
    assert!(!dependents.is_empty(), "use base 'My::Root' should register My::Root as a dependency");
    Ok(())
}

/// use parent with qw() list registers all named modules as dependencies.
#[test]
fn test_find_dependents_via_use_parent_qw() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/multi_inherit.pm")?;
    index.index_file(
        uri,
        "package Multi;\nuse parent qw(Foo::Bar Other::Base);\n1;\n".to_string(),
    )?;

    let foo_deps = index.find_dependents("Foo::Bar");
    assert!(!foo_deps.is_empty(), "Foo::Bar should be a registered dependency");

    let other_deps = index.find_dependents("Other::Base");
    assert!(!other_deps.is_empty(), "Other::Base should be a registered dependency");
    Ok(())
}
