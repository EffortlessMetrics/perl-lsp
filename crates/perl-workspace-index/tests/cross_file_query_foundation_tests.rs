use perl_workspace::workspace::workspace_index::{StableSymbolId, WorkspaceIndex};
use url::Url;

#[test]
fn query_symbol_returns_definition_and_cross_file_references()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(
        Url::parse("file:///workspace/lib/A.pm")?,
        "package Demo::A;\nsub run { return 1; }\n1;".to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/lib/B.pm")?,
        "package Demo::B;\nuse Demo::A;\nsub use_run { Demo::A::run(); }\n1;".to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/lib/C.pm")?,
        "package Demo::C;\nuse Demo::A;\nsub call_run { run(); }\n1;".to_string(),
    )?;

    let queried =
        index.query_symbol("Demo::A::run").ok_or_else(|| "expected query result".to_string())?;

    assert_eq!(queried.symbol_id.0, "sub:Demo::A::run");
    assert_eq!(queried.definition.uri, "file:///workspace/lib/A.pm");

    let uris: Vec<&str> = queried.references.iter().map(|loc| loc.uri.as_str()).collect();
    assert_eq!(
        uris,
        vec![
            "file:///workspace/lib/A.pm",
            "file:///workspace/lib/B.pm",
            "file:///workspace/lib/C.pm",
        ],
        "references should be deterministic and cross-file",
    );

    let by_id = index
        .query_symbol_by_id(&StableSymbolId("sub:Demo::A::run".to_string()))
        .ok_or_else(|| "expected id query result".to_string())?;

    assert_eq!(by_id.symbol_id.0, queried.symbol_id.0);
    assert_eq!(by_id.definition.uri, queried.definition.uri);
    assert_eq!(by_id.references.len(), queried.references.len());

    Ok(())
}

#[test]
fn query_symbol_reports_not_found_cleanly() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(
        Url::parse("file:///workspace/lib/Only.pm")?,
        "package Demo::Only;\nsub known { return 1; }\n1;".to_string(),
    )?;

    assert!(index.query_symbol("Demo::Only::missing").is_none());
    assert!(
        index.query_symbol_by_id(&StableSymbolId("sub:Demo::Only::missing".to_string())).is_none()
    );

    Ok(())
}
