use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const CONCEPTS_DIR: &str = "concepts";
const ALLOWED_TOP_LEVEL_SECTIONS: &[&str] = &["concepts"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptRegistry {
    pub concepts: Vec<ConceptRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptRow {
    pub id: String,
    pub status: ConceptStatus,
    pub scope: ConceptScope,
    pub fixtures: ConceptFixtures,
    pub expect: ConceptExpect,
    pub snapshots: ConceptSnapshots,
    pub run: ConceptRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptStatus {
    Seeded,
    Active,
    Planned,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptScope {
    pub crates: Vec<String>,
    pub risk_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptFixtures {
    pub floors: Vec<String>,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptExpect {
    pub panic: bool,
    pub timeout: bool,
    pub mode: ConceptMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptMode {
    Parse,
    Recover,
    Compare,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptSnapshots {
    pub tokens: bool,
    pub ast: bool,
    pub spans: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptRun {
    pub pr: bool,
    pub nightly: bool,
    pub release: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConceptFile {
    concepts: Vec<ConceptRow>,
}

pub fn load_concept_registry() -> Result<ConceptRegistry> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .context("failed to derive workspace root")?;
    load_concept_registry_from_dir(&crate_root.join(CONCEPTS_DIR), &repo_root)
}

pub fn load_concept_registry_from_dir(
    concepts_dir: &Path,
    fixture_root: &Path,
) -> Result<ConceptRegistry> {
    let mut files = collect_toml_files(concepts_dir)?;
    files.sort();

    let mut concepts = Vec::new();
    for path in files {
        concepts.extend(load_concepts_from_file(&path)?);
    }

    validate_registry(&concepts, fixture_root)?;
    concepts.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ConceptRegistry { concepts })
}

fn collect_toml_files(concepts_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(concepts_dir).with_context(|| {
        format!("failed to read concepts directory: {}", concepts_dir.display())
    })?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            files.push(path);
        }
    }

    if files.is_empty() {
        bail!("no concept TOML files found in {}", concepts_dir.display());
    }

    Ok(files)
}

fn load_concepts_from_file(path: &Path) -> Result<Vec<ConceptRow>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read concept file {}", path.display()))?;
    let value: toml::Value = toml::from_str(&content)
        .with_context(|| format!("failed to parse TOML in {}", path.display()))?;

    validate_top_level_sections(path, &value)?;

    let file: ConceptFile = value
        .try_into()
        .with_context(|| format!("invalid concept registry shape in {}", path.display()))?;
    Ok(file.concepts)
}

fn validate_top_level_sections(path: &Path, value: &toml::Value) -> Result<()> {
    let Some(table) = value.as_table() else {
        bail!("concept file {} must contain a TOML table", path.display());
    };

    let allowed: HashSet<&str> = ALLOWED_TOP_LEVEL_SECTIONS.iter().copied().collect();
    for key in table.keys() {
        if !allowed.contains(key.as_str()) {
            bail!(
                "unknown top-level TOML section '{key}' in {} (allowed: concepts)",
                path.display()
            );
        }
    }

    Ok(())
}

fn validate_registry(concepts: &[ConceptRow], fixture_root: &Path) -> Result<()> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for (idx, concept) in concepts.iter().enumerate() {
        if let Some(existing_idx) = seen.insert(&concept.id, idx) {
            bail!("duplicate concept id '{}' at indexes {} and {}", concept.id, existing_idx, idx);
        }

        validate_fixture_paths(&concept.fixtures.floors, fixture_root, &concept.id, "floors")?;
        validate_fixture_paths(&concept.fixtures.variants, fixture_root, &concept.id, "variants")?;
    }

    Ok(())
}

fn validate_fixture_paths(
    paths: &[String],
    fixture_root: &Path,
    concept_id: &str,
    fixture_kind: &str,
) -> Result<()> {
    for relative in paths {
        let candidate = fixture_root.join(relative);
        if !candidate.exists() {
            bail!(
                "fixture path '{}' ({} for concept '{}') does not exist under {}",
                relative,
                fixture_kind,
                concept_id,
                fixture_root.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> Result<PathBuf> {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        root.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    #[test]
    fn rejects_duplicate_concept_ids() -> Result<()> {
        let root = temp_root("perl_corpus_concepts_dup")?;
        let concepts_dir = root.join("concepts");
        fs::create_dir_all(&concepts_dir)?;

        fs::write(
            concepts_dir.join("lexer.toml"),
            r#"
[[concepts]]
id = "lexer.regex.duplicate"
status = "seeded"
[concepts.scope]
crates = ["perl-lexer"]
risk_tags = ["lexing"]
[concepts.fixtures]
floors = []
variants = []
[concepts.expect]
panic = false
timeout = false
mode = "parse"
[concepts.snapshots]
tokens = true
ast = false
spans = false
[concepts.run]
pr = true
nightly = true
release = true

[[concepts]]
id = "lexer.regex.duplicate"
status = "seeded"
[concepts.scope]
crates = ["perl-lexer"]
risk_tags = ["lexing"]
[concepts.fixtures]
floors = []
variants = []
[concepts.expect]
panic = false
timeout = false
mode = "parse"
[concepts.snapshots]
tokens = true
ast = false
spans = false
[concepts.run]
pr = true
nightly = true
release = true
"#,
        )?;

        let err = load_concept_registry_from_dir(&concepts_dir, &root)
            .expect_err("must fail duplicate ids");
        assert!(err.to_string().contains("duplicate concept id"));

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn rejects_missing_fixture_path() -> Result<()> {
        let root = temp_root("perl_corpus_concepts_fixture")?;
        let concepts_dir = root.join("concepts");
        fs::create_dir_all(&concepts_dir)?;

        fs::write(
            concepts_dir.join("parser.toml"),
            r#"
[[concepts]]
id = "parser.fixture.missing"
status = "seeded"
[concepts.scope]
crates = ["perl-parser"]
risk_tags = ["parser"]
[concepts.fixtures]
floors = ["test_corpus/does_not_exist.pl"]
variants = []
[concepts.expect]
panic = false
timeout = false
mode = "parse"
[concepts.snapshots]
tokens = false
ast = true
spans = true
[concepts.run]
pr = true
nightly = true
release = true
"#,
        )?;

        let err = load_concept_registry_from_dir(&concepts_dir, &root)
            .expect_err("must fail missing fixture");
        assert!(err.to_string().contains("does not exist"));

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn loads_registry_deterministically() -> Result<()> {
        let root = temp_root("perl_corpus_concepts_order")?;
        let concepts_dir = root.join("concepts");
        fs::create_dir_all(&concepts_dir)?;

        fs::write(
            concepts_dir.join("zeta.toml"),
            concept_entry("parser.zeta", "test_corpus/basic_constructs.pl"),
        )?;
        fs::write(
            concepts_dir.join("alpha.toml"),
            concept_entry("lexer.alpha", "test_corpus/advanced_regex.pl"),
        )?;
        fs::create_dir_all(root.join("test_corpus"))?;
        fs::write(root.join("test_corpus/basic_constructs.pl"), "print 1;\n")?;
        fs::write(root.join("test_corpus/advanced_regex.pl"), "print 1;\n")?;

        let registry = load_concept_registry_from_dir(&concepts_dir, &root)?;
        let ids: Vec<&str> = registry.concepts.iter().map(|concept| concept.id.as_str()).collect();
        assert_eq!(ids, vec!["lexer.alpha", "parser.zeta"]);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    fn concept_entry(id: &str, fixture: &str) -> String {
        format!(
            r#"
[[concepts]]
id = "{id}"
status = "seeded"
[concepts.scope]
crates = ["perl-parser"]
risk_tags = ["parser"]
[concepts.fixtures]
floors = ["{fixture}"]
variants = []
[concepts.expect]
panic = false
timeout = false
mode = "parse"
[concepts.snapshots]
tokens = false
ast = true
spans = true
[concepts.run]
pr = true
nightly = true
release = true
"#
        )
    }
}
