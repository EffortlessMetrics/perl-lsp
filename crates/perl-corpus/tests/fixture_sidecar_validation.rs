use perl_corpus::{ConceptRegistry, ValidationNote, validate_sidecars_under};
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }

        if let Ok(contents) = std::fs::read_to_string(&cargo_toml)
            && contents.contains("[workspace]")
        {
            return ancestor.to_path_buf();
        }
    }

    manifest_dir
}

#[test]
fn validates_seed_sidecars_without_hard_failing_when_registry_is_absent()
-> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let sidecar_root = root.join("tests/perl-corpus");
    let concept_registry_path = root.join("tests/perl-corpus/concepts.toml");

    let concept_registry = ConceptRegistry::load_if_present(&concept_registry_path)?;
    let (_fixtures, notes) = validate_sidecars_under(&sidecar_root, concept_registry.as_ref())?;

    if concept_registry.is_none() {
        assert!(!notes.is_empty(), "Expected concept resolution pending notes without registry");
        assert!(
            notes
                .iter()
                .all(|note| matches!(note, ValidationNote::ConceptResolutionPending { .. }))
        );
    }

    Ok(())
}
