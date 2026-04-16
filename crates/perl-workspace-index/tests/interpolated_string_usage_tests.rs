use perl_workspace::workspace::workspace_index::WorkspaceIndex;
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

#[test]
fn escaped_interpolated_string_variable_is_unused() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/escaped.pl")?;
    index.index_file(uri, "my $name = 'World';\nprint \"\\$name\\n\";\n".to_string())?;

    let unused = index.find_unused_symbols();
    let unused_names: Vec<&str> = unused.iter().map(|symbol| symbol.name.as_str()).collect();
    assert!(
        unused_names.contains(&"$name"),
        "$name escaped in a string should still be unused: {unused_names:?}"
    );

    Ok(())
}

#[test]
fn heredoc_interpolated_variable_is_not_unused() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/heredoc.pl")?;
    index.index_file(uri, "my $name = 'World';\nprint <<\"EOF\";\n$name\nEOF\n".to_string())?;

    let unused = index.find_unused_symbols();
    let unused_names: Vec<&str> = unused.iter().map(|symbol| symbol.name.as_str()).collect();
    assert!(
        !unused_names.contains(&"$name"),
        "$name used in interpolated heredoc should not be unused: {unused_names:?}"
    );

    Ok(())
}

#[test]
fn hash_marker_in_string_does_not_count_as_use() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/hash.pl")?;
    index.index_file(uri, "my %seen = (name => 1);\nprint \"%seen\\n\";\n".to_string())?;

    let unused = index.find_unused_symbols();
    let unused_names: Vec<&str> = unused.iter().map(|symbol| symbol.name.as_str()).collect();
    assert!(
        unused_names.contains(&"%seen"),
        "%seen in a string should not count as interpolation: {unused_names:?}"
    );

    Ok(())
}
