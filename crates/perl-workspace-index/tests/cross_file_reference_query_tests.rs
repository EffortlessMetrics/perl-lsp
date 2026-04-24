use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

#[test]
fn query_symbol_references_returns_cross_file_definition_and_references(
) -> Result<(), Box<dyn std::error::Error>> {
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
fn query_symbol_references_avoids_false_positives_for_ambiguous_bare_symbols(
) -> Result<(), Box<dyn std::error::Error>> {
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
