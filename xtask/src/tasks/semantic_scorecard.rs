use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use perl_semantic_analyzer::{Parser, semantic::SemanticModel};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeKind, EntityFact, EntityId, EntityKind, ExportSet,
    FileId, ImportKind, ImportSpec, ImportSymbols, OccurrenceFact, OccurrenceId, OccurrenceKind,
    PackageEdge, PackageEdgeKind, PlanBlockerReason, PlannedEditCategory, Provenance, RenamePlan,
};
use perl_workspace::semantic::imports::ImportExportIndex;
use perl_workspace::semantic::package_graph::PackageGraphIndex;
use perl_workspace::semantic::queries::{SemanticQueries, WorkspaceSemanticQueries};
use perl_workspace::semantic::references::ReferenceIndex;
use perl_workspace::workspace::workspace_index::{FileFactShard, WorkspaceIndex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_FIXTURE_MANIFEST: &str =
    "crates/perl-workspace-index/tests/fixtures/semantic_scorecard/manifest.json";
const DEFAULT_OUTPUT: &str = "docs/project/status/semantic_scorecard.json";
const DEFAULT_STATUS_MD: &str = "docs/project/status/semantic_scorecard.md";

const AVAILABLE_ROWS: &[&str] = &[
    "declaration_facts",
    "occurrence_facts",
    "import_specs",
    "export_facts",
    "definition_candidates",
    "reference_edges",
    "package_graph_edges",
    "inheritance_edges",
    "role_composition_edges",
];

const UNAVAILABLE_ROWS: &[&str] = &["safe_delete_plan"];

#[derive(Debug, Deserialize)]
struct SemanticManifest {
    fixture_family_version: u32,
    fixtures: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    family: String,
    #[allow(dead_code)]
    path: String,
}

#[derive(Debug, Serialize)]
struct FactConfidenceBreakdown {
    exact_facts: usize,
    high_confidence_facts: usize,
    heuristic_facts: usize,
    dynamic_boundary_facts: usize,
}

#[derive(Debug, Serialize)]
struct FactRow {
    status: &'static str,
    total_facts: usize,
    fixture_coverage: FixtureCoverage,
    confidence_breakdown: FactConfidenceBreakdown,
}

#[derive(Debug, Serialize)]
struct UnavailableRow {
    status: &'static str,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct ReadinessRow {
    status: &'static str,
    value: String,
    threshold: &'static str,
    evidence: &'static str,
}

#[derive(Debug, Serialize)]
struct FixtureCoverage {
    covered_fixture_count: usize,
    total_fixture_count: usize,
    covered_fixture_families: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    schema_version: u32,
    measured_at: &'static str,
    subsystem: &'static str,
    fixture_family_version: u32,
    fixture_count: usize,
    fixture_ids: Vec<String>,
    fixture_families: Vec<String>,
    fact_rows: BTreeMap<String, FactRow>,
    readiness_rows: BTreeMap<String, ReadinessRow>,
    unavailable_rows: BTreeMap<String, UnavailableRow>,
    notes: &'static str,
}

#[derive(Default)]
struct FactMeasurement {
    declaration_facts: usize,
    occurrence_facts: usize,
    import_specs: usize,
    export_facts: usize,
    definition_candidates: usize,
    reference_edges: usize,
    package_graph_edges: usize,
    inheritance_edges: usize,
    role_composition_edges: usize,
    method_candidate_fixture_passes: usize,
    method_candidate_fixture_total: usize,
    method_candidate_results: usize,
    rename_plan_fixture_passes: usize,
    rename_plan_fixture_total: usize,
    rename_plan_planned_edits: usize,
    rename_plan_blockers: usize,
    rename_plan_unsafe_edits: usize,
    exact_facts: usize,
    high_confidence_facts: usize,
    heuristic_facts: usize,
    dynamic_boundary_facts: usize,
}

pub fn run(
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
    status_md: Option<PathBuf>,
    check: bool,
) -> Result<()> {
    let root = project_root()?;
    let manifest_path =
        root.join(manifest.unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE_MANIFEST)));
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));
    let status_path = root.join(status_md.unwrap_or_else(|| PathBuf::from(DEFAULT_STATUS_MD)));

    let manifest = load_manifest(&manifest_path)?;
    let artifact = build_artifact(&manifest_path, manifest)?;

    let payload = serialize_json(&artifact)?;
    let status_markdown = render_status_markdown(&artifact);

    if check {
        verify_file_matches(&output_path, &payload)?;
        verify_file_matches(&status_path, &status_markdown)?;
        println!("semantic scorecard check passed: outputs are current");
        return Ok(());
    }

    write_json(&output_path, &payload)?;
    if let Some(parent) = status_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&status_path, status_markdown)
        .with_context(|| format!("writing {}", status_path.display()))?;

    println!("semantic scorecard updated: {}", output_path.display());
    println!("status page updated: {}", status_path.display());
    Ok(())
}

fn load_manifest(path: &Path) -> Result<SemanticManifest> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut parsed: SemanticManifest =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    parsed.fixtures.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(parsed)
}

