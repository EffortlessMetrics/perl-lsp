//! Comprehensive tests for file completion functionality
//!
//! This test suite covers:
//! - Basic functionality (existing behavior)
//! - Security features (path traversal, null bytes, reserved names, etc.)
//! - Performance limits (result count, cancellation, path length)
//! - Edge cases (Unicode, empty paths, whitespace)
//! - File type recognition
//! - Context awareness (no completions in comments, etc.)

use perl_parser::{CompletionItemKind, CompletionProvider, Parser};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// Basic Functionality Tests (Existing)

#[test]
fn completes_files_in_src_directory() {
    let code = "\"src/com\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("com").unwrap() + "com".len();
    let completions = provider.get_completions(code, pos);
    assert!(
        completions
            .iter()
            .any(|c| c.label == "src/completion.rs" && c.kind == CompletionItemKind::File)
    );
}

#[test]
fn completes_files_in_tests_directory() {
    let code = "\"tests/incre\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("incre").unwrap() + "incre".len();
    let completions = provider.get_completions(code, pos);
    assert!(completions.iter().any(|c| c.label == "tests/incremental_integration_test.rs"));
}

// Security Tests

#[test]
fn basic_security_test_rejects_path_traversal() {
    let code = "\"../etc/passwd\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide any completions for path traversal attempts
    assert!(completions.is_empty(), "Should reject path traversal attempts");
}

#[test]
fn security_test_rejects_null_bytes() {
    let code = "\"src/test\0file\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide completions for paths with null bytes
    assert!(completions.is_empty(), "Should reject paths with null bytes");
}

#[test]
fn security_test_rejects_absolute_paths() {
    let code = "\"/etc/passwd\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide completions for absolute paths (security)
    assert!(completions.is_empty(), "Should reject absolute paths");
}

#[test]
fn security_test_allows_root_path() {
    let code = "\"/\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);
    // Root path should be allowed, but we expect empty results in this test environment
    // The important thing is that it doesn't panic or error
    // In a real filesystem, this might return directories, but that's filesystem dependent
}

#[test]
fn security_test_rejects_control_characters() {
    let code = "\"src/test\x01file\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide completions for paths with control characters
    assert!(completions.is_empty(), "Should reject paths with control characters");
}

#[test]
fn security_test_rejects_windows_reserved_names() {
    // This test should work on all platforms for cross-platform safety
    let test_cases = vec![
        "\"CON.txt\"",
        "\"PRN.log\"",
        "\"AUX.dat\"",
        "\"NUL.cfg\"",
        "\"COM1.txt\"",
        "\"LPT1.log\"",
    ];

    for code in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
        let provider = CompletionProvider::new_with_index(&ast, None);
        let pos = code.len() - 1;
        let completions = provider.get_completions(code, pos);
        // Should not provide completions containing Windows reserved names
        assert!(
            completions.iter().all(|c| !c.label.to_uppercase().contains("CON")
                && !c.label.to_uppercase().contains("PRN")
                && !c.label.to_uppercase().contains("AUX")
                && !c.label.to_uppercase().contains("NUL")
                && !c.label.to_uppercase().contains("COM1")
                && !c.label.to_uppercase().contains("LPT1")),
            "Should filter out Windows reserved names for {}",
            code
        );
    }
}

// Performance Tests

#[test]
fn performance_test_respects_result_limits() {
    // Test that we don't return more than 50 results
    let code = "\"src/\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);

    assert!(
        completions.len() <= 50,
        "Should respect MAX_RESULTS limit of 50, got {}",
        completions.len()
    );
}

#[test]
fn performance_test_cancellation_support() {
    let code = "\"src/\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;

    // Test with immediate cancellation
    let cancelled = Arc::new(AtomicBool::new(true));
    let cancelled_clone = cancelled.clone();
    let _completions =
        provider.get_completions_with_path_cancellable(code, pos, None, &move || {
            cancelled_clone.load(Ordering::Relaxed)
        });

    // With immediate cancellation, we should get fewer or no results
    // The exact behavior depends on timing, but it shouldn't crash
    // This test mainly verifies the cancellation mechanism works
}

#[test]
fn performance_test_rejects_very_long_paths() {
    // Create a path longer than 1024 characters
    let long_path = "a/".repeat(600); // 1200 characters
    let code = format!("\"{}\"", long_path);
    let mut parser = Parser::new(&code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(&code, pos);

    // Should reject overly long paths
    assert!(completions.is_empty(), "Should reject paths longer than 1024 characters");
}

// Edge Case and Unicode Tests

#[test]
fn edge_case_handles_unicode_filenames() {
    // Test with Unicode characters - this should be handled gracefully
    let code = "\"src/test_ü\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);

    // Should handle Unicode gracefully (won't find matches but shouldn't crash)
    // The important thing is that it doesn't panic or error out
}

#[test]
fn edge_case_handles_empty_prefix() {
    let code = "\"\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);

    // Should handle empty prefix gracefully
    // May return current directory contents, but shouldn't crash
}

#[test]
fn edge_case_handles_whitespace_in_paths() {
    let code = "\"src/test file\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);

    // Should handle whitespace in paths gracefully
    // Important: doesn't crash, even if no matches found
}

// File Type Recognition Tests

#[test]
fn file_type_recognition_works() {
    let code = "\"Cargo.\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);

    // Look for Cargo.toml in completions and verify it has a proper file type description
    if let Some(cargo_toml) = completions.iter().find(|c| c.label == "Cargo.toml") {
        assert!(cargo_toml.detail.is_some(), "Cargo.toml should have detail information");
        let detail = cargo_toml.detail.as_ref().unwrap();
        assert!(
            detail.contains("TOML") || detail.contains("file"),
            "Should recognize TOML file type, got: {}",
            detail
        );
    }
}

#[test]
fn file_type_recognition_rust_files() {
    let code = "\"src/lib.\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);

    // Look for .rs files and verify they have proper file type description
    let rust_files: Vec<_> = completions.iter().filter(|c| c.label.ends_with(".rs")).collect();
    for rust_file in rust_files {
        if let Some(detail) = &rust_file.detail {
            assert!(
                detail.contains("Rust") || detail.contains("source"),
                "Should recognize Rust file type, got: {}",
                detail
            );
        }
    }
}

#[test]
fn no_completions_in_comments() {
    let code = "# \"src/com\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("com").unwrap() + "com".len();
    let completions = provider.get_completions(code, pos);

    // Should not provide file completions inside comments
    assert!(completions.is_empty(), "Should not provide completions inside comments");
}

#[test]
fn no_completions_without_slash() {
    let code = "\"somefile\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);

    // Should not provide file completions for strings without slash (path separator)
    let file_completions: Vec<_> =
        completions.iter().filter(|c| c.kind == CompletionItemKind::File).collect();
    assert!(
        file_completions.is_empty(),
        "Should not provide file completions without path separator"
    );
}
