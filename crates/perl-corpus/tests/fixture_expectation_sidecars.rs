use anyhow::{Context, Result};
use perl_corpus::fixture_expectations::{
    ConceptRegistry, ValidationIssue, discover_sidecars, parse_sidecar, validate_sidecar,
};
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
    let sidecars = discover_sidecars(&sidecar_root);
    assert!(!sidecars.is_empty(), "expected seeded parser sidecars");

    let registry_path = sidecar_root.join("concepts.toml");
    let registry = ConceptRegistry::from_optional_file(&registry_path)?;

    for sidecar in sidecars {
        let expectation = parse_sidecar(&sidecar)
            .with_context(|| format!("sidecar should parse: {}", sidecar.display()))?;
        let report = validate_sidecar(&sidecar, &expectation, registry.as_ref());

        assert!(
            !report
                .issues
                .iter()
                .any(|issue| matches!(issue, ValidationIssue::MissingFixtureFile(_))),
            "fixture file should exist for {}",
            sidecar.display()
        );

        if registry.is_none() {
            assert!(
                report
                    .issues
                    .iter()
                    .any(|issue| matches!(issue, ValidationIssue::PendingConceptRegistry(_))),
                "missing registry should report pending concept resolution for {}",
                sidecar.display()
            );
        }
    }

    Ok(())
}
