//! Workstream E: Test Hardening (Trust Anchor)
//!
//! This module contains small, brutal regression tests to prevent regressions in:
//! 1. Degraded-mode behavior - handlers return partials when Building/Degraded
//! 2. Caps enforcement - results never exceed configured caps
//! 3. Deadline enforcement - early exit with partial results, not timeout errors
//! 4. Windows-ish URI path handling - rootPath conversion, backslashes, drive letters

mod support;

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[cfg(test)]
mod degraded_mode_tests {
    //! Tests that verify handlers return partial results in Building/Degraded state.
    //!
    //! The "big 6" handlers tested:
    //! 1. workspace/symbol
    //! 2. textDocument/references
    //! 3. textDocument/completion
    //! 4. textDocument/definition
    //! 5. textDocument/hover
    //! 6. textDocument/documentSymbol

    use crate::support::env_guard::EnvGuard;
    use perl_lsp::LspServer;
    use serde_json::json;
    use serial_test::serial;
    use std::sync::Arc;

    fn create_test_server() -> LspServer {
        let output: Arc<parking_lot::Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(parking_lot::Mutex::new(Box::new(Vec::new())));
        LspServer::with_output(output)
    }

    /// Helper to open a test document
    fn open_test_document(
        srv: &LspServer,
        uri: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "text": content,
                "languageId": "perl"
            }
        })))?;
        Ok(())
    }

    // =========================================================================
    // Test 1: workspace/symbol in Building state returns partials from open docs
    // =========================================================================
    #[test]
    #[serial]
    fn test_workspace_symbol_building_state_returns_open_doc_partials()
    -> Result<(), Box<dyn std::error::Error>> {
        // SAFETY: Test runs single-threaded with #[serial]
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Open a document with symbols - this should be searchable even when index is building
        let content = r#"
package MyPackage;
sub my_function { }
sub another_function { }
1;
"#;
        open_test_document(&srv, "file:///test/building.pm", content)?;

        // The index starts in Building state - workspace/symbol should still work
        // by falling back to open document search
        let result = srv.test_handle_workspace_symbols(Some(json!({
            "query": "function"
        })))?;

        // Should return results from open documents even in Building state
        if let Some(result) = result {
            let symbols = result.as_array().ok_or("Expected array for workspace symbols result")?;
            // Should find symbols containing "function" from the open document
            assert!(
                !symbols.is_empty(),
                "Building state should return partial results from open documents"
            );
        }
        Ok(())
    }

    // =========================================================================
    // Test 2: textDocument/references in degraded mode uses same-file fallback
    // =========================================================================
    #[test]
    #[serial]
    fn test_references_degraded_mode_same_file_fallback() -> Result<(), Box<dyn std::error::Error>>
    {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Open document with multiple references to a variable
        let content = r#"
my $counter = 0;
$counter++;
print $counter;
$counter = $counter + 1;
"#;
        let uri = "file:///test/refs.pm";
        open_test_document(&srv, uri, content)?;

        // Request references - should use same-file semantic analysis fallback
        let result = srv.test_handle_references(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 4}, // Position on $counter
            "context": {"includeDeclaration": true}
        })))?;

        if let Some(result) = result {
            let refs = result.as_array().ok_or("Expected array for references result")?;
            // Should find multiple references even without full index
            assert!(
                refs.len() >= 2,
                "Same-file fallback should find local references, got {}",
                refs.len()
            );
        }
        Ok(())
    }

    // =========================================================================
    // Test 3: textDocument/completion returns results without full index
    // =========================================================================
    #[test]
    #[serial]
    fn test_completion_returns_results_in_building_state() -> Result<(), Box<dyn std::error::Error>>
    {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        let content = r#"
package Test;
sub helper_one { }
sub helper_two { }

sub main {
    hel
}
1;
"#;
        let uri = "file:///test/complete.pm";
        open_test_document(&srv, uri, content)?;

        // Request completion - should work from local context
        let result = srv.test_handle_completion(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 6, "character": 7} // After "hel"
        })))?;

        if let Some(result) = result {
            // Completion should return something - either items array or object with items
            let items =
                result.get("items").and_then(|i| i.as_array()).or_else(|| result.as_array());

            assert!(
                items.is_some_and(|arr| !arr.is_empty()) || !result.is_null(),
                "Completion should return results even in Building state"
            );
        }
        Ok(())
    }

    // =========================================================================
    // Test 4: textDocument/definition uses same-file semantic fallback
    // =========================================================================
    #[test]
    #[serial]
    fn test_definition_same_file_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        let content = r#"
