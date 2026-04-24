use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

#[test]
fn query_symbol_across_files_returns_definition_identity_and_references()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let def_uri = file_url("/lib/Utils.pm")?;
    let caller_a = file_url("/app/a.pl")?;
    let caller_b = file_url("/app/b.pl")?;

    index.index_file(def_uri, "package Utils;\nsub process_data { return 1; }".to_string())?;
    index.index_file(caller_a, "use Utils;\nprocess_data();\n".to_string())?;
    index.index_file(caller_b, "use Utils;\nUtils::process_data();\n".to_string())?;

    let result = index
        .query_symbol_across_files("Utils::process_data")
        .ok_or("expected query to resolve symbol")?;

    assert_eq!(result.identity.key.pkg.as_ref(), "Utils");
    assert_eq!(result.identity.key.name.as_ref(), "process_data");
    assert_eq!(result.definition.uri, "file:///lib/Utils.pm");
    assert_eq!(result.identity.definition_uri.as_ref(), result.definition.uri);

    let uris: Vec<&str> = result.references.iter().map(|loc| loc.uri.as_str()).collect();
    assert_eq!(uris, vec!["file:///app/a.pl", "file:///app/b.pl"]);

    Ok(())
}

#[test]
fn query_symbol_across_files_not_found_is_none() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Only.pm")?;
    index.index_file(uri, "package Only;\nsub present { return 1; }".to_string())?;

    assert!(index.query_symbol_across_files("Only::missing").is_none());
    assert!(index.symbol_key_for("Only::missing").is_none());

    Ok(())
}
