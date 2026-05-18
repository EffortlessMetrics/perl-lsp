use perl_semantic_facts::{OccurrenceKind, PlannedEditCategory, Provenance};
use perl_workspace::semantic::queries::SemanticQueries;
use perl_workspace::workspace::workspace_index::{FileFactShard, WorkspaceIndex};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Default)]
struct BaselineTotals {
    files: usize,
    entities: usize,
    occurrences: usize,
    edges: usize,
    dynamic_boundary_occurrences: usize,
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("semantic_real_workspace")
        .join("cpan_style")
}

fn perl_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pm" || ext == "pl"))
        .map(|entry| entry.into_path())
        .collect();
    files.sort();
    files
}

fn index_fixture_workspace(root: &Path) -> Result<(WorkspaceIndex, Vec<FileFactShard>)> {
    let index = WorkspaceIndex::new();
    let mut shards = Vec::new();

    for path in perl_files(root) {
        let source = std::fs::read_to_string(&path)?;
        let uri = url::Url::from_file_path(&path)
            .map_err(|()| format!("fixture path cannot become file URI: {}", path.display()))?;

        index.index_file(uri.clone(), source)?;
        let shard = index.file_fact_shard(uri.as_str()).ok_or_else(|| {
            format!("missing fact shard for real-workspace fixture {}", path.display())
        })?;
        shards.push(shard);
    }

    Ok((index, shards))
}

fn measure(shards: &[FileFactShard]) -> BaselineTotals {
    BaselineTotals {
        files: shards.len(),
        entities: shards.iter().map(|shard| shard.entities.len()).sum(),
        occurrences: shards.iter().map(|shard| shard.occurrences.len()).sum(),
        edges: shards.iter().map(|shard| shard.edges.len()).sum(),
        dynamic_boundary_occurrences: shards
            .iter()
            .flat_map(|shard| shard.occurrences.iter())
            .filter(|occ| {
                occ.provenance == Provenance::DynamicBoundary
                    || matches!(
                        occ.kind,
                        OccurrenceKind::DynamicBoundary | OccurrenceKind::TypeglobReference
                    )
            })
            .count(),
    }
}

fn occurrence_kinds(shards: &[FileFactShard]) -> BTreeSet<OccurrenceKind> {
    shards.iter().flat_map(|shard| shard.occurrences.iter().map(|occ| occ.kind)).collect()
}

fn entity_names(shards: &[FileFactShard]) -> BTreeSet<String> {
    shards
        .iter()
        .flat_map(|shard| shard.entities.iter().map(|entity| entity.canonical_name.clone()))
        .collect()
}

#[test]
fn real_workspace_baseline_indexes_cpan_style_project() -> Result<()> {
    let root = fixture_root();
    let (index, shards) = index_fixture_workspace(&root)?;
    let totals = measure(&shards);

    assert_eq!(totals.files, 4, "baseline fixture should stay small and deterministic");
    assert_eq!(index.fact_shard_count(), totals.files);
    assert!(index.symbol_count() >= 8, "expected package/subroutine symbols in baseline");
    assert!(totals.entities >= 8, "expected package and subroutine facts: {totals:?}");
    assert!(totals.occurrences >= 8, "expected cross-file reference facts: {totals:?}");
    assert!(totals.edges >= 8, "expected definition/reference edge facts: {totals:?}");

    let names = entity_names(&shards);
    for expected in [
        "RealBaseline::App",
        "RealBaseline::App::run",
        "RealBaseline::Base::shared",
        "RealBaseline::Util::helper",
    ] {
        assert!(names.contains(expected), "missing expected semantic entity {expected}");
    }

    Ok(())
}

#[test]
fn real_workspace_rename_plan_includes_source_backed_sub_definition_edit() -> Result<()> {
    let root = fixture_root();
    let (index, shards) = index_fixture_workspace(&root)?;
    let entity_id = shards
        .iter()
        .flat_map(|shard| shard.entities.iter())
        .find(|entity| entity.canonical_name == "RealBaseline::Base::shared")
        .map(|entity| entity.id)
        .ok_or("missing RealBaseline::Base::shared entity")?;
    let base_uri = url::Url::from_file_path(root.join("lib").join("RealBaseline").join("Base.pm"))
        .map_err(|()| "fixture path cannot become file URI")?;

    let plan = index
        .with_semantic_queries_for_uri(base_uri.as_str(), |_file_id, queries| {
            queries.rename_plan(entity_id, "renamed_shared")
        })
        .ok_or("missing semantic queries for RealBaseline::Base")?;

    assert_eq!(plan.old_name, "RealBaseline::Base::shared");
    assert!(plan.blockers.is_empty(), "source-backed sub definition should not block: {plan:?}");
    assert!(
        plan.edits.iter().any(|edit| edit.category == PlannedEditCategory::Definition),
        "rename plan should include a source-backed definition edit: {plan:?}"
    );
    assert!(!plan.edits.is_empty(), "source-backed declaration must not produce empty plan");

    Ok(())
}

#[test]
fn real_workspace_baseline_covers_method_coderef_and_typeglob_shapes() -> Result<()> {
    let root = fixture_root();
    let (_index, shards) = index_fixture_workspace(&root)?;
    let totals = measure(&shards);
    let kinds = occurrence_kinds(&shards);

    for expected in [
        OccurrenceKind::Call,
        OccurrenceKind::MethodCall,
        OccurrenceKind::StaticMethodCall,
        OccurrenceKind::CoderefReference,
        OccurrenceKind::TypeglobReference,
    ] {
        assert!(kinds.contains(&expected), "missing expected occurrence kind {expected:?}");
    }
    assert!(
        totals.dynamic_boundary_occurrences >= 1,
        "typeglob alias should register a dynamic-boundary occurrence: {totals:?}"
    );

    Ok(())
}
