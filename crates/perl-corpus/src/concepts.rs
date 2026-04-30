use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CONCEPT_FILES: &[&str] = &[
    "lexer.toml",
    "parser.toml",
    "recovery.toml",
    "positions.toml",
    "incremental.toml",
    "tree_sitter.toml",
];

const KNOWN_STATUS: &[&str] = &["seed", "active", "planned", "deprecated"];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptRegistryFile {
    #[serde(default)]
    pub concept: Vec<ConceptRow>,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptScope {
    pub crates: Vec<String>,
    pub risk_tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptFixtures {
    #[serde(default)]
    pub floors: Vec<String>,
    #[serde(default)]
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptExpect {
    pub panic: bool,
    pub timeout: bool,
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptSnapshots {
    pub tokens: bool,
    pub ast: bool,
    pub spans: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConceptRun {
    pub pr: bool,
    pub nightly: bool,
    pub release: bool,
}

#[derive(Debug, Clone)]
pub struct LoadedConcept {
    pub source_file: String,
    pub row: ConceptRow,
}

pub fn load_concept_registry() -> Result<Vec<LoadedConcept>> {
    let root = super::files::CorpusPaths::discover().root;
    let concept_dir = root.join("crates/perl-corpus/concepts");
    load_concept_registry_from(&concept_dir, &root)
}

fn load_concept_registry_from(
    concept_dir: &Path,
    workspace_root: &Path,
) -> Result<Vec<LoadedConcept>> {
    let mut concepts = Vec::new();

    for file_name in CONCEPT_FILES {
        let path = concept_dir.join(file_name);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("reading concept registry file {}", path.display()))?;

        let parsed: ConceptRegistryFile =
            toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;

        for row in parsed.concept {
            validate_row(&row, workspace_root, &path)?;
            concepts.push(LoadedConcept { source_file: (*file_name).to_string(), row });
        }
    }

    concepts
        .sort_by(|a, b| a.row.id.cmp(&b.row.id).then_with(|| a.source_file.cmp(&b.source_file)));

    let mut seen = BTreeSet::new();
    for concept in &concepts {
        if !seen.insert(concept.row.id.clone()) {
            bail!("duplicate concept id: {}", concept.row.id);
        }
    }

    Ok(concepts)
}

fn validate_row(row: &ConceptRow, workspace_root: &Path, source_file: &Path) -> Result<()> {
    if !KNOWN_STATUS.contains(&row.status.as_str()) {
        bail!(
            "unknown status '{}' for concept '{}' in {}",
            row.status,
            row.id,
            source_file.display()
        );
    }

    for fixture in row.fixtures.floors.iter().chain(row.fixtures.variants.iter()) {
        let fixture_path = fixture_path(workspace_root, fixture);
        if !fixture_path.exists() {
            bail!(
                "fixture path '{}' for concept '{}' does not exist (resolved to {})",
                fixture,
                row.id,
                fixture_path.display()
            );
        }
    }

    Ok(())
}

fn fixture_path(workspace_root: &Path, fixture: &str) -> PathBuf {
    let path = Path::new(fixture);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    workspace_root.join(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        dir.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        must(fs::create_dir_all(&dir));
        dir
    }

    fn write_registry(path: &Path, body: &str) {
        let concept_dir = path.join("concepts");
        must(fs::create_dir_all(&concept_dir));

        for file in CONCEPT_FILES {
            let content = if *file == "lexer.toml" { body } else { "" };
            must(fs::write(concept_dir.join(file), content));
        }
    }

    #[test]
    fn registry_loads_deterministically() {
        let root = temp_dir("concept_registry_deterministic");
        must(fs::create_dir_all(root.join("fixtures")));
        must(fs::write(root.join("fixtures/ok.pl"), "my $x = 1;\n"));

        write_registry(
            &root,
            r#"
[[concept]]
id = "z.last"
status = "seed"
scope = { crates = ["perl-parser"], risk_tags = ["parser"] }
fixtures = { floors = ["fixtures/ok.pl"], variants = [] }
expect = { panic = false, timeout = false, mode = "parse" }
snapshots = { tokens = true, ast = true, spans = false }
run = { pr = true, nightly = false, release = false }

[[concept]]
id = "a.first"
status = "seed"
scope = { crates = ["perl-lexer"], risk_tags = ["lexer"] }
fixtures = { floors = ["fixtures/ok.pl"], variants = [] }
expect = { panic = false, timeout = false, mode = "lex" }
snapshots = { tokens = true, ast = false, spans = true }
run = { pr = true, nightly = true, release = false }
"#,
        );

        let loaded = must(load_concept_registry_from(&root.join("concepts"), &root));
        let ids: Vec<_> = loaded.iter().map(|c| c.row.id.as_str()).collect();
        assert_eq!(ids, vec!["a.first", "z.last"]);
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let root = temp_dir("concept_registry_dupe");
        must(fs::create_dir_all(root.join("fixtures")));
        must(fs::write(root.join("fixtures/ok.pl"), "my $x = 1;\n"));

        write_registry(
            &root,
            r#"
[[concept]]
id = "dup.id"
status = "seed"
scope = { crates = ["perl-parser"], risk_tags = ["parser"] }
fixtures = { floors = ["fixtures/ok.pl"], variants = [] }
expect = { panic = false, timeout = false, mode = "parse" }
snapshots = { tokens = true, ast = true, spans = false }
run = { pr = true, nightly = true, release = false }

[[concept]]
id = "dup.id"
status = "seed"
scope = { crates = ["perl-parser"], risk_tags = ["parser"] }
fixtures = { floors = ["fixtures/ok.pl"], variants = [] }
expect = { panic = false, timeout = false, mode = "parse" }
snapshots = { tokens = true, ast = true, spans = false }
run = { pr = true, nightly = true, release = false }
"#,
        );

        let err = load_concept_registry_from(&root.join("concepts"), &root)
            .expect_err("expected duplicate id failure");
        assert!(err.to_string().contains("duplicate concept id"));
    }

    #[test]
    fn missing_fixture_path_is_rejected() {
        let root = temp_dir("concept_registry_fixture");

        write_registry(
            &root,
            r#"
[[concept]]
id = "bad.fixture"
status = "seed"
scope = { crates = ["perl-parser"], risk_tags = ["parser"] }
fixtures = { floors = ["fixtures/does-not-exist.pl"], variants = [] }
expect = { panic = false, timeout = false, mode = "parse" }
snapshots = { tokens = true, ast = false, spans = false }
run = { pr = true, nightly = false, release = false }
"#,
        );

        let err = load_concept_registry_from(&root.join("concepts"), &root)
            .expect_err("expected fixture existence failure");
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn unknown_top_level_sections_are_rejected() {
        let root = temp_dir("concept_registry_unknown");
        must(fs::create_dir_all(root.join("fixtures")));
        must(fs::write(root.join("fixtures/ok.pl"), "my $x = 1;\n"));

        write_registry(
            &root,
            r#"
[[concept]]
id = "good.one"
status = "seed"
scope = { crates = ["perl-parser"], risk_tags = ["parser"] }
fixtures = { floors = ["fixtures/ok.pl"], variants = [] }
expect = { panic = false, timeout = false, mode = "parse" }
snapshots = { tokens = true, ast = true, spans = false }
run = { pr = true, nightly = true, release = false }

[extra]
hello = "world"
"#,
        );

        let err = load_concept_registry_from(&root.join("concepts"), &root)
            .expect_err("expected unknown section failure");
        let message = err.to_string();
        assert!(message.contains("unknown") || message.contains("extra"));
    }
}
