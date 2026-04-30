use anyhow::{Context, Result};
use perl_corpus::{ConceptRegistry, discover_sidecars, load_and_validate_sidecar};
use std::path::PathBuf;

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(root) = manifest_dir.parent().and_then(|path| path.parent()) else {
        anyhow::bail!("unable to discover workspace root from {}", manifest_dir.display());
    };
    Ok(root.to_path_buf())
}

#[test]
fn seeded_sidecars_parse_and_validate() -> Result<()> {
    let root = workspace_root()?;
    let sidecar_root = root.join("tests/perl-corpus");
    let sidecars = discover_sidecars(&sidecar_root)?;
    assert!(!sidecars.is_empty(), "expected seeded parser sidecars");

    let registry_path = sidecar_root.join("concepts.toml");
    let registry = if registry_path.exists() {
        Some(
            ConceptRegistry::load(&registry_path)
                .with_context(|| format!("loading registry {}", registry_path.display()))?,
        )
    } else {
        None
    };

    for sidecar in &sidecars {
        let validation = load_and_validate_sidecar(sidecar, registry.as_ref())
            .with_context(|| format!("sidecar should load and validate: {}", sidecar.display()))?;

        assert!(
            !validation.errors.iter().any(|error| error.contains("fixture file does not exist")),
            "fixture file should exist for {}\nerrors: {:?}",
            sidecar.display(),
            validation.errors,
        );

        if registry.is_none() {
            assert!(
                validation
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("concept resolution pending")),
                "missing registry should report pending concept resolution for {}\nwarnings: {:?}",
                sidecar.display(),
                validation.warnings,
            );
        }
    }

    Ok(())
}
