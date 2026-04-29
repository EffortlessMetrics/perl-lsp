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

    let fixture_name = file_name.trim_end_matches(".meta.toml").to_string() + ".pl";
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
}
