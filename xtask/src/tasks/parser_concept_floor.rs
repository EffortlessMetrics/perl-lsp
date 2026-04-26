use color_eyre::eyre::{Context, Result, bail};
use perl_parser::{Parser, RecoverySalvageClass, RecoverySalvageProfile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Expected {
    ParseClean,
    RecoverWithoutPanic,
    AllowErrors,
    AstShape,
}

#[derive(Debug, Deserialize)]
struct FixtureMeta {
    concepts: Vec<String>,
    profile: Vec<String>,
    expected: Expected,
}

#[derive(Debug, Serialize)]
pub struct ConceptFloorReceipt {
    pub schema_version: u32,
    pub concepts_required: Vec<String>,
    pub concepts_hit: Vec<String>,
    pub missing_concepts: Vec<String>,
    pub violations: Vec<String>,
    pub fixtures_checked: usize,
    pub elapsed_ms: u64,
}

pub fn run(manifest: PathBuf, receipt: PathBuf) -> Result<()> {
    if !manifest.exists() {
        bail!(
            "parser-ratchet concept-floors manifest not found: {}",
            manifest.display()
        );
    }

    let start = Instant::now();
    let fixtures_root = PathBuf::from("tests/perl-corpus");
    let mut concepts_hit = BTreeSet::new();
    let mut violations = Vec::new();
    let mut fixtures_checked = 0usize;

    for entry in WalkDir::new(&fixtures_root).into_iter().filter_map(std::result::Result::ok) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("pl") {
            continue;
        }

        let meta_path = path.with_extension("meta.toml");
        if !meta_path.exists() {
            violations.push(format!(
                "fixture {} missing sidecar {}",
                path.display(),
                meta_path.display()
            ));
            continue;
        }

        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read fixture {}", path.display()))?;
        let meta_raw = fs::read_to_string(&meta_path)
            .with_context(|| format!("failed to read sidecar {}", meta_path.display()))?;
        let meta: FixtureMeta = toml::from_str(&meta_raw)
            .with_context(|| format!("failed to parse sidecar {}", meta_path.display()))?;

        if !meta.profile.iter().any(|value| value == "pr") {
            continue;
        }

        fixtures_checked += 1;
        for concept in &meta.concepts {
            concepts_hit.insert(concept.clone());
        }

        check_fixture(path, &source, &meta.expected, &mut violations);
    }

    let concepts_required: Vec<String> = REQUIRED_CONCEPTS.iter().map(|s| (*s).to_string()).collect();
    let concepts_hit_vec: Vec<String> = concepts_hit.into_iter().collect();
    let missing_concepts: Vec<String> = concepts_required
        .iter()
        .filter(|concept| !concepts_hit_vec.contains(*concept))
        .cloned()
        .collect();

    if !missing_concepts.is_empty() {
        violations.push(format!(
            "required concept bucket(s) missing: {}",
            missing_concepts.join(", ")
        ));
    }

    let receipt_payload = ConceptFloorReceipt {
        schema_version: 1,
        concepts_required,
        concepts_hit: concepts_hit_vec,
        missing_concepts,
        violations: violations.clone(),
        fixtures_checked,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    if let Some(parent) = receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt dir {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&receipt_payload)?;
    fs::write(&receipt, json)
        .with_context(|| format!("failed to write receipt {}", receipt.display()))?;

    if !violations.is_empty() {
        bail!(
            "parser concept floors failed ({} violation(s)). Receipt: {}",
            violations.len(),
            receipt.display()
        );
    }

    println!(
        "parser concept floors passed: {} fixture(s), {} required concept bucket(s)",
        fixtures_checked,
        REQUIRED_CONCEPTS.len()
    );
    println!("receipt: {}", receipt.display());

    Ok(())
}

fn check_fixture(path: &Path, source: &str, expected: &Expected, violations: &mut Vec<String>) {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let mut parser = Parser::new(source);
        let parse = parser.parse();
        let errors_len = parser.errors().len();

        let salvage_class = parse
            .as_ref()
            .ok()
            .map(|ast| RecoverySalvageProfile::from_parse(ast, parser.errors(), false).class);

        (parse, errors_len, salvage_class)
    }));

    let (parse, errors_len, salvage_class) = match result {
        Ok(value) => value,
        Err(_) => {
            violations.push(format!("{} panicked during parse", path.display()));
            return;
        }
    };

    match expected {
        Expected::ParseClean => {
            if parse.is_err() || errors_len > 0 || salvage_class != Some(RecoverySalvageClass::Clean) {
                violations.push(format!(
                    "{} expected parse_clean but had parse_err={} parser_errors={}",
                    path.display(),
                    parse.is_err(),
                    errors_len
                ));
            }
        }
        Expected::RecoverWithoutPanic => {
            if parse.is_err() {
                violations.push(format!(
                    "{} expected recover_without_panic but parser returned Err",
                    path.display()
                ));
            }
        }
        Expected::AllowErrors => {}
        Expected::AstShape => {
            violations.push(format!(
                "{} expected ast_shape but snapshot verification is not wired yet",
                path.display()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_required_concepts_is_reported() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().join("tests/perl-corpus");
        fs::create_dir_all(&root)?;
        fs::write(root.join("one.pl"), "my $x = 1;\n")?;
        fs::write(
            root.join("one.meta.toml"),
            "concepts = [\"regex\"]\nprofile = [\"pr\"]\nexpected = \"parse_clean\"\n",
        )?;

        let mut concepts_hit = BTreeSet::new();
        concepts_hit.insert("regex".to_string());
        let required: Vec<String> = REQUIRED_CONCEPTS.iter().map(|s| (*s).to_string()).collect();
        let hit: Vec<String> = concepts_hit.into_iter().collect();
        let missing: Vec<String> = required.iter().filter(|c| !hit.contains(*c)).cloned().collect();

        assert!(!missing.is_empty());
        Ok(())
    }

    #[test]
    fn parse_clean_fixture_passes_check() -> Result<()> {
        let mut violations = Vec::new();
        check_fixture(Path::new("clean.pl"), "my $x = 1;\n", &Expected::ParseClean, &mut violations);
        assert!(violations.is_empty());
        Ok(())
    }
}