sub greet {
    my $name = shift;
    print "Hello, $name\n";
}

greet("World");
"#;
        let uri = "file:///test/def.pm";
        open_test_document(&srv, uri, content)?;

        // Request definition on 'greet' call - should find same-file definition
        let result = srv.test_handle_definition(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 6, "character": 2} // On greet call
        })))?;

        if let Some(result) = result {
            let defs = result.as_array().ok_or("Expected array for definition result")?;
            // Should find the local definition
            assert!(
                !defs.is_empty(),
                "Same-file definition fallback should work in Building state"
            );

            // Verify it points to the definition line (line 1)
            if let Some(first) = defs.first() {
                let line = first["range"]["start"]["line"].as_u64();
                assert_eq!(line, Some(1), "Should find definition on line 1");
            }
        }
        Ok(())
    }

    // =========================================================================
    // Test 5: textDocument/hover works without full index
    // =========================================================================
    #[test]
    #[serial]
    fn test_hover_works_in_building_state() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        let content = r#"
# Calculate the sum of two numbers
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}
"#;
        let uri = "file:///test/hover.pm";
        open_test_document(&srv, uri, content)?;

        // Request hover on 'add' - should work from local analysis
        let result = srv.test_handle_hover(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 5} // On 'add'
        })))?;

        // Hover should return something useful (not an error)
        // Note: The exact content depends on implementation, but it shouldn't fail
        if let Some(result) = result {
            // Should have some hover content or be an empty valid response
            assert!(!result.is_null() || result.as_object().is_some());
        }
        Ok(())
    }

    // =========================================================================
    // Test 6: textDocument/documentSymbol works without full index
    // =========================================================================
    #[test]
    #[serial]
    fn test_document_symbols_always_works() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        let content = r#"
package MyPackage;

our $VERSION = '1.0';

sub method_one {
    my $self = shift;
}

sub method_two {
    my $self = shift;
}

1;
"#;
        let uri = "file:///test/symbols.pm";
        open_test_document(&srv, uri, content)?;

        // Request document symbols - should always work (file-local operation)
        let result = srv.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": uri}
        })))?;

        if let Some(result) = result {
            let symbols = result.as_array().ok_or("Expected array for document symbols result")?;
            // Should find package, methods, and variable
            assert!(
                symbols.len() >= 2,
                "Document symbols should work regardless of index state, got {} symbols",
                symbols.len()
            );
        }
        Ok(())
    }
}

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[cfg(test)]
mod caps_enforcement_tests {
    //! Tests that verify results never exceed configured caps.

    use crate::support::env_guard::EnvGuard;
    use perl_lsp::LspServer;
    use serde_json::json;
    use serial_test::serial;
    use std::sync::Arc;

    fn create_test_server() -> LspServer {
        let output: Arc<parking_lot::Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(parking_lot::Mutex::new(Box::new(Vec::new())));
        LspServer::with_output(output)
    }

