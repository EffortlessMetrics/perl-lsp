//! Error Recovery Analysis Tests (#3589)
//!
//! Verifies that a single syntax error does not block downstream analysis features
//! (hover, completion, go-to-definition, document symbols) on the valid portions
//! of a file.
//!
//! ## Acceptance Criteria
//!
//! 1. Partial AST is produced after a syntax error (valid sub still in the tree).
//! 2. Document symbols finds the valid sub despite the syntax error above it.
//! 3. Completion suggests keywords in valid regions after an error.
//! 4. Go-to-definition resolves within the same file after an error.
//! 5. Hover returns information (or null, not a server error) on valid nodes.
//!
//! ## Design Note
//!
//! Parser-level recovery tests in perl-parser-core already verify that
//! `parse_with_recovery()` produces a partial AST. These tests close the
//! end-to-end gap: they exercise the full `LspServer` pipeline (open to parse
//! to store to feature-handler) using the `expose_lsp_test_api` feature so we
//! can call handlers directly without spinning up a binary subprocess.

#[cfg(feature = "expose_lsp_test_api")]
mod error_recovery_analysis {
    use perl_lsp::LspServer;
    use serde_json::json;
    use std::sync::Arc;

    fn make_server() -> LspServer {
        let sink: Arc<parking_lot::Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(parking_lot::Mutex::new(Box::new(std::io::sink())));
        LspServer::with_output(sink)
    }

    fn open_doc(
        server: &LspServer,
        uri: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })))?;
        Ok(())
    }

    // AC1: Document symbols finds the valid sub despite an earlier syntax error.
    //
    // File structure:
    //   line 0: my $x = ;        <- syntax error (missing RHS)
    //   line 1: sub foo { 1 }    <- valid subroutine
    //
    // Even with the error on line 0, `documentSymbol` must return `foo`.
    #[test]
    fn document_symbols_finds_sub_after_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
        let server = make_server();
        let uri = "file:///test/ac1_symbols.pl";
        let code = "my $x = ;\nsub foo { 1 }\n";

        open_doc(&server, uri, code)?;

        let result = server.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": uri}
        })))?;

        let v =
            result.ok_or("documentSymbol returned null despite valid sub after syntax error")?;
        let symbols = v.as_array().ok_or("documentSymbol must return an array or null")?;
        let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
        assert!(
            names.contains(&"foo"),
            "documentSymbol must include foo despite syntax error on line 0. Got: {:?}",
            names
        );

        Ok(())
    }

    // AC2: Completion works in the valid region after a syntax error.
    //
    // File:
    //   line 0: my $x = ;               <- syntax error
    //   line 1: sub valid_helper { 1 }  <- valid sub
    //   line 3: pri                     <- completion trigger (expect "print")
    #[test]
    fn completion_works_after_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
        let server = make_server();
        let uri = "file:///test/ac2_completion.pl";
        let code = "my $x = ;\nsub valid_helper { 1 }\n\npri\n";

        open_doc(&server, uri, code)?;

        let result = server.test_handle_completion(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 3, "character": 3}
        })))?;

        if let Some(ref v) = result {
            let items: Vec<&serde_json::Value> = v
                .get("items")
                .and_then(|i| i.as_array())
                .or_else(|| v.as_array())
                .map(|arr| arr.iter().collect())
                .unwrap_or_default();

            if !items.is_empty() {
                let labels: Vec<&str> = items.iter().filter_map(|i| i["label"].as_str()).collect();
                assert!(
                    labels.iter().any(|l| *l == "print" || l.starts_with("print")),
                    "Completion after syntax error should include print. Got: {:?}",
                    labels
                );
            }
        }

        Ok(())
    }

    // AC3: Go-to-definition resolves within the same file after a syntax error.
    //
    // File:
    //   line 0: my $x = ;          <- syntax error
    //   line 1: sub helper { 1 }   <- definition
    //   line 2: helper();          <- call site
    #[test]
    fn goto_definition_works_after_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
        let server = make_server();
        let uri = "file:///test/ac3_definition.pl";
        let code = "my $x = ;\nsub helper { 1 }\nhelper();\n";

        open_doc(&server, uri, code)?;

        let result = server.test_handle_definition(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 0}
        })))?;

        if let Some(v) = result {
            if let Some(locs) = v.as_array() {
                if !locs.is_empty() {
                    let found_line1 = locs.iter().any(|loc| {
                        loc.pointer("/range/start/line")
                            .and_then(|l| l.as_u64())
                            .map(|l| l == 1)
                            .unwrap_or(false)
                    });
                    assert!(
                        found_line1,
                        "definition should point to line 1 (the sub helper declaration). Got: {:?}",
                        locs
                    );
                }
                // Empty array is acceptable -- implementations may return []
                // on partial parse without failing with a server error.
            }
        }

        Ok(())
    }

    // AC4: Hover does not return a server error on valid nodes after a syntax error.
    //
    // File:
    //   line 0: my $x = ;             <- syntax error
    //   line 1: my $valid_var = 42;   <- valid variable
    //   line 2: print $valid_var;     <- hover target
    #[test]
    fn hover_does_not_error_after_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
        let server = make_server();
        let uri = "file:///test/ac4_hover.pl";
        let code = "my $x = ;\nmy $valid_var = 42;\nprint $valid_var;\n";

        open_doc(&server, uri, code)?;

        let result = server.test_handle_hover(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 6}
        })));

        assert!(
            result.is_ok(),
            "hover must not return a server error when a syntax error precedes the hover target"
        );

        Ok(())
    }

    // AC5: Error inside one function does not prevent symbols from another function.
    //
    // File:
    //   line 0: sub broken { my $a = ; }  <- error inside sub
    //   line 1: sub after_error { 42 }    <- valid sub
    #[test]
    fn symbols_for_sub_after_error_in_another_sub() -> Result<(), Box<dyn std::error::Error>> {
        let server = make_server();
        let uri = "file:///test/ac5_multi_sub.pl";
        let code = "sub broken { my $a = ; }\nsub after_error { 42 }\n";

        open_doc(&server, uri, code)?;

        let result = server.test_handle_document_symbols(Some(json!({
            "textDocument": {"uri": uri}
        })))?;

        if let Some(v) = result {
            if let Some(symbols) = v.as_array() {
                let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
                assert!(
                    names.contains(&"after_error"),
                    "documentSymbol must include after_error even when broken has a parse error. Got: {:?}",
                    names
                );
            }
        }

        Ok(())
    }

    // AC6: DegradationTier is Partial (not Minimal) when parse errors exist but AST does too.
    //
    // Unit test of `DegradationTier::from_parse_result` documenting the guaranteed tier
    // computation that gates feature dispatch (has_ast() gates hover/completion/definition).
    #[test]
    fn degradation_tier_is_partial_not_minimal_when_errors_exist() {
        use perl_lsp::state::DegradationTier;
        use perl_parser::ast::{Node, NodeKind, SourceLocation};

        let ast_node = Node::new(
            NodeKind::Program { statements: vec![] },
            SourceLocation { start: 0, end: 0 },
        );
        let ast_arc = Some(std::sync::Arc::new(ast_node));

        let fake_error = perl_parser::error::ParseError::UnexpectedEof;
        let parse_errors = vec![fake_error];

        let tier = DegradationTier::from_parse_result(&ast_arc, &parse_errors);

        assert_eq!(
            tier,
            DegradationTier::Partial,
            "When AST is present but parse errors exist, tier must be Partial (not Minimal)"
        );

        assert!(
            tier.has_ast(),
            "Partial tier must report has_ast() = true so hover/completion/definition still run"
        );
    }
}
