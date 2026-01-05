//! Crash reproducer tests for heredoc edge cases.
//!
//! These tests reproduce known parser hangs and are disabled by default.
//! To run them: `cargo test -p perl-parser --features crash-repros --test crash_reproducer_test`

#[cfg(feature = "crash-repros")]
use perl_parser::Parser;

#[test]
#[cfg(feature = "crash-repros")]
fn test_crash_reproducer_b6dd6f9afe3c18f3efa0b5bb8454be7744f2a458() {
    // This is the crash case found in perl-corpus/fuzz/
    // Input: "xqN<<\""
    // This likely triggered the original off-by-one error in heredoc delimiter parsing
    let crash_input = "xqN<<\"";

    println!("Testing crash reproducer: {}", crash_input);

    // Should not panic after the boundary fix in parse_heredoc_delimiter
    let result = std::panic::catch_unwind(|| {
        let mut parser = Parser::new(crash_input);
        parser.parse()
    });

    assert!(result.is_ok(), "Crash reproducer should not panic after boundary fix");
}

#[test]
#[cfg(feature = "crash-repros")]
fn test_related_heredoc_edge_cases() {
    // Test variations of the crash pattern to ensure robustness
    let edge_cases = [
        "xqN<<\"", // Original crash case
        "xqN<<'",  // Single quote variant
        "abc<<\"", // Different prefix
        "x<<\"",   // Minimal prefix
        "<<\"",    // No prefix
    ];

    for (i, case) in edge_cases.iter().enumerate() {
        println!("Testing edge case {}: {}", i + 1, case);

        let result = std::panic::catch_unwind(|| {
            let mut parser = Parser::new(case);
            parser.parse()
        });

        assert!(result.is_ok(), "Edge case {} should not panic: {}", i + 1, case);
    }
}
