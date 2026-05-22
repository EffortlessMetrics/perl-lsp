//! Validate the differential real-Perl oracle fixture manifest.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const MANIFEST_PATH: &str = "crates/perl-corpus/fixtures/differential_oracle/manifest.json";
const SCHEMA_PATH: &str = "schemas/oracle_fixture_manifest.v1.schema.json";
const SCHEMA_VERSION: &str = "oracle_fixture_manifest.v1";
const MANIFEST_NAME: &str = "differential-real-perl-oracle-fixtures";
const ORACLE_SPEC: &str = "docs/specs/PLSP-SPEC-0027-differential-real-perl-oracle.md";

const REQUIRED_COMPARISON_CLASSES: &[&str] = &[
    "PackageSubTable",
    "ImportExport",
    "IsaComposition",
    "ConstantPrototype",
    "FrameworkGeneratedMember",
    "CompileEffect",
];

const REQUIRED_RESULT_CLASSES: &[&str] = &[
    "oracle_agrees",
    "compiler_missing",
    "compiler_extra",
    "range_mismatch",
    "provenance_mismatch",
    "confidence_or_freshness_mismatch",
    "dynamic_or_unsupported",
    "oracle_ambient_unbounded",
    "stale_or_partial",
    "unknown",
];

const REQUIRED_ENVIRONMENT_DENIALS: &[&str] = &["PERL5LIB", "PERL5OPT", "local::lib"];
const ALLOWED_PATH_CLASSES: &[&str] = &["public_test_fixture", "redacted_private_fixture"];
const ALLOWED_INCLUDE_PATH_AUTHORITIES: &[&str] =
    &["declared_fixture_root", "declared_module_roots", "ambient_reported"];
const REQUIRED_CLAIM_PHRASES: &[&str] = &[
    "no oracle runner",
    "Perl execution",
    "provider behavior",
    "support-tier promotion",
    "parser/corpus bucket movement",
];

#[derive(Debug, Deserialize)]
struct OracleFixtureManifest {
    schema_version: String,
    manifest: String,
    owner: String,
    status: String,
    updated: String,
    spec: String,
    runner: String,
    editor_runtime_dependency: bool,
    comparison_classes: Vec<String>,
    result_classes: Vec<String>,
    required_environment_denials: Vec<String>,
    default_claim_boundary: String,
    #[serde(default)]
    fixtures: Vec<OracleFixture>,
}

#[derive(Debug, Deserialize)]
struct OracleFixture {
    id: String,
    source: String,
    path_class: String,
    perl_version_constraint: String,
    include_path_authority: String,
    module_roots: Vec<String>,
    environment_denials: Vec<String>,
    comparison_classes: Vec<String>,
    #[serde(default)]
    dynamic_boundaries: Vec<String>,
    #[serde(default)]
    unsupported_effects: Vec<String>,
    #[serde(default)]
    framework_adapters: Vec<String>,
    claim_boundary: String,
}

#[derive(Debug)]
struct ValidationStats {
    fixtures: usize,
    comparison_classes: usize,
    result_classes: usize,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let stats = validate(&root)?;
    println!(
        "oracle fixture manifest check passed: {} fixtures, {} comparison classes, {} result classes",
        stats.fixtures, stats.comparison_classes, stats.result_classes
    );
    Ok(())
}

fn validate(root: &Path) -> Result<ValidationStats> {
    validate_json_parse(root, SCHEMA_PATH)?;
    let manifest = read_manifest(root, MANIFEST_PATH)?;
    let mut violations = Vec::new();

    validate_manifest_shape(root, &manifest, &mut violations);
    validate_fixtures(root, &manifest, &mut violations);

    if !violations.is_empty() {
        eprintln!("oracle fixture manifest violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("oracle fixture manifest check failed with {} violation(s)", violations.len());
    }

    Ok(ValidationStats {
        fixtures: manifest.fixtures.len(),
        comparison_classes: manifest.comparison_classes.len(),
        result_classes: manifest.result_classes.len(),
    })
}

fn validate_json_parse(root: &Path, rel: &str) -> Result<()> {
    let text = read_text(root, rel)?;
    let _: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("failed to parse {rel} as JSON"))?;
    Ok(())
}

fn read_manifest(root: &Path, rel: &str) -> Result<OracleFixtureManifest> {
    let text = read_text(root, rel)?;
    serde_json::from_str(&text).with_context(|| format!("failed to parse {rel}"))
}

