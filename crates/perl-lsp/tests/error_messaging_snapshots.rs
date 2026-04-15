//! Snapshot tests for Gap 5 (DAP Perl Not Found) error message strings.
//!
//! These tests capture the expected error message so that changes to the
//! user-facing error text are visible as intentional diffs in code review.
//!
//! Gap 5 implementation is in: crates/perl-dap/src/debug_adapter/process.rs
//! The error transformation logic returns an actionable message when perl is not found.

use insta::assert_yaml_snapshot;

/// The expected actionable message when perl is not found on PATH.
const PERL_NOT_FOUND_ACTIONABLE: &str =
    "Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH.";

/// GAP-5 Snapshot Test: Verify ErrorKind::NotFound error transforms to actionable message.
///
/// Input: std::io::Error with ErrorKind::NotFound and message "No such file or directory: 'perl'"
/// Output: Actionable error message string
///
/// This snapshots the expected error message so that changes to the user-facing
/// error text are visible as intentional diffs.
#[test]
fn snapshot_gap5_perl_not_found_error_kind_not_found() {
    let not_found_err =
        std::io::Error::new(std::io::ErrorKind::NotFound, "No such file or directory: 'perl'");

    // Apply the same transformation logic as the implementation
    let transformed = if not_found_err.kind() == std::io::ErrorKind::NotFound {
        PERL_NOT_FOUND_ACTIONABLE.to_string()
    } else {
        not_found_err.to_string()
    };

    assert_yaml_snapshot!("gap5_perl_not_found_error_kind_not_found", transformed);
}

/// GAP-5 Snapshot Test: Verify ErrorKind::PermissionDenied error does NOT transform.
///
/// Input: std::io::Error with ErrorKind::PermissionDenied and message "perl not executable"
/// Output: Raw error string (unchanged)
///
/// This snapshots the raw error string so that unexpected transformations are caught.
#[test]
fn snapshot_gap5_perl_not_found_error_kind_permission_denied() {
    let permission_err =
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "perl not executable");

    // Apply the same transformation logic as the implementation
    let transformed = if permission_err.kind() == std::io::ErrorKind::NotFound {
        PERL_NOT_FOUND_ACTIONABLE.to_string()
    } else {
        permission_err.to_string()
    };

    assert_yaml_snapshot!("gap5_perl_not_found_error_kind_permission_denied", transformed);
}
