//! Durable concept registry for parser-focused corpus coverage.

use crate::files::CorpusPaths;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use toml::Value;

const CONCEPT_FILES: [&str; 6] = [
    "lexer.toml",
    "parser.toml",
    "recovery.toml",
    "positions.toml",
    "incremental.toml",
    "tree_sitter.toml",
];

const KNOWN_STATUS: [&str; 4] = ["seed", "active", "planned", "deprecated"];
const KNOWN_TOP_LEVEL_KEYS: [&str; 1] = ["concepts"];
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptRegistry {
    pub concepts: Vec<ConceptRow>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptRow {
    pub id: String,
    pub status: String,
    pub scope: ConceptScope,
    pub fixtures: ConceptFixtures,
    pub expect: ConceptExpect,
    pub snapshots: ConceptSnapshots,
    pub run: ConceptRun,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptScope {
    pub crates: Vec<String>,
    pub risk_tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptFixtures {
    pub floors: Vec<String>,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptExpect {
    pub panic: bool,
    pub timeout: bool,
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptSnapshots {
    pub tokens: bool,
    pub ast: bool,
    pub spans: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptRun {
    pub pr: bool,
    pub nightly: bool,
    pub release: bool,
}

pub fn load_concept_registry() -> Result<ConceptRegistry> {
    load_concept_registry_from(&CorpusPaths::discover().root)
}

pub fn load_concept_registry_from(workspace_root: &Path) -> Result<ConceptRegistry> {
    let concepts_dir = workspace_root.join("crates/perl-corpus/concepts");
    let mut concepts = Vec::new();

    for file_name in CONCEPT_FILES {
        let path = concepts_dir.join(file_name);
        let mut file_concepts = load_concepts_file(&path)?;
        concepts.append(&mut file_concepts);
    }

    concepts.sort_by(|a, b| a.id.cmp(&b.id));
    validate_registry(workspace_root, &concepts)?;

    Ok(ConceptRegistry { concepts })
}

fn load_concepts_file(path: &Path) -> Result<Vec<ConceptRow>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading concepts file {}", path.display()))?;

    let raw_value: Value =
        toml::from_str::<Value>(&text).with_context(|| format!("parsing TOML {}", path.display()))?;
    validate_top_level_keys(path, &raw_value)?;

    #[derive(Deserialize)]
    struct ConceptsFile {
        concepts: Vec<ConceptRow>,
    }

    let parsed: ConceptsFile = raw_value
        .try_into()
        .with_context(|| format!("deserializing concepts file {}", path.display()))?;

    Ok(parsed.concepts)
}

fn validate_top_level_keys(path: &Path, value: &Value) -> Result<()> {
    let table = value
        .as_table()
        .with_context(|| format!("concept file {} must be a TOML table", path.display()))?;

    for key in table.keys() {
        if !KNOWN_TOP_LEVEL_KEYS.contains(&key.as_str()) {
            bail!("unknown top-level TOML section '{}' in {}", key, path.display());
        }
    }

    Ok(())
}

fn validate_registry(workspace_root: &Path, concepts: &[ConceptRow]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for concept in concepts {
        if !ids.insert(concept.id.clone()) {
            bail!("duplicate concept id '{}'", concept.id);
        }

        if !KNOWN_STATUS.contains(&concept.status.as_str()) {
            bail!("unknown status '{}' for concept '{}'", concept.status, concept.id);
        }

        validate_fixture_paths(workspace_root, concept)?;
    }

    Ok(())
}

fn validate_fixture_paths(workspace_root: &Path, concept: &ConceptRow) -> Result<()> {
    for fixture_path in concept.fixtures.floors.iter().chain(concept.fixtures.variants.iter()) {
        let full_path = workspace_root.join(fixture_path);
        if !full_path.exists() {
            bail!("fixture path '{}' for concept '{}' does not exist", fixture_path, concept.id);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> Result<PathBuf> {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        root.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn write_file(root: &Path, relative: &str, content: &str) -> Result<()> {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn minimal_concepts_doc(id: &str, fixture: &str) -> String {
        format!(
            r#"[[concepts]]
id = "{id}"
status = "seed"
scope.crates = ["perl-lexer"]
scope.risk_tags = ["lexing"]
fixtures.floors = ["{fixture}"]
fixtures.variants = []
expect.panic = false
expect.timeout = false
expect.mode = "parse"
snapshots.tokens = true
snapshots.ast = false
snapshots.spans = true
run.pr = true
run.nightly = true
run.release = false
"#
        )
    }

    fn write_all_concept_files(root: &Path, shared_content: &str) -> Result<()> {
        for file_name in CONCEPT_FILES {
            write_file(root, &format!("crates/perl-corpus/concepts/{file_name}"), shared_content)?;
        }
        Ok(())
    }

    #[test]
    fn registry_rejects_duplicate_concept_ids() -> Result<()> {
        let root = temp_root("perl_corpus_duplicate_concepts")?;
        write_file(&root, "test_corpus/sample.pl", "print 1;\n")?;
        write_all_concept_files(&root, &minimal_concepts_doc("dup.id", "test_corpus/sample.pl"))?;

        let message = match load_concept_registry_from(&root) {
            Ok(_) => bail!("expected duplicate id validation failure"),
            Err(err) => err.to_string(),
        };
        assert!(message.contains("duplicate concept id 'dup.id'"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn registry_rejects_missing_fixture_path() -> Result<()> {
        let root = temp_root("perl_corpus_missing_fixture")?;
        write_all_concept_files(
            &root,
            &minimal_concepts_doc("unique.id", "test_corpus/does_not_exist.pl"),
        )?;

        let message = match load_concept_registry_from(&root) {
            Ok(_) => bail!("expected missing fixture path validation failure"),
            Err(err) => err.to_string(),
        };
        assert!(message.contains("does not exist"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn registry_load_is_deterministic() -> Result<()> {
        let root = temp_root("perl_corpus_deterministic_concepts")?;
        write_file(&root, "test_corpus/sample.pl", "print 1;\n")?;

        let mut index = 0usize;
        for file_name in CONCEPT_FILES {
            let id = format!("zeta.{index}");
            write_file(
                &root,
                &format!("crates/perl-corpus/concepts/{file_name}"),
                &minimal_concepts_doc(&id, "test_corpus/sample.pl"),
            )?;
            index += 1;
        }

        let first = load_concept_registry_from(&root)?;
        let second = load_concept_registry_from(&root)?;
        assert_eq!(first, second);

        fs::remove_dir_all(root)?;
        Ok(())
    }
}
