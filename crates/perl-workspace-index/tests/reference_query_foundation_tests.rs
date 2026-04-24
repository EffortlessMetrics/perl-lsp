use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

#[test]
fn query_symbol_references_returns_cross_file_results() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let def_uri = file_url("/lib/Math.pm")?;
    let caller_a_uri = file_url("/scripts/use_math.pl")?;
    let caller_b_uri = file_url("/scripts/qualified.pl")?;

    index.index_file(def_uri, "package Math;\nsub add { return 1; }\n".to_string())?;
    index.index_file(caller_a_uri, "add();\n".to_string())?;
    index.index_file(caller_b_uri, "Math::add();\n".to_string())?;

    let query = index
        .query_symbol_references("Math::add")
        .ok_or("expected query result for Math::add")?;

    assert_eq!(query.identity.qualified_name.as_deref(), Some("Math::add"));
    assert_eq!(query.identity.stable_key, "Math::add");
    assert!(query.references.len() >= 2, "expected cross-file references for Math::add");
    assert_eq!(query.definition.uri, "file:///lib/Math.pm");

    let ordered_uris: Vec<&str> = query.references.iter().map(|loc| loc.uri.as_str()).collect();
    assert_eq!(ordered_uris, vec!["file:///scripts/qualified.pl", "file:///scripts/use_math.pl"]);

    Ok(())
}

#[test]
fn query_symbol_references_separates_definition_from_references(
) -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Single.pm")?;

    index.index_file(uri, "package Single;\nsub helper { return 1; }\nhelper();\n".to_string())?;

    let query = index
        .query_symbol_references("Single::helper")
        .ok_or("expected query result for Single::helper")?;

    assert_eq!(query.definition.uri, "file:///lib/Single.pm");
    assert!(query.references.iter().all(|loc| {
        !(loc.uri == query.definition.uri
            && loc.range.start.line == query.definition.range.start.line
            && loc.range.start.column == query.definition.range.start.column
            && loc.range.end.line == query.definition.range.end.line
            && loc.range.end.column == query.definition.range.end.column)
    }));

    Ok(())
}
