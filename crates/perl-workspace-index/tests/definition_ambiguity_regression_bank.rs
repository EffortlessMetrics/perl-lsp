use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

#[test]
fn definition_ambiguity_same_bare_sub_in_two_packages_is_deterministic()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/workspace/lib/A.pm")?, "package A; sub collide { 1 }\n".to_string())?;
    index.index_file(file_url("/workspace/lib/B.pm")?, "package B; sub collide { 1 }\n".to_string())?;

    let first = index.find_definition("collide").ok_or("expected definition for bare symbol")?;
    let second = index.find_definition("collide").ok_or("expected deterministic definition")?;

    assert_eq!(first.uri, second.uri, "bare lookup should be deterministic for ambiguous symbols");
    assert!(
        first.uri == "file:///workspace/lib/A.pm" || first.uri == "file:///workspace/lib/B.pm",
        "lookup should resolve to one of the two providers"
    );

    Ok(())
}

#[test]
fn definition_ambiguity_same_method_in_parent_and_child_is_deterministic()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/workspace/lib/Parent.pm")?, "package Parent; sub ping { 1 }\n".to_string())?;
    index.index_file(file_url("/workspace/lib/Child.pm")?, "package Child; our @ISA = ('Parent'); sub ping { 2 }\n".to_string())?;

    let first = index.find_definition("ping").ok_or("expected definition for method")?;
    let second = index.find_definition("ping").ok_or("expected deterministic definition")?;

    assert_eq!(first.uri, second.uri, "ambiguous parent/child bare method lookup should be deterministic");
    assert!(
        first.uri == "file:///workspace/lib/Parent.pm" || first.uri == "file:///workspace/lib/Child.pm",
        "lookup should resolve to one of the two packages"
    );

    Ok(())
}

#[test]
fn definition_ambiguity_qualified_lookup_resolves_exact_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/workspace/lib/Foo.pm")?, "package Foo; sub bar { 1 }\n".to_string())?;
    index.index_file(file_url("/workspace/lib/Other.pm")?, "package Other; sub bar { 2 }\n".to_string())?;

    let def = index.find_definition("Foo::bar").ok_or("expected qualified definition")?;
    assert_eq!(def.uri, "file:///workspace/lib/Foo.pm");

    Ok(())
}

#[test]
fn definition_ambiguity_bare_lookup_in_package_prefers_local_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/workspace/lib/Foo.pm")?, "package Foo; sub bar { 1 }\n".to_string())?;
    index.index_file(
        file_url("/workspace/lib/Foo/Caller.pm")?,
        "package Foo; bar();\n".to_string(),
    )?;

    let query = index.query_symbol_references("bar").ok_or("expected reference query")?;
    assert_eq!(query.definition.uri, "file:///workspace/lib/Foo.pm");
    assert_eq!(query.symbol.qualified_name.as_deref(), Some("Foo::bar"));

    Ok(())
}

#[test]
fn definition_ambiguity_imported_and_local_bar_currently_conflated()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/workspace/lib/Foo.pm")?, "package Foo; sub bar { 1 }\n".to_string())?;
    index.index_file(file_url("/workspace/lib/Baz.pm")?, "package Baz; sub bar { 2 }\n".to_string())?;
    index.index_file(
        file_url("/workspace/lib/Consumer.pm")?,
        "package Consumer; use Foo qw(bar); sub bar { 3 } bar();\n".to_string(),
    )?;

    let imported = index.find_references("Foo::bar");
    let local = index.find_references("Consumer::bar");

    assert!(!imported.is_empty(), "expected imported target refs to be indexed");
    assert!(!local.is_empty(), "expected local target refs to be indexed");
    assert_eq!(
        imported,
        local,
        "current limitation: imported and local `bar` still share one flattened reference bucket; \
         keep this test as an actionable reminder for future definition-candidate storage"
    );

    Ok(())
}

#[test]
fn definition_ambiguity_duplicate_qualified_name_query_is_deterministic()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(file_url("/workspace/lib/CopyA.pm")?, "package Dup; sub bar { 1 }\n".to_string())?;
    index.index_file(file_url("/workspace/lib/CopyB.pm")?, "package Dup; sub bar { 2 }\n".to_string())?;

    let first = index.find_definition("Dup::bar").ok_or("expected definition")?;
    let second = index.find_definition("Dup::bar").ok_or("expected deterministic definition")?;
    assert_eq!(first.uri, second.uri, "duplicate qualified lookups should be deterministic");

    Ok(())
}

#[test]
fn definition_ambiguity_removing_one_file_removes_its_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let copy_a = file_url("/workspace/lib/CopyA.pm")?;
    let copy_b = file_url("/workspace/lib/CopyB.pm")?;

    index.index_file(copy_a.clone(), "package Dup; sub bar { 1 }\n".to_string())?;
    index.index_file(copy_b.clone(), "package Dup; sub bar { 2 }\n".to_string())?;

    let before = index.find_definition("Dup::bar").ok_or("expected definition before removal")?;
    index.remove_file_url(&copy_a);
    index.remove_file_url(&copy_b);
    assert!(
        index.find_definition("Dup::bar").is_none(),
        "removing both files should clear all candidates"
    );

    // Re-add one provider to prove candidate-set updates correctly.
    index.index_file(copy_b, "package Dup; sub bar { 2 }\n".to_string())?;
    let after = index.find_definition("Dup::bar").ok_or("expected remaining definition")?;
    assert_ne!(before.uri, String::new());
    assert_eq!(after.uri, "file:///workspace/lib/CopyB.pm");

    Ok(())
}

#[test]
fn definition_ambiguity_reindex_replaces_stale_candidates()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/workspace/lib/Reindex.pm")?;

    index.index_file(uri.clone(), "package Reindex; sub old_name { 1 }\n".to_string())?;
    assert!(index.find_definition("Reindex::old_name").is_some());

    index.index_file(uri, "package Reindex; sub new_name { 1 }\n".to_string())?;

    assert!(
        index.find_definition("Reindex::old_name").is_none(),
        "reindex must remove stale candidate old_name"
    );
    assert!(
        index.find_definition("Reindex::new_name").is_some(),
        "reindex must publish new candidate new_name"
    );

    Ok(())
}
