//! DAP Breakpoint Matrix Tests (AC7, AC14)
//!
//! Tests for breakpoint verification edge cases across file boundaries, comments, blank lines, heredocs, BEGIN/END blocks
//!
//! Specification: docs/DAP_IMPLEMENTATION_SPECIFICATION.md#ac7-breakpoint-management
//!
//! Run with: cargo test -p perl-dap

use anyhow::Result;
use perl_dap::breakpoints::BreakpointStore;
use perl_dap::protocol::{SetBreakpointsArguments, Source, SourceBreakpoint};
use std::io::Write;
use tempfile::NamedTempFile;
use perl_tdd_support::must;

/// Helper to create a temporary Perl file and set breakpoints
fn create_test_file_and_set_breakpoints(
    content: &str,
    lines: Vec<i64>,
) -> Result<Vec<perl_dap::protocol::Breakpoint>> {
    // Create temporary file
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(content.as_bytes())?;
    temp_file.flush()?;

    // Get the file path
    let path = temp_file
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert path to string"))?
        .to_string();

    // Create breakpoint store
    let store = BreakpointStore::new();

    // Create breakpoint arguments
    let source_breakpoints: Vec<SourceBreakpoint> = lines
        .iter()
        .map(|&line| SourceBreakpoint { line, column: None, condition: None })
        .collect();

    let args = SetBreakpointsArguments {
        source: Source { path: Some(path.clone()), name: Some("test.pl".to_string()) },
        breakpoints: Some(source_breakpoints),
        source_modified: None,
    };

    // Set breakpoints
    let breakpoints = store.set_breakpoints(&args);

    // Keep temp file alive until we're done
    std::mem::forget(temp_file);

    Ok(breakpoints)
}

/// AC7.1: Breakpoints on comment-only lines should be rejected
#[test]
fn test_breakpoint_on_comment_line() -> Result<()> {
    let source = r#"#!/usr/bin/perl
# This is a comment
my $x = 42;
"#;

    let breakpoints = create_test_file_and_set_breakpoints(source, vec![2])?;

    assert_eq!(breakpoints.len(), 1);
    assert!(!breakpoints[0].verified, "Breakpoint on comment line should be unverified");
    let message = breakpoints[0]
        .message
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Expected breakpoint message"))?;
    assert!(message.contains("comment"));

    Ok(())
}

/// AC7.1: Breakpoints on blank lines should be rejected
#[test]
fn test_breakpoint_on_blank_line() -> Result<()> {
    let source = r#"my $x = 42;

my $y = 100;
"#;

    let breakpoints = create_test_file_and_set_breakpoints(source, vec![2])?;

    assert_eq!(breakpoints.len(), 1);
    assert!(!breakpoints[0].verified, "Breakpoint on blank line should be unverified");
    let message = breakpoints[0]
        .message
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Expected breakpoint message"))?;
    assert!(message.contains("blank"));

    Ok(())
}

/// AC7.3: Breakpoints on executable lines should be verified
#[test]
fn test_breakpoint_on_executable_line() -> Result<()> {
    let source = r#"# Comment
my $x = 42;
print $x;
"#;

    let breakpoints = create_test_file_and_set_breakpoints(source, vec![2, 3])?;

    assert_eq!(breakpoints.len(), 2);
    assert!(breakpoints[0].verified, "Breakpoint on executable line should be verified");
    assert!(breakpoints[1].verified, "Breakpoint on executable line should be verified");

    Ok(())
}

/// AC7.3: Breakpoints inside heredoc interiors should be rejected
#[test]
fn test_breakpoint_inside_heredoc() -> Result<()> {
    let source = r#"my $doc = <<'END';
This is heredoc content
More content here
END
my $x = 42;
"#;

    let breakpoints = create_test_file_and_set_breakpoints(source, vec![2, 3, 5])?;

    assert_eq!(breakpoints.len(), 3);

    // Lines 2 and 3 are inside heredoc - should be unverified
    assert!(!breakpoints[0].verified, "Breakpoint inside heredoc should be unverified");
    let message_0 = breakpoints[0]
        .message
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Expected breakpoint message for line 2"))?;
    assert!(message_0.contains("heredoc"));

    assert!(!breakpoints[1].verified, "Breakpoint inside heredoc should be unverified");
    let message_1 = breakpoints[1]
        .message
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Expected breakpoint message for line 3"))?;
    assert!(message_1.contains("heredoc"));

    // Line 5 is executable code - should be verified
    assert!(breakpoints[2].verified, "Breakpoint on executable line should be verified");

    Ok(())
}

/// AC7.5: Multiple breakpoints validation in single request
#[test]
fn test_multiple_breakpoints_validation() -> Result<()> {
    let source = r#"#!/usr/bin/perl
# Comment line
my $x = 42;

print $x;
# Another comment
my $y = 100;
"#;

    let breakpoints = create_test_file_and_set_breakpoints(source, vec![1, 2, 3, 4, 5, 6, 7])?;

    assert_eq!(breakpoints.len(), 7);

    // Line 1: shebang comment - unverified
    assert!(!breakpoints[0].verified);

    // Line 2: comment - unverified
    assert!(!breakpoints[1].verified);

    // Line 3: executable - verified
    assert!(breakpoints[2].verified);

    // Line 4: blank - unverified
    assert!(!breakpoints[3].verified);

    // Line 5: executable - verified
    assert!(breakpoints[4].verified);

    // Line 6: comment - unverified
    assert!(!breakpoints[5].verified);

    // Line 7: executable - verified
    assert!(breakpoints[6].verified);

    Ok(())
}

