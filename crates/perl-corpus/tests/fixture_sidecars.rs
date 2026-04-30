use perl_corpus::fixture_expectations::{ExpectationMode, parse_sidecar, validate_sidecars_in_dir};
use std::error::Error;
use std::path::Path;

#[test]
fn sidecars_parse_for_seed_fixtures() -> Result<(), Box<dyn Error>> {
    let root = Path::new("tests/perl-corpus");
    let known_modes = [
        ExpectationMode::ParseClean,
        ExpectationMode::RecoverWithoutPanic,
        ExpectationMode::ExpectedError,
        ExpectationMode::TokenOnly,
        ExpectationMode::SpanOnly,
    ];

    for mode in known_modes {
        let matches_mode = validate_sidecars_in_dir(root, None)?
            .iter()
            .filter_map(|validation| parse_sidecar(&validation.sidecar_path).ok())
            .any(|sidecar| sidecar.expect.mode == mode);
        assert!(matches_mode, "missing seed sidecar for mode");
    }

    Ok(())
}

#[test]
fn sidecars_validate_without_hard_failing_on_missing_registry() -> Result<(), Box<dyn Error>> {
    let root = Path::new("tests/perl-corpus");
    let validations = validate_sidecars_in_dir(root, None)?;

    assert!(!validations.is_empty());
    assert!(validations.iter().all(|validation| validation.is_valid()));
    assert!(
        validations
            .iter()
            .all(|validation| validation.warnings.iter().any(|w| w.contains("resolution pending")))
    );

    Ok(())
}

#[test]
fn sidecars_have_matching_fixture_files() -> Result<(), Box<dyn Error>> {
    let root = Path::new("tests/perl-corpus");
    let validations = validate_sidecars_in_dir(root, None)?;

    for validation in validations {
        assert!(validation.fixture_path.exists());
    }

    Ok(())
}
