use color_eyre::eyre::{Context, Result, anyhow, bail};
use perl_parser::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct FixtureMeta {
    concepts: Vec<String>,
    #[serde(default)]
    profile: Vec<String>,
    expected: String,
}

#[derive(Debug, Serialize)]
pub struct ConceptFloorReceipt {
    pub concepts_required: Vec<String>,
    pub concepts_hit: Vec<String>,
    pub missing_concepts: Vec<String>,
    pub violations: Vec<ConceptViolation>,
}

#[derive(Debug, Serialize)]
pub struct ConceptViolation {
    pub fixture: String,
    pub expected: String,
    pub message: String,
}

pub fn run(manifest: PathBuf, receipt: PathBuf) -> Result<()> {
    let workspace_root = super::cpan_corpus::workspace_root();
    let fixture_paths = parse_manifest(&manifest)?;

    let mut metas = Vec::new();
    for fixture in &fixture_paths {
        let fixture_abs = absolutize(&workspace_root, fixture);
        let sidecar = fixture_abs.with_extension("meta.toml");
        let meta_text = fs::read_to_string(&sidecar)
            .with_context(|| format!("failed to read sidecar {}", sidecar.display()))?;
        let meta: FixtureMeta = toml::from_str(&meta_text)
            .with_context(|| format!("invalid sidecar TOML {}", sidecar.display()))?;
        if !meta.profile.is_empty() && !meta.profile.iter().any(|entry| entry == "pr") {
            continue;
        }
        metas.push((fixture_abs, fixture.clone(), meta));
    }

    let report = evaluate_fixtures(&metas)?;
    if let Some(parent) = receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt parent {}", parent.display()))?;
    }
    fs::write(&receipt, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("failed to write receipt {}", receipt.display()))?;

    if !report.missing_concepts.is_empty() {
        bail!(
            "parser concept floors missing required concept buckets: {:?}",
            report.missing_concepts
        );
    }
    if !report.violations.is_empty() {
        bail!("parser concept floor violations: {}", report.violations.len());
    }

    println!(
        "parser concept floors passed: {} concepts hit, {} fixtures",
        report.concepts_hit.len(),
        metas.len()
    );
    Ok(())
}

fn parse_manifest(path: &Path) -> Result<Vec<PathBuf>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("invalid manifest JSON {}", path.display()))?;

    let files = if let Some(array) = json.as_array() {
        array
    } else {
        json.get("files").and_then(Value::as_array).ok_or_else(|| {
            anyhow!("manifest must be an array of paths or object with `files` array")
        })?
    };

    let mut out = Vec::new();
    for entry in files {
        if let Some(path_str) = entry.as_str() {
            out.push(PathBuf::from(path_str));
        }
    }

    if out.is_empty() {
        bail!("manifest {} resolved to zero fixture paths", path.display());
    }
    Ok(out)
}