fn read_text(root: &Path, rel: &str) -> Result<String> {
    let path = root.join(rel);
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

fn validate_manifest_shape(
    root: &Path,
    manifest: &OracleFixtureManifest,
    violations: &mut Vec<String>,
) {
    if manifest.schema_version != SCHEMA_VERSION {
        violations.push(format!(
            "{MANIFEST_PATH}: schema_version is {:?}; expected {:?}",
            manifest.schema_version, SCHEMA_VERSION
        ));
    }
    if manifest.manifest != MANIFEST_NAME {
        violations.push(format!(
            "{MANIFEST_PATH}: manifest is {:?}; expected {:?}",
            manifest.manifest, MANIFEST_NAME
        ));
    }
    require_non_empty(MANIFEST_PATH, "owner", &manifest.owner, violations);
    require_non_empty(MANIFEST_PATH, "updated", &manifest.updated, violations);
    if manifest.status != "declaration-only" {
        violations.push(format!(
            "{MANIFEST_PATH}: status is {:?}; expected \"declaration-only\"",
            manifest.status
        ));
    }
    if manifest.spec != ORACLE_SPEC {
        violations.push(format!(
            "{MANIFEST_PATH}: spec is {:?}; expected {:?}",
            manifest.spec, ORACLE_SPEC
        ));
    }
    validate_relative_existing_path(root, MANIFEST_PATH, "spec", &manifest.spec, violations);
    validate_relative_existing_path(root, MANIFEST_PATH, "schema", SCHEMA_PATH, violations);
    if manifest.runner != "none" {
        violations.push(format!(
            "{MANIFEST_PATH}: runner is {:?}; expected \"none\" for declaration-only manifest",
            manifest.runner
        ));
    }
    if manifest.editor_runtime_dependency {
        violations.push(format!(
            "{MANIFEST_PATH}: editor_runtime_dependency must be false for oracle fixtures"
        ));
    }

    require_exact_set(
        MANIFEST_PATH,
        "comparison_classes",
        &manifest.comparison_classes,
        REQUIRED_COMPARISON_CLASSES,
        violations,
    );
    require_exact_set(
        MANIFEST_PATH,
        "result_classes",
        &manifest.result_classes,
        REQUIRED_RESULT_CLASSES,
        violations,
    );
    require_exact_set(
        MANIFEST_PATH,
        "required_environment_denials",
        &manifest.required_environment_denials,
        REQUIRED_ENVIRONMENT_DENIALS,
        violations,
    );
    validate_claim_boundary(
        MANIFEST_PATH,
        "default_claim_boundary",
        &manifest.default_claim_boundary,
        violations,
    );

    if manifest.fixtures.is_empty() {
        violations.push(format!("{MANIFEST_PATH}: fixtures list must not be empty"));
    }
}

fn validate_fixtures(root: &Path, manifest: &OracleFixtureManifest, violations: &mut Vec<String>) {
    let comparison_class_set =
        manifest.comparison_classes.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();

    for fixture in &manifest.fixtures {
        let doc = format!("{MANIFEST_PATH}: fixture {}", fixture.id);
        require_non_empty(&doc, "id", &fixture.id, violations);
        if !seen.insert(fixture.id.clone()) {
            violations.push(format!("{MANIFEST_PATH}: duplicate fixture id {:?}", fixture.id));
        }

        validate_relative_existing_path(root, &doc, "source", &fixture.source, violations);
        validate_allowed(&doc, "path_class", &fixture.path_class, ALLOWED_PATH_CLASSES, violations);
        require_non_empty(
            &doc,
            "perl_version_constraint",
            &fixture.perl_version_constraint,
            violations,
        );
        validate_allowed(
            &doc,
            "include_path_authority",
            &fixture.include_path_authority,
            ALLOWED_INCLUDE_PATH_AUTHORITIES,
            violations,
        );
        validate_non_empty_path_list(root, &doc, "module_roots", &fixture.module_roots, violations);
        require_contains_all(
            &doc,
            "environment_denials",
            &fixture.environment_denials,
            REQUIRED_ENVIRONMENT_DENIALS,
            violations,
        );
        validate_fixture_comparison_classes(
            &doc,
            &comparison_class_set,
            &fixture.comparison_classes,
            violations,
        );
        validate_string_list(&doc, "dynamic_boundaries", &fixture.dynamic_boundaries, violations);
        validate_string_list(&doc, "unsupported_effects", &fixture.unsupported_effects, violations);
        validate_string_list(&doc, "framework_adapters", &fixture.framework_adapters, violations);
        validate_framework_adapter_requirement(&doc, fixture, violations);
        validate_claim_boundary(&doc, "claim_boundary", &fixture.claim_boundary, violations);
    }
}

fn validate_fixture_comparison_classes(
    doc: &str,
    allowed: &BTreeSet<&str>,
    values: &[String],
    violations: &mut Vec<String>,
) {
    if values.is_empty() {
        violations.push(format!("{doc}: comparison_classes must not be empty"));
        return;
    }
    for value in values {
        if value.trim().is_empty() {
            violations.push(format!("{doc}: comparison_classes contains an empty item"));
            continue;
        }
        if !allowed.contains(value.as_str()) {
            violations
                .push(format!("{doc}: comparison_classes contains unknown class {:?}", value));
        }
    }
}

fn validate_framework_adapter_requirement(
    doc: &str,
    fixture: &OracleFixture,
    violations: &mut Vec<String>,
) {
    if fixture.comparison_classes.iter().any(|class| class == "FrameworkGeneratedMember")
        && fixture.framework_adapters.is_empty()
    {
        violations.push(format!(
            "{doc}: FrameworkGeneratedMember fixtures must declare at least one framework_adapter"
        ));
    }
}

fn validate_relative_existing_path(
    root: &Path,
    doc: &str,
    field: &str,
    value: &str,
    violations: &mut Vec<String>,
) {
    if value.trim().is_empty() {
        violations.push(format!("{doc}: {field} must not be empty"));
        return;
    }
    if Path::new(value).is_absolute() || value.contains(':') || value.contains('\\') {
        violations.push(format!("{doc}: {field} must be a repo-relative slash path: {value}"));
        return;
    }
    let path = root.join(value);
    if !path.exists() {
        violations.push(format!("{doc}: {field} points to missing path {value}"));
        return;
    }

    let Ok(root) = root.canonicalize() else {
        violations.push(format!("{doc}: could not canonicalize repo root {}", root.display()));
        return;
    };
    let Ok(path) = path.canonicalize() else {
        violations.push(format!("{doc}: could not canonicalize {field} path {value}"));
        return;
    };
    if !path.starts_with(&root) {
        violations.push(format!("{doc}: {field} escapes repo root: {value}"));
    }
}

fn validate_non_empty_path_list(
    root: &Path,
    doc: &str,
    field: &str,
    values: &[String],
    violations: &mut Vec<String>,
) {
    if values.is_empty() {
        violations.push(format!("{doc}: {field} must not be empty"));
        return;
    }
    for value in values {
        validate_relative_existing_path(root, doc, field, value, violations);
    }
}

fn validate_string_list(doc: &str, field: &str, values: &[String], violations: &mut Vec<String>) {
    for value in values {
        if value.trim().is_empty() {
            violations.push(format!("{doc}: {field} contains an empty item"));
        }
    }
}

fn validate_allowed(
    doc: &str,
    field: &str,
    value: &str,
    allowed: &[&str],
    violations: &mut Vec<String>,
) {
    if !allowed.contains(&value) {
        violations.push(format!("{doc}: {field} {:?} is not allowed", value));
    }
}

fn validate_claim_boundary(doc: &str, field: &str, value: &str, violations: &mut Vec<String>) {
    require_non_empty(doc, field, value, violations);
    for phrase in REQUIRED_CLAIM_PHRASES {
        if !value.contains(phrase) {
            violations.push(format!("{doc}: {field} must include phrase {phrase:?}"));
        }
    }
}

fn require_exact_set(
    doc: &str,
    field: &str,
    actual: &[String],
    expected: &[&str],
    violations: &mut Vec<String>,
) {
    let actual_set = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let expected_set = expected.iter().copied().collect::<BTreeSet<_>>();

    for missing in expected_set.difference(&actual_set) {
        violations.push(format!("{doc}: {field} missing required entry {missing:?}"));
    }
    for unexpected in actual_set.difference(&expected_set) {
        violations.push(format!("{doc}: {field} contains unsupported entry {unexpected:?}"));
    }
}

fn require_contains_all(
    doc: &str,
    field: &str,
    actual: &[String],
    expected: &[&str],
    violations: &mut Vec<String>,
) {
    if actual.is_empty() {
        violations.push(format!("{doc}: {field} must not be empty"));
        return;
    }
    let actual_set = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for missing in expected {
        if !actual_set.contains(missing) {
            violations.push(format!("{doc}: {field} missing required entry {missing:?}"));
        }
    }
    validate_string_list(doc, field, actual, violations);
}

fn require_non_empty(doc: &str, field: &str, value: &str, violations: &mut Vec<String>) {
    if value.trim().is_empty() {
        violations.push(format!("{doc}: {field} must not be empty"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T>;

    #[test]
    fn accepts_minimal_valid_manifest() -> TestResult {
        let tempdir = valid_manifest_workspace()?;

        let stats = validate(tempdir.path())?;

        assert_eq!(stats.fixtures, 1);
        assert_eq!(stats.comparison_classes, REQUIRED_COMPARISON_CLASSES.len());
        assert_eq!(stats.result_classes, REQUIRED_RESULT_CLASSES.len());
        Ok(())
    }

    #[test]
    fn rejects_missing_required_environment_denial() -> TestResult {
        let tempdir = valid_manifest_workspace()?;
        let manifest_path = tempdir.path().join(MANIFEST_PATH);
        let text = fs::read_to_string(&manifest_path)?;
        fs::write(
            &manifest_path,
            text.replace(
                r#""environment_denials": ["PERL5LIB", "PERL5OPT", "local::lib"]"#,
                r#""environment_denials": ["PERL5LIB", "PERL5OPT"]"#,
            ),
        )?;

        let err = validate(tempdir.path()).expect_err("missing local::lib denial should fail");

        assert!(
            err.to_string().contains("oracle fixture manifest check failed"),
            "unexpected error: {err:?}"
        );
        Ok(())
    }

    #[test]
    fn rejects_framework_generated_member_without_adapter() -> TestResult {
        let tempdir = valid_manifest_workspace()?;
        let manifest_path = tempdir.path().join(MANIFEST_PATH);
        let text = fs::read_to_string(&manifest_path)?;
        fs::write(
            &manifest_path,
            text.replace(r#""framework_adapters": ["Moo"]"#, r#""framework_adapters": []"#),
        )?;

        let err = validate(tempdir.path()).expect_err("missing framework adapter should fail");

        assert!(
            err.to_string().contains("oracle fixture manifest check failed"),
            "unexpected error: {err:?}"
        );
        Ok(())
    }

    #[test]
    fn rejects_absolute_source_path() -> TestResult {
        let tempdir = valid_manifest_workspace()?;
        let manifest_path = tempdir.path().join(MANIFEST_PATH);
        let text = fs::read_to_string(&manifest_path)?;
        fs::write(
            &manifest_path,
            text.replace(
                r#""source": "fixtures/package_basic.pl""#,
                r#""source": "C:/tmp/package_basic.pl""#,
            ),
        )?;

        let err = validate(tempdir.path()).expect_err("absolute source path should fail");

        assert!(
            err.to_string().contains("oracle fixture manifest check failed"),
            "unexpected error: {err:?}"
        );
        Ok(())
    }

    fn valid_manifest_workspace() -> TestResult<tempfile::TempDir> {
        let tempdir = tempfile::tempdir()?;
        fs::create_dir_all(tempdir.path().join("schemas"))?;
        fs::create_dir_all(tempdir.path().join("docs/specs"))?;
        fs::create_dir_all(tempdir.path().join("crates/perl-corpus/fixtures/differential_oracle"))?;
        fs::create_dir_all(tempdir.path().join("fixtures"))?;
        fs::write(tempdir.path().join(SCHEMA_PATH), "{}\n")?;
        fs::write(tempdir.path().join(ORACLE_SPEC), "# oracle spec\n")?;
        fs::write(tempdir.path().join("fixtures/package_basic.pl"), "package Demo; 1;\n")?;
        fs::write(tempdir.path().join(MANIFEST_PATH), valid_manifest_text())?;
        Ok(tempdir)
    }

    fn valid_manifest_text() -> String {
        format!(
            r#"{{
  "schema_version": "{SCHEMA_VERSION}",
  "manifest": "{MANIFEST_NAME}",
  "owner": "perl-lsp maintainers",
  "status": "declaration-only",
  "updated": "2026-05-22",
  "spec": "{ORACLE_SPEC}",
  "runner": "none",
  "editor_runtime_dependency": false,
  "comparison_classes": [{comparison_classes}],
  "result_classes": [{result_classes}],
  "required_environment_denials": ["PERL5LIB", "PERL5OPT", "local::lib"],
  "default_claim_boundary": "Fixture declaration only; no oracle runner, Perl execution, provider behavior, support-tier promotion, or parser/corpus bucket movement.",
  "fixtures": [
    {{
      "id": "package_basic",
      "source": "fixtures/package_basic.pl",
      "path_class": "public_test_fixture",
      "perl_version_constraint": "any-supported-real-perl",
      "include_path_authority": "declared_fixture_root",
      "module_roots": ["fixtures"],
      "environment_denials": ["PERL5LIB", "PERL5OPT", "local::lib"],
      "comparison_classes": ["FrameworkGeneratedMember"],
      "dynamic_boundaries": [],
      "unsupported_effects": [],
      "framework_adapters": ["Moo"],
      "claim_boundary": "Fixture declaration only; no oracle runner, Perl execution, provider behavior, support-tier promotion, or parser/corpus bucket movement."
    }}
  ]
}}
"#,
            comparison_classes = quoted_list(REQUIRED_COMPARISON_CLASSES),
            result_classes = quoted_list(REQUIRED_RESULT_CLASSES),
        )
    }

    fn quoted_list(values: &[&str]) -> String {
        values.iter().map(|value| format!("\"{value}\"")).collect::<Vec<_>>().join(", ")
    }
}
