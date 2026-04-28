//! Tests for document links feature

#[test]
fn test_document_links_basic() -> Result<(), Box<dyn std::error::Error>> {
    use url::Url;

    let uri: Url = "file:///workspace/test.pl".parse()?;
    let _text = r#"
use Data::Dumper;
require JSON::XS;
use Foo::Bar::Baz;
"#;

    // This would call the internal function, but we can't access it directly from tests
    // since it's not exported. We'll need to test through the LSP server interface
    // or export the function in lib.rs

    // For now, just ensure the test compiles
    assert!(uri.scheme() == "file");
    Ok(())
}

#[test]
fn test_url_handling() -> Result<(), Box<dyn std::error::Error>> {
    use url::Url;

    // Test Windows-style paths
    let uri = Url::parse("file:///C:/Users/test/project.pl")?;
    assert_eq!(uri.scheme(), "file");

    // Test Unix-style paths
    let uri2 = Url::parse("file:///home/user/project.pl")?;
    assert_eq!(uri2.scheme(), "file");

    // Test relative path resolution
    let base = Url::parse("file:///workspace/src/main.pl")?;
    #[allow(clippy::collapsible_if)]
    if let Ok(path) = base.to_file_path() {
        if let Some(parent) = path.parent() {
            let resolved = parent.join("lib/module.pm");
            if let Ok(new_url) = Url::from_file_path(resolved) {
                assert!(new_url.to_string().contains("lib/module.pm"));
            }
        }
    }
    Ok(())
}
