use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectationMode {
    ParseClean,
    RecoverWithoutPanic,
    ExpectedError,
    TokenOnly,
    SpanOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarConcept {
    pub id: String,
    pub tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarExpect {
    pub panic: bool,
    pub timeout: bool,
    pub mode: ExpectationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarMetrics {
    pub max_error_nodes: Option<u32>,
    pub must_emit_node_kinds: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarSnapshots {
    pub tokens: bool,
    pub ast: bool,
    pub spans: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FixtureExpectationSidecar {
    pub concept: SidecarConcept,
    pub expect: SidecarExpect,
    pub metrics: Option<SidecarMetrics>,
    pub snapshots: Option<SidecarSnapshots>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SidecarValidation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SidecarValidation {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ConceptRegistry {
    concept_ids: HashSet<String>,
}

impl ConceptRegistry {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading concept registry {}", path.display()))?;
        let value = toml::from_str::<toml::Value>(&raw)
            .with_context(|| format!("parsing concept registry TOML {}", path.display()))?;

        let mut concept_ids = HashSet::new();
        collect_concept_ids(&value, &mut concept_ids);

        Ok(Self { concept_ids })
    }

    pub fn contains(&self, concept_id: &str) -> bool {
        self.concept_ids.contains(concept_id)
    }
}

pub fn parse_sidecar(path: &Path) -> Result<FixtureExpectationSidecar> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading sidecar {}", path.display()))?;
    toml::from_str::<toml::Value>(&raw)
        .with_context(|| format!("parsing sidecar TOML {}", path.display()))?;
    let parsed: FixtureExpectationSidecar = toml::from_str(&raw)
        .with_context(|| format!("deserializing sidecar {}", path.display()))?;
    Ok(parsed)
}

pub fn expected_fixture_path(sidecar_path: &Path) -> Result<PathBuf> {
    let file_name = sidecar_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid sidecar path: {}", sidecar_path.display()))?;

    if !file_name.ends_with(".meta.toml") {
        bail!("sidecar filename must end with .meta.toml: {}", sidecar_path.display());
    }

    let fixture_stem = file_name.trim_end_matches(".meta.toml");
    if fixture_stem.is_empty() {
        bail!("fixture stem must not be empty: {}", sidecar_path.display());
    }

    let fixture_name = fixture_stem.to_string() + ".pl";
    let parent = sidecar_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(fixture_name))
}

pub fn validate_sidecar(
    sidecar_path: &Path,
    sidecar: &FixtureExpectationSidecar,
    concept_registry: Option<&ConceptRegistry>,
) -> SidecarValidation {
    let mut validation = SidecarValidation::default();

    match expected_fixture_path(sidecar_path) {
        Ok(fixture_path) => {
            if !fixture_path.exists() {
                validation
                    .errors
                    .push(format!("fixture file does not exist: {}", fixture_path.display()));
            }
        }
        Err(error) => validation.errors.push(error.to_string()),
    }

    if sidecar.concept.id.trim().is_empty() {
        validation.errors.push("concept.id must not be empty".to_string());
    } else if let Some(registry) = concept_registry {
        if !registry.contains(&sidecar.concept.id) {
            validation.errors.push(format!(
                "concept.id '{}' is not present in the loaded concept registry",
                sidecar.concept.id
            ));
        }
    } else {
        validation.warnings.push(format!(
            "concept registry unavailable; concept resolution pending for '{}'",
            sidecar.concept.id
        ));
    }

    validation
}

pub fn load_and_validate_sidecar(
    sidecar_path: &Path,
    concept_registry: Option<&ConceptRegistry>,
) -> Result<SidecarValidation> {
    let sidecar = parse_sidecar(sidecar_path)?;
    Ok(validate_sidecar(sidecar_path, &sidecar, concept_registry))
}

pub fn discover_sidecars(root: &Path) -> Result<Vec<PathBuf>> {
    let mut sidecars = Vec::new();
    if !root.exists() {
        return Ok(sidecars);
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            fs::read_dir(&dir).with_context(|| format!("reading directory {}", dir.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("reading entry in {}", dir.display()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .with_context(|| format!("getting file type for {}", path.display()))?;

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file() && path.to_string_lossy().ends_with(".meta.toml") {
                sidecars.push(path);
            }
        }
    }

    sidecars.sort();
    Ok(sidecars)
}

fn collect_concept_ids(value: &toml::Value, concept_ids: &mut HashSet<String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, item) in table {
                if key == "id"
                    && let toml::Value::String(id) = item
                    && !id.trim().is_empty()
                {
                    concept_ids.insert(id.clone());
                }
                collect_concept_ids(item, concept_ids);
            }
        }
        toml::Value::Array(items) => {
            for item in items {
                collect_concept_ids(item, concept_ids);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    // helpers

    fn minimal_sidecar_toml(id: &str, mode: &str) -> String {
        format!(
            r#"
[concept]
id = "{id}"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "{mode}"
"#
        )
    }

    fn write_temp_file(dir: &std::path::Path, name: &str, contents: &str) -> Result<PathBuf> {
        let path = dir.join(name);
        fs::write(&path, contents)?;
        Ok(path)
    }

    // ExpectationMode deserialization

    #[test]
    fn expectation_mode_rejects_unknown_value() {
        let raw = r#"
[concept]
id = "parser.recovery.missing"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "mystery"
"#;

        let parsed = toml::from_str::<FixtureExpectationSidecar>(raw);
        assert!(parsed.is_err());
    }

    #[test]
    fn expectation_mode_parse_clean_roundtrips() -> Result<()> {
        let raw = minimal_sidecar_toml("some.concept", "parse_clean");
        let sidecar: FixtureExpectationSidecar = toml::from_str(&raw)?;
        assert_eq!(sidecar.expect.mode, ExpectationMode::ParseClean);
        Ok(())
    }

    #[test]
    fn expectation_mode_recover_without_panic_roundtrips() -> Result<()> {
        let raw = minimal_sidecar_toml("some.concept", "recover_without_panic");
        let sidecar: FixtureExpectationSidecar = toml::from_str(&raw)?;
        assert_eq!(sidecar.expect.mode, ExpectationMode::RecoverWithoutPanic);
        Ok(())
    }

    #[test]
    fn expectation_mode_expected_error_roundtrips() -> Result<()> {
        let raw = minimal_sidecar_toml("some.concept", "expected_error");
        let sidecar: FixtureExpectationSidecar = toml::from_str(&raw)?;
        assert_eq!(sidecar.expect.mode, ExpectationMode::ExpectedError);
        Ok(())
    }

    #[test]
    fn expectation_mode_token_only_roundtrips() -> Result<()> {
        let raw = minimal_sidecar_toml("some.concept", "token_only");
        let sidecar: FixtureExpectationSidecar = toml::from_str(&raw)?;
        assert_eq!(sidecar.expect.mode, ExpectationMode::TokenOnly);
        Ok(())
    }

    #[test]
    fn expectation_mode_span_only_roundtrips() -> Result<()> {
        let raw = minimal_sidecar_toml("some.concept", "span_only");
        let sidecar: FixtureExpectationSidecar = toml::from_str(&raw)?;
        assert_eq!(sidecar.expect.mode, ExpectationMode::SpanOnly);
        Ok(())
    }

    // SidecarValidation::is_ok

    #[test]
    fn sidecar_validation_is_ok_when_no_errors() {
        let v = SidecarValidation::default();
        assert!(v.is_ok());
    }

    #[test]
    fn sidecar_validation_is_not_ok_when_errors_present() {
        let mut v = SidecarValidation::default();
        v.errors.push("something went wrong".to_string());
        assert!(!v.is_ok());
    }

    #[test]
    fn sidecar_validation_ok_with_warnings_but_no_errors() {
        let mut v = SidecarValidation::default();
        v.warnings.push("just a warning".to_string());
        assert!(v.is_ok(), "warnings alone do not make validation fail");
    }

    // expected_fixture_path

    #[test]
    fn expected_fixture_path_rejects_empty_fixture_stem() {
        let result = expected_fixture_path(Path::new(".meta.toml"));
        assert!(result.is_err(), "empty fixture stem should be rejected");
        let error = result.err().map(|err| err.to_string()).unwrap_or_default();
        assert!(error.contains("fixture stem must not be empty"));
    }

    #[test]
    fn expected_fixture_path_resolves_valid_sidecar_name() {
        let result = expected_fixture_path(Path::new("quote_like/delimiter.meta.toml"));
        assert!(result.is_ok(), "valid sidecar name should resolve to fixture path");
        let path = result.ok().unwrap_or_default();
        assert_eq!(path, Path::new("quote_like/delimiter.pl"));
    }

    #[test]
    fn expected_fixture_path_rejects_wrong_extension() {
        let result = expected_fixture_path(Path::new("foo/bar.toml"));
        assert!(result.is_err(), "wrong extension should be rejected");
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("must end with .meta.toml"));
    }

    #[test]
    fn expected_fixture_path_root_level_sidecar() -> Result<()> {
        let result = expected_fixture_path(Path::new("basic.meta.toml"))?;
        assert_eq!(result, Path::new("basic.pl"));
        Ok(())
    }

    #[test]
    fn expected_fixture_path_deep_nested() -> Result<()> {
        let result = expected_fixture_path(Path::new("a/b/c/foo.meta.toml"))?;
        assert_eq!(result, Path::new("a/b/c/foo.pl"));
        Ok(())
    }

    // parse_sidecar

    #[test]
    fn parse_sidecar_reads_valid_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let sidecar_path = write_temp_file(
            dir.path(),
            "foo.meta.toml",
            &minimal_sidecar_toml("my.concept", "parse_clean"),
        )?;
        let sidecar = parse_sidecar(&sidecar_path)?;
        assert_eq!(sidecar.concept.id, "my.concept");
        assert_eq!(sidecar.concept.tier, "pr");
        assert!(!sidecar.expect.panic);
        assert!(!sidecar.expect.timeout);
        assert_eq!(sidecar.expect.mode, ExpectationMode::ParseClean);
        Ok(())
    }

    #[test]
    fn parse_sidecar_fails_on_missing_file() {
        let result = parse_sidecar(Path::new("/nonexistent/path/test.meta.toml"));
        assert!(result.is_err());
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("reading sidecar"));
    }

    #[test]
    fn parse_sidecar_fails_on_invalid_toml() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = write_temp_file(dir.path(), "bad.meta.toml", "not valid toml }{{{")?;
        let result = parse_sidecar(&path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn parse_sidecar_fails_on_wrong_schema() -> Result<()> {
        let dir = tempfile::tempdir()?;
        // Valid TOML but missing required fields
        let path = write_temp_file(dir.path(), "schema.meta.toml", "[concept]\nid = \"x\"\n")?;
        let result = parse_sidecar(&path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn parse_sidecar_handles_optional_metrics_and_snapshots() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let full_toml = r#"
[concept]
id = "full.case"
tier = "tier1"

[expect]
panic = false
timeout = true
mode = "parse_clean"

[metrics]
max_error_nodes = 5
must_emit_node_kinds = ["foo", "bar"]

[snapshots]
tokens = true
ast = false
spans = true
"#;
        let path = write_temp_file(dir.path(), "full.meta.toml", full_toml)?;
        let sidecar = parse_sidecar(&path)?;
        assert!(sidecar.expect.timeout);
        let metrics = sidecar.metrics.as_ref().ok_or_else(|| anyhow::anyhow!("missing metrics"))?;
        assert_eq!(metrics.max_error_nodes, Some(5));
        let kinds = metrics
            .must_emit_node_kinds
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing node kinds"))?;
        assert_eq!(kinds, &["foo", "bar"]);
        let snaps =
            sidecar.snapshots.as_ref().ok_or_else(|| anyhow::anyhow!("missing snapshots"))?;
        assert!(snaps.tokens);
        assert!(!snaps.ast);
        assert!(snaps.spans);
        Ok(())
    }

    // validate_sidecar

    fn make_sidecar(id: &str) -> FixtureExpectationSidecar {
        FixtureExpectationSidecar {
            concept: SidecarConcept { id: id.to_string(), tier: "pr".to_string() },
            expect: SidecarExpect {
                panic: false,
                timeout: false,
                mode: ExpectationMode::ParseClean,
            },
            metrics: None,
            snapshots: None,
        }
    }

    #[test]
    fn validate_sidecar_empty_concept_id_produces_error() -> Result<()> {
        let dir = tempfile::tempdir()?;
        // create the .pl fixture so the path check passes
        write_temp_file(dir.path(), "empty.pl", "# perl")?;
        let sidecar_path = dir.path().join("empty.meta.toml");
        let sidecar = make_sidecar("   "); // whitespace-only id
        let v = validate_sidecar(&sidecar_path, &sidecar, None);
        assert!(!v.is_ok());
        assert!(v.errors.iter().any(|e| e.contains("concept.id must not be empty")));
        Ok(())
    }

    #[test]
    fn validate_sidecar_missing_fixture_produces_error() -> Result<()> {
        let dir = tempfile::tempdir()?;
        // do NOT create the .pl file
        let sidecar_path = dir.path().join("missing.meta.toml");
        let sidecar = make_sidecar("some.id");
        let v = validate_sidecar(&sidecar_path, &sidecar, None);
        assert!(!v.is_ok());
        assert!(v.errors.iter().any(|e| e.contains("fixture file does not exist")));
        Ok(())
    }

    #[test]
    fn validate_sidecar_no_registry_emits_warning() -> Result<()> {
        let dir = tempfile::tempdir()?;
        write_temp_file(dir.path(), "warn.pl", "# perl")?;
        let sidecar_path = dir.path().join("warn.meta.toml");
        let sidecar = make_sidecar("concept.pending");
        let v = validate_sidecar(&sidecar_path, &sidecar, None);
        assert!(v.is_ok(), "should have no errors");
        assert!(v.warnings.iter().any(|w| w.contains("concept registry unavailable")));
        assert!(v.warnings.iter().any(|w| w.contains("concept.pending")));
        Ok(())
    }

    #[test]
    fn validate_sidecar_known_concept_is_clean() -> Result<()> {
        let dir = tempfile::tempdir()?;
        write_temp_file(dir.path(), "ok.pl", "# perl")?;
        let sidecar_path = dir.path().join("ok.meta.toml");
        let sidecar = make_sidecar("known.concept");

        // Build a registry with the concept present
        let registry_toml = r#"
[[concepts]]
id = "known.concept"
"#;
        let reg_path = write_temp_file(dir.path(), "concepts.toml", registry_toml)?;
        let registry = ConceptRegistry::load(&reg_path)?;

        let v = validate_sidecar(&sidecar_path, &sidecar, Some(&registry));
        assert!(v.is_ok(), "known concept with fixture should be clean: {:?}", v.errors);
        assert!(v.warnings.is_empty());
        Ok(())
    }

    #[test]
    fn validate_sidecar_unknown_concept_in_registry_produces_error() -> Result<()> {
        let dir = tempfile::tempdir()?;
        write_temp_file(dir.path(), "unknown.pl", "# perl")?;
        let sidecar_path = dir.path().join("unknown.meta.toml");
        let sidecar = make_sidecar("not.in.registry");

        let registry_toml = r#"
[[concepts]]
id = "known.concept"
"#;
        let reg_path = write_temp_file(dir.path(), "concepts.toml", registry_toml)?;
        let registry = ConceptRegistry::load(&reg_path)?;

        let v = validate_sidecar(&sidecar_path, &sidecar, Some(&registry));
        assert!(!v.is_ok());
        assert!(v.errors.iter().any(|e| e.contains("not.in.registry")));
        assert!(v.errors.iter().any(|e| e.contains("not present in the loaded concept registry")));
        Ok(())
    }

    #[test]
    fn validate_sidecar_invalid_sidecar_path_extension_produces_error() -> Result<()> {
        let dir = tempfile::tempdir()?;
        // path has wrong extension, so expected_fixture_path will fail
        let sidecar_path = dir.path().join("foo.toml"); // not .meta.toml
        let sidecar = make_sidecar("some.id");
        let v = validate_sidecar(&sidecar_path, &sidecar, None);
        assert!(!v.is_ok());
        assert!(v.errors.iter().any(|e| e.contains("must end with .meta.toml")));
        Ok(())
    }

    // load_and_validate_sidecar

    #[test]
    fn load_and_validate_sidecar_succeeds_for_valid_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        write_temp_file(dir.path(), "good.pl", "# perl")?;
        let sidecar_path = write_temp_file(
            dir.path(),
            "good.meta.toml",
            &minimal_sidecar_toml("good.concept", "parse_clean"),
        )?;
        let v = load_and_validate_sidecar(&sidecar_path, None)?;
        // Fixture exists, concept id is not empty, and no registry yields a warning.
        assert!(v.is_ok());
        Ok(())
    }

    #[test]
    fn load_and_validate_sidecar_fails_on_missing_file() {
        let result = load_and_validate_sidecar(Path::new("/nonexistent/ghost.meta.toml"), None);
        assert!(result.is_err());
    }

    // discover_sidecars

    #[test]
    fn discover_sidecars_returns_empty_for_nonexistent_root() -> Result<()> {
        let sidecars = discover_sidecars(Path::new("/this/does/not/exist"))?;
        assert!(sidecars.is_empty());
        Ok(())
    }

    #[test]
    fn discover_sidecars_finds_meta_toml_files() -> Result<()> {
        let dir = tempfile::tempdir()?;
        write_temp_file(dir.path(), "a.meta.toml", "# meta")?;
        write_temp_file(dir.path(), "b.meta.toml", "# meta")?;
        write_temp_file(dir.path(), "not_a_sidecar.txt", "ignored")?;
        write_temp_file(dir.path(), "also_ignored.toml", "ignored")?;

        let sidecars = discover_sidecars(dir.path())?;
        assert_eq!(sidecars.len(), 2);
        assert!(sidecars.iter().any(|p| p.ends_with("a.meta.toml")));
        assert!(sidecars.iter().any(|p| p.ends_with("b.meta.toml")));
        Ok(())
    }

    #[test]
    fn discover_sidecars_recurses_into_subdirectories() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let subdir = dir.path().join("sub");
        fs::create_dir(&subdir)?;
        write_temp_file(dir.path(), "root.meta.toml", "# meta")?;
        write_temp_file(&subdir, "nested.meta.toml", "# meta")?;

        let sidecars = discover_sidecars(dir.path())?;
        assert_eq!(sidecars.len(), 2);
        Ok(())
    }

    #[test]
    fn discover_sidecars_returns_sorted_paths() -> Result<()> {
        let dir = tempfile::tempdir()?;
        write_temp_file(dir.path(), "zzz.meta.toml", "# meta")?;
        write_temp_file(dir.path(), "aaa.meta.toml", "# meta")?;
        write_temp_file(dir.path(), "mmm.meta.toml", "# meta")?;

        let sidecars = discover_sidecars(dir.path())?;
        assert_eq!(sidecars.len(), 3);
        // Verify sorted
        let names: Vec<_> =
            sidecars.iter().filter_map(|p| p.file_name().and_then(|n| n.to_str())).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "sidecars should be returned in sorted order");
        Ok(())
    }

    #[test]
    fn discover_sidecars_ignores_non_meta_toml_files() -> Result<()> {
        let dir = tempfile::tempdir()?;
        write_temp_file(dir.path(), "only_a.meta.toml", "# meta")?;
        write_temp_file(dir.path(), "ignore.json", "{}")?;
        write_temp_file(dir.path(), "ignore.toml", "[t]")?;
        write_temp_file(dir.path(), "ignore.pl", "1;")?;

        let sidecars = discover_sidecars(dir.path())?;
        assert_eq!(sidecars.len(), 1);
        Ok(())
    }

    // ConceptRegistry

    #[test]
    fn concept_registry_load_fails_on_missing_file() {
        let result = ConceptRegistry::load(Path::new("/no/such/file.toml"));
        assert!(result.is_err());
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("reading concept registry"));
    }

    #[test]
    fn concept_registry_load_fails_on_invalid_toml() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = write_temp_file(dir.path(), "bad.toml", "{{not valid")?;
        let result = ConceptRegistry::load(&path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn concept_registry_contains_returns_true_for_known_id() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let toml = r#"
[[concepts]]
id = "parser.basic"

[[concepts]]
id = "regex.quantifier"
"#;
        let path = write_temp_file(dir.path(), "concepts.toml", toml)?;
        let registry = ConceptRegistry::load(&path)?;
        assert!(registry.contains("parser.basic"));
        assert!(registry.contains("regex.quantifier"));
        Ok(())
    }

    #[test]
    fn concept_registry_contains_returns_false_for_unknown_id() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let toml = r#"
[[concepts]]
id = "parser.basic"
"#;
        let path = write_temp_file(dir.path(), "concepts.toml", toml)?;
        let registry = ConceptRegistry::load(&path)?;
        assert!(!registry.contains("not.there"));
        Ok(())
    }

    #[test]
    fn concept_registry_ignores_empty_ids() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let toml = r#"
[[concepts]]
id = ""

[[concepts]]
id = "   "

[[concepts]]
id = "real.concept"
"#;
        let path = write_temp_file(dir.path(), "concepts.toml", toml)?;
        let registry = ConceptRegistry::load(&path)?;
        assert!(!registry.contains(""));
        assert!(!registry.contains("   "));
        assert!(registry.contains("real.concept"));
        Ok(())
    }

    #[test]
    fn concept_registry_handles_nested_toml_tables() -> Result<()> {
        let dir = tempfile::tempdir()?;
        // Nested table structure should be traversed by collect_concept_ids.
        let toml = r#"
[outer]
id = "outer.concept"

[outer.inner]
id = "inner.concept"
"#;
        let path = write_temp_file(dir.path(), "nested.toml", toml)?;
        let registry = ConceptRegistry::load(&path)?;
        assert!(registry.contains("outer.concept"));
        assert!(registry.contains("inner.concept"));
        Ok(())
    }

    #[test]
    fn concept_registry_handles_array_of_tables() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let toml = r#"
