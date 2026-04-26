use color_eyre::eyre::{Context, Result, bail};
use perl_parser::{ParseError, Parser, RecoverySalvageClass, RecoverySalvageProfile};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Deserialize)]
struct FixtureMeta {
    concepts: Vec<String>,
    #[serde(default)]
    profile: Vec<String>,
    expected: ExpectedBehavior,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ExpectedBehavior {
    ParseClean,
    RecoverWithoutPanic,
    AllowErrors,
    AstShape,
}

#[derive(Debug, Serialize)]
pub struct ParserConceptFloorReceipt {
    schema_version: String,
    concepts_required: Vec<String>,
    concepts_hit: Vec<String>,
    missing_concepts: Vec<String>,
    violations: Vec<ConceptViolation>,
}

#[derive(Debug, Serialize)]
struct ConceptViolation {
    fixture: String,
    expected: String,
    actual: String,
    detail: String,
}

#[derive(Debug)]
pub struct ConceptFloorsConfig {
    pub manifest: PathBuf,
    pub receipt: PathBuf,
}

pub fn run(config: &ConceptFloorsConfig) -> Result<()> {
    let root = std::env::current_dir().context("resolve workspace root")?;
    let fixtures = load_fixture_candidates(&root, Some(&config.manifest))?;
    let receipt = evaluate_fixtures(&root, &fixtures)?;

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create receipt parent {}", parent.display()))?;
    }
    fs::write(&config.receipt, serde_json::to_vec_pretty(&receipt)?)
        .with_context(|| format!("write receipt {}", config.receipt.display()))?;

    if !receipt.missing_concepts.is_empty() || !receipt.violations.is_empty() {
        bail!(
            "parser concept floors failed: {} missing concepts, {} violations",
            receipt.missing_concepts.len(),
            receipt.violations.len()
        );
    }

    Ok(())
}

fn evaluate_fixtures(root: &Path, fixtures: &[PathBuf]) -> Result<ParserConceptFloorReceipt> {
    let required: BTreeSet<String> = REQUIRED_CONCEPTS.iter().map(|c| (*c).to_string()).collect();
    let mut hit = BTreeSet::new();
    let mut violations = Vec::new();

    for fixture in fixtures {
        let meta = parse_meta(fixture)?;
        if !meta.profile.is_empty() && !meta.profile.iter().any(|p| p == "pr") {
            continue;
        }

        for concept in &meta.concepts {
            if required.contains(concept) {
                hit.insert(concept.clone());
            }
        }

        let source = fs::read_to_string(fixture)
            .with_context(|| format!("read fixture {}", fixture.display()))?;
        let mut parser = Parser::new(&source);
        let parse_outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parser.parse()));

        match (&meta.expected, parse_outcome) {
            (ExpectedBehavior::ParseClean, Ok(Ok(ast))) => {
                let errors: Vec<ParseError> = parser.errors().to_vec();
                let salvage = RecoverySalvageProfile::from_parse(&ast, &errors, false);
                if !matches!(salvage.class, RecoverySalvageClass::Clean) {
                    violations.push(build_violation(
                        root,
                        fixture,
                        "parse_clean",
                        "dirty_parse",
                        "fixture expected clean parse but parser reported recovery/errors",
                    ));
                }
            }
            (ExpectedBehavior::ParseClean, Ok(Err(err))) => {
                violations.push(build_violation(
                    root,
                    fixture,
                    "parse_clean",
                    "panic_or_catastrophic",
                    &format!("catastrophic parse failure: {err}"),
                ));
            }
            (ExpectedBehavior::ParseClean, Err(_)) => {
                violations.push(build_violation(
                    root,
                    fixture,
                    "parse_clean",
                    "panic",
                    "parser panicked",
                ));
            }
            (ExpectedBehavior::RecoverWithoutPanic, Ok(_)) => {}
            (ExpectedBehavior::RecoverWithoutPanic, Err(_)) => violations.push(build_violation(
                root,
                fixture,
                "recover_without_panic",
                "panic",
                "parser panicked",
            )),
            (ExpectedBehavior::AllowErrors, Ok(_)) => {}
            (ExpectedBehavior::AllowErrors, Err(_)) => violations.push(build_violation(
                root,
                fixture,
                "allow_errors",
                "panic",
                "parser panicked",
            )),
            (ExpectedBehavior::AstShape, Ok(_)) => {}
            (ExpectedBehavior::AstShape, Err(_)) => violations.push(build_violation(
                root,
                fixture,
                "ast_shape",
                "panic",
                "parser panicked",
            )),
        }
    }

    let hit_vec: Vec<String> = hit.into_iter().collect();
    let missing: Vec<String> =
        required.iter().filter(|concept| !hit_vec.contains(concept)).cloned().collect();

    Ok(ParserConceptFloorReceipt {
        schema_version: "1.0.0".to_string(),
        concepts_required: REQUIRED_CONCEPTS.iter().map(|c| (*c).to_string()).collect(),
        concepts_hit: hit_vec,
        missing_concepts: missing,
        violations,
    })
}

