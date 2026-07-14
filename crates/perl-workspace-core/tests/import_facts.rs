//! Integration tests for PR 4: import / compile-effect / dynamic-boundary facts.
// Integration-test helpers live outside `#[test]` fns, so clippy's
// allow-unwrap-in-tests does not reach them; allow unwrap for the whole file.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use perl_workspace_core::{
    DynamicBoundaryKind, ExportKind, FactClasses, ImportKind, ProjectModel, ProjectModelRequest,
    build_project_model,
};

/// Materialize a single-file fixture, build the model with all fact classes,
/// clean up, and return the model.
fn model_for(dir: &str, rel: &str, content: &str) -> ProjectModel {
    let root: PathBuf = std::env::temp_dir().join(format!("pwc-import-{dir}"));
    let _ = std::fs::remove_dir_all(&root);
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    let model = build_project_model(&ProjectModelRequest {
        root: root.to_str().unwrap(),
        fact_classes: FactClasses::all(),
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(&root);
    model
}

#[test]
fn extracts_pragma_and_module_imports() {
    let model = model_for(
        "imports",
        "lib/App.pm",
        "package App;\nuse strict;\nuse warnings;\nuse POSIX qw(floor ceil);\n1;\n",
    );
    let modules: Vec<&str> = model.imports.iter().map(|i| i.module.as_str()).collect();
    assert!(modules.contains(&"strict"), "strict imported; got {modules:?}");
    assert!(modules.contains(&"warnings"));
    assert!(modules.contains(&"POSIX"));

    let strict = model.imports.iter().find(|i| i.module == "strict").unwrap();
    assert_eq!(strict.kind, ImportKind::Use);
    assert!(strict.is_pragma, "strict is a pragma");

    let posix = model.imports.iter().find(|i| i.module == "POSIX").unwrap();
    assert_eq!(posix.imports, vec!["floor", "ceil"], "qw() import list normalized");
    assert!(!posix.is_pragma);
}

#[test]
fn computes_compile_effects_via_pragma() {
    let model =
        model_for("effects", "lib/App.pm", "package App;\nuse strict;\nuse warnings;\n1;\n");
    let file = model.file_by_path("lib/App.pm").unwrap();
    let effects = model.compile_effects_for_file(&file.file_id).unwrap();
    assert!(effects.strict, "use strict → strict effect");
    assert!(effects.warnings, "use warnings → warnings effect");
}

#[test]
fn version_use_sets_perl_version_and_features() {
    let model = model_for("version", "lib/App.pm", "use v5.38;\npackage App;\n1;\n");
    let file = model.file_by_path("lib/App.pm").unwrap();
    let effects = model.compile_effects_for_file(&file.file_id).unwrap();
    assert_eq!(effects.perl_version.as_deref(), Some("v5.38"), "bare version captured");
    // v5.38 implies strict and enables a feature bundle (perl-pragma's tables).
    assert!(effects.strict, "use v5.12+ implies strict");
    assert!(!effects.features.is_empty(), "a version bundle enables features");
}

#[test]
fn use_parent_populates_package_inheritance() {
    let model =
        model_for("parent", "lib/Child.pm", "package Child;\nuse parent -norequire, 'Base';\n1;\n");
    let pkg = model.packages.iter().find(|p| p.name == "Child").unwrap();
    assert_eq!(pkg.parents, vec!["Base"], "use parent → inheritance (flag stripped)");
}

#[test]
fn static_require_is_an_import_dynamic_require_is_a_boundary() {
    let model = model_for(
        "require",
        "lib/App.pm",
        "package App;\nrequire Foo::Bar;\nmy $m = 'Baz';\nrequire $m;\n1;\n",
    );
    // Static bareword require → an import fact.
    let has_static =
        model.imports.iter().any(|i| i.kind == ImportKind::Require && i.module == "Foo::Bar");
    assert!(has_static, "static require Foo::Bar is an import; imports={:?}", model.imports);
    // Dynamic require($var) → a RuntimeRequire boundary.
    let has_dynamic =
        model.dynamic_boundaries.iter().any(|b| b.kind == DynamicBoundaryKind::RuntimeRequire);
    assert!(has_dynamic, "require $m is a runtime boundary");
}

#[test]
fn string_eval_is_a_boundary_block_eval_is_not() {
    let string_eval = model_for("seval", "lib/App.pm", "package App;\neval \"1 + 1\";\n1;\n");
    assert!(
        string_eval.dynamic_boundaries.iter().any(|b| b.kind == DynamicBoundaryKind::StringEval),
        "string eval is a boundary"
    );

    let block_eval = model_for("beval", "lib/App.pm", "package App;\neval { 1 + 1 };\n1;\n");
    assert!(
        !block_eval.dynamic_boundaries.iter().any(|b| b.kind == DynamicBoundaryKind::StringEval),
        "block eval is NOT a string-eval boundary"
    );
}

#[test]
fn statement_package_in_nested_block_does_not_leak_context() {
    // `use base` after a bare block must attribute to the OUTER package, not the
    // package declared inside the block (block-scoped `package`).
    let model = model_for(
        "nested-pkg",
        "lib/App.pm",
        "package Outer;\n{\n    package Inner;\n}\nuse base 'Role';\n1;\n",
    );
    let outer = model.packages.iter().find(|p| p.name == "Outer").unwrap();
    assert_eq!(outer.parents, vec!["Role"], "inheritance belongs to Outer");
    if let Some(inner) = model.packages.iter().find(|p| p.name == "Inner") {
        assert!(inner.parents.is_empty(), "Inner must NOT inherit Role");
    }
}

#[test]
fn numeric_require_is_not_a_boundary_or_import() {
    // `require 5.010;` is a version assertion, not a module load.
    let model = model_for("num-require", "lib/App.pm", "package App;\nrequire 5.010;\n1;\n");
    assert!(
        !model.dynamic_boundaries.iter().any(|b| b.kind == DynamicBoundaryKind::RuntimeRequire),
        "numeric require is not a runtime boundary"
    );
    assert!(
        !model.imports.iter().any(|i| i.kind == ImportKind::Require),
        "numeric require is not an import"
    );
}

#[test]
fn extracts_exporter_symbol_lists() {
    let model = model_for(
        "exports",
        "lib/Api.pm",
        "package Api;\nour @EXPORT = qw(run stop);\nour @EXPORT_OK = ('reset');\n1;\n",
    );
    let default = model.exports.iter().find(|e| e.kind == ExportKind::Default).unwrap();
    assert_eq!(default.symbols, vec!["run", "stop"], "qw list normalized");
    assert_eq!(default.package.as_deref(), Some("Api"));

    let optional = model.exports.iter().find(|e| e.kind == ExportKind::Optional).unwrap();
    assert_eq!(optional.symbols, vec!["reset"], "parenthesized quoted list normalized");
}

#[test]
fn exports_only_present_when_requested() {
    let root = std::env::temp_dir().join("pwc-import-exports-gate");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(root.join("lib/Api.pm"), "package Api;\nour @EXPORT = qw(run);\n1;\n").unwrap();
    // FILES only: no exports, and no "unimplemented" limitation for exports.
    let model = build_project_model(&ProjectModelRequest {
        root: root.to_str().unwrap(),
        fact_classes: FactClasses::FILES,
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(&root);
    assert!(model.exports.is_empty(), "EXPORTS not requested → no export facts");
    assert!(
        !model.limitations.iter().any(|l| l.id == "unimplemented-fact-class:exports"),
        "exports is implemented — no unimplemented limitation"
    );
}

#[test]
fn discovers_test_framework_and_assertions() {
    // A `.t` file (FileRole::Test) yields a TestFact when TESTS is requested.
    let model = model_for(
        "testfacts",
        "t/basic.t",
        "use Test::More;\nok(1, 'truthy');\nis(1, 1);\ndone_testing;\n",
    );
    let file = model.file_by_path("t/basic.t").unwrap();
    let tf = model.tests.iter().find(|t| t.file_id == file.file_id).unwrap();
    assert_eq!(tf.framework.as_deref(), Some("Test::More"));
    assert!(tf.assertion_count >= 2, "ok + is counted; got {}", tf.assertion_count);
    assert!(tf.has_plan, "done_testing → has_plan");
}

#[test]
fn typeglob_is_a_boundary() {
    let model =
        model_for("glob", "lib/App.pm", "package App;\nno strict 'refs';\n*alias = \\&orig;\n1;\n");
    assert!(
        model.dynamic_boundaries.iter().any(|b| b.kind == DynamicBoundaryKind::TypeglobAssignment),
        "typeglob assignment is a boundary; boundaries={:?}",
        model.dynamic_boundaries
    );
}

#[test]
fn imports_only_request_skips_symbols_and_effects() {
    let root: PathBuf = std::env::temp_dir().join("pwc-import-only");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(root.join("lib/App.pm"), "package App;\nuse strict;\nsub run { 1 }\n1;\n")
        .unwrap();
    let model = build_project_model(&ProjectModelRequest {
        root: root.to_str().unwrap(),
        fact_classes: FactClasses::FILES | FactClasses::IMPORTS,
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(&root);

    assert!(!model.imports.is_empty(), "imports requested → present");
    assert!(model.symbols.is_empty(), "symbols not requested → absent");
    assert!(model.compile_effects.is_empty(), "compile effects not requested → absent");
}
