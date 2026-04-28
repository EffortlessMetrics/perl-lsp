//! Durable concept registry for corpus-driven parser coverage planning.

use crate::files::CorpusPaths;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const CONCEPT_FILES: &[&str] = &[
    "lexer.toml",
    "parser.toml",
    "recovery.toml",
    "positions.toml",
    "incremental.toml",
    "tree_sitter.toml",
];

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptRegistryFile {
    #[serde(default)]
    pub concepts: Vec<ConceptRow>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConceptStatus {
    Proposed,
    Seeded,
    Active,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptScope {
    #[serde(default)]
    pub crates: Vec<String>,
    #[serde(default)]
    pub risk_tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptFixtures {
    #[serde(default)]
    pub floors: Vec<String>,
    #[serde(default)]
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptExpect {
    pub panic: bool,
    pub timeout: bool,
    pub mode: ConceptExpectMode,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConceptExpectMode {
    ParseOnly,
    ParseAndRecover,
    SnapshotOnly,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptSnapshots {
    pub tokens: bool,
    pub ast: bool,
    pub spans: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConceptRun {
    pub pr: RunTier,
    pub nightly: RunTier,
    pub release: RunTier,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunTier {
    Skip,
    Smoke,
    Full,
}

/// Load registry rows from `crates/perl-corpus/concepts/*.toml`.
pub fn load_concept_registry() -> Result<Vec<ConceptRow>> {
    let root = CorpusPaths::discover().root;
    load_concept_registry_from_root(&root)
}

/// Load registry rows from a provided workspace root.
pub fn load_concept_registry_from_root(root: &Path) -> Result<Vec<ConceptRow>> {
    let concepts_dir = root.join("crates/perl-corpus/concepts");
    let mut rows = Vec::new();

    for file_name in CONCEPT_FILES {
        let path = concepts_dir.join(file_name);
        let source = fs::read_to_string(&path)
            .with_context(|| format!("reading concept file {}", path.display()))?;
        let parsed: ConceptRegistryFile = toml::from_str(&source)
            .with_context(|| format!("parsing concept file {}", path.display()))?;
        rows.extend(parsed.concepts);
    }

    validate_registry(&rows, root)?;

    rows.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(rows)
}

fn validate_registry(rows: &[ConceptRow], root: &Path) -> Result<()> {
    let mut seen = HashSet::new();
    for row in rows {
        if !seen.insert(row.id.as_str()) {
            bail!("duplicate concept id: {}", row.id);
        }

        validate_fixture_paths(&row.fixtures.floors, root, &row.id, "floors")?;
        validate_fixture_paths(&row.fixtures.variants, root, &row.id, "variants")?;
    }

    Ok(())
}

fn validate_fixture_paths(paths: &[String], root: &Path, id: &str, bucket: &str) -> Result<()> {
    for fixture in paths {
        let fixture_path = root.join(fixture);
        if !fixture_path.exists() {
            bail!("concept '{}' {} fixture does not exist: {}", id, bucket, fixture);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> std::io::Result<PathBuf> {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        root.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        fs::create_dir_all(root.join("crates/perl-corpus/concepts"))?;
        Ok(root)
    }

    fn write_concept_file(root: &Path, file_name: &str, body: &str) -> std::io::Result<()> {
        fs::write(root.join("crates/perl-corpus/concepts").join(file_name), body)
    }

    fn write_minimal_files(root: &Path, body_fn: impl Fn(usize) -> String) -> std::io::Result<()> {
        for (index, file_name) in CONCEPT_FILES.iter().enumerate() {
            write_concept_file(root, file_name, &body_fn(index))?;
        }
        Ok(())
    }

    #[test]
    fn registry_load_is_deterministic() -> Result<()> {
        let root = temp_root("perl_corpus_concepts_det")?;
        fs::write(root.join("exists.pl"), "print 1;\n")?;

        write_minimal_files(&root, |index| {
            format!(
                r#"
[[concepts]]
id = "z.id.{index}"
status = "seeded"
scope = {{ crates = ["perl-parser"], risk_tags = [] }}
fixtures = {{ floors = ["exists.pl"], variants = [] }}
expect = {{ panic = false, timeout = false, mode = "parse_only" }}
snapshots = {{ tokens = true, ast = false, spans = true }}
run = {{ pr = "smoke", nightly = "full", release = "full" }}

[[concepts]]
id = "a.id.{index}"
status = "seeded"
scope = {{ crates = ["perl-parser"], risk_tags = [] }}
fixtures = {{ floors = [], variants = [] }}
expect = {{ panic = false, timeout = false, mode = "parse_only" }}
snapshots = {{ tokens = true, ast = true, spans = true }}
run = {{ pr = "smoke", nightly = "full", release = "full" }}
"#
            )
        })?;

        let rows = load_concept_registry_from_root(&root)?;
        assert_eq!(rows.first().map(|row| row.id.as_str()), Some("a.id.0"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn registry_rejects_duplicate_ids() -> Result<()> {
        let root = temp_root("perl_corpus_concepts_dup")?;

        write_minimal_files(&root, |_| {
            r#"
[[concepts]]
id = "dup.id"
status = "seeded"
scope = { crates = [], risk_tags = [] }
fixtures = { floors = [], variants = [] }
expect = { panic = false, timeout = false, mode = "parse_only" }
snapshots = { tokens = true, ast = false, spans = false }
run = { pr = "smoke", nightly = "full", release = "full" }

[[concepts]]
id = "dup.id"
status = "seeded"
scope = { crates = [], risk_tags = [] }
fixtures = { floors = [], variants = [] }
expect = { panic = false, timeout = false, mode = "parse_only" }
snapshots = { tokens = true, ast = false, spans = false }
run = { pr = "smoke", nightly = "full", release = "full" }
"#
            .to_string()
        })?;

        let error = load_concept_registry_from_root(&root)
            .expect_err("duplicate concept ids should fail validation");
        assert!(error.to_string().contains("duplicate concept id"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn registry_rejects_missing_fixture_paths() -> Result<()> {
        let root = temp_root("perl_corpus_concepts_fixture")?;

        write_minimal_files(&root, |_| {
            r#"
[[concepts]]
id = "fixture.bad"
status = "seeded"
scope = { crates = [], risk_tags = [] }
fixtures = { floors = ["does/not/exist.pl"], variants = [] }
expect = { panic = false, timeout = false, mode = "parse_only" }
snapshots = { tokens = true, ast = true, spans = false }
run = { pr = "smoke", nightly = "full", release = "full" }
"#
            .to_string()
        })?;

        let error = load_concept_registry_from_root(&root)
            .expect_err("missing fixture paths should fail validation");
        assert!(error.to_string().contains("fixture does not exist"));

        fs::remove_dir_all(root)?;
        Ok(())
    }
}