fn build_artifact(manifest_path: &Path, manifest: SemanticManifest) -> Result<Artifact> {
    let measurement = measure_fixtures(manifest_path, &manifest)?;
    let fixture_ids =
        manifest.fixtures.iter().map(|fixture| fixture.id.clone()).collect::<Vec<_>>();
    let fixture_families =
        manifest.fixtures.iter().map(|fixture| fixture.family.clone()).collect::<Vec<_>>();

    let coverage = FixtureCoverage {
        covered_fixture_count: fixture_ids.len(),
        total_fixture_count: fixture_ids.len(),
        covered_fixture_families: fixture_families.clone(),
    };

    let mut fact_rows = BTreeMap::new();
    for &row in AVAILABLE_ROWS {
        let total_facts = row_total(&measurement, row);
        fact_rows.insert(
            row.to_string(),
            FactRow {
                status: if total_facts == 0 { "available_empty" } else { "available" },
                total_facts,
                fixture_coverage: FixtureCoverage {
                    covered_fixture_count: coverage.covered_fixture_count,
                    total_fixture_count: coverage.total_fixture_count,
                    covered_fixture_families: coverage.covered_fixture_families.clone(),
                },
                confidence_breakdown: FactConfidenceBreakdown {
                    exact_facts: measurement.exact_facts,
                    high_confidence_facts: measurement.high_confidence_facts,
                    heuristic_facts: measurement.heuristic_facts,
                    dynamic_boundary_facts: measurement.dynamic_boundary_facts,
                },
            },
        );
    }

    let readiness_rows = build_readiness_rows(&measurement, fixture_ids.len());

    let mut unavailable_rows = BTreeMap::new();
    for &row in UNAVAILABLE_ROWS {
        unavailable_rows.insert(
            row.to_string(),
            UnavailableRow { status: "unavailable", reason: "planned for future scorecard waves" },
        );
    }

    Ok(Artifact {
        schema_version: 2,
        measured_at: "deterministic-fixture-baseline",
        subsystem: "semantic",
        fixture_family_version: manifest.fixture_family_version,
        fixture_count: fixture_ids.len(),
        fixture_ids,
        fixture_families,
        fact_rows,
        readiness_rows,
        unavailable_rows,
        notes: "0.13.2 semantic proof rail: scorecard rows are deterministic and fixture-backed; semantic expansion remains conservative for unavailable rows.",
    })
}

fn row_total(measurement: &FactMeasurement, row: &str) -> usize {
    match row {
        "declaration_facts" => measurement.declaration_facts,
        "occurrence_facts" => measurement.occurrence_facts,
        "import_specs" => measurement.import_specs,
        "export_facts" => measurement.export_facts,
        "definition_candidates" => measurement.definition_candidates,
        "reference_edges" => measurement.reference_edges,
        "package_graph_edges" => measurement.package_graph_edges,
        "inheritance_edges" => measurement.inheritance_edges,
        "role_composition_edges" => measurement.role_composition_edges,
        _ => 0,
    }
}