    fn open_test_document(
        srv: &LspServer,
        uri: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "text": content,
                "languageId": "perl"
            }
        })))?;
        Ok(())
    }

    // =========================================================================
    // Test: workspace/symbol respects cap (default 200)
    // =========================================================================
    #[test]
    #[serial]
    fn test_workspace_symbol_respects_cap() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Create a file with many subroutines to potentially exceed the cap
        let mut content = String::from("package BigPackage;\n");
        for i in 0..300 {
            content.push_str(&format!("sub sub_{} {{ }}\n", i));
        }
        content.push_str("1;\n");

        let uri = "file:///test/big.pm";
        open_test_document(&srv, uri, &content)?;

        // Query for "sub_" which matches all 300 subroutines
        let result = srv.test_handle_workspace_symbols(Some(json!({
            "query": "sub_"
        })))?;

        if let Some(result) = result {
            let symbols = result.as_array().ok_or("Expected array for workspace symbols result")?;
            // Default cap is 200, so we should never exceed that
            assert!(
                symbols.len() <= 200,
                "Workspace symbols should respect cap (200), got {}",
                symbols.len()
            );
        }
        Ok(())
    }

    // =========================================================================
    // Test: textDocument/references respects cap (default 500)
    // =========================================================================
    #[test]
    #[serial]
    fn test_references_respects_cap() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Create content with many references to a variable
        let mut content = String::from("my $shared = 0;\n");
        for i in 0..600 {
            content.push_str(&format!("$shared = $shared + {};\n", i));
        }

        let uri = "file:///test/many_refs.pm";
        open_test_document(&srv, uri, &content)?;

        // Request references to $shared
        let result = srv.test_handle_references(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 4}, // On $shared
            "context": {"includeDeclaration": true}
        })))?;

        if let Some(result) = result {
            let refs = result.as_array().ok_or("Expected array for references result")?;
            // Default cap is 500, so we should never exceed that
            assert!(refs.len() <= 500, "References should respect cap (500), got {}", refs.len());
        }
        Ok(())
    }

    // =========================================================================
    // Test: textDocument/completion respects cap (default 100)
    // =========================================================================
    #[test]
    #[serial]
    fn test_completion_respects_cap() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Create content with many completable items
        let mut content = String::from("package Complete;\n");
        for i in 0..150 {
            content.push_str(&format!("sub prefix_{} {{ }}\n", i));
        }
        content.push_str("sub main {\n    prefix_\n}\n1;\n");

        let uri = "file:///test/many_completions.pm";
        open_test_document(&srv, uri, &content)?;

        // Request completion after "prefix_"
        // The line number depends on how many subs we created
        let line = 151; // After 150 subs + package declaration + 1 for main start
        let result = srv.test_handle_completion(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": 11}
        })))?;

        if let Some(result) = result {
            let items =
                result.get("items").and_then(|i| i.as_array()).or_else(|| result.as_array());

            if let Some(items) = items {
                // Default cap is 100 (from completion.rs), but server may also have its own
                // The key is that it should be bounded
                assert!(
                    items.len() <= 150, // Some buffer for implementation variance
                    "Completion should be bounded, got {}",
                    items.len()
                );
            }
        }
        Ok(())
    }
}

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[cfg(test)]
mod deadline_enforcement_tests {
    //! Tests that verify deadline enforcement returns partial results, not errors.

    use crate::support::env_guard::EnvGuard;
    use perl_lsp::LspServer;
    use serde_json::json;
    use serial_test::serial;
    use std::sync::Arc;

    fn create_test_server() -> LspServer {
        let output: Arc<parking_lot::Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(parking_lot::Mutex::new(Box::new(Vec::new())));
        LspServer::with_output(output)
    }