fn build_violation(
    root: &Path,
    fixture: &Path,
    expected: &str,
    actual: &str,
    detail: &str,
) -> ConceptViolation {
    ConceptViolation {
        fixture: portable_relative(root, fixture),
        expected: expected.to_string(),
        actual: actual.to_string(),
        detail: detail.to_string(),
    }
}

fn parse_meta(fixture: &Path) -> Result<FixtureMeta> {
    let meta_path = fixture.with_extension("meta.toml");
    let raw = fs::read_to_string(&meta_path)
        .with_context(|| format!("read sidecar metadata {}", meta_path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parse sidecar metadata {}", meta_path.display()))
}

fn load_fixture_candidates(root: &Path, manifest_path: Option<&Path>) -> Result<Vec<PathBuf>> {
    let mut fixtures = Vec::new();
    for entry in WalkDir::new(root.join("tests/perl-corpus")).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "pl") {
            fixtures.push(path.to_path_buf());
        }
    }

    if let Some(manifest_path) = manifest_path {
        if manifest_path.exists() {
            let listed = parse_manifest_file_list(manifest_path)?;
            if !listed.is_empty() {
                let listed_set: BTreeSet<PathBuf> = listed.into_iter().collect();
                fixtures.retain(|fixture| {
                    let rel = portable_relative(root, fixture);
                    listed_set.contains(&PathBuf::from(&rel)) || listed_set.contains(fixture)
                });
            }
        }
    }

    fixtures.sort();
    Ok(fixtures)
}

fn parse_manifest_file_list(path: &Path) -> Result<Vec<PathBuf>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read concept-floor manifest {}", path.display()))?;
    let json: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse concept-floor manifest {}", path.display()))?;
    let mut out = Vec::new();
    collect_pl_paths(&json, &mut out);
    Ok(out)
}

fn collect_pl_paths(value: &Value, out: &mut Vec<PathBuf>) {
    match value {
        Value::String(s) => {
            if s.ends_with(".pl") {
                out.push(PathBuf::from(s));
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_pl_paths(value, out);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_pl_paths(value, out);
            }
        }
        _ => {}
    }
}

fn portable_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn write_fixture(root: &Path, name: &str, source: &str, concepts: &[&str]) -> Result<()> {
        let fixture = root.join("tests/perl-corpus").join(format!("{name}.pl"));
        let meta = root.join("tests/perl-corpus").join(format!("{name}.meta.toml"));
        fs::create_dir_all(fixture.parent().expect("fixture parent"))?;
        fs::write(&fixture, source)?;
        let concepts_list =
            concepts.iter().map(|concept| format!("\"{concept}\"")).collect::<Vec<_>>().join(", ");
        fs::write(
            &meta,
            format!(
                "concepts = [{concepts_list}]\nprofile = [\"pr\"]\nexpected = \"parse_clean\"\n"
            ),
        )?;
        Ok(())
    }

    #[test]
    fn fails_when_required_concepts_missing() -> Result<()> {
        let dir = tempdir()?;
        write_fixture(dir.path(), "tiny", "my $x = 1;", &["lexical"])?;
        let receipt =
            evaluate_fixtures(dir.path(), &[dir.path().join("tests/perl-corpus/tiny.pl")])?;
        assert!(!receipt.missing_concepts.is_empty());
        Ok(())
    }

    #[test]
    fn passes_when_all_required_concepts_present() -> Result<()> {
        let dir = tempdir()?;
        let fixtures_root = dir.path().join("tests/perl-corpus");
        fs::create_dir_all(&fixtures_root)?;

        let samples: BTreeMap<&str, (&str, &str)> = BTreeMap::from([
            ("regex", ("my $x = qr/foo/;", "regex")),
            ("interpolation", ("my $x = \"hello $name\";", "interpolation")),
            ("heredoc", ("my $x = <<'END';\nhello\nEND\n", "heredoc")),
            ("package", ("package My::Pkg; 1;", "package")),
            ("subroutine", ("sub greet { return 1; }", "subroutine")),
            ("lexical", ("my $value = 2;", "lexical")),
            ("references", ("my $r = \\%ENV;", "references")),
            ("hash_array_deref", ("my $x = $h->{k}[0];", "hash_array_deref")),
            ("pod", ("=pod\ntext\n=cut\nmy $x = 1;", "pod")),
            ("data_section", ("__DATA__\nvalue\n", "data_section")),
            ("recovery", ("my $x = ;", "recovery")),
        ]);

        let mut fixtures = Vec::new();
        for (name, (source, concept)) in samples {
            let fixture = fixtures_root.join(format!("{name}.pl"));
            let meta = fixtures_root.join(format!("{name}.meta.toml"));
            fs::write(&fixture, source)?;
            let expected =
                if concept == "recovery" { "recover_without_panic" } else { "parse_clean" };
            fs::write(
                &meta,
                format!(
                    "concepts = [\"{concept}\"]\nprofile = [\"pr\"]\nexpected = \"{expected}\"\n"
                ),
            )?;
            fixtures.push(fixture);
        }

        let receipt = evaluate_fixtures(dir.path(), &fixtures)?;
        assert!(receipt.missing_concepts.is_empty());
        assert!(receipt.violations.is_empty());
        Ok(())
    }
}