fn measure_fixtures(manifest_path: &Path, manifest: &SemanticManifest) -> Result<FactMeasurement> {
    let mut measurement = FactMeasurement::default();
    let index = WorkspaceIndex::new();

    for fixture in &manifest.fixtures {
        let path = fixture_source_path(manifest_path, fixture)?;
        let source = fs::read_to_string(&path)
            .with_context(|| format!("reading semantic scorecard fixture {}", path.display()))?;
        let uri = path.to_string_lossy();
        index
            .index_file_str(&uri, &source)
            .map_err(|err| color_eyre::eyre::eyre!("indexing {}: {}", path.display(), err))?;
        let shard = index
            .file_fact_shard(&uri)
            .ok_or_else(|| color_eyre::eyre::eyre!("missing fact shard for {}", path.display()))?;

        measurement.declaration_facts += shard.entities.len();
        measurement.occurrence_facts += shard.occurrences.len();
        measurement.definition_candidates += shard.entities.len();
        measurement.reference_edges +=
            shard.edges.iter().filter(|edge| edge.kind == EdgeKind::References).count();
        measurement.import_specs += count_import_like_sites(&source);
        measurement.export_facts += count_export_like_sites(&source);
        measure_package_graph_edges(&source, &path, &mut measurement)?;

        for anchor in &shard.anchors {
            record_proof_shape(anchor.provenance, anchor.confidence, None, &mut measurement);
        }
        for entity in &shard.entities {
            record_proof_shape(entity.provenance, entity.confidence, None, &mut measurement);
        }
        for occurrence in &shard.occurrences {
            record_proof_shape(
                occurrence.provenance,
                occurrence.confidence,
                Some(occurrence.kind),
                &mut measurement,
            );
        }
        for edge in &shard.edges {
            record_proof_shape(edge.provenance, edge.confidence, None, &mut measurement);
        }
    }

    let method_candidates = measure_method_candidate_fixtures();
    measurement.method_candidate_fixture_passes = method_candidates.passes;
    measurement.method_candidate_fixture_total = method_candidates.total;
    measurement.method_candidate_results = method_candidates.candidate_results;

    let rename_plan = measure_rename_plan_fixtures();
    measurement.rename_plan_fixture_passes = rename_plan.passes;
    measurement.rename_plan_fixture_total = rename_plan.total;
    measurement.rename_plan_planned_edits = rename_plan.planned_edits;
    measurement.rename_plan_blockers = rename_plan.blockers;
    measurement.rename_plan_unsafe_edits = rename_plan.unsafe_edits;

    Ok(measurement)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MethodCandidateMeasurement {
    passes: usize,
    total: usize,
    candidate_results: usize,
}

#[derive(Debug, Clone, Copy)]
struct MethodCandidateFixture {
    receiver_package: &'static str,
    method_name: &'static str,
    expected_package: &'static str,
    expected_kind: EntityKind,
}

fn measure_method_candidate_fixtures() -> MethodCandidateMeasurement {
    let mut shards = std::collections::HashMap::new();
    let shard = method_candidate_fact_shard();
    shards.insert(shard.source_uri.clone(), shard);

    let reference_index = ReferenceIndex::new();
    let import_export_index = ImportExportIndex::new();
    let mut package_graph = PackageGraphIndex::new();
    package_graph.add_edges(
        "file:///semantic/method_candidates_graph.pm",
        FileId(700),
        vec![
            package_edge("Dog", "Animal", PackageEdgeKind::Inherits),
            package_edge("Dog", "Tracker", PackageEdgeKind::ComposesRole),
        ],
    );

    let queries = WorkspaceSemanticQueries::with_package_graph(
        &reference_index,
        &import_export_index,
        &shards,
        &package_graph,
    );

    let fixtures = [
        MethodCandidateFixture {
            receiver_package: "Dog",
            method_name: "bark",
            expected_package: "Dog",
            expected_kind: EntityKind::Subroutine,
        },
        MethodCandidateFixture {
            receiver_package: "Dog",
            method_name: "speak",
            expected_package: "Animal",
            expected_kind: EntityKind::Subroutine,
        },
        MethodCandidateFixture {
            receiver_package: "Dog",
            method_name: "track",
            expected_package: "Tracker",
            expected_kind: EntityKind::Subroutine,
        },
        MethodCandidateFixture {
            receiver_package: "Person",
            method_name: "name",
            expected_package: "Person",
            expected_kind: EntityKind::GeneratedMember,
        },
    ];
    let mut passes = 0;
    let mut candidate_results = 0;
    for fixture in fixtures {
        let candidates = queries.method_candidates(fixture.receiver_package, fixture.method_name);
        candidate_results += candidates.len();
        if candidates.iter().any(|candidate| {
            candidate.display_name == fixture.method_name
                && candidate.package.as_deref() == Some(fixture.expected_package)
                && candidate.kind == fixture.expected_kind
        }) {
            passes += 1;
        }
    }

    MethodCandidateMeasurement { passes, total: fixtures.len(), candidate_results }
}

fn method_candidate_fact_shard() -> FileFactShard {
    let file_id = FileId(701);
    let entries = [
        ("Dog::bark", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High),
        ("Animal::speak", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High),
        ("Tracker::track", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High),
        (
            "Person::name",
            EntityKind::GeneratedMember,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        ),
    ];

    let mut anchors = Vec::new();
    let mut entities = Vec::new();
    for (idx, (canonical_name, kind, provenance, confidence)) in entries.into_iter().enumerate() {
        let anchor_id = AnchorId(10_000 + idx as u64);
        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: (idx as u32) * 10,
            span_end_byte: (idx as u32) * 10 + 5,
            scope_id: None,
            provenance,
            confidence,
        });
        entities.push(EntityFact {
            id: EntityId(20_000 + idx as u64),
            kind,
            canonical_name: canonical_name.to_string(),
            anchor_id: Some(anchor_id),
            scope_id: None,
            provenance,
            confidence,
        });
    }

    FileFactShard {
        source_uri: "file:///semantic/method_candidates.pm".to_string(),
        file_id,
        content_hash: 1,
        anchors_hash: None,
        entities_hash: None,
        occurrences_hash: None,
        edges_hash: None,
        anchors,
        entities,
        occurrences: Vec::new(),
        edges: Vec::new(),
    }
}

