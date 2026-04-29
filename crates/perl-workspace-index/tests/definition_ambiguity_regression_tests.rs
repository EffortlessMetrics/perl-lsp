use perl_tdd_support::must_some;
use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{path}"))?)
}

#[test]
fn ambiguity_same_bare_sub_name_in_two_packages_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let alpha = file_url("/lib/Alpha.pm")?;
    let beta = file_url("/lib/Beta.pm")?;

    index.index_file(alpha.clone(), "package Alpha; sub collide { 1 }".to_string())?;
    index.index_file(beta.clone(), "package Beta; sub collide { 1 }".to_string())?;

    let first = must_some(index.find_definition("collide"));
    let second = must_some(index.find_definition("collide"));
    assert_eq!(first.uri, second.uri, "bare lookup should be deterministic");

    // Current API returns one winner; future candidate API should expose both Alpha::collide and Beta::collide.
    Ok(())
}

#[test]
fn ambiguity_same_method_name_in_parent_and_child_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(
        file_url("/lib/Parent.pm")?,
        "package Parent; sub run { 1 }".to_string(),
    )?;
    index.index_file(
        file_url("/lib/Child.pm")?,
        "package Child; use parent 'Parent'; sub run { 1 }".to_string(),
    )?;

    let first = must_some(index.find_definition("run"));
    let second = must_some(index.find_definition("run"));
    assert_eq!(first.uri, second.uri, "bare method lookup should be deterministic");
    Ok(())
}

#[test]
fn qualified_name_resolves_to_matching_package_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/lib/Foo.pm")?, "package Foo; sub bar { 1 }".to_string())?;
    index.index_file(file_url("/lib/Other.pm")?, "package Other; sub bar { 1 }".to_string())?;

    let def = must_some(index.find_definition("Foo::bar"));
    assert!(def.uri.ends_with("/lib/Foo.pm"));
    Ok(())
}

#[test]
fn bare_name_in_package_local_preference_is_not_guaranteed_yet() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/lib/Foo.pm")?, "package Foo; sub bar { 1 }".to_string())?;
    index.index_file(file_url("/lib/Else.pm")?, "package Else; sub bar { 1 }".to_string())?;

    let first = must_some(index.find_definition("bar"));
    let second = must_some(index.find_definition("bar"));
    assert_eq!(first.uri, second.uri, "current single-winner behavior should remain deterministic");
    // Desired future behavior: in package Foo context, bare `bar` should prefer Foo::bar over imported/other candidates.
    Ok(())
}

#[test]
fn imported_symbol_is_distinguishable_from_local_symbol_in_references()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let consumer_uri = file_url("/lib/Consumer.pm")?;

    index.index_file(
        file_url("/lib/Exporter.pm")?,
        "package Exporter; sub bar { 1 }".to_string(),
    )?;
    index.index_file(
        consumer_uri,
        "package Consumer; use Exporter qw(bar); sub bar { 2 } bar();".to_string(),
    )?;

    let refs = index.find_references("bar");
    assert!(refs.iter().any(|loc| loc.uri.ends_with("/lib/Consumer.pm")));
    assert!(refs.iter().any(|loc| loc.uri.ends_with("/lib/Exporter.pm")));
    Ok(())
}

#[test]
fn duplicate_qualified_definition_across_files_remains_deterministic()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/dup_one/Foo.pm")?, "package Foo; sub bar { 1 }".to_string())?;
    index.index_file(file_url("/dup_two/Foo.pm")?, "package Foo; sub bar { 2 }".to_string())?;

    let first = must_some(index.find_definition("Foo::bar"));
    let second = must_some(index.find_definition("Foo::bar"));
    assert_eq!(first.uri, second.uri, "winner must be deterministic");
    Ok(())
}

#[test]
fn removing_one_duplicate_file_removes_its_definition_candidate() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let one = file_url("/dup_one/Foo.pm")?;
    let two = file_url("/dup_two/Foo.pm")?;

    index.index_file(one.clone(), "package Foo; sub bar { 1 }".to_string())?;
    index.index_file(two.clone(), "package Foo; sub bar { 2 }".to_string())?;

    let before = must_some(index.find_definition("Foo::bar"));
    index.remove_file(&before.uri);
    let after = must_some(index.find_definition("Foo::bar"));

    assert_ne!(before.uri, after.uri, "removing winner should promote remaining definition");
    Ok(())
}

#[test]
fn reindexing_file_does_not_leave_stale_definition_candidate() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Reindex.pm")?;

    index.index_file(uri.clone(), "package Reindex; sub old_name { 1 }".to_string())?;
    assert!(index.find_definition("Reindex::old_name").is_some());

    index.index_file(uri, "package Reindex; sub new_name { 1 }".to_string())?;

    assert!(index.find_definition("Reindex::new_name").is_some());
    assert!(index.find_definition("Reindex::old_name").is_none());
    Ok(())
}