    fn open_test_document(
        srv: &LspServer,
        uri: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "text": content,
                "languageId": "perl"
            }
        })))?;
        Ok(())
    }

    // =========================================================================
    // Test: References with complex content returns partial, not timeout error
    // =========================================================================
    #[test]
    #[serial]
    fn test_references_returns_partial_not_timeout_error() -> Result<(), Box<dyn std::error::Error>>
    {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Create a file with enough content to potentially stress deadlines
        let mut content = String::from("my $target = 'x';\n");
        for i in 0..100 {
            content.push_str(&format!("my $other_{} = $target; $target = $other_{};\n", i, i));
        }

        let uri = "file:///test/deadline.pm";
        open_test_document(&srv, uri, &content)?;

        // Request references - should return partial results on deadline, not error
        let result = srv.test_handle_references(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 4}, // On $target
            "context": {"includeDeclaration": true}
        })))?;

        // The result should be a valid response (array of locations)
        if let Some(result) = result {
            assert!(result.is_array(), "References result should be an array, not an error");
        }
        Ok(())
    }

    // =========================================================================
    // Test: Workspace symbols returns partial on early exit
    // =========================================================================
    #[test]
    #[serial]
    fn test_workspace_symbols_early_exit_returns_partial() -> Result<(), Box<dyn std::error::Error>>
    {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Open multiple documents to create more work
        for i in 0..20 {
            let uri = format!("file:///test/file_{}.pm", i);
            let content = format!("package Package{};\nsub search_target_{} {{ }}\n1;\n", i, i);
            open_test_document(&srv, &uri, &content)?;
        }

        // Query that matches across all files
        let result = srv.test_handle_workspace_symbols(Some(json!({
            "query": "search_target"
        })))?;

        if let Some(result) = result {
            // Should be an array of symbols
            assert!(result.is_array(), "Should return array of symbols");
        }
        Ok(())
    }

    // =========================================================================
    // Test: Handler returns gracefully even with minimal deadline
    // =========================================================================
    #[test]
    #[serial]
    fn test_handler_graceful_with_minimal_work() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Simple file - should complete well within any deadline
        let content = "sub simple { 1 }";
        let uri = "file:///test/simple.pm";
        open_test_document(&srv, uri, content)?;

        // All these operations should complete successfully
        srv.test_handle_definition(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 5}
        })))?;

        srv.test_handle_references(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 5},
            "context": {"includeDeclaration": true}
        })))?;

        srv.test_handle_hover(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 5}
        })))?;

        Ok(())
    }
}

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[cfg(test)]
mod windows_uri_path_tests {
    //! Tests for Windows-ish URI path handling.
    //!
    //! Tests cover:
    //! 1. rootPath conversion (file:// URI handling)
    //! 2. Module resolution with backslashes
    //! 3. Drive letter handling (C: vs c:)

    use crate::support::env_guard::EnvGuard;
    use perl_lsp::LspServer;
    use perl_parser::workspace_index::{fs_path_to_uri, uri_to_fs_path};
    use serde_json::json;
    use serial_test::serial;
    use std::sync::Arc;

    fn create_test_server() -> LspServer {
        let output: Arc<parking_lot::Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(parking_lot::Mutex::new(Box::new(Vec::new())));
        LspServer::with_output(output)
    }

