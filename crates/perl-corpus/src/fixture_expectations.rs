use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const SIDECAR_SUFFIX: &str = ".meta.toml";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FixtureExpectation {
    pub concept: Concept,
    pub expect: Expect,
    pub metrics: Metrics,
    pub snapshots: Snapshots,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Concept {
    pub id: String,
    pub tier: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Expect {
    pub panic: bool,
    pub timeout: bool,
    pub mode: ExpectationMode,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectationMode {
    ParseClean,
    RecoverWithoutPanic,
    ExpectedError,
    TokenOnly,
    SpanOnly,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Metrics {
    pub max_error_nodes: usize,
    pub must_emit_node_kinds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Snapshots {
    pub tokens: bool,
    pub ast: bool,
    pub spans: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    MissingFixtureFile(PathBuf),
    UnknownConceptId(String),
    PendingConceptRegistry(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConceptRegistry {
    ids: HashSet<String>,
}

impl ConceptRegistry {
    pub fn from_ids(ids: impl IntoIterator<Item = String>) -> Self {
        Self { ids: ids.into_iter().collect() }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    pub fn from_optional_file(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed reading concept registry {}", path.display()))?;
        let value: toml::Value =
            toml::from_str(&contents).context("failed parsing concept registry TOML")?;

        let mut ids = HashSet::new();
        collect_string_ids_from_value(&value, &mut ids);

        Ok(Some(Self { ids }))
    }
}

pub fn parse_sidecar(path: &Path) -> Result<FixtureExpectation> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed reading sidecar {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("failed parsing sidecar {}", path.display()))
}

pub fn validate_sidecar(
    sidecar_path: &Path,
    expectation: &FixtureExpectation,
    registry: Option<&ConceptRegistry>,
) -> ValidationReport {
    let mut report = ValidationReport::default();
    let fixture_path = fixture_path_for_sidecar(sidecar_path);

    if !fixture_path.exists() {
        report.issues.push(ValidationIssue::MissingFixtureFile(fixture_path));
    }

    if let Some(registry) = registry {
        if !registry.contains(&expectation.concept.id) {
            report.issues.push(ValidationIssue::UnknownConceptId(expectation.concept.id.clone()));
        }
    } else {
        report.issues.push(ValidationIssue::PendingConceptRegistry(expectation.concept.id.clone()));
    }

    report
}

pub fn discover_sidecars(root: &Path) -> Vec<PathBuf> {
    let mut sidecars = Vec::new();
    if !root.exists() {
        return sidecars;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file() {
                let Some(name) = path.file_name().and_then(|part| part.to_str()) else {
                    continue;
                };
                if name.ends_with(SIDECAR_SUFFIX) {
                    sidecars.push(path);
                }
            }
        }
    }

    sidecars.sort();
    sidecars
}

fn fixture_path_for_sidecar(sidecar_path: &Path) -> PathBuf {
    let file_name = sidecar_path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map_or_else(String::new, ToString::to_string);

    let fixture_name = file_name
        .strip_suffix(SIDECAR_SUFFIX)
        .map_or_else(|| format!("{file_name}.pl"), |base| format!("{base}.pl"));

    sidecar_path.with_file_name(fixture_name)
}

fn collect_string_ids_from_value(value: &toml::Value, ids: &mut HashSet<String>) {
    match value {
        toml::Value::String(id) => {
            ids.insert(id.to_string());
        }
        toml::Value::Array(entries) => {
            for entry in entries {
                collect_string_ids_from_value(entry, ids);
            }
        }
        toml::Value::Table(entries) => {
            for (key, value) in entries {
                if key.eq_ignore_ascii_case("id") {
                    if let toml::Value::String(id) = value {
                        ids.insert(id.to_string());
                    }
                }
                collect_string_ids_from_value(value, ids);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> Result<PathBuf> {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system time before unix epoch")?
            .as_nanos();
        root.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        fs::create_dir_all(&root)
            .with_context(|| format!("failed to create temp directory {}", root.display()))?;
        Ok(root)
    }

    #[test]
    fn parse_sidecar_accepts_known_mode() -> Result<()> {
        let sidecar = r#"
[concept]
id = "parser.recovery.missing_closing_brace"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "recover_without_panic"

[metrics]
max_error_nodes = 2
must_emit_node_kinds = ["SubDecl", "Block"]

[snapshots]
tokens = false
ast = true
spans = true
"#;

        let parsed: FixtureExpectation = toml::from_str(sidecar)?;
        assert_eq!(parsed.expect.mode, ExpectationMode::RecoverWithoutPanic);
        Ok(())
    }

    #[test]
    fn parse_sidecar_rejects_unknown_mode() {
        let sidecar = r#"
[concept]
id = "parser.invalid"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "does_not_exist"

[metrics]
max_error_nodes = 0
must_emit_node_kinds = []

[snapshots]
tokens = false
ast = true
spans = true
"#;

        let parsed = toml::from_str::<FixtureExpectation>(sidecar);
        assert!(parsed.is_err());
    }

    #[test]
    fn validate_sidecar_reports_missing_fixture() -> Result<()> {
        let root = temp_root("perl_corpus_sidecar_validate")?;
        let sidecar_path = root.join("missing_fixture.meta.toml");
        let sidecar = r#"
[concept]
id = "parser.recovery.missing_delimiter"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "expected_error"

[metrics]
max_error_nodes = 1
must_emit_node_kinds = ["Expr"]

[snapshots]
tokens = true
ast = true
spans = false
"#;

        fs::write(&sidecar_path, sidecar)?;

        let parsed = parse_sidecar(&sidecar_path)?;
        let report = validate_sidecar(&sidecar_path, &parsed, None);

        assert!(
            report
                .issues
                .iter()
                .any(|issue| matches!(issue, ValidationIssue::MissingFixtureFile(_)))
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| matches!(issue, ValidationIssue::PendingConceptRegistry(_)))
        );

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn validate_sidecar_accepts_known_concept_registry() -> Result<()> {
        let root = temp_root("perl_corpus_sidecar_registry")?;
        let sidecar_path = root.join("known.meta.toml");
        let fixture_path = root.join("known.pl");
        fs::write(&fixture_path, "my $value = 1;\n")?;
        fs::write(
            &sidecar_path,
            r#"
[concept]
id = "parser.ambiguity.regex_vs_division"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "parse_clean"

[metrics]
max_error_nodes = 0
must_emit_node_kinds = ["Expr"]

[snapshots]
tokens = true
ast = true
spans = true
"#,
        )?;

        let parsed = parse_sidecar(&sidecar_path)?;
        let registry =
            ConceptRegistry::from_ids([String::from("parser.ambiguity.regex_vs_division")]);
        let report = validate_sidecar(&sidecar_path, &parsed, Some(&registry));

        assert!(report.is_ok());

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn discover_sidecars_finds_meta_toml_files() -> Result<()> {
        let root = temp_root("perl_corpus_sidecar_discover")?;
        fs::create_dir_all(root.join("recovery"))?;
        fs::write(root.join("recovery/case.meta.toml"), "")?;
        fs::write(root.join("recovery/case.pl"), "")?;
        fs::write(root.join("recovery/other.toml"), "")?;

        let sidecars = discover_sidecars(&root);
        assert_eq!(sidecars.len(), 1);
        assert!(sidecars[0].ends_with("case.meta.toml"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn concept_registry_can_parse_common_id_shapes() -> Result<()> {
        let root = temp_root("perl_corpus_sidecar_concepts")?;
        let registry_path = root.join("concepts.toml");
        fs::write(
            &registry_path,
            r#"
ids = ["parser.quote_like.delimiter", "parser.heredoc.basics"]

[[concepts]]
id = "parser.recovery.missing_closing_brace"
"#,
        )?;

        let registry = ConceptRegistry::from_optional_file(&registry_path)?;
        let registry = registry.context("concept registry should exist")?;
        assert!(registry.contains("parser.quote_like.delimiter"));
        assert!(registry.contains("parser.heredoc.basics"));
        assert!(registry.contains("parser.recovery.missing_closing_brace"));

        fs::remove_dir_all(root)?;
        Ok(())
    }
}
