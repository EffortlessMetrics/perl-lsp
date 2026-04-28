use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const META_TOML_SUFFIX: &str = ".meta.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FixtureExpectation {
    pub concept: ConceptSpec,
    pub expect: ExpectSpec,
    #[serde(default)]
    pub metrics: MetricsSpec,
    #[serde(default)]
    pub snapshots: SnapshotSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConceptSpec {
    pub id: String,
    pub tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpectSpec {
    pub panic: bool,
    pub timeout: bool,
    pub mode: ExpectationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectationMode {
    ParseClean,
    RecoverWithoutPanic,
    ExpectedError,
    TokenOnly,
    SpanOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MetricsSpec {
    pub max_error_nodes: Option<u32>,
    #[serde(default)]
    pub must_emit_node_kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SnapshotSpec {
    #[serde(default)]
    pub tokens: bool,
    #[serde(default)]
    pub ast: bool,
    #[serde(default)]
    pub spans: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarFixture {
    pub sidecar_path: PathBuf,
    pub fixture_path: PathBuf,
    pub expectation: FixtureExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptRegistry {
    ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationNote {
    ConceptResolutionPending { concept_id: String },
}

impl ConceptRegistry {
    pub fn from_ids<I>(ids: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        Self { ids: ids.into_iter().collect() }
    }

    pub fn load_if_present(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading concept registry {}", path.display()))?;
        let parsed: ConceptRegistryFile = toml::from_str(&raw)
            .with_context(|| format!("parsing concept registry {}", path.display()))?;

        let ids: Vec<String> =
            parsed.concept.into_iter().map(|item| item.id).chain(parsed.concepts).collect();

        Ok(Some(Self::from_ids(ids)))
    }

    pub fn contains(&self, concept_id: &str) -> bool {
        self.ids.contains(concept_id)
    }
}

#[derive(Debug, Deserialize)]
struct ConceptRegistryFile {
    #[serde(default)]
    concept: Vec<ConceptRegistryItem>,
    #[serde(default)]
    concepts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ConceptRegistryItem {
    id: String,
}

pub fn parse_sidecar_file(path: &Path) -> Result<FixtureExpectation> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading sidecar {}", path.display()))?;
    let parsed =
        toml::from_str(&raw).with_context(|| format!("parsing sidecar {}", path.display()))?;
    Ok(parsed)
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
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            let path = entry.path();

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(META_TOML_SUFFIX))
            {
                sidecars.push(path);
            }
        }
    }

    sidecars.sort();
    sidecars.dedup();
    sidecars
}

pub fn validate_sidecar(
    sidecar_path: &Path,
    concept_registry: Option<&ConceptRegistry>,
) -> Result<(SidecarFixture, Vec<ValidationNote>)> {
    let expectation = parse_sidecar_file(sidecar_path)?;

    let fixture_path = fixture_path_for_sidecar(sidecar_path)?;
    if !fixture_path.exists() {
        bail!("fixture {} missing for sidecar {}", fixture_path.display(), sidecar_path.display());
    }

    let mut notes = Vec::new();
    if let Some(registry) = concept_registry {
        if !registry.contains(&expectation.concept.id) {
            bail!(
                "concept id '{}' from {} not present in concept registry",
                expectation.concept.id,
                sidecar_path.display()
            );
        }
    } else {
        notes.push(ValidationNote::ConceptResolutionPending {
            concept_id: expectation.concept.id.clone(),
        });
    }

    Ok((
        SidecarFixture { sidecar_path: sidecar_path.to_path_buf(), fixture_path, expectation },
        notes,
    ))
}

pub fn validate_sidecars_under(
    sidecar_root: &Path,
    concept_registry: Option<&ConceptRegistry>,
) -> Result<(Vec<SidecarFixture>, Vec<ValidationNote>)> {
    let sidecar_paths = discover_sidecars(sidecar_root);
    if sidecar_paths.is_empty() {
        return Err(anyhow!("no sidecars found under {}", sidecar_root.display()));
    }

    let mut fixtures = Vec::new();
    let mut notes = Vec::new();
    for sidecar_path in sidecar_paths {
        let (fixture, mut sidecar_notes) = validate_sidecar(&sidecar_path, concept_registry)?;
        fixtures.push(fixture);
        notes.append(&mut sidecar_notes);
    }

    Ok((fixtures, notes))
}

fn fixture_path_for_sidecar(sidecar_path: &Path) -> Result<PathBuf> {
    let Some(file_name) = sidecar_path.file_name().and_then(|name| name.to_str()) else {
        bail!("sidecar {} missing filename", sidecar_path.display());
    };

    let Some(fixture_name) = file_name.strip_suffix(META_TOML_SUFFIX) else {
        bail!("sidecar {} does not end with {}", sidecar_path.display(), META_TOML_SUFFIX);
    };

    Ok(sidecar_path.with_file_name(format!("{fixture_name}.pl")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> std::io::Result<PathBuf> {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        root.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn write_sidecar(path: &Path, mode: &str, concept_id: &str) -> Result<()> {
        fs::write(
            path,
            format!(
                r#"[concept]
id = "{concept_id}"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "{mode}"

[snapshots]
tokens = false
ast = true
spans = true
"#
            ),
        )?;
        Ok(())
    }

    #[test]
    fn validate_sidecar_accepts_known_mode_and_fixture_pair() -> Result<()> {
        let root = temp_root("perl_corpus_sidecar")?;
        let sidecar_path = root.join("missing_brace.meta.toml");
        let fixture_path = root.join("missing_brace.pl");
        fs::write(&fixture_path, "sub broken {\n")?;
        write_sidecar(&sidecar_path, "recover_without_panic", "parser.recovery.missing_brace")?;

        let registry = ConceptRegistry::from_ids(["parser.recovery.missing_brace".to_string()]);
        let (_, notes) = validate_sidecar(&sidecar_path, Some(&registry))?;
        assert!(notes.is_empty());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn validate_sidecar_reports_pending_concepts_without_registry() -> Result<()> {
        let root = temp_root("perl_corpus_sidecar_pending")?;
        let sidecar_path = root.join("regex.meta.toml");
        let fixture_path = root.join("regex.pl");
        fs::write(&fixture_path, "my $ratio = 6 / 3;\n")?;
        write_sidecar(&sidecar_path, "parse_clean", "parser.ambiguity.regex_vs_division")?;

        let (_, notes) = validate_sidecar(&sidecar_path, None)?;
        assert_eq!(notes.len(), 1);
        assert!(matches!(notes[0], ValidationNote::ConceptResolutionPending { .. }));

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn validate_sidecar_rejects_unknown_mode() -> Result<()> {
        let root = temp_root("perl_corpus_sidecar_invalid_mode")?;
        let sidecar_path = root.join("invalid.meta.toml");
        let fixture_path = root.join("invalid.pl");
        fs::write(&fixture_path, "print \"ok\\n\";\n")?;
        write_sidecar(&sidecar_path, "unknown_mode", "parser.test.invalid_mode")?;

        let err =
            validate_sidecar(&sidecar_path, None).expect_err("unknown mode must fail parsing");
        assert!(err.to_string().contains("parsing sidecar"));

        fs::remove_dir_all(&root)?;
        Ok(())
    }
}
