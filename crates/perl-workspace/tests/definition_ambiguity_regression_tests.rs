use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

#[test]
fn same_bare_sub_name_in_two_packages_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let alpha_uri = file_url("/workspace/lib/Alpha.pm")?;
    let beta_uri = file_url("/workspace/lib/Beta.pm")?;

    index.index_file(alpha_uri, "package Alpha;\nsub collide { 1 }\n".to_string())?;
    index.index_file(beta_uri, "package Beta;\nsub collide { 1 }\n".to_string())?;

    let first = index.find_definition("collide").ok_or("definition should resolve")?;
    let second = index.find_definition("collide").ok_or("definition should resolve")?;

    assert_eq!(first.uri, second.uri, "bare lookup should be deterministic");
    assert!(
        first.uri == "file:///workspace/lib/Alpha.pm"
            || first.uri == "file:///workspace/lib/Beta.pm",
        "current implementation returns a single winner; future candidate API should expose both"
    );
    Ok(())
}

#[test]
fn same_method_name_in_parent_and_child_package_is_qualified()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let parent_uri = file_url("/workspace/lib/Parent.pm")?;
    let child_uri = file_url("/workspace/lib/Parent/Child.pm")?;

    index.index_file(parent_uri, "package Parent;\nsub run { 1 }\n".to_string())?;
    index.index_file(child_uri, "package Parent::Child;\nsub run { 1 }\n".to_string())?;

    let parent = index.find_definition("Parent::run").ok_or("Parent::run should resolve")?;
    let child =
        index.find_definition("Parent::Child::run").ok_or("Parent::Child::run should resolve")?;

    assert_eq!(parent.uri, "file:///workspace/lib/Parent.pm");
    assert_eq!(child.uri, "file:///workspace/lib/Parent/Child.pm");
    Ok(())
}

#[test]
fn qualified_name_resolves_to_matching_package_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let foo_uri = file_url("/workspace/lib/Foo.pm")?;
    let bar_uri = file_url("/workspace/lib/Bar.pm")?;

    index.index_file(foo_uri, "package Foo;\nsub bar { 1 }\n".to_string())?;
    index.index_file(bar_uri, "package Bar;\nsub bar { 1 }\n".to_string())?;

    let resolved = index.find_definition("Foo::bar").ok_or("Foo::bar should resolve")?;
    assert_eq!(resolved.uri, "file:///workspace/lib/Foo.pm");
    Ok(())
}

#[test]
fn bare_lookup_prefers_same_package_when_unambiguous() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let foo_uri = file_url("/workspace/lib/Foo.pm")?;

    index.index_file(foo_uri, "package Foo;\nsub bar { 1 }\n".to_string())?;

    let qualified = index.find_definition("Foo::bar").ok_or("Foo::bar should resolve")?;
    let bare = index.find_definition("bar").ok_or("bar should resolve")?;

    assert_eq!(bare.uri, qualified.uri);
    Ok(())
}

#[test]
fn imported_bar_is_distinguishable_from_local_bar_in_symbol_surface()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let exporter_uri = file_url("/workspace/lib/ExporterPkg.pm")?;
    let consumer_uri = file_url("/workspace/lib/Consumer.pm")?;

    index.index_file(exporter_uri, "package ExporterPkg;\nsub bar { 1 }\n".to_string())?;
    index.index_file(
        consumer_uri,
        "package Consumer;\nuse ExporterPkg qw(bar);\nsub bar { 2 }\nsub call { bar(); }\n"
            .to_string(),
    )?;

    let refs =
        index.query_symbol_references("Consumer::bar").ok_or("Consumer::bar should resolve")?;

    assert_eq!(refs.symbol.qualified_name.as_deref(), Some("Consumer::bar"));
    assert_eq!(refs.symbol.stable_key, "Consumer::bar");
    Ok(())
}

#[test]
fn duplicate_qualified_name_across_files_remains_deterministic()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let first_uri = file_url("/workspace/lib/DupeA.pm")?;
    let second_uri = file_url("/workspace/lib/DupeB.pm")?;

    index.index_file(first_uri, "package Dupe;\nsub same { 1 }\n".to_string())?;
    index.index_file(second_uri, "package Dupe;\nsub same { 2 }\n".to_string())?;

    let first = index.find_definition("Dupe::same").ok_or("definition should resolve")?;
    let second = index.find_definition("Dupe::same").ok_or("definition should resolve")?;

    assert_eq!(first.uri, second.uri, "lookup must be deterministic");
    assert!(
        first.uri == "file:///workspace/lib/DupeA.pm"
            || first.uri == "file:///workspace/lib/DupeB.pm",
        "current implementation returns one deterministic winner; future candidate API should expose both"
    );
    Ok(())
}

#[test]
fn removing_one_duplicate_definition_removes_that_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let first_uri = file_url("/workspace/lib/DupeA.pm")?;
    let second_uri = file_url("/workspace/lib/DupeB.pm")?;

    index.index_file(first_uri, "package Dupe;\nsub same { 1 }\n".to_string())?;
    index.index_file(second_uri, "package Dupe;\nsub same { 2 }\n".to_string())?;

    index.remove_file("file:///workspace/lib/DupeA.pm");

    let resolved = index.find_definition("Dupe::same").ok_or("definition should resolve")?;
    assert_eq!(resolved.uri, "file:///workspace/lib/DupeB.pm");
    Ok(())
}

#[test]
fn reindexing_one_file_does_not_leave_stale_duplicate_candidates()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let first_uri = file_url("/workspace/lib/DupeA.pm")?;
    let second_uri = file_url("/workspace/lib/DupeB.pm")?;

    index.index_file(first_uri.clone(), "package Dupe;\nsub same { 1 }\n".to_string())?;
    index.index_file(second_uri, "package Dupe;\nsub same { 2 }\n".to_string())?;

    index.index_file(first_uri, "package Dupe;\nsub renamed { 1 }\n".to_string())?;

    assert!(index.find_definition("Dupe::renamed").is_some(), "new symbol should exist");
    let resolved =
        index.find_definition("Dupe::same").ok_or("remaining duplicate should resolve")?;
    assert_eq!(resolved.uri, "file:///workspace/lib/DupeB.pm");
    Ok(())
}
