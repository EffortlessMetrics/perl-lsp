//! Integration tests for the POD and RELATIONS fact classes (11/11 coverage).
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use perl_workspace_core::{
    FactClasses, PodSectionKind, ProjectModel, ProjectModelRequest, RelationKind,
    build_project_model,
};

fn build(dir: &str, files: &[(&str, &str)], classes: FactClasses) -> ProjectModel {
    let root: PathBuf = std::env::temp_dir().join(format!("pwc-podrel-{dir}"));
    let _ = std::fs::remove_dir_all(&root);
    for (rel, content) in files {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
    }
    let model = build_project_model(&ProjectModelRequest {
        root: root.to_str().unwrap(),
        fact_classes: classes,
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(&root);
    model
}

#[test]
fn extracts_pod_facts() {
    let model = build(
        "pod",
        &[(
            "lib/App.pm",
            "package App;\n\n=head1 NAME\n\nApp - the app\n\n=head2 run\n\nRuns.\n\n=cut\n\nsub run { 1 }\n1;\n",
        )],
        FactClasses::FILES | FactClasses::POD,
    );
    let pod = model.pod.iter().find(|p| p.name.is_some()).unwrap();
    assert_eq!(pod.name.as_deref(), Some("App - the app"));
    assert!(pod.documented_methods.contains(&"run".to_string()));
    assert!(pod.sections.iter().any(|s| s.kind == PodSectionKind::Head1 && s.title == "NAME"));
}

#[test]
fn pod_absent_when_not_requested() {
    let model = build(
        "pod-gate",
        &[("lib/App.pm", "=head1 NAME\n\nApp\n\n=cut\npackage App;\n1;\n")],
        FactClasses::FILES,
    );
    assert!(model.pod.is_empty(), "POD not requested → no pod facts");
    assert!(!model.limitations.iter().any(|l| l.id == "unimplemented-fact-class:pod"));
}

#[test]
fn synthesizes_inherits_uses_and_tests_relations() {
    let model = build(
        "rel",
        &[
            ("lib/Child.pm", "package Child;\nuse parent -norequire, 'Base';\nuse Moo;\n1;\n"),
            ("t/child.t", "use Test::More;\nuse Child;\nok(1);\ndone_testing;\n"),
        ],
        FactClasses::all(),
    );
    assert!(
        model
            .relations
            .iter()
            .any(|r| r.kind == RelationKind::Inherits && r.source == "Child" && r.target == "Base"),
        "inherits edge; relations={:?}",
        model.relations
    );
    assert!(
        model.relations.iter().any(|r| r.kind == RelationKind::Uses && r.target == "Moo"),
        "uses edge for Moo"
    );
    assert!(
        model.relations.iter().any(|r| r.kind == RelationKind::Tests && r.target == "Child"),
        "test file → Tests edge for Child"
    );
    // Pragmas are not module-use edges.
    assert!(!model.relations.iter().any(|r| r.target == "strict" || r.target == "parent"));
}

#[test]
fn relations_absent_when_not_requested() {
    let model = build(
        "rel-gate",
        &[("lib/Child.pm", "package Child;\nuse parent 'Base';\n1;\n")],
        FactClasses::FILES,
    );
    assert!(model.relations.is_empty(), "RELATIONS not requested → none");
    assert!(!model.limitations.iter().any(|l| l.id == "unimplemented-fact-class:relations"));
}

#[test]
fn full_coverage_no_unimplemented_limitations() {
    // With every fact class requested, no class reports itself unimplemented.
    let model = build(
        "full",
        &[("lib/App.pm", "package App;\nuse strict;\nsub run { 1 }\n1;\n")],
        FactClasses::all(),
    );
    let unimpl: Vec<&str> = model
        .limitations
        .iter()
        .filter(|l| l.kind == "unimplemented_fact_class")
        .map(|l| l.id.as_str())
        .collect();
    assert!(unimpl.is_empty(), "no fact class is unimplemented; got {unimpl:?}");
}
