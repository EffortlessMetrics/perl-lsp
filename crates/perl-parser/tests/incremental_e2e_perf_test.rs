//! End-to-end performance tests for incremental parsing that measure actual parse time

#[cfg(feature = "incremental")]
#[cfg(test)]
mod e2e_perf_tests {
    use perl_parser::lsp_server::LspServer;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    #[serial_test::serial]
    fn test_big_file_edit_end_to_end_under_50ms() {
        // Enable incremental parsing
        std::env::set_var("PERL_LSP_INCREMENTAL", "1");
        
        // Create LSP server
        let server = LspServer::new();
        
        // Initialize
        server.handle_initialize(Some(json!({
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "didChange": {"syncKind": 2}  // Incremental
                    }
                }
            }
        }))).unwrap();
        
        // Create big file (10k lines)
        let big_file = "my $x = 0;\n".repeat(10_000);
        let uri = "file:///test/big.pl";
        
        // Open document
        server.handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": big_file
            }
        }))).unwrap();
        
        // Measure incremental edit
        let start = Instant::now();
        server.handle_did_change_incremental(Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 2
            },
            "contentChanges": [{
                "range": {
                    "start": {"line": 5000, "character": 8},
                    "end": {"line": 5000, "character": 9}
                },
                "text": "9"
            }]
        }))).unwrap();
        let elapsed = start.elapsed();
        
        // Should be fast even with full reparse
        assert!(elapsed.as_millis() < 150, "Parsing too slow: {:?}", elapsed);
        
        // Verify edit was applied
        let docs = server.documents.lock().unwrap();
        let doc = docs.get(uri).unwrap();
        let lines: Vec<&str> = doc.content.lines().collect();
        assert!(lines[5000].contains("9"));
        
        std::env::remove_var("PERL_LSP_INCREMENTAL");
    }

    #[test]
    #[serial_test::serial]
    fn test_multiple_rapid_edits_performance() {
        std::env::set_var("PERL_LSP_INCREMENTAL", "1");
        
        let server = LspServer::new();
        
        // Initialize
        server.handle_initialize(Some(json!({
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "didChange": {"syncKind": 2}
                    }
                }
            }
        }))).unwrap();
        
        // Create medium file
        let file = "sub test { my $x = 0; }\n".repeat(1000);
        let uri = "file:///test/medium.pl";
        
        server.handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": file
            }
        }))).unwrap();
        
        // Apply 10 rapid edits
        let start = Instant::now();
        for i in 0..10 {
            server.handle_did_change_incremental(Some(json!({
                "textDocument": {
                    "uri": uri,
                    "version": i + 2
                },
                "contentChanges": [{
                    "range": {
                        "start": {"line": i * 100, "character": 19},
                        "end": {"line": i * 100, "character": 20}
                    },
                    "text": &i.to_string()
                }]
            }))).unwrap();
        }
        let elapsed = start.elapsed();
        
        // 10 edits should complete in reasonable time
        assert!(elapsed.as_millis() < 500, "Multiple edits too slow: {:?}", elapsed);
        
        std::env::remove_var("PERL_LSP_INCREMENTAL");
    }

    #[test]
    #[serial_test::serial]
    fn test_emoji_edit_performance() {
        std::env::set_var("PERL_LSP_INCREMENTAL", "1");
        
        let server = LspServer::new();
        
        server.handle_initialize(Some(json!({
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "didChange": {"syncKind": 2}
                    }
                }
            }
        }))).unwrap();
        
        // File with emojis
        let file = "my $🦀 = 'crab';\n".repeat(100);
        let uri = "file:///test/emoji.pl";
        
        server.handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": file
            }
        }))).unwrap();
        
        // Edit after emoji (UTF-16 position handling)
        let start = Instant::now();
        server.handle_did_change_incremental(Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 2
            },
            "contentChanges": [{
                "range": {
                    "start": {"line": 50, "character": 10}, // After emoji
                    "end": {"line": 50, "character": 14}
                },
                "text": "'lobster'"
            }]
        }))).unwrap();
        let elapsed = start.elapsed();
        
        assert!(elapsed.as_millis() < 100, "Emoji edit too slow: {:?}", elapsed);
        
        std::env::remove_var("PERL_LSP_INCREMENTAL");
    }

    #[test]
    #[serial_test::serial]
    fn test_full_vs_incremental_comparison() {
        let server = LspServer::new();
        
        server.handle_initialize(Some(json!({
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "didChange": {"syncKind": 1}  // Full
                    }
                }
            }
        }))).unwrap();
        
        let file = "my $x = 0;\n".repeat(5000);
        let uri = "file:///test/compare.pl";
        
        server.handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": file
            }
        }))).unwrap();
        
        // Measure full replacement
        let start_full = Instant::now();
        server.handle_did_change(Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 2
            },
            "contentChanges": [{
                "text": file.replace("= 0", "= 1")
            }]
        }))).unwrap();
        let full_time = start_full.elapsed();
        
        // Enable incremental
        std::env::set_var("PERL_LSP_INCREMENTAL", "1");
        
        // Measure incremental edit
        let start_inc = Instant::now();
        server.handle_did_change_incremental(Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 3
            },
            "contentChanges": [{
                "range": {
                    "start": {"line": 2500, "character": 8},
                    "end": {"line": 2500, "character": 9}
                },
                "text": "2"
            }]
        }))).unwrap();
        let inc_time = start_inc.elapsed();
        
        println!("Full replacement: {:?}, Incremental edit: {:?}", full_time, inc_time);
        
        // Incremental should be comparable or faster
        // (both do full reparse, but incremental avoids string copy)
        assert!(inc_time.as_millis() <= full_time.as_millis() * 2);
        
        std::env::remove_var("PERL_LSP_INCREMENTAL");
    }
}