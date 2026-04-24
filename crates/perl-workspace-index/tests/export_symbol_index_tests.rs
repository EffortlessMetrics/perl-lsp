use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{path}"))?)
}

#[test]
fn exporter_module_contributes_export_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let exporter_uri = file_url("/lib/My/Exporter.pm")?;

    let exporter = r#"
package My::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(greet wave);

sub greet { return 'hello'; }
sub wave { return 'o/'; }
sub hidden { return 'x'; }
"#;

    index.index_file(exporter_uri, exporter.to_string())?;

    assert!(index.find_exported_symbol_definition("My::Exporter", "greet").is_some());
    assert!(index.find_exported_symbol_definition("My::Exporter", "wave").is_some());
    assert!(index.find_exported_symbol_definition("My::Exporter", "hidden").is_none());

    Ok(())
}

#[test]
fn workspace_lookup_finds_exported_symbol_by_module_and_name()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let exporter_uri = file_url("/lib/My/Exporter.pm")?;
    let consumer_uri = file_url("/app/main.pl")?;

    let exporter = r#"
package My::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(greet);
sub greet { return 'hello'; }
"#;

    let consumer = r#"
use My::Exporter qw(greet);
greet();
"#;

    index.index_file(exporter_uri, exporter.to_string())?;
    index.index_file(consumer_uri, consumer.to_string())?;

    let definition = index
        .find_exported_symbol_definition("My::Exporter", "greet")
        .ok_or("expected exported definition")?;

    assert!(definition.uri.contains("My/Exporter.pm"));

    Ok(())
}

#[test]
fn missing_export_symbol_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let exporter_uri = file_url("/lib/My/Exporter.pm")?;

    let exporter = r#"
package My::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(greet);
sub greet { return 'hello'; }
"#;

    index.index_file(exporter_uri, exporter.to_string())?;

    assert!(index.find_exported_symbol_definition("My::Exporter", "unknown").is_none());
    assert!(index.find_exported_symbol_definition("My::Missing", "greet").is_none());

    Ok(())
}
