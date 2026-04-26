use color_eyre::eyre::{Context, Result, bail};
use perl_parser::Parser;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

const REQUIRED_CONCEPTS: &[&str] = &[
    "regex",
    "interpolation",
    "heredoc",
    "package",
    "subroutine",
    "lexical",
    "references",
    "hash_array_deref",
    "pod",
    "data_section",
    "recovery",
];

#[derive(Debug, Clone)]
pub struct ConceptFloorConfig {
    pub manifest: PathBuf,
    pub receipt: PathBuf,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptFloorReceipt {
    pub schema_version: String,
    pub concepts_required: Vec<String>,
    pub concepts_hit: Vec<String>,
    pub missing_concepts: Vec<String>,
    pub violations: Vec<ConceptViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptViolation {
    pub fixture: String,
    pub expected: String,
    pub details: String,
}

#[derive(Debug, Deserialize)]
struct FixtureMeta {
    concepts: Vec<String>,
    profile: Vec<String>,
    expected: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CorpusManifest {
    Paths(Vec<String>),
    Fixtures { fixtures: Vec<ManifestFixture> },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ManifestFixture {
    Path(String),
    Record { path: String },
}

pub fn run(config: ConceptFloorConfig) -> Result<()> {
    let manifest_text = fs::read_to_string(&config.manifest)
        .with_context(|| format!("failed to read manifest: {}", config.manifest.display()))?;
    let manifest: CorpusManifest = serde_json::from_str(&manifest_text)
        .with_context(|| format!("failed to parse manifest JSON: {}", config.manifest.display()))?;

    let fixtures = manifest_paths(&manifest).into_iter().map(PathBuf::from).collect::<Vec<_>>();

    let mut concepts_hit = BTreeSet::new();
    let mut violations = Vec::new();

    for fixture in fixtures {
        let meta_path = sidecar_path(&fixture);
        if !meta_path.exists() {
            continue;
        }

        let meta_text = fs::read_to_string(&meta_path)
            .with_context(|| format!("failed to read metadata: {}", meta_path.display()))?;
        let meta: FixtureMeta = toml::from_str(&meta_text)
            .with_context(|| format!("failed to parse metadata TOML: {}", meta_path.display()))?;

        if !meta.profile.iter().any(|bucket| bucket == &config.profile) {
            continue;
        }

        for concept in &meta.concepts {
            concepts_hit.insert(concept.clone());
        }

        let source = fs::read_to_string(&fixture)
            .with_context(|| format!("failed to read fixture: {}", fixture.display()))?;
        if let Some(violation) = evaluate_fixture(&fixture, &source, &meta.expected) {
            violations.push(violation);
        }
    }

    let concepts_required =
        REQUIRED_CONCEPTS.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>();

    let missing_concepts = concepts_required.difference(&concepts_hit).cloned().collect::<Vec<_>>();

    let receipt = ConceptFloorReceipt {
        schema_version: "1.0.0".to_string(),
        concepts_required: concepts_required.into_iter().collect(),
        concepts_hit: concepts_hit.into_iter().collect(),
        missing_concepts: missing_concepts.clone(),
        violations,
    };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt directory: {}", parent.display()))?;
    }

    let receipt_text = serde_json::to_string_pretty(&receipt)?;
    fs::write(&config.receipt, receipt_text)
        .with_context(|| format!("failed to write receipt: {}", config.receipt.display()))?;

    if !missing_concepts.is_empty() {
        bail!("missing required concept buckets: {}", missing_concepts.join(", "));
    }

    if !receipt.violations.is_empty() {
        bail!("concept floor violations: {}", receipt.violations.len());
    }

    println!(
        "Concept floors passed: {} required concepts hit, {} fixtures violated.",
        receipt.concepts_hit.len(),
        receipt.violations.len()
    );

    Ok(())
}

fn evaluate_fixture(fixture: &Path, source: &str, expected: &str) -> Option<ConceptViolation> {
    let parse_attempt = catch_unwind(AssertUnwindSafe(|| {
        let mut parser = Parser::new(source);
        let parse_result = parser.parse();
        let has_errors = !parser.errors().is_empty();
        (parse_result.is_ok(), has_errors)
    }));

    match expected {
        "parse_clean" => match parse_attempt {
            Ok((true, false)) => None,
            Ok((false, _)) => {
                Some(violation(fixture, expected, "parser returned catastrophic error"))
            }
            Ok((true, true)) => {
                Some(violation(fixture, expected, "parse completed with parser errors"))
            }
            Err(_) => Some(violation(fixture, expected, "parser panicked")),
        },
        "recover_without_panic" => match parse_attempt {
            Ok(_) => None,
            Err(_) => Some(violation(fixture, expected, "parser panicked")),
        },
        "allow_errors" => match parse_attempt {
            Ok(_) => None,
            Err(_) => Some(violation(fixture, expected, "parser panicked")),
        },
        "ast_shape" => match parse_attempt {
            Ok((true, _)) => None,
            Ok((false, _)) => {
                Some(violation(fixture, expected, "parser returned catastrophic error"))
            }
            Err(_) => Some(violation(fixture, expected, "parser panicked")),
        },
        other => Some(violation(fixture, expected, &format!("unknown expected value: {other}"))),
    }
}

fn violation(fixture: &Path, expected: &str, details: &str) -> ConceptViolation {
    ConceptViolation {
        fixture: fixture.display().to_string(),
        expected: expected.to_string(),
        details: details.to_string(),
    }
}

fn sidecar_path(fixture_path: &Path) -> PathBuf {
    fixture_path.with_extension("meta.toml")
}

fn manifest_paths(manifest: &CorpusManifest) -> Vec<String> {
    match manifest {
        CorpusManifest::Paths(items) => items.clone(),
        CorpusManifest::Fixtures { fixtures } => fixtures
            .iter()
            .map(|fixture| match fixture {
                ManifestFixture::Path(path) => path.clone(),
                ManifestFixture::Record { path } => path.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(prefix: &Path, rel: &str, source: &str, meta: &str) -> Result<()> {
        let fixture_path = prefix.join(rel);
        if let Some(parent) = fixture_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&fixture_path, source)?;
        fs::write(fixture_path.with_extension("meta.toml"), meta)?;
        Ok(())
    }

    #[test]
    fn concept_floors_detect_missing_concepts() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        fixture(
            root,
            "tests/perl-corpus/minimal.pl",
            "my $x = 1;",
            "concepts=[\"regex\"]\nprofile=[\"pr\"]\nexpected=\"parse_clean\"\n",
        )?;
        let manifest_payload = serde_json::to_string(&vec![
            root.join("tests/perl-corpus/minimal.pl").display().to_string(),
        ])?;
        fs::write(root.join("manifest.json"), manifest_payload)?;

        let outcome = run(ConceptFloorConfig {
            manifest: root.join("manifest.json"),
            receipt: root.join("receipt.json"),
            profile: "pr".to_string(),
        });

        assert!(outcome.is_err());
        let receipt: ConceptFloorReceipt =
            serde_json::from_str(&fs::read_to_string(root.join("receipt.json"))?)?;
        assert!(receipt.missing_concepts.contains(&"heredoc".to_string()));
        Ok(())
    }

    #[test]
    fn concept_floors_pass_with_all_required_concepts() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        fixture(
            root,
            "tests/perl-corpus/all-concepts.pl",
            "package Demo; sub run { my $s = q{abc}; my $h = {a => [1]}; my $r = $h; return $r->{a}->[0]; }\n1;\n__DATA__\n",
            "concepts=[\"regex\",\"interpolation\",\"heredoc\",\"package\",\"subroutine\",\"lexical\",\"references\",\"hash_array_deref\",\"pod\",\"data_section\",\"recovery\"]\nprofile=[\"pr\"]\nexpected=\"allow_errors\"\n",
        )?;
        let manifest_payload = serde_json::to_string(&vec![
            root.join("tests/perl-corpus/all-concepts.pl").display().to_string(),
        ])?;
        fs::write(root.join("manifest.json"), manifest_payload)?;

        run(ConceptFloorConfig {
            manifest: root.join("manifest.json"),
            receipt: root.join("receipt.json"),
            profile: "pr".to_string(),
        })?;

        let receipt: ConceptFloorReceipt =
            serde_json::from_str(&fs::read_to_string(root.join("receipt.json"))?)?;
        assert!(receipt.missing_concepts.is_empty());
        assert!(receipt.violations.is_empty());
        Ok(())
    }
}
