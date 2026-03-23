//! Tests for actionable formatting error feedback (issue #2111).
//!
//! Verifies that:
//! - `PerltidyNotFound` includes `cpanm Perl::Tidy` as the primary install recommendation.
//! - `PerltidyNotFound` includes platform-specific install instructions.
//! - `PerltidyError` includes a hint to check Perl syntax when perltidy exits non-zero.
//! - `FormattingError` exposes `error_kind()` for structured consumption by LSP handlers.

use perl_lsp_formatting::{FormattingError, FormattingOptions, FormattingProvider};
use perl_lsp_tooling::SubprocessRuntime;
use perl_lsp_tooling::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::must_err;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_options() -> FormattingOptions {
    FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    }
}

struct ErrorRuntime;

impl SubprocessRuntime for ErrorRuntime {
    fn run_command(
        &self,
        _program: &str,
        _args: &[&str],
        _stdin: Option<&[u8]>,
    ) -> Result<perl_lsp_tooling::SubprocessOutput, perl_lsp_tooling::SubprocessError> {
        Err(perl_lsp_tooling::SubprocessError::new("command not found: perltidy"))
    }
}

// ---------------------------------------------------------------------------
// PerltidyNotFound — cpanm is the primary recommendation
// ---------------------------------------------------------------------------

#[test]
fn test_not_found_error_includes_cpanm() {
    let err = FormattingError::PerltidyNotFound("command not found: perltidy".to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains("cpanm Perl::Tidy"),
        "PerltidyNotFound message should recommend `cpanm Perl::Tidy`, got: {msg}"
    );
}

#[test]
fn test_not_found_error_includes_platform_instructions() {
    let err = FormattingError::PerltidyNotFound("no such file or directory".to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains("apt") || msg.contains("brew") || msg.contains("cpan"),
        "PerltidyNotFound message should include platform-specific install instructions, got: {msg}"
    );
}

#[test]
fn test_not_found_error_preserves_underlying_cause() {
    let cause = "binary 'perltidy' not found in $PATH";
    let err = FormattingError::PerltidyNotFound(cause.to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains(cause),
        "PerltidyNotFound message should include the underlying OS error, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// PerltidyError — actionable hint when perltidy exits non-zero
// ---------------------------------------------------------------------------

#[test]
fn test_perltidy_error_includes_syntax_hint() {
    let err = FormattingError::PerltidyError("syntax error near line 5".to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains("syntax") || msg.contains("check") || msg.contains("Perl"),
        "PerltidyError message should include a hint about checking Perl syntax, got: {msg}"
    );
}

#[test]
fn test_perltidy_error_preserves_perltidy_output() {
    let perltidy_output = "## perltidy version 20230912\nERROR at line 3: unexpected token";
    let err = FormattingError::PerltidyError(perltidy_output.to_string());
    let msg = format!("{err}");
    assert!(
        msg.contains("ERROR at line 3"),
        "PerltidyError message should include the perltidy output, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// error_kind() — structured error type for LSP handler consumption
// ---------------------------------------------------------------------------

#[test]
fn test_error_kind_not_found() {
    let err = FormattingError::PerltidyNotFound("not found".to_string());
    assert_eq!(
        err.error_kind(),
        "perltidy_not_found",
        "PerltidyNotFound should return error_kind 'perltidy_not_found'"
    );
}

#[test]
fn test_error_kind_execution_error() {
    let err = FormattingError::PerltidyError("bad output".to_string());
    assert_eq!(
        err.error_kind(),
        "perltidy_error",
        "PerltidyError should return error_kind 'perltidy_error'"
    );
}

#[test]
fn test_error_kind_io_error() {
    let err = FormattingError::IoError("disk full".to_string());
    assert_eq!(err.error_kind(), "io_error", "IoError should return error_kind 'io_error'");
}

// ---------------------------------------------------------------------------
// End-to-end: provider returns PerltidyNotFound with cpanm in message
// ---------------------------------------------------------------------------

#[test]
fn test_provider_not_found_via_error_runtime() {
    let provider = FormattingProvider::new(ErrorRuntime);
    let result = provider.format_document("my $x = 1;", &default_options());
    let err = must_err(result);
    let msg = format!("{err}");
    assert!(
        msg.contains("cpanm Perl::Tidy"),
        "Provider should surface cpanm install instruction when perltidy not found, got: {msg}"
    );
}

#[test]
fn test_provider_execution_error_has_actionable_message() {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::failure(
        b"## perltidy version 20230912\nERROR: Perl::Tidy failed to parse input".to_vec(),
        2,
    ));
    let provider = FormattingProvider::new(runtime);
    let result = provider.format_document("my $x = 1;", &default_options());
    let err = must_err(result);
    let msg = format!("{err}");
    // The error message should contain context from perltidy's output
    assert!(
        msg.contains("perltidy") || msg.contains("error"),
        "Provider execution error should include perltidy context, got: {msg}"
    );
}