concepts = [
    { id = "array.one" },
    { id = "array.two" },
]
"#;
        let path = write_temp_file(dir.path(), "array.toml", toml)?;
        let registry = ConceptRegistry::load(&path)?;
        assert!(registry.contains("array.one"));
        assert!(registry.contains("array.two"));
        Ok(())
    }

    // SidecarExpect/SidecarConcept equality

    #[test]
    fn sidecar_types_support_equality() {
        let c1 = SidecarConcept { id: "x".to_string(), tier: "t".to_string() };
        let c2 = SidecarConcept { id: "x".to_string(), tier: "t".to_string() };
        let c3 = SidecarConcept { id: "y".to_string(), tier: "t".to_string() };
        assert_eq!(c1, c2);
        assert_ne!(c1, c3);

        let e1 = SidecarExpect { panic: true, timeout: false, mode: ExpectationMode::ParseClean };
        let e2 = SidecarExpect { panic: true, timeout: false, mode: ExpectationMode::ParseClean };
        let e3 = SidecarExpect { panic: false, timeout: false, mode: ExpectationMode::ParseClean };
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn sidecar_metrics_supports_equality() {
        let m1 = SidecarMetrics {
            max_error_nodes: Some(3),
            must_emit_node_kinds: Some(vec!["a".to_string()]),
        };
        let m2 = SidecarMetrics {
            max_error_nodes: Some(3),
            must_emit_node_kinds: Some(vec!["a".to_string()]),
        };
        assert_eq!(m1, m2);
    }

    #[test]
    fn sidecar_snapshots_supports_equality() {
        let s1 = SidecarSnapshots { tokens: true, ast: false, spans: true };
        let s2 = SidecarSnapshots { tokens: true, ast: false, spans: true };
        let s3 = SidecarSnapshots { tokens: false, ast: false, spans: true };
        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }
}