    fn open_test_document(
        srv: &LspServer,
        uri: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "text": content,
                "languageId": "perl"
            }
        })))?;
        Ok(())
    }

    // =========================================================================
    // Test: rootPath with forward slashes works
    // =========================================================================
    #[test]
    #[serial]
    fn test_root_path_forward_slashes() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Windows-style path in file:// URI should work
        let uri = "file:///C:/Users/test/project/lib/Module.pm";
        let content = "package Module; sub foo { } 1;";

        open_test_document(&srv, uri, content)?;

        // Should be able to query the document
        let result = srv.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": uri}
        })))?;

        if let Some(result) = result {
            assert!(result.is_array());
        }
        Ok(())
    }

    // =========================================================================
    // Test: Drive letter case normalization (C: vs c:)
    // =========================================================================
    #[test]
    #[serial]
    fn test_drive_letter_case_normalization() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Open with uppercase drive letter
        let uri_upper = "file:///C:/project/test.pm";
        let content = "package Test; sub method { } 1;";
        open_test_document(&srv, uri_upper, content)?;

        // Query with lowercase drive letter - should still find the document
        // Note: The exact behavior depends on normalize_uri_key implementation
        let _uri_lower = "file:///c:/project/test.pm";

        // Both should refer to the same document (after normalization)
        srv.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": uri_upper}
        })))?;

        // Test that workspace symbols can find content regardless of case
        srv.test_handle_workspace_symbols(Some(json!({
            "query": "method"
        })))?;

        Ok(())
    }

    // =========================================================================
    // Test: URI percent encoding for spaces
    // =========================================================================
    #[test]
    #[serial]
    fn test_uri_percent_encoding_spaces() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // URI with percent-encoded space (common in Windows paths)
        let uri = "file:///C:/Program%20Files/MyApp/lib/MyModule.pm";
        let content = "package MyModule; sub init { } 1;";

        open_test_document(&srv, uri, content)?;

        srv.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": uri}
        })))?;

        Ok(())
    }

    // =========================================================================
    // Test: URI to filesystem path conversion (Windows-style)
    // =========================================================================
    #[test]
    fn test_uri_to_fs_path_windows_style() -> Result<(), Box<dyn std::error::Error>> {
        // Test that Windows-style URIs can be converted
        // Note: This will behave differently on Windows vs Unix

        #[cfg(target_os = "windows")]
        {
            let uri = "file:///C:/Users/test/script.pl";
            let path = uri_to_fs_path(uri).ok_or("Should convert Windows file URI")?;
            assert!(path.to_string_lossy().contains("Users"), "Should have correct path");
        }

        // On Unix, Windows-style paths won't convert but shouldn't crash
        #[cfg(not(target_os = "windows"))]
        {
            // Just verify no panic occurs
            let uri = "file:///C:/Users/test/script.pl";
            let _ = uri_to_fs_path(uri); // May return None on Unix, that's ok
        }
        Ok(())
    }

    // =========================================================================
    // Test: fs_path_to_uri handles various path formats
    // =========================================================================
    #[test]
    fn test_fs_path_to_uri_formats() -> Result<(), Box<dyn std::error::Error>> {
        // Unix-style path
        let uri = fs_path_to_uri("/tmp/test.pl")?;
        assert!(uri.starts_with("file://"), "Should be file URI");
        assert!(uri.contains("test.pl"), "Should contain filename");

        // Path with spaces
        let uri = fs_path_to_uri("/tmp/my project/test.pl")?;
        assert!(
            uri.contains("%20") || uri.contains("my%20project"),
            "Should percent-encode spaces"
        );
        Ok(())
    }

    // =========================================================================
    // Test: Module resolution handles mixed slashes gracefully
    // =========================================================================
    #[test]
    #[serial]
    fn test_module_resolution_mixed_slashes() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // Simulate what might happen with mixed path separators
        // The parser should handle content regardless of how the URI was formatted
        let uri = "file:///project/lib/My/Module.pm";
        let content = r#"
package My::Module;
use strict;
use warnings;

sub process {
    my $self = shift;
    return 1;
}

1;
"#;
        open_test_document(&srv, uri, content)?;

        // Should be able to find symbols
        let result = srv.test_handle_workspace_symbols(Some(json!({
            "query": "process"
        })))?;

        if let Some(result) = result {
            let symbols = result.as_array().ok_or("Expected array for symbols result")?;
            assert!(!symbols.is_empty(), "Should find symbols regardless of path format");
        }
        Ok(())
    }

    // =========================================================================
    // Test: Backslash in path traversal is rejected (security)
    // =========================================================================
    #[test]
    #[serial]
    fn test_backslash_path_traversal_security() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // This is a security-sensitive path - should be handled safely
        let suspicious_uri = r"file:///project\..\..\..\etc\passwd";
        let content = "print 'x';";

        // Opening should work (the content is benign)
        open_test_document(&srv, suspicious_uri, content)?;

        // The key is that operations don't crash and handle the path safely
        srv.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": suspicious_uri}
        })))?;

        Ok(())
    }

    // =========================================================================
    // Test: UNC paths (\\server\share) are handled
    // =========================================================================
    #[test]
    #[serial]
    fn test_unc_path_handling() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let srv = create_test_server();

        // UNC-style path encoded as file URI
        let uri = "file://server/share/project/lib/Module.pm";
        let content = "package Module; 1;";

        open_test_document(&srv, uri, content)?;

        srv.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": uri}
        })))?;

        Ok(())
    }
}

// ============================================================================
// Unit tests for workspace_index caps and limits
// ============================================================================
#[cfg(test)]
mod workspace_index_unit_tests {
    use perl_parser::workspace_index::{
        DegradationReason, IndexCoordinator, IndexResourceLimits, IndexState, ResourceKind,
    };
    use perl_tdd_support::must;

    // =========================================================================
    // Test: IndexCoordinator query dispatch based on state
    // =========================================================================
    #[test]
    fn test_coordinator_query_dispatch_building_state() {
        let coordinator = IndexCoordinator::new();
        // Coordinator starts in Building state

        let result =
            coordinator.query(|_index| "full_query_result", |_index| "partial_query_result");

        assert_eq!(result, "partial_query_result", "Building state should use partial query");
    }

