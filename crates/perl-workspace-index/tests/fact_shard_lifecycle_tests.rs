use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

#[test]
fn index_file_populates_fact_shard() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = Url::parse("file:///workspace/test.pl")?;

    index.index_file(uri.clone(), "package A; sub foo { 1 }".to_string())?;

    let shard = index.file_fact_shard(uri.as_str()).ok_or("missing shard")?;
    assert_eq!(shard.source_uri, uri.as_str());
    assert!(!shard.anchors.is_empty());
    assert!(!shard.entities.is_empty());
    assert!(shard.occurrences.is_empty());
    assert!(shard.edges.is_empty());
    assert_eq!(index.fact_shard_count(), 1);
    Ok(())
}

#[test]
fn reindex_replaces_stale_fact_shard() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = Url::parse("file:///workspace/reindex.pl")?;

    index.index_file(uri.clone(), "package A; sub foo { 1 }".to_string())?;
    let first = index.file_fact_shard(uri.as_str()).ok_or("missing first")?;

    index.index_file(uri.clone(), "package A; sub bar { 2 }".to_string())?;
    let second = index.file_fact_shard(uri.as_str()).ok_or("missing second")?;

    assert_ne!(first.content_hash, second.content_hash);
    assert_ne!(first.entities, second.entities);
    assert_eq!(index.fact_shard_count(), 1);
    Ok(())
}

#[test]
fn remove_file_removes_fact_shard() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = Url::parse("file:///workspace/remove.pl")?;

    index.index_file(uri.clone(), "package A; sub foo { 1 }".to_string())?;
    assert!(index.file_fact_shard(uri.as_str()).is_some());

    index.remove_file(uri.as_str());
    assert!(index.file_fact_shard(uri.as_str()).is_none());
    assert_eq!(index.fact_shard_count(), 0);
    Ok(())
}

#[test]
fn clear_removes_fact_shards() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri1 = Url::parse("file:///workspace/a.pl")?;
    let uri2 = Url::parse("file:///workspace/b.pl")?;

    index.index_file(uri1.clone(), "sub a { 1 }".to_string())?;
    index.index_file(uri2.clone(), "sub b { 2 }".to_string())?;
    assert_eq!(index.fact_shard_count(), 2);

    index.clear();
    assert_eq!(index.fact_shard_count(), 0);
    assert!(index.file_fact_shard(uri1.as_str()).is_none());
    assert!(index.file_fact_shard(uri2.as_str()).is_none());
    Ok(())
}
