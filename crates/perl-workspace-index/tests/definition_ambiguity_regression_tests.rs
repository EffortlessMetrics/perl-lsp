use std::error::Error;

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn index_source(index: &WorkspaceIndex, uri: &str, src: &str) -> Result<(), Box<dyn Error>> {
    index.index_file(Url::parse(uri)?, src.to_string())?;
    Ok(())
}

#[test]
fn bare_sub_name_in_two_packages_is_deterministic() -> Result<(), Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index_source(&index, "file:///workspace/lib/Alpha.pm", "package Alpha; sub ping { 1 }")?;
    index_source(&index, "file:///workspace/lib/Beta.pm", "package Beta; sub ping { 1 }")?;

    let first = index.find_definition("ping").ok_or("missing ping")?;
    let second = index.find_definition("ping").ok_or("missing ping")?;

    assert!(matches!(first.uri.as_str(), "file:///workspace/lib/Alpha.pm" | "file:///workspace/lib/Beta.pm"));
    assert_eq!(second, first);
    Ok(())
}

#[test]
fn bare_method_prefers_local_package_definition() -> Result<(), Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index_source(&index, "file:///workspace/lib/Parent.pm", "package Parent; sub work { 1 }")?;
    index_source(&index, "file:///workspace/lib/Foo.pm", "package Foo; our @ISA=('Parent'); sub work { 2 }")?;

    let local = index.find_definition("Foo::work").ok_or("missing Foo::work")?;
    let bare = index.find_definition("work").ok_or("missing work")?;

    assert_eq!(local.uri, "file:///workspace/lib/Foo.pm");
    assert_eq!(bare.uri, "file:///workspace/lib/Foo.pm");
    Ok(())
}

#[test]
fn qualified_lookup_resolves_exact_package_member() -> Result<(), Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index_source(&index, "file:///workspace/lib/Foo.pm", "package Foo; sub bar { 1 }")?;
    index_source(&index, "file:///workspace/lib/Bar.pm", "package Bar; sub bar { 1 }")?;

    let definition = index.find_definition("Foo::bar").ok_or("missing Foo::bar")?;
    assert_eq!(definition.uri, "file:///workspace/lib/Foo.pm");
    Ok(())
}

#[test]
fn imported_and_local_bare_name_currently_collide_but_are_stable() -> Result<(), Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index_source(
        &index,
        "file:///workspace/lib/Util.pm",
        "package Util; use Exporter 'import'; our @EXPORT = qw(bar); sub bar { 1 }",
    )?;
    index_source(
        &index,
        "file:///workspace/lib/Foo.pm",
        "package Foo; use Util qw(bar); sub bar { 2 }",
    )?;

    let bare = index.find_definition("bar").ok_or("missing bar")?;
    let local = index.find_definition("Foo::bar").ok_or("missing Foo::bar")?;
    let imported = index.find_definition("Util::bar").ok_or("missing Util::bar")?;

    assert_eq!(local.uri, "file:///workspace/lib/Foo.pm");
    assert_eq!(imported.uri, "file:///workspace/lib/Util.pm");
    assert_eq!(bare.uri, "file:///workspace/lib/Foo.pm");
    Ok(())
}

#[test]
fn duplicate_qualified_names_are_deterministic_and_track_removal() -> Result<(), Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index_source(&index, "file:///workspace/lib/A1.pm", "package A; sub dup { 1 }")?;
    index_source(&index, "file:///workspace/lib/A2.pm", "package A; sub dup { 2 }")?;

    let first = index.find_definition("A::dup").ok_or("missing A::dup")?;
    assert!(matches!(first.uri.as_str(), "file:///workspace/lib/A1.pm" | "file:///workspace/lib/A2.pm"));

    index.remove_file("file:///workspace/lib/A1.pm");
    let after_remove = index.find_definition("A::dup").ok_or("missing A::dup after remove")?;
    assert_eq!(after_remove.uri, "file:///workspace/lib/A2.pm");
    Ok(())
}

#[test]
fn reindexing_file_does_not_leave_stale_definition_candidates() -> Result<(), Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index_source(&index, "file:///workspace/lib/Foo.pm", "package Foo; sub old_name { 1 }")?;

    assert!(index.find_definition("old_name").is_some());

    index_source(&index, "file:///workspace/lib/Foo.pm", "package Foo; sub new_name { 1 }")?;

    assert!(index.find_definition("old_name").is_none());
    let new_def = index.find_definition("new_name").ok_or("missing new_name")?;
    assert_eq!(new_def.uri, "file:///workspace/lib/Foo.pm");
    Ok(())
}
