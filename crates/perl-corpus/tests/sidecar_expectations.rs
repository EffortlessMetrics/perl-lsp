use perl_corpus::{
    ConceptRegistry, SidecarValidation, discover_sidecars, load_and_validate_sidecar, parse_sidecar,
};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn Error>> {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    path.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_file(path: &std::path::Path, contents: &str) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn valid_sidecar(concept_id: &str) -> String {
    format!(
        r#"[concept]
id = "{concept_id}"
tier = "pr"

[expect]
panic = false
timeout = false
mode = "recover_without_panic"

[metrics]
max_error_nodes = 2
must_emit_node_kinds = ["SubDecl", "Block"]

[snapshots]
tokens = false
ast = true
spans = true
"#
    )
}

#[test]
fn sidecar_parse_and_validate_succeeds_without_registry() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("perl_corpus_sidecar_no_registry")?;
    let meta = root.join("tests/perl-corpus/recovery/missing_brace.meta.toml");
    let fixture = root.join("tests/perl-corpus/recovery/missing_brace.pl");

    write_file(&fixture, "sub x { my $v = 1;\n")?;
    write_file(&meta, &valid_sidecar("parser.recovery.missing_closing_brace"))?;

    let parsed = parse_sidecar(&meta)?;
    assert_eq!(parsed.concept.id, "parser.recovery.missing_closing_brace");

    let validation = load_and_validate_sidecar(&meta, None)?;
    assert!(validation.is_ok());
    assert_eq!(validation.warnings.len(), 1);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn sidecar_requires_fixture_file_to_exist() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("perl_corpus_sidecar_missing_fixture")?;
    let meta = root.join("tests/perl-corpus/recovery/missing_brace.meta.toml");

    write_file(&meta, &valid_sidecar("parser.recovery.missing_closing_brace"))?;

    let validation = load_and_validate_sidecar(&meta, None)?;
    assert!(!validation.is_ok());
    assert!(validation.errors.iter().any(|item| item.contains("fixture file does not exist")));

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn sidecar_validates_concept_id_when_registry_is_present() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("perl_corpus_sidecar_registry")?;
    let meta = root.join("tests/perl-corpus/recovery/missing_brace.meta.toml");
    let fixture = root.join("tests/perl-corpus/recovery/missing_brace.pl");
    let registry_path = root.join("concept-registry.toml");

    write_file(&fixture, "sub x { my $v = 1;\n")?;
    write_file(&meta, &valid_sidecar("parser.recovery.missing_closing_brace"))?;
    write_file(&registry_path, "[[concepts]]\nid = \"parser.recovery.missing_closing_brace\"\n")?;

    let registry = ConceptRegistry::load(&registry_path)?;
    let validation = load_and_validate_sidecar(&meta, Some(&registry))?;
    assert_eq!(validation, SidecarValidation::default());

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn discover_sidecars_finds_nested_meta_files() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("perl_corpus_sidecar_discovery")?;
    write_file(
        &root.join("tests/perl-corpus/recovery/one.meta.toml"),
        &valid_sidecar("parser.recovery.missing_closing_brace"),
    )?;
    write_file(
        &root.join("tests/perl-corpus/recovery/two.meta.toml"),
        &valid_sidecar("parser.recovery.missing_delimiter"),
    )?;

    let sidecars = discover_sidecars(&root)?;
    assert_eq!(sidecars.len(), 2);

    fs::remove_dir_all(&root)?;
    Ok(())
}