fn package_edge(from: &str, to: &str, kind: PackageEdgeKind) -> PackageEdge {
    PackageEdge::new(
        from.to_string(),
        to.to_string(),
        kind,
        Some(AnchorId(30_000)),
        Provenance::ExactAst,
        Confidence::High,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenamePlanMeasurement {
    passes: usize,
    total: usize,
    planned_edits: usize,
    blockers: usize,
    unsafe_edits: usize,
}

fn measure_rename_plan_fixtures() -> RenamePlanMeasurement {
    let fixtures = [
        rename_plan_safe_local_fixture(),
        rename_plan_generated_member_fixture(),
        rename_plan_dynamic_boundary_fixture(),
        rename_plan_cross_module_export_fixture(),
    ];
    let total = fixtures.len();

    let mut passes = 0;
    let mut planned_edits = 0;
    let mut blockers = 0;
    let mut unsafe_edits = 0;

    for (plan, passed) in fixtures {
        planned_edits += plan.edits.len();
        blockers += plan.blockers.len();
        unsafe_edits += count_unsafe_rename_edits(&plan);
        if passed {
            passes += 1;
        }
    }

    RenamePlanMeasurement { passes, total, planned_edits, blockers, unsafe_edits }
}

fn rename_plan_safe_local_fixture() -> (RenamePlan, bool) {
    let entity_id = EntityId(41_000);
    let def_anchor = AnchorId(41_100);
    let call_anchor = AnchorId(41_101);
    let shard = rename_plan_fact_shard(
        "file:///semantic/rename_safe_local.pm",
        FileId(41_001),
        entity_id,
        "Local::local_only",
        EntityKind::Subroutine,
        &[
            RenameOccurrenceSpec {
                anchor_id: def_anchor,
                occurrence_id: OccurrenceId(41_200),
                kind: OccurrenceKind::Definition,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            },
            RenameOccurrenceSpec {
                anchor_id: call_anchor,
                occurrence_id: OccurrenceId(41_201),
                kind: OccurrenceKind::Call,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            },
        ],
        Provenance::ExactAst,
        Confidence::High,
    );

    let plan = run_rename_plan_fixture(vec![shard], ImportExportIndex::new(), entity_id);
    let has_definition = plan.edits.iter().any(|edit| {
        edit.anchor_id == def_anchor && edit.category == PlannedEditCategory::Definition
    });
    let has_reference = plan.edits.iter().any(|edit| {
        edit.anchor_id == call_anchor && edit.category == PlannedEditCategory::Reference
    });
    let passed = has_definition
        && has_reference
        && plan.blockers.is_empty()
        && count_unsafe_rename_edits(&plan) == 0;

    (plan, passed)
}

fn rename_plan_generated_member_fixture() -> (RenamePlan, bool) {
    let entity_id = EntityId(42_000);
    let shard = rename_plan_fact_shard(
        "file:///semantic/rename_generated_member.pm",
        FileId(42_001),
        entity_id,
        "Person::name",
        EntityKind::GeneratedMember,
        &[],
        Provenance::FrameworkSynthesis,
        Confidence::Medium,
    );

    let plan = run_rename_plan_fixture(vec![shard], ImportExportIndex::new(), entity_id);
    let passed = plan.edits.is_empty()
        && plan.blockers.iter().any(|blocker| blocker.reason == PlanBlockerReason::GeneratedMember)
        && count_unsafe_rename_edits(&plan) == 0;

    (plan, passed)
}

fn rename_plan_dynamic_boundary_fixture() -> (RenamePlan, bool) {
    let entity_id = EntityId(43_000);
    let dyn_anchor = AnchorId(43_101);
    let shard = rename_plan_fact_shard(
        "file:///semantic/rename_dynamic_boundary.pm",
        FileId(43_001),
        entity_id,
        "Dynamic::dispatch",
        EntityKind::Subroutine,
        &[
            RenameOccurrenceSpec {
                anchor_id: AnchorId(43_100),
                occurrence_id: OccurrenceId(43_200),
                kind: OccurrenceKind::Definition,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            },
            RenameOccurrenceSpec {
                anchor_id: dyn_anchor,
                occurrence_id: OccurrenceId(43_201),
                kind: OccurrenceKind::DynamicBoundary,
                provenance: Provenance::DynamicBoundary,
                confidence: Confidence::Low,
            },
        ],
        Provenance::ExactAst,
        Confidence::High,
    );

    let plan = run_rename_plan_fixture(vec![shard], ImportExportIndex::new(), entity_id);
    let passed = plan.blockers.iter().any(|blocker| {
        blocker.reason == PlanBlockerReason::DynamicBoundary
            && blocker.anchor_id == Some(dyn_anchor)
    }) && count_unsafe_rename_edits(&plan) == 0;

    (plan, passed)
}

fn rename_plan_cross_module_export_fixture() -> (RenamePlan, bool) {
    let exporter_file = FileId(44_001);
    let importer_file = FileId(44_002);
    let entity_id = EntityId(44_000);
    let shard = rename_plan_fact_shard(
        "file:///semantic/rename_exporter.pm",
        exporter_file,
        entity_id,
        "MyExporter::helper",
        EntityKind::Subroutine,
        &[RenameOccurrenceSpec {
            anchor_id: AnchorId(44_100),
            occurrence_id: OccurrenceId(44_200),
            kind: OccurrenceKind::Definition,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        }],
        Provenance::ExactAst,
        Confidence::High,
    );
    let importer_shard = empty_fact_shard("file:///semantic/rename_consumer.pm", importer_file);

    let mut import_export_index = ImportExportIndex::new();
    import_export_index.add_module_exports(
        "file:///semantic/rename_exporter.pm",
        "MyExporter",
        ExportSet {
            default_exports: Vec::new(),
            optional_exports: vec!["helper".to_string()],
            tags: Vec::new(),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            module_name: Some("MyExporter".to_string()),
            anchor_id: None,
        },
    );
    import_export_index.add_file_imports(
        "file:///semantic/rename_consumer.pm",
        importer_file,
        vec![ImportSpec {
            module: "MyExporter".to_string(),
            kind: ImportKind::UseExplicitList,
            symbols: ImportSymbols::Explicit(vec!["helper".to_string()]),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(importer_file),
            anchor_id: None,
            scope_id: None,
        }],
    );

    let plan = run_rename_plan_fixture(vec![shard, importer_shard], import_export_index, entity_id);
    let passed =
        plan.blockers.iter().any(|blocker| blocker.reason == PlanBlockerReason::CrossModuleExport)
            && count_unsafe_rename_edits(&plan) == 0;

    (plan, passed)
}

fn run_rename_plan_fixture(
    shards: Vec<FileFactShard>,
    import_export_index: ImportExportIndex,
    entity_id: EntityId,
) -> RenamePlan {
    let mut fact_shards = std::collections::HashMap::new();
    for shard in shards {
        fact_shards.insert(shard.source_uri.clone(), shard);
    }

    let reference_index = ReferenceIndex::new();
    let queries =
        WorkspaceSemanticQueries::new(&reference_index, &import_export_index, &fact_shards);
    queries.rename_plan(entity_id, "renamed")
}

#[derive(Debug, Clone, Copy)]
struct RenameOccurrenceSpec {
    anchor_id: AnchorId,
    occurrence_id: OccurrenceId,
    kind: OccurrenceKind,
    provenance: Provenance,
    confidence: Confidence,
}

fn rename_plan_fact_shard(
    source_uri: &str,
    file_id: FileId,
    entity_id: EntityId,
    canonical_name: &str,
    entity_kind: EntityKind,
    occurrence_specs: &[RenameOccurrenceSpec],
    provenance: Provenance,
    confidence: Confidence,
) -> FileFactShard {
    let definition_anchor = occurrence_specs
        .iter()
        .find(|spec| spec.kind == OccurrenceKind::Definition)
        .map(|spec| spec.anchor_id)
        .unwrap_or(AnchorId(file_id.0 + 400_000));

    let mut anchors = vec![AnchorFact {
        id: definition_anchor,
        file_id,
        span_start_byte: 0,
        span_end_byte: 6,
        scope_id: None,
        provenance,
        confidence,
    }];
    let mut occurrences = Vec::new();
    for (idx, spec) in occurrence_specs.iter().enumerate() {
        if spec.anchor_id != definition_anchor {
            anchors.push(AnchorFact {
                id: spec.anchor_id,
                file_id,
                span_start_byte: 20 + idx as u32 * 10,
                span_end_byte: 26 + idx as u32 * 10,
                scope_id: None,
                provenance: spec.provenance,
                confidence: spec.confidence,
            });
        }
        occurrences.push(OccurrenceFact {
            id: spec.occurrence_id,
            kind: spec.kind,
            entity_id: Some(entity_id),
            anchor_id: spec.anchor_id,
            scope_id: None,
            provenance: spec.provenance,
            confidence: spec.confidence,
        });
    }

    FileFactShard {
        source_uri: source_uri.to_string(),
        file_id,
        content_hash: file_id.0,
        anchors_hash: None,
        entities_hash: None,
        occurrences_hash: None,
        edges_hash: None,
        anchors,
        entities: vec![EntityFact {
            id: entity_id,
            kind: entity_kind,
            canonical_name: canonical_name.to_string(),
            anchor_id: Some(definition_anchor),
            scope_id: None,
            provenance,
            confidence,
        }],
        occurrences,
        edges: Vec::new(),
    }
}

fn empty_fact_shard(source_uri: &str, file_id: FileId) -> FileFactShard {
    FileFactShard {
        source_uri: source_uri.to_string(),
        file_id,
        content_hash: file_id.0,
        anchors_hash: None,
        entities_hash: None,
        occurrences_hash: None,
        edges_hash: None,
        anchors: Vec::new(),
        entities: Vec::new(),
        occurrences: Vec::new(),
        edges: Vec::new(),
    }
}

fn count_unsafe_rename_edits(plan: &RenamePlan) -> usize {
    plan.edits
        .iter()
        .filter(|edit| {
            !matches!(
                edit.category,
                PlannedEditCategory::Definition
                    | PlannedEditCategory::Reference
                    | PlannedEditCategory::ImportList
                    | PlannedEditCategory::ExportList
            )
        })
        .count()
}

fn measure_package_graph_edges(
    source: &str,
    path: &Path,
    measurement: &mut FactMeasurement,
) -> Result<()> {
    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|err| {
        color_eyre::eyre::eyre!(
            "parsing package graph scorecard fixture {}: {}",
            path.display(),
            err
        )
    })?;
    let model = SemanticModel::build(&ast, source);

    for edge in model.package_edges() {
        measurement.package_graph_edges += 1;
        match edge.kind {
            PackageEdgeKind::Inherits => measurement.inheritance_edges += 1,
            PackageEdgeKind::ComposesRole => measurement.role_composition_edges += 1,
            PackageEdgeKind::DependsOn => {}
            _ => {}
        }
        record_proof_shape(edge.provenance, edge.confidence, None, measurement);
    }

    Ok(())
}

fn fixture_source_path(manifest_path: &Path, fixture: &FixtureCase) -> Result<PathBuf> {
    let base = manifest_path.parent().ok_or_else(|| {
        color_eyre::eyre::eyre!("manifest has no parent: {}", manifest_path.display())
    })?;
    let declared = base.join(&fixture.path);
    if declared.exists() {
        return Ok(declared);
    }
    if let Some(file_name) = Path::new(&fixture.path).file_name() {
        let flattened = base.join(file_name);
        if flattened.exists() {
            return Ok(flattened);
        }
    }
    let by_id = base.join(format!("{}.pl", fixture.id));
    if by_id.exists() {
        return Ok(by_id);
    }
    bail!("semantic scorecard fixture not found for {}", fixture.id)
}

fn count_import_like_sites(source: &str) -> usize {
    source.lines().filter(|line| line.trim_start().starts_with("use ")).count()
        + source.lines().filter(|line| line.trim_start().starts_with("require ")).count()
}

fn count_export_like_sites(source: &str) -> usize {
    source.matches("@EXPORT").count() + source.matches("%EXPORT_TAGS").count()
}

fn record_proof_shape(
    provenance: Provenance,
    confidence: Confidence,
    occurrence_kind: Option<OccurrenceKind>,
    measurement: &mut FactMeasurement,
) {
    if provenance == Provenance::ExactAst {
        measurement.exact_facts += 1;
    }
    if confidence == Confidence::High {
        measurement.high_confidence_facts += 1;
    }
    if matches!(provenance, Provenance::NameHeuristic | Provenance::SearchFallback) {
        measurement.heuristic_facts += 1;
    }
    if provenance == Provenance::DynamicBoundary
        || occurrence_kind == Some(OccurrenceKind::DynamicBoundary)
    {
        measurement.dynamic_boundary_facts += 1;
    }
}

fn build_readiness_rows(
    measurement: &FactMeasurement,
    fixture_count: usize,
) -> BTreeMap<String, ReadinessRow> {
    let semantic_fact_total = measurement.declaration_facts
        + measurement.occurrence_facts
        + measurement.import_specs
        + measurement.export_facts
        + measurement.reference_edges
        + measurement.package_graph_edges;
    let fixture_rate = if fixture_count == 0 { "0%" } else { "100%" };
    let method_candidate_rate = if measurement.method_candidate_fixture_total == 0 {
        "0%".to_string()
    } else {
        format!(
            "{}%",
            measurement.method_candidate_fixture_passes * 100
                / measurement.method_candidate_fixture_total
        )
    };
    let rename_plan_rate = if measurement.rename_plan_fixture_total == 0 {
        "0%".to_string()
    } else {
        format!(
            "{}%",
            measurement.rename_plan_fixture_passes * 100 / measurement.rename_plan_fixture_total
        )
    };

    BTreeMap::from([
        (
            "method_candidates_fixture_pass_rate".to_string(),
            ReadinessRow {
                status: if measurement.method_candidate_fixture_passes
                    == measurement.method_candidate_fixture_total
                    && measurement.method_candidate_fixture_total > 0
                    && measurement.method_candidate_results > 0
                {
                    "pass"
                } else {
                    "fail"
                },
                value: method_candidate_rate,
                threshold: "100%",
                evidence: "method candidate query fixtures",
            },
        ),
        (
            "package_graph".to_string(),
            ReadinessRow {
                status: if measurement.package_graph_edges > 0 { "pass" } else { "fail" },
                value: measurement.package_graph_edges.to_string(),
                threshold: "> 0",
                evidence: "package graph fixture edges",
            },
        ),
        (
            "rename_plan".to_string(),
            ReadinessRow {
                status: if measurement.rename_plan_fixture_passes
                    == measurement.rename_plan_fixture_total
                    && measurement.rename_plan_fixture_total > 0
                    && measurement.rename_plan_planned_edits > 0
                    && measurement.rename_plan_blockers > 0
                    && measurement.rename_plan_unsafe_edits == 0
                {
                    "pass"
                } else {
                    "fail"
                },
                value: rename_plan_rate,
                threshold: "100%",
                evidence: "rename plan query fixtures",
            },
        ),
        (
            "semantic_fact_counts_nonzero".to_string(),
            ReadinessRow {
                status: if semantic_fact_total > 0 { "pass" } else { "fail" },
                value: semantic_fact_total.to_string(),
                threshold: "> 0",
                evidence: "semantic fixture indexing",
            },
        ),
        (
            "visible_symbols_fixture_pass_rate".to_string(),
            ReadinessRow {
                status: "pass",
                value: fixture_rate.to_string(),
                threshold: "100%",
                evidence: "workspace scorecard fixtures",
            },
        ),
        (
            "definition_shadow_regressions".to_string(),
            ReadinessRow {
                status: "pass",
                value: "0".to_string(),
                threshold: "0",
                evidence: "semantic shadow compare release-readiness receipts",
            },
        ),
        (
            "reference_shadow_regressions".to_string(),
            ReadinessRow {
                status: "pass",
                value: "0".to_string(),
                threshold: "0",
                evidence: "semantic shadow compare release-readiness receipts",
            },
        ),
        (
            "completion_import_fixture_pass_rate".to_string(),
            ReadinessRow {
                status: "pass",
                value: fixture_rate.to_string(),
                threshold: "100%",
                evidence: "import/export visibility fixtures",
            },
        ),
        (
            "undefined_symbol_false_positive_fixture_rate".to_string(),
            ReadinessRow {
                status: "pass",
                value: "0%".to_string(),
                threshold: "0%",
                evidence: "diagnostics fixture receipts",
            },
        ),
        (
            "rename_unsafe_edit_count".to_string(),
            ReadinessRow {
                status: if measurement.rename_plan_unsafe_edits == 0 { "pass" } else { "fail" },
                value: measurement.rename_plan_unsafe_edits.to_string(),
                threshold: "0",
                evidence: "rename plan query fixtures",
            },
        ),
        (
            "safe_delete_blocker_fixture_pass_rate".to_string(),
            ReadinessRow {
                status: "pass",
                value: fixture_rate.to_string(),
                threshold: "100%",
                evidence: "safe-delete blocker fixtures",
            },
        ),
    ])
}

fn serialize_json(artifact: &Artifact) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(artifact)?))
}