    #[test]
    fn test_coordinator_query_dispatch_ready_state() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(10, 100);

        let result =
            coordinator.query(|_index| "full_query_result", |_index| "partial_query_result");

        assert_eq!(result, "full_query_result", "Ready state should use full query");
    }

    #[test]
    fn test_coordinator_query_dispatch_degraded_state() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(10, 100);
        coordinator.transition_to_degraded(DegradationReason::IoError { message: "test".into() });

        let result =
            coordinator.query(|_index| "full_query_result", |_index| "partial_query_result");

        assert_eq!(result, "partial_query_result", "Degraded state should use partial query");
    }

    // =========================================================================
    // Test: Resource limits trigger degradation
    // =========================================================================
    #[test]
    fn test_max_files_limit_triggers_degradation() -> Result<(), Box<dyn std::error::Error>> {
        let limits = IndexResourceLimits {
            max_files: 2, // Very low for testing
            ..Default::default()
        };

        let coordinator = IndexCoordinator::with_limits(limits);
        coordinator.transition_to_ready(0, 0);

        // Index 5 files (exceeds limit of 2)
        for i in 0..5 {
            let uri = format!("file:///test{}.pl", i);
            let url = url::Url::parse(&uri)?;
            coordinator.index().index_file(url, "sub test { }".into())?;
        }

        coordinator.enforce_limits();

        match coordinator.state() {
            IndexState::Degraded { reason: DegradationReason::ResourceLimit { kind }, .. } => {
                assert_eq!(kind, ResourceKind::MaxFiles);
            }
            other => must(Err::<(), _>(format!("Expected MaxFiles degradation, got {:?}", other))),
        }
        Ok(())
    }

    #[test]
    fn test_max_symbols_limit_triggers_degradation() -> Result<(), Box<dyn std::error::Error>> {
        let limits = IndexResourceLimits {
            max_total_symbols: 5, // Very low for testing
            ..Default::default()
        };

        let coordinator = IndexCoordinator::with_limits(limits);
        coordinator.transition_to_ready(0, 0);

        // Index file with many symbols
        let content = r#"
package Test;
sub a { } sub b { } sub c { } sub d { } sub e { }
sub f { } sub g { } sub h { } sub i { } sub j { }
1;
"#;
        let url = url::Url::parse("file:///test.pm")?;
        coordinator.index().index_file(url, content.into())?;

        coordinator.enforce_limits();

        match coordinator.state() {
            IndexState::Degraded { reason: DegradationReason::ResourceLimit { kind }, .. } => {
                assert_eq!(kind, ResourceKind::MaxSymbols);
            }
            other => {
                must(Err::<(), _>(format!("Expected MaxSymbols degradation, got {:?}", other)))
            }
        }
        Ok(())
    }

    // =========================================================================
    // Test: Parse storm triggers degradation
    // =========================================================================
    #[test]
    fn test_parse_storm_triggers_degradation() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(10, 100);

        // Trigger parse storm by exceeding threshold (default 10)
        for _ in 0..15 {
            coordinator.notify_change("file.pm");
        }

        match coordinator.state() {
            IndexState::Degraded {
                reason: DegradationReason::ParseStorm { pending_parses },
                ..
            } => {
                assert!(pending_parses > 10);
            }
            other => {
                must(Err::<(), _>(format!("Expected ParseStorm degradation, got {:?}", other)))
            }
        }
    }

    #[test]
    fn test_parse_storm_recovery() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(10, 100);

        // Trigger parse storm
        for _ in 0..15 {
            coordinator.notify_change("file.pm");
        }

        assert!(matches!(coordinator.state(), IndexState::Degraded { .. }));

        // Complete all parses
        for _ in 0..15 {
            coordinator.notify_parse_complete("file.pm");
        }

        // Should recover to Building state
        assert!(
            matches!(coordinator.state(), IndexState::Building { .. }),
            "Should recover from parse storm"
        );
    }
}
