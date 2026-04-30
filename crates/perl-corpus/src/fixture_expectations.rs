use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixtureExpectation {
    pub concept: Option<ConceptInfo>,
    pub expect: ExpectBlock,
    pub metrics: Option<MetricsBlock>,
    pub snapshots: Option<SnapshotBlock>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptInfo {
    pub id: String,
    pub tier: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectBlock {
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct MetricsBlock {
    pub max_error_nodes: Option<u32>,
    pub must_emit_node_kinds: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct SnapshotBlock {
    pub tokens: Option<bool>,
    pub ast: Option<bool>,
    pub spans: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarValidation {
    pub sidecar_path: PathBuf,
    pub fixture_path: PathBuf,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SidecarValidation {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn parse_sidecar(path: &Path) -> Result<FixtureExpectation> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read sidecar {}", path.display()))?;

    toml::from_str(&contents)
        .with_context(|| format!("failed to parse sidecar TOML {}", path.display()))
}

fn fixture_path_for_sidecar(path: &Path) -> PathBuf {
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();

    if let Some(base_name) = file_name.strip_suffix(".meta.toml") {
        return path.with_file_name(format!("{base_name}.pl"));
    }

    path.with_extension("pl")
}

pub fn discover_sidecars(root: &Path) -> Result<Vec<PathBuf>> {
    let pattern = root.join("**/*.meta.toml");
    let pattern = pattern.to_string_lossy().into_owned();

    let mut sidecars = Vec::new();
    for entry in glob::glob(&pattern).with_context(|| format!("invalid glob pattern: {pattern}"))? {
        let path =
            entry.with_context(|| format!("failed to read sidecar path from glob {pattern}"))?;
        sidecars.push(path);
    }

    sidecars.sort();
    Ok(sidecars)
}

pub fn validate_sidecar(
    path: &Path,
    concept_registry: Option<&HashSet<String>>,
) -> SidecarValidation {
    let fixture_path = fixture_path_for_sidecar(path);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    match parse_sidecar(path) {
        Ok(sidecar) => {
            if !fixture_path.exists() {
                errors.push(format!("fixture missing for sidecar: {}", fixture_path.display()));
            }

            if let Some(concept) = sidecar.concept {
                if let Some(registry) = concept_registry {
                    if !registry.contains(&concept.id) {
                        errors.push(format!("concept id not found in registry: {}", concept.id));
                    }
                } else {
                    warnings.push(format!(
                        "concept registry unavailable; resolution pending for {}",
                        concept.id
                    ));
                }
            }
        }
        Err(error) => {
            errors.push(error.to_string());
        }
    }

    SidecarValidation { sidecar_path: path.to_path_buf(), fixture_path, errors, warnings }
}

pub fn validate_sidecars_in_dir(
    root: &Path,
    concept_registry: Option<&HashSet<String>>,
) -> Result<Vec<SidecarValidation>> {
    let sidecars = discover_sidecars(root)?;
    Ok(sidecars.iter().map(|sidecar| validate_sidecar(sidecar, concept_registry)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> Result<PathBuf> {
        let mut path = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        path.push(format!("{}_{}_{}", prefix, pid, nanos));
        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create temp dir {}", path.display()))?;
        Ok(path)
    }

    fn write_fixture_pair(root: &Path, area: &str, name: &str, meta_toml: &str) -> Result<PathBuf> {
        let area_dir = root.join(area);
        fs::create_dir_all(&area_dir)
            .with_context(|| format!("failed to create area dir {}", area_dir.display()))?;

        let fixture = area_dir.join(format!("{name}.pl"));
        fs::write(&fixture, "my $x = 1;\n")
            .with_context(|| format!("failed to write fixture {}", fixture.display()))?;

        let sidecar = area_dir.join(format!("{name}.meta.toml"));
        fs::write(&sidecar, meta_toml)
            .with_context(|| format!("failed to write sidecar {}", sidecar.display()))?;

        Ok(sidecar)
    }

    #[test]
    fn parses_known_expectation_mode() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("perl_corpus_sidecar_parse")?;
        let sidecar = write_fixture_pair(
            &root,
            "recovery",
            "missing_brace",
            r#"
[concept]
id = "parser.recovery.missing_closing_brace"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "recover_without_panic"

[snapshots]
ast = true
spans = true
"#,
        )?;

        let parsed = parse_sidecar(&sidecar)?;
        assert_eq!(parsed.expect.mode, ExpectationMode::RecoverWithoutPanic);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn rejects_unknown_expectation_mode() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("perl_corpus_sidecar_mode")?;
        let sidecar = write_fixture_pair(
            &root,
            "recovery",
            "unknown_mode",
            r#"
[expect]
panic = false
timeout = false
mode = "totally_unknown"
"#,
        )?;

        let validation = validate_sidecar(&sidecar, None);
        assert!(!validation.is_valid());
        assert!(validation.errors.iter().any(|error| error.contains("mode")));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn reports_missing_fixture_file() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("perl_corpus_sidecar_fixture")?;
        let sidecar_path = root.join("quote_like").join("delimiter.meta.toml");
        let parent =
            sidecar_path.parent().ok_or_else(|| anyhow::anyhow!("sidecar path had no parent"))?;
        fs::create_dir_all(parent)?;
        fs::write(
            &sidecar_path,
            r#"
[expect]
panic = false
timeout = false
mode = "parse_clean"
"#,
        )?;

        let validation = validate_sidecar(&sidecar_path, None);
        assert!(!validation.is_valid());
        assert!(validation.errors.iter().any(|error| error.contains("fixture missing")));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn does_not_hard_fail_when_registry_is_unavailable() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("perl_corpus_sidecar_pending")?;
        let sidecar = write_fixture_pair(
            &root,
            "ambiguity",
            "regex_vs_division",
            r#"
[concept]
id = "parser.ambiguity.regex_vs_division"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "parse_clean"
"#,
        )?;

        let validation = validate_sidecar(&sidecar, None);
        assert!(validation.is_valid());
        assert!(validation.warnings.iter().any(|warning| warning.contains("resolution pending")));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn fails_when_registry_is_present_and_id_is_unknown() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("perl_corpus_sidecar_registry")?;
        let sidecar = write_fixture_pair(
            &root,
            "heredoc",
            "terminator",
            r#"
[concept]
id = "parser.heredoc.terminator"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "expected_error"
"#,
        )?;

        let registry = HashSet::from(["parser.other.known".to_string()]);
        let validation = validate_sidecar(&sidecar, Some(&registry));
        assert!(!validation.is_valid());
        assert!(validation.errors.iter().any(|error| error.contains("concept id not found")));

        fs::remove_dir_all(root)?;
        Ok(())
    }
}
