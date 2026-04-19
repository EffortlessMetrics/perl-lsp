use perl_lsp_perltidy::{PerlTidyConfig, PerlTidyFormatter};
use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};
use std::path::Path;
use std::sync::Arc;

// --- Missing perltidy binary handling ---

/// A mock runtime that always returns an error, simulating a missing binary.
struct MissingBinaryRuntime;

impl SubprocessRuntime for MissingBinaryRuntime {
    fn run_command(
        &self,
        program: &str,
        _args: &[&str],
        _stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError> {
        Err(SubprocessError::new(format!(
            "Failed to start {program}: No such file or directory"
        )))
    }
}

#[test]
fn format_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format("my $x = 1;");

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

#[test]
fn format_file_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format_file(Path::new("test.pl"));

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

#[test]
fn format_range_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "line0\nline1\nline2";
    let result = formatter.format_range(code, 1, 1);

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

#[test]
fn get_suggestions_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.get_suggestions("my $x = 1;");

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

// ──────────────────────────── PerlTidyConfig: timeout field ──────────────────

#[test]
fn perltidy_config_default_has_timeout() {
    let config = PerlTidyConfig::default();
    assert_eq!(config.timeout_secs, 10);
}
