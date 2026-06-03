// Shared fixture helpers are imported per test binary, so some helpers are
// intentionally unused in each binary under `-D warnings`.
#![allow(dead_code)]
//! Shared test harness for multi-file workspace fixture tests.
//!
//! Each file directly under `tests/` compiles as its own binary. This module is
//! consumed via `mod common;` in those binaries. The `dead_code` allow is required
//! because helpers unused by a given binary would otherwise fail `-D warnings`.

use perl_semantic_facts::{EdgeKind, EntityKind, OccurrenceKind};
use perl_workspace::workspace::workspace_index::{FileFactShard, WorkspaceIndex};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Walk a fixture root for `.pm` and `.pl` files, index them all, and return
/// the `WorkspaceIndex` plus one `FileFactShard` per file (sorted by path).
///
/// Each path is canonicalized before being turned into a `file://` URI so that
/// `Url::from_file_path` does not fail silently on relative components.
pub fn load_fixture_workspace(root: &Path) -> Result<(WorkspaceIndex, Vec<FileFactShard>)> {
    let mut paths: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pm" || ext == "pl"))
        .map(|entry| entry.into_path())
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(format!("fixture root {} contains no .pm or .pl files", root.display()).into());
    }

    let index = WorkspaceIndex::new();
    let mut shards = Vec::new();

    for path in &paths {
        let canonical = path.canonicalize()?;
        let uri = url::Url::from_file_path(&canonical)
            .map_err(|()| format!("path cannot become file URI: {}", canonical.display()))?;
        let source = std::fs::read_to_string(path)?;
        index.index_file(uri.clone(), source)?;
        let shard = index
            .file_fact_shard(uri.as_str())
            .ok_or_else(|| format!("missing fact shard for {}", canonical.display()))?;
        shards.push(shard);
    }

    Ok((index, shards))
}

/// Return a sorted set of all entity canonical names across all shards.
pub fn entity_names(shards: &[FileFactShard]) -> BTreeSet<String> {
    shards.iter().flat_map(|s| s.entities.iter().map(|e| e.canonical_name.clone())).collect()
}

/// Return a sorted set of all entity kinds across all shards.
pub fn entity_kinds(shards: &[FileFactShard]) -> BTreeSet<EntityKind> {
    shards.iter().flat_map(|s| s.entities.iter().map(|e| e.kind)).collect()
}

/// Return a map of `EntityKind -> count` across all shards.
pub fn entity_count_by_kind(shards: &[FileFactShard]) -> BTreeMap<EntityKind, usize> {
    let mut map: BTreeMap<EntityKind, usize> = BTreeMap::new();
    for shard in shards {
        for entity in &shard.entities {
            *map.entry(entity.kind).or_default() += 1;
        }
    }
    map
}

/// Return all entity canonical names that belong to the given package prefix.
///
/// An entity is considered to be in `package_name` when its canonical name
/// starts with `"<package_name>::"` or equals `package_name` exactly.
pub fn entities_in_package(shards: &[FileFactShard], package_name: &str) -> Vec<String> {
    let prefix = format!("{package_name}::");
    shards
        .iter()
        .flat_map(|s| s.entities.iter())
        .filter(|e| e.canonical_name == package_name || e.canonical_name.starts_with(&prefix))
        .map(|e| e.canonical_name.clone())
        .collect()
}

/// Return a sorted set of all occurrence kinds across all shards.
pub fn occurrence_kinds(shards: &[FileFactShard]) -> BTreeSet<OccurrenceKind> {
    shards.iter().flat_map(|s| s.occurrences.iter().map(|o| o.kind)).collect()
}

/// Return a sorted set of all edge kinds across all shards.
pub fn edge_kinds(shards: &[FileFactShard]) -> BTreeSet<EdgeKind> {
    shards.iter().flat_map(|s| s.edges.iter().map(|e| e.kind)).collect()
}

/// Return a map of `EdgeKind -> count` across all shards.
pub fn edge_count_by_kind(shards: &[FileFactShard]) -> BTreeMap<EdgeKind, usize> {
    let mut map: BTreeMap<EdgeKind, usize> = BTreeMap::new();
    for shard in shards {
        for edge in &shard.edges {
            *map.entry(edge.kind).or_default() += 1;
        }
    }
    map
}