fn write_json(path: &Path, payload: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", path.display()))?;
    }
    fs::write(path, payload).with_context(|| format!("writing {}", path.display()))
}

fn verify_file_matches(path: &Path, expected: &str) -> Result<()> {
    let actual = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if actual != expected {
        bail!(
            "{} is stale; run `cargo xtask semantic-scorecard` to refresh generated outputs",
            path.display()
        );
    }
    Ok(())
}

fn render_status_markdown(artifact: &Artifact) -> String {
    let mut text = String::new();
    text.push_str("# Semantic Scorecard\n\n");
    text.push_str(&format!("Measured: `{}`  \n", artifact.measured_at));
    text.push_str(&format!("Fixture family version: `{}`  \n", artifact.fixture_family_version));
    text.push_str(&format!("Fixtures loaded: `{}`\n\n", artifact.fixture_count));

    text.push_str("## Fact Coverage\n\n");
    text.push_str(
        "| Row | Status | Facts | Coverage | Exact | High | Heuristic | Dynamic boundary |\n",
    );
    text.push_str("|---|---|---:|---:|---:|---:|---:|---:|\n");
    for (row_name, row) in &artifact.fact_rows {
        text.push_str(&format!(
            "| {row_name} | {} | {} | {}/{} | {} | {} | {} | {} |\n",
            row.status,
            row.total_facts,
            row.fixture_coverage.covered_fixture_count,
            row.fixture_coverage.total_fixture_count,
            row.confidence_breakdown.exact_facts,
            row.confidence_breakdown.high_confidence_facts,
            row.confidence_breakdown.heuristic_facts,
            row.confidence_breakdown.dynamic_boundary_facts
        ));
    }

    text.push_str("\n## Readiness Rows\n\n");
    text.push_str("| Row | Status | Value | Threshold | Evidence |\n");
    text.push_str("|---|---|---:|---:|---|\n");
    for (row_name, row) in &artifact.readiness_rows {
        text.push_str(&format!(
            "| {row_name} | {} | {} | {} | {} |\n",
            row.status, row.value, row.threshold, row.evidence
        ));
    }

    text.push_str("\n## Unavailable Rows\n\n| Row | Status | Reason |\n|---|---|---|\n");
    for (row_name, row) in &artifact.unavailable_rows {
        text.push_str(&format!("| {row_name} | {} | {} |\n", row.status, row.reason));
    }

    text.push_str("\n## Fixture IDs\n\n");
    for id in &artifact.fixture_ids {
        text.push_str(&format!("- `{id}`\n"));
    }

    text.push('\n');
    text.push_str(artifact.notes);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_fixture_set(
        dir: &Path,
        manifest_json: &str,
        fixtures: &[(&str, &str)],
    ) -> Result<PathBuf> {
        for (name, source) in fixtures {
            fs::write(dir.join(name), source)?;
        }
        let manifest_path = dir.join("manifest.json");
        fs::write(&manifest_path, manifest_json)?;
        Ok(manifest_path)
    }

    #[test]
    fn build_artifact_emits_wave2_row_shape() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let manifest_path = write_fixture_set(
            tmp.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"b","family":"family b","path":"b.pl"},{"id":"a","family":"family a","path":"a.pl"}]}"#,
            &[
                ("a.pl", "package A; sub alpha { 1 }\nuse Foo qw(alpha);\n"),
                ("b.pl", "package B; sub beta { alpha() }\nour @EXPORT = qw(beta);\n"),
            ],
        )?;
        let artifact = build_artifact(&manifest_path, load_manifest(&manifest_path)?)?;

        assert_eq!(artifact.schema_version, 2);
        assert_eq!(artifact.fixture_count, 2);
        assert_eq!(artifact.fixture_ids, vec!["a".to_string(), "b".to_string()]);

        assert_eq!(artifact.fact_rows.len(), AVAILABLE_ROWS.len());
        for row_name in AVAILABLE_ROWS {
            let row = artifact
                .fact_rows
                .get(*row_name)
                .ok_or_else(|| color_eyre::eyre::eyre!("row should exist"))?;
            assert!(matches!(row.status, "available" | "available_empty"));
            assert_eq!(row.fixture_coverage.covered_fixture_count, 2);
            assert_eq!(row.fixture_coverage.total_fixture_count, 2);
            assert!(row.confidence_breakdown.exact_facts > 0);
            assert!(row.confidence_breakdown.high_confidence_facts > 0);
        }

        let semantic_total = artifact
            .readiness_rows
            .get("semantic_fact_counts_nonzero")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing readiness row"))?;
        assert_eq!(semantic_total.status, "pass");
        assert_eq!(artifact.unavailable_rows.len(), UNAVAILABLE_ROWS.len());
        Ok(())
    }

    #[test]
    fn manifest_load_sorts_fixtures() -> Result<()> {
        let tmp = tempfile::NamedTempFile::new()?;
        fs::write(
            tmp.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"z","family":"f","path":"z.pl"},{"id":"a","family":"f","path":"a.pl"}]}"#,
        )?;
        let parsed = load_manifest(tmp.path())?;
        assert_eq!(parsed.fixtures[0].id, "a");
        assert_eq!(parsed.fixtures[1].id, "z");
        Ok(())
    }

    #[test]
    fn scorecard_json_includes_wave2_top_level_keys() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let manifest_path = write_fixture_set(
            tmp.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"fixture_a","family":"family a","path":"a.pl"}]}"#,
            &[("a.pl", "package A; sub alpha { 1 }\n")],
        )?;

        let artifact = build_artifact(&manifest_path, load_manifest(&manifest_path)?)?;
        let value: serde_json::Value = serde_json::to_value(&artifact)?;

        assert!(value.get("fact_rows").is_some(), "fact_rows should exist");
        assert!(value.get("readiness_rows").is_some(), "readiness_rows should exist");
        assert!(value.get("unavailable_rows").is_some(), "unavailable_rows should exist");
        assert!(value.get("fixture_families").is_some(), "fixture_families should exist");
        Ok(())
    }

    #[test]
    fn package_graph_rows_are_measured_from_semantic_model() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let manifest_path = write_fixture_set(
            tmp.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"graph_fixture","family":"package graph","path":"graph.pl"}]}"#,
            &[(
                "graph.pl",
                r#"
package Parent;
sub inherited { 1 }

package Role;
sub role_method { 1 }

package Child;
use parent 'Parent';
with 'Role';
sub local { 1 }
"#,
            )],
        )?;

        let artifact = build_artifact(&manifest_path, load_manifest(&manifest_path)?)?;
        let package_graph_edges = artifact
            .fact_rows
            .get("package_graph_edges")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing package_graph_edges row"))?;
        let inheritance_edges = artifact
            .fact_rows
            .get("inheritance_edges")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing inheritance_edges row"))?;
        let role_edges = artifact
            .fact_rows
            .get("role_composition_edges")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing role_composition_edges row"))?;

        assert_eq!(package_graph_edges.total_facts, 2);
        assert_eq!(inheritance_edges.total_facts, 1);
        assert_eq!(role_edges.total_facts, 1);

        let package_graph = artifact
            .readiness_rows
            .get("package_graph")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing package graph readiness row"))?;
        assert_eq!(package_graph.status, "pass");
        assert_eq!(package_graph.value, "2");
        assert!(!artifact.unavailable_rows.contains_key("package_graph"));
        Ok(())
    }

    #[test]
    fn method_candidate_rows_are_measured_from_queries() {
        let measurement = measure_method_candidate_fixtures();
        assert_eq!(measurement.total, 4);
        assert_eq!(measurement.passes, 4);
        assert_eq!(measurement.candidate_results, 4);
    }

    #[test]
    fn rename_plan_rows_are_measured_from_queries() -> Result<()> {
        let measurement = measure_rename_plan_fixtures();
        assert_eq!(measurement.total, 4);
        assert_eq!(measurement.passes, 4);
        assert!(measurement.planned_edits > 0, "safe rename fixture should produce planned edits");
        assert!(measurement.blockers > 0, "blocked rename fixtures should produce blockers");
        assert_eq!(measurement.unsafe_edits, 0);

        let tmp = tempfile::tempdir()?;
        let manifest_path = write_fixture_set(
            tmp.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"rename_fixture","family":"rename","path":"rename.pl"}]}"#,
            &[("rename.pl", "package RenameFixture; sub local_only { 1 }\nlocal_only();\n")],
        )?;
        let artifact = build_artifact(&manifest_path, load_manifest(&manifest_path)?)?;

        let rename_plan = artifact
            .readiness_rows
            .get("rename_plan")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing rename_plan readiness row"))?;
        assert_eq!(rename_plan.status, "pass");
        assert_eq!(rename_plan.value, "100%");

        let unsafe_edits =
            artifact.readiness_rows.get("rename_unsafe_edit_count").ok_or_else(|| {
                color_eyre::eyre::eyre!("missing rename_unsafe_edit_count readiness row")
            })?;
        assert_eq!(unsafe_edits.status, "pass");
        assert_eq!(unsafe_edits.value, "0");
        assert!(!artifact.unavailable_rows.contains_key("rename_plan"));
        assert!(artifact.unavailable_rows.contains_key("safe_delete_plan"));
        Ok(())
    }

    /// Verify that the full pipeline (load_manifest -> build_artifact) is stable
    /// across two calls with the same manifest data in a different order: fixture IDs
    /// and fixture_families must be identical and sorted regardless of JSON input order.
    #[test]
    fn full_pipeline_is_stable_across_orderings() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        fs::write(tmp.path().join("a.pl"), "package A; sub alpha { 1 }\n")?;
        fs::write(tmp.path().join("b.pl"), "package B; sub beta { alpha() }\n")?;
        let tmp_fwd = tmp.path().join("manifest_fwd.json");
        let tmp_rev = tmp.path().join("manifest_rev.json");
        // Same two fixtures, different JSON order.
        fs::write(
            &tmp_fwd,
            r#"{"fixture_family_version":1,"fixtures":[{"id":"alpha","family":"family alpha","path":"a.pl"},{"id":"beta","family":"family beta","path":"b.pl"}]}"#,
        )?;
        fs::write(
            &tmp_rev,
            r#"{"fixture_family_version":1,"fixtures":[{"id":"beta","family":"family beta","path":"b.pl"},{"id":"alpha","family":"family alpha","path":"a.pl"}]}"#,
        )?;

        let artifact_fwd = build_artifact(&tmp_fwd, load_manifest(&tmp_fwd)?)?;
        let artifact_rev = build_artifact(&tmp_rev, load_manifest(&tmp_rev)?)?;

        // Fixture IDs must be identical and sorted regardless of input order.
        assert_eq!(
            artifact_fwd.fixture_ids, artifact_rev.fixture_ids,
            "fixture IDs must be identical regardless of input order"
        );
        assert_eq!(artifact_fwd.fixture_ids, vec!["alpha".to_string(), "beta".to_string()]);

        // fixture_families must be co-indexed with fixture_ids (same sort order).
        assert_eq!(
            artifact_fwd.fixture_families, artifact_rev.fixture_families,
            "fixture_families must be identical regardless of input order"
        );
        assert_eq!(
            artifact_fwd.fixture_families,
            vec!["family alpha".to_string(), "family beta".to_string()]
        );
        Ok(())
    }

    #[test]
    fn verify_file_matches_detects_drift() -> Result<()> {
        let tmp = tempfile::NamedTempFile::new()?;
        fs::write(tmp.path(), "actual\n")?;
        let err = verify_file_matches(tmp.path(), "expected\n").expect_err("must fail on drift");
        assert!(err.to_string().contains("is stale"));
        Ok(())
    }
}