/// AC7.1: Whitespace-only lines should be rejected
#[test]
fn test_breakpoint_on_whitespace_line() -> Result<()> {
    let source = "my $x = 42;\n    \t  \nmy $y = 100;\n";

    let breakpoints = create_test_file_and_set_breakpoints(source, vec![2])?;

    assert_eq!(breakpoints.len(), 1);
    assert!(!breakpoints[0].verified, "Breakpoint on whitespace-only line should be unverified");

    Ok(())
}

/// AC7.3: Inline comments should not invalidate breakpoints
#[test]
fn test_breakpoint_on_line_with_inline_comment() -> Result<()> {
    let source = "my $x = 42; # This is an inline comment\n";

    let breakpoints = create_test_file_and_set_breakpoints(source, vec![1])?;

    assert_eq!(breakpoints.len(), 1);
    assert!(
        breakpoints[0].verified,
        "Breakpoint on line with code and inline comment should be verified"
    );

    Ok(())
}

/// AC7: REPLACE semantics - clearing old breakpoints
#[test]
fn test_breakpoint_replace_semantics_with_validation() -> Result<()> {
    let source = r#"# Comment
my $x = 42;
my $y = 100;
"#;

    // Create temp file
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(source.as_bytes())?;
    temp_file.flush()?;
    let path = temp_file
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert path to string"))?
        .to_string();

    let store = BreakpointStore::new();

    // Set initial breakpoints
    let args1 = SetBreakpointsArguments {
        source: Source { path: Some(path.clone()), name: Some("test.pl".to_string()) },
        breakpoints: Some(vec![SourceBreakpoint { line: 2, column: None, condition: None }]),
        source_modified: None,
    };
    store.set_breakpoints(&args1);

    // Replace with new breakpoints
    let args2 = SetBreakpointsArguments {
        source: Source { path: Some(path.clone()), name: Some("test.pl".to_string()) },
        breakpoints: Some(vec![
            SourceBreakpoint { line: 1, column: None, condition: None },
            SourceBreakpoint { line: 3, column: None, condition: None },
        ]),
        source_modified: None,
    };
    let breakpoints = store.set_breakpoints(&args2);

    assert_eq!(breakpoints.len(), 2);
    assert!(!breakpoints[0].verified, "Line 1 is comment, should be unverified");
    assert!(breakpoints[1].verified, "Line 3 is executable, should be verified");

    // Verify stored breakpoints
    let stored = store.get_breakpoints(&path);
    assert_eq!(stored.len(), 2, "Should have exactly 2 breakpoints after REPLACE");

    Ok(())
}

/// AC7: File not found should mark breakpoints as unverified
#[test]
fn test_breakpoint_file_not_found() -> Result<()> {
    let store = BreakpointStore::new();

    let args = SetBreakpointsArguments {
        source: Source {
            path: Some("/nonexistent/file.pl".to_string()),
            name: Some("file.pl".to_string()),
        },
        breakpoints: Some(vec![SourceBreakpoint { line: 1, column: None, condition: None }]),
        source_modified: None,
    };

    let breakpoints = store.set_breakpoints(&args);

    assert_eq!(breakpoints.len(), 1);
    assert!(!breakpoints[0].verified, "Breakpoint on nonexistent file should be unverified");
    let message = breakpoints[0]
        .message
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Expected breakpoint message"))?;
    assert!(message.contains("Unable to read"));

    Ok(())
}

#[cfg(feature = "dap-phase2")]
mod dap_breakpoint_matrix_phase2 {
    use super::*;

    /// Tests feature spec: DAP_BREAKPOINT_VALIDATION_GUIDE.md#file-boundaries
    #[tokio::test]
    #[ignore]
    // AC:14
    async fn test_breakpoints_file_boundaries() -> Result<()> {
        // First/last line breakpoint behavior
        must(Err::<(), _>("File boundary breakpoints not yet implemented (AC14)"));
        Ok(())
    }

    /// Tests feature spec: DAP_BREAKPOINT_VALIDATION_GUIDE.md#begin-end-blocks
    #[tokio::test]
    #[ignore]
    // AC:14
    async fn test_breakpoints_begin_end_blocks() -> Result<()> {
        // BEGIN/END block breakpoint validation
        must(Err::<(), _>("BEGIN/END block breakpoints not yet implemented (AC14)"));
        Ok(())
    }

    /// Tests feature spec: DAP_BREAKPOINT_VALIDATION_GUIDE.md#multiline-statements
    #[tokio::test]
    #[ignore]
    // AC:14
    async fn test_breakpoints_multiline_statements() -> Result<()> {
        // Multi-line statement breakpoint behavior
        must(Err::<(), _>("Multiline statement breakpoints not yet implemented (AC14)"));
        Ok(())
    }

    /// Tests feature spec: DAP_BREAKPOINT_VALIDATION_GUIDE.md#pod-documentation
    #[tokio::test]
    #[ignore]
    // AC:14
    async fn test_breakpoints_in_pod_documentation() -> Result<()> {
        // POD documentation breakpoint behavior
        must(Err::<(), _>("POD documentation breakpoints not yet implemented (AC14)"));
        Ok(())
    }

    /// Tests feature spec: DAP_BREAKPOINT_VALIDATION_GUIDE.md#string-literals
    #[tokio::test]
    #[ignore]
    // AC:14
    async fn test_breakpoints_in_string_literals() -> Result<()> {
        // String literal breakpoint behavior
        must(Err::<(), _>("String literal breakpoints not yet implemented (AC14)"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac14-performance-baselines
    #[test]
    #[ignore]
    // AC:14
    fn test_performance_benchmark_baselines() -> Result<()> {
        // Benchmark suite: small (100 lines), medium (1000 lines), large (10K+ lines)
        must(Err::<(), _>("Performance benchmark baselines not yet implemented (AC14)"));
        Ok(())
    }
}
