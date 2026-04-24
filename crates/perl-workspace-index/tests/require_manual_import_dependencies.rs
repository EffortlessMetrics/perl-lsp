//! Coverage for static `require Module; Module->import(...)` dependency tracking.
//!
//! Issue #3476 (literal-only slice): keep dependency discovery conservative and
//! avoid inferring dynamic/manual import receivers.

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

#[test]
fn static_require_import_string_tracks_module_dependency() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();
    let module_uri = file_url("/lib/Foo.pm")?;
    let consumer_uri = file_url("/app/main.pl")?;

    index.index_file(module_uri, "package Foo;\nsub bar { 1 }\n1;\n".to_string())?;
    index.index_file(
        consumer_uri.clone(),
        "require Foo;\nFoo->import('bar');\nbar();\n".to_string(),
    )?;

    let deps = index.file_dependencies(consumer_uri.as_str());
    assert!(deps.contains("Foo"), "expected file dependency set to contain Foo, got {deps:?}");

    let dependents = index.find_dependents("Foo");
    assert!(
        dependents.iter().any(|uri| uri == consumer_uri.as_str()),
        "expected consumer file in dependents(Foo), got {dependents:?}"
    );

    Ok(())
}

#[test]
fn static_require_import_qw_tracks_module_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let module_uri = file_url("/lib/Foo.pm")?;
    let consumer_uri = file_url("/app/main_qw.pl")?;

    index.index_file(module_uri, "package Foo;\nsub bar { 1 }\nsub baz { 1 }\n1;\n".to_string())?;
    index.index_file(
        consumer_uri.clone(),
        "require Foo;\nFoo->import(qw(bar baz));\nbar();\nbaz();\n".to_string(),
    )?;

    let deps = index.file_dependencies(consumer_uri.as_str());
    assert!(deps.contains("Foo"), "expected file dependency set to contain Foo, got {deps:?}");

    Ok(())
}

#[test]
fn dynamic_manual_import_receiver_does_not_create_dependency()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let consumer_uri = file_url("/app/dynamic.pl")?;

    index.index_file(
        consumer_uri.clone(),
        "my $m = 'Foo';\n$m->import('bar');\nbar();\n".to_string(),
    )?;

    let deps = index.file_dependencies(consumer_uri.as_str());
    assert!(
        !deps.contains("Foo"),
        "dynamic receiver should stay unresolved; unexpected dependency set: {deps:?}"
    );

    Ok(())
}
