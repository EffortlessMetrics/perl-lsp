//! Integration tests for PR 7: distribution-metadata facts via the builder.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use perl_workspace_core::{FactClasses, ProjectModelRequest, build_project_model};

fn build(
    dir: &str,
    files: &[(&str, &str)],
    classes: FactClasses,
) -> perl_workspace_core::ProjectModel {
    let root: PathBuf = std::env::temp_dir().join(format!("pwc-dist-{dir}"));
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
fn builder_extracts_meta_json_facts() {
    let model = build(
        "meta",
        &[(
            "META.json",
            r#"{"name":"Foo-Bar","version":"1.23","license":["perl_5"],
                "prereqs":{"runtime":{"requires":{"Moo":"2.0"}}}}"#,
        )],
        FactClasses::FILES | FactClasses::DIST,
    );
    let facts = model.dist_metadata.iter().find(|d| d.name.as_deref() == Some("Foo-Bar")).unwrap();
    assert_eq!(facts.version.as_deref(), Some("1.23"));
    assert_eq!(facts.licenses, vec!["perl_5"]);
    assert!(facts.prereqs.iter().any(|p| p.module == "Moo"));
    assert!(model.all_prereqs().iter().any(|p| p.module == "Moo"));
}

#[test]
fn builder_extracts_cpanfile_facts() {
    let model = build(
        "cpan",
        &[("cpanfile", "requires 'Path::Tiny', '0.100';\ntest_requires 'Test::More';\n")],
        FactClasses::FILES | FactClasses::DIST,
    );
    let facts = model.dist_metadata.iter().find(|d| d.file_id.as_str().contains("fnv64")).unwrap();
    assert!(facts.prereqs.iter().any(|p| p.module == "Path::Tiny" && p.phase == "runtime"));
    assert!(facts.prereqs.iter().any(|p| p.module == "Test::More" && p.phase == "test"));
}

#[test]
fn dist_facts_absent_when_not_requested() {
    let model =
        build("no-dist", &[("META.json", r#"{"name":"X","version":"1"}"#)], FactClasses::FILES);
    // The file is still indexed…
    assert!(model.file_by_path("META.json").is_some());
    // …but its content is not parsed into dist facts.
    assert!(model.dist_metadata.is_empty(), "DIST not requested → no dist facts");
}