fn evaluate_fixtures(metas: &[(PathBuf, PathBuf, FixtureMeta)]) -> Result<ConceptFloorReceipt> {
    let required: Vec<String> =
        REQUIRED_CONCEPTS.iter().map(|entry| (*entry).to_string()).collect();
    let mut concepts_hit = BTreeSet::new();
    let mut violations = Vec::new();

    for (abs_path, rel_path, meta) in metas {
        for concept in &meta.concepts {
            concepts_hit.insert(concept.clone());
        }

        let source = fs::read_to_string(abs_path)
            .with_context(|| format!("failed reading fixture {}", abs_path.display()))?;

        match meta.expected.as_str() {
            "parse_clean" => {
                let mut parser = Parser::new(&source);
                if let Err(err) = parser.parse() {
                    violations.push(ConceptViolation {
                        fixture: rel_path.display().to_string(),
                        expected: meta.expected.clone(),
                        message: format!("parse_clean failed: {err}"),
                    });
                }
            }
            "recover_without_panic" => {
                let outcome = parse_with_timeout(&source)?;
                if let Some(message) = outcome {
                    violations.push(ConceptViolation {
                        fixture: rel_path.display().to_string(),
                        expected: meta.expected.clone(),
                        message,
                    });
                }
            }
            "allow_errors" => {
                let outcome = parse_with_timeout(&source)?;
                if let Some(message) = outcome {
                    violations.push(ConceptViolation {
                        fixture: rel_path.display().to_string(),
                        expected: meta.expected.clone(),
                        message,
                    });
                }
            }
            "ast_shape" => {
                let snapshot_path = abs_path.with_extension("ast.snap");
                if snapshot_path.exists() {
                    let expected = fs::read_to_string(&snapshot_path).with_context(|| {
                        format!("failed reading snapshot {}", snapshot_path.display())
                    })?;
                    let mut parser = Parser::new(&source);
                    let output = parser.parse_with_recovery();
                    if output.ast.to_sexp().trim() != expected.trim() {
                        violations.push(ConceptViolation {
                            fixture: rel_path.display().to_string(),
                            expected: meta.expected.clone(),
                            message: "ast_shape snapshot mismatch".to_string(),
                        });
                    }
                }
            }
            other => {
                violations.push(ConceptViolation {
                    fixture: rel_path.display().to_string(),
                    expected: meta.expected.clone(),
                    message: format!("unknown expected value: {other}"),
                });
            }
        }
    }

    let concepts_hit_vec: Vec<String> = concepts_hit.into_iter().collect();
    let hit_set: HashSet<String> = concepts_hit_vec.iter().cloned().collect();
    let missing_concepts =
        required.iter().filter(|concept| !hit_set.contains(*concept)).cloned().collect();

    Ok(ConceptFloorReceipt {
        concepts_required: required,
        concepts_hit: concepts_hit_vec,
        missing_concepts,
        violations,
    })
}

fn parse_with_timeout(source: &str) -> Result<Option<String>> {
    let source_owned = source.to_string();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut parser = Parser::new(&source_owned);
            parser.parse_with_recovery();
        }));
        let _ = tx.send(run);
    });

    match rx.recv_timeout(TIMEOUT) {
        Ok(Ok(())) => Ok(None),
        Ok(Err(_)) => Ok(Some("parser panic".to_string())),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(Some("parser timeout".to_string())),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Ok(Some("parser worker disconnected".to_string()))
        }
    }
}

fn absolutize(root: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() { candidate.to_path_buf() } else { root.join(candidate) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_set_with_all_required_concepts_passes() -> Result<()> {
        let root = tempfile::tempdir()?;
        let fixture_path = root.path().join("clean.pl");
        fs::write(&fixture_path, "my $x = 1;\n")?;

        let mut concept_list = Vec::new();
        for concept in REQUIRED_CONCEPTS {
            concept_list.push((*concept).to_string());
        }

        let meta = FixtureMeta {
            concepts: concept_list,
            profile: vec!["pr".to_string()],
            expected: "parse_clean".to_string(),
        };

        let report = evaluate_fixtures(&[(fixture_path.clone(), PathBuf::from("clean.pl"), meta)])?;
        assert!(report.missing_concepts.is_empty());
        assert!(report.violations.is_empty());
        Ok(())
    }

    #[test]
    fn fixture_set_missing_required_concept_fails() -> Result<()> {
        let root = tempfile::tempdir()?;
        let fixture_path = root.path().join("clean.pl");
        fs::write(&fixture_path, "my $x = 1;\n")?;

        let meta = FixtureMeta {
            concepts: vec!["regex".to_string()],
            profile: vec!["pr".to_string()],
            expected: "parse_clean".to_string(),
        };

        let report = evaluate_fixtures(&[(fixture_path.clone(), PathBuf::from("clean.pl"), meta)])?;
        assert!(!report.missing_concepts.is_empty());
        Ok(())
    }
}
