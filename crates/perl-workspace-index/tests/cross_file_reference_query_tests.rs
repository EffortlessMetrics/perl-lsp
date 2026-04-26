use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

// --- edge cases added by deep-review ---

#[test]
fn query_symbol_references_returns_none_on_empty_index() {
    let index = WorkspaceIndex::new();
    assert!(index.query_symbol_references("anything").is_none());
    assert!(index.query_symbol_references("A::B::C").is_none());
    assert!(index.query_symbol_references("").is_none());
}

#[test]
fn query_symbol_references_definition_always_in_references()
-> Result<(), Box<dyn std::error::Error>> {
    // The spec says references includes the definition site even when there are no callers.
    let index = WorkspaceIndex::new();
    index.index_file(
        file_url("/workspace/lib/Standalone.pm")?,
        "package Standalone;\nsub lone_wolf { 1 }\n".to_string(),
    )?;

    let query =
        index.query_symbol_references("Standalone::lone_wolf").ok_or("query should resolve")?;

    assert!(
        query.references.iter().any(|loc| loc.uri == query.definition.uri),
        "definition site must be present in references vec"
    );
    assert_eq!(query.definition.uri, "file:///workspace/lib/Standalone.pm");
    Ok(())
}

#[test]
fn query_symbol_references_is_stable_after_reindex() -> Result<(), Box<dyn std::error::Error>> {
    // Idempotency: re-indexing a file with identical content must not change results.
    let index = WorkspaceIndex::new();
    let def_uri = file_url("/workspace/lib/Svc.pm")?;
    let caller_uri = file_url("/workspace/lib/Cli.pm")?;
    let src = "package Svc;\nsub run { 1 }\n".to_string();

    index.index_file(def_uri.clone(), src.clone())?;
    index.index_file(caller_uri, "package Cli;\nSvc::run();\n".to_string())?;

    let first = index.query_symbol_references("Svc::run").ok_or("first query must resolve")?;

    // Re-index the definition file with identical content — must be idempotent.
    index.index_file(def_uri, src)?;

    let second = index.query_symbol_references("Svc::run").ok_or("second query must resolve")?;

    assert_eq!(
        first.symbol.stable_key, second.symbol.stable_key,
        "stable_key must not change on reindex with same content"
    );
    assert_eq!(
        first.references.len(),
        second.references.len(),
        "reference count must be stable after reindex with same content"
    );
    Ok(())
}

// --- original builder tests ---

#[test]
fn query_symbol_references_returns_cross_file_definition_and_references()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let def_uri = file_url("/workspace/lib/Service.pm")?;
    let call_a_uri = file_url("/workspace/lib/CallerA.pm")?;
    let call_b_uri = file_url("/workspace/bin/run.pl")?;

    index.index_file(def_uri, "package Service;\nsub process_payload { 1 }\n".to_string())?;
    index.index_file(call_a_uri, "package CallerA;\nService::process_payload();\n".to_string())?;
    index.index_file(call_b_uri, "package main;\nprocess_payload();\n".to_string())?;

    let query =
        index.query_symbol_references("Service::process_payload").ok_or("query should resolve")?;

    assert_eq!(query.symbol.stable_key, "Service::process_payload");
    assert_eq!(query.symbol.qualified_name.as_deref(), Some("Service::process_payload"));

    let references: Vec<&str> =
        query.references.iter().map(|location| location.uri.as_str()).collect();
    assert_eq!(
        references,
        vec![
            "file:///workspace/bin/run.pl",
            "file:///workspace/lib/CallerA.pm",
            "file:///workspace/lib/Service.pm",
        ]
    );

    assert_eq!(query.definition.uri, "file:///workspace/lib/Service.pm");

    Ok(())
}

#[test]
fn query_symbol_references_returns_none_for_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/workspace/lib/Only.pm")?;
    index.index_file(uri, "package Only;\nsub existing { 1 }\n".to_string())?;

    assert!(index.query_symbol_references("Only::missing").is_none());
    assert!(index.query_symbol_references("missing").is_none());

    Ok(())
}

#[test]
fn query_symbol_references_avoids_false_positives_for_ambiguous_bare_symbols()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(
        file_url("/workspace/lib/A.pm")?,
        "package A;\nsub collide { 1 }\n".to_string(),
    )?;
    index.index_file(
        file_url("/workspace/lib/B.pm")?,
        "package B;\nsub collide { 1 }\n".to_string(),
    )?;
    index.index_file(
        file_url("/workspace/lib/Caller.pm")?,
        "package Caller;\ncollide();\n".to_string(),
    )?;

    let query = index.query_symbol_references("A::collide").ok_or("query should resolve")?;

    let reference_uris: Vec<&str> =
        query.references.iter().map(|location| location.uri.as_str()).collect();
    assert_eq!(reference_uris, vec!["file:///workspace/lib/A.pm"]);

    Ok(())
}

// --- regression tests for #6799 ---

#[test]
fn static_method_calls_found_by_bare_name_query() -> Result<(), Box<dyn std::error::Error>> {
    // Regression test for #6799: MethodCall references must be stored under both the
    // qualified form (when static) AND the bare method name, mirroring the
    // FunctionCall storage pattern (workspace_index.rs:3127-3138).
    //
    // A bare-name reference query for `process` must find the static call
    // `Helper->process()` in Caller.pm. This passes today via the iterator-fallback
    // path in `find_references` (which scans `*::process` keys), but the underlying
    // storage asymmetry is what this test pins.
    let index = WorkspaceIndex::new();
    index.index_file(
        file_url("/lib/Helper.pm")?,
        "package Helper;\nsub process { 1 }\n".to_string(),
    )?;
    index.index_file(
        file_url("/lib/Caller.pm")?,
        "package Caller;\nHelper->process();\n".to_string(),
    )?;

    let refs = index.find_references("process");
    assert!(
        refs.iter().any(|loc| loc.uri.contains("Caller.pm")),
        "expected Helper->process() in Caller.pm to appear in bare-name reference query, \
         got refs: {:?}",
        refs.iter().map(|loc| loc.uri.as_str()).collect::<Vec<_>>(),
    );
    Ok(())
}

#[test]
fn static_method_callee_is_not_flagged_unused() -> Result<(), Box<dyn std::error::Error>> {
    // Regression test for #6799 (storage-shape demonstration): `find_unused_symbols`
    // performs an exact-key lookup of `fi.references.get(&symbol.name)` for the bare
    // sub name. Before the fix, MethodCall stored static method references only under
    // the qualified key (`Helper::process`), so the bare-key lookup `process` returned
    // None for Caller.pm, and `process` was incorrectly flagged as unused.
    let index = WorkspaceIndex::new();
    index.index_file(
        file_url("/lib/Helper.pm")?,
        "package Helper;\nsub process { 1 }\n".to_string(),
    )?;
    index.index_file(
        file_url("/lib/Caller.pm")?,
        "package Caller;\nHelper->process();\n".to_string(),
    )?;

    let unused: Vec<String> =
        index.find_unused_symbols().iter().map(|s| s.name.clone()).collect();
    assert!(
        !unused.contains(&"process".to_string()),
        "Helper::process is called via Helper->process() in Caller.pm and must not be \
         flagged unused; got unused={:?}",
        unused,
    );
    Ok(())
}
