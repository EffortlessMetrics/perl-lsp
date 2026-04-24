use anyhow::Result;
use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

#[test]
fn exported_symbols_are_indexed_and_queryable_across_files() -> Result<()> {
    let index = WorkspaceIndex::new();

    let exporter_uri = Url::parse("file:///workspace/lib/My/Exporter.pm")?;
    let exporter_source = r#"
package My::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(greet);

sub greet { return "hello"; }
1;
"#;
    index
        .index_file(exporter_uri.clone(), exporter_source.to_string())
        .map_err(anyhow::Error::msg)?;

    let consumer_uri = Url::parse("file:///workspace/lib/My/Consumer.pm")?;
    let consumer_source = r#"
package My::Consumer;
use My::Exporter qw(greet);
sub run { return greet(); }
1;
"#;
    index.index_file(consumer_uri, consumer_source.to_string()).map_err(anyhow::Error::msg)?;

    let location = index.find_exported_symbol_definition("My::Exporter", "greet");
    let Some(location) = location else {
        return Err(anyhow::anyhow!("expected My::Exporter::greet export to resolve"));
    };

    assert_eq!(location.uri, exporter_uri.to_string());
    assert!(index.find_exported_symbol_definition("My::Exporter", "missing").is_none());
    assert!(index.find_exported_symbol_definition("No::Such::Module", "greet").is_none());

    Ok(())
}
