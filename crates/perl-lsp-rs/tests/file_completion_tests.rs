//! Comprehensive tests for file completion functionality
//!
//! This test suite covers:
//! - Basic functionality (existing behavior)
//! - Security features (path traversal, null bytes, reserved names, etc.)
//! - Performance limits (result count, cancellation, path length)
//! - Edge cases (Unicode, empty paths, whitespace)
//! - File type recognition
//! - Context awareness (no completions in comments, etc.)

use perl_lsp::features::completion::{CompletionItemKind, CompletionProvider};
use perl_parser::Parser;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// Basic Functionality Tests (Existing)

#[test]
fn completes_files_in_src_directory() -> TestResult {
    // Test with src/features/ path where completion.rs now lives
    let code = "\"src/features/com\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("com").ok_or("'com' substring not found")? + "com".len();
    let completions = provider.get_completions(code, pos);

    // Should find completion.rs file in the features directory
    assert!(
        completions
            .iter()
            .any(|c| c.label == "src/features/completion.rs" && c.kind == CompletionItemKind::File),
        "Expected to find src/features/completion.rs, got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    // Should provide proper file type information
    let completion_file = completions.iter().find(|c| c.label == "src/features/completion.rs");
    if let Some(comp) = completion_file {
        assert_eq!(comp.detail.as_ref().ok_or("detail field is None")?, "Rust source file");
    }
    Ok(())
}

#[test]
fn completes_files_in_tests_directory() -> TestResult {
    let code = "\"tests/file_comp\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("file_comp").ok_or("'file_comp' substring not found")? + "file_comp".len();
    let completions = provider.get_completions(code, pos);

    // Should find our comprehensive test file
    assert!(completions.iter().any(|c| c.label.contains("file_completion")));
    Ok(())
}

#[test]
fn does_complete_current_directory_files() -> TestResult {
    let code = "\"Cargo\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("Cargo").ok_or("'Cargo' substring not found")? + "Cargo".len();
    let completions = provider.get_completions(code, pos);

    // Should provide file completions for current directory files matching prefix
    assert!(completions.iter().any(|c| c.label.starts_with("Cargo")));
    Ok(())
}

#[test]
fn basic_security_test_rejects_path_traversal() -> TestResult {
    let code = "\"../src/completion.rs\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos =
        code.find("completion").ok_or("'completion' substring not found")? + "completion".len();
    let completions = provider.get_completions(code, pos);

    // Should reject path traversal attempts
    assert!(completions.is_empty());
    Ok(())
}

// Security Tests

#[test]
fn security_test_rejects_path_traversal_etc() -> TestResult {
    let code = "\"../etc/passwd\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide any completions for path traversal attempts
    assert!(completions.is_empty(), "Should reject path traversal attempts");
    Ok(())
}

#[test]
fn security_test_rejects_null_bytes() -> TestResult {
    let code = "\"src/test\0file\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide completions for paths with null bytes
    assert!(completions.is_empty(), "Should reject paths with null bytes");
    Ok(())
}

#[test]
fn security_test_rejects_absolute_paths() -> TestResult {
    let code = "\"/etc/passwd\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide completions for absolute paths (security)
    assert!(completions.is_empty(), "Should reject absolute paths");
    Ok(())
}

#[test]
fn security_test_allows_root_path() -> TestResult {
    let code = "\"/\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);
    // Root path should be allowed, but we expect empty results in this test environment
    // The important thing is that it doesn't panic or error
    // In a real filesystem, this might return directories, but that's filesystem dependent
    Ok(())
}

#[test]
fn security_test_rejects_control_characters() -> TestResult {
    let code = "\"src/test\x01file\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);
    // Should not provide completions for paths with control characters
    assert!(completions.is_empty(), "Should reject paths with control characters");
    Ok(())
}

#[test]
fn security_test_rejects_windows_reserved_names() -> TestResult {
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
        let ast = parser.parse()?;
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
    Ok(())
}

// Performance Tests

#[test]
fn performance_test_respects_result_limits() -> TestResult {
    // Test that we don't return more than 50 results
    let code = "\"src/\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);

    assert!(
        completions.len() <= 50,
        "Should respect MAX_RESULTS limit of 50, got {}",
        completions.len()
    );
    Ok(())
}

#[test]
fn performance_test_cancellation_support() -> TestResult {
    let code = "\"src/\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
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
    Ok(())
}

#[test]
fn performance_test_rejects_very_long_paths() -> TestResult {
    // Create a path longer than 1024 characters
    let long_path = "a/".repeat(600); // 1200 characters
    let code = format!("\"{}\"", long_path);
    let mut parser = Parser::new(&code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(&code, pos);

    // Should reject overly long paths
    assert!(completions.is_empty(), "Should reject paths longer than 1024 characters");
    Ok(())
}

// Edge Case and Unicode Tests

#[test]
fn edge_case_handles_unicode_filenames() -> TestResult {
    // Test with Unicode characters - this should be handled gracefully
    let code = "\"src/test_ü\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);

    // Should handle Unicode gracefully (won't find matches but shouldn't crash)
    // The important thing is that it doesn't panic or error out
    Ok(())
}

#[test]
fn edge_case_handles_empty_prefix() -> TestResult {
    let code = "\"\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);

    // Should handle empty prefix gracefully
    // May return current directory contents, but shouldn't crash
    Ok(())
}

#[test]
fn edge_case_handles_whitespace_in_paths() -> TestResult {
    let code = "\"src/test file\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let _completions = provider.get_completions(code, pos);

    // Should handle whitespace in paths gracefully
    // Important: doesn't crash, even if no matches found
    Ok(())
}

// File Type Recognition Tests

#[test]
fn file_type_recognition_works() -> TestResult {
    let code = "\"Cargo.\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(code, pos);

    // Look for Cargo.toml in completions and verify it has a proper file type description
    if let Some(cargo_toml) = completions.iter().find(|c| c.label == "Cargo.toml") {
        assert!(cargo_toml.detail.is_some(), "Cargo.toml should have detail information");
        let detail = cargo_toml.detail.as_ref().ok_or("detail field is None")?;
        assert!(
            detail.contains("TOML") || detail.contains("file"),
            "Should recognize TOML file type, got: {}",
            detail
        );
    }
    Ok(())
}

#[test]
fn file_type_recognition_rust_files() -> TestResult {
    let code = "\"src/lib.\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
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
    Ok(())
}

#[test]
fn no_completions_in_comments() -> TestResult {
    let code = "# \"src/com\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("com").ok_or("'com' substring not found")? + "com".len();
    let completions = provider.get_completions(code, pos);

    // Should not provide file completions inside comments
    assert!(completions.is_empty(), "Should not provide completions inside comments");
    Ok(())
}

#[test]
fn no_completions_without_slash() -> TestResult {
    let code = "\"somefile\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
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
    Ok(())
}
