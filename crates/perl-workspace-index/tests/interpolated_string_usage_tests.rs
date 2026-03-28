use perl_workspace_index::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{path}"))?)
}

#[test]
fn interpolated_string_variable_is_not_unused() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/interpolated.pl")?;
    index.index_file(uri, "my $name = 'World';\nprint \"Hello, $name!\\n\";\n".to_string())?;

    let unused = index.find_unused_symbols();
    let unused_names: Vec<&str> = unused.iter().map(|symbol| symbol.name.as_str()).collect();
    assert!(
        !unused_names.contains(&"$name"),
        "$name should be treated as used when referenced from an interpolated string: {unused_names:?}"
    );

    Ok(())
}
