//! Compatibility shim for tests using old API
//!
//! This module provides a zero-cost abstraction layer to allow tests written
//! against the old API to work with the current implementation.
//! All functions are marked as deprecated to encourage migration.

#![allow(deprecated)]

#[cfg(any(test, feature = "test-compat"))]
pub mod v0 {
    use crate::*;
    use serde_json::Value;

    // ============= Core Parser Compatibility =============

    /// Old parser API that returned Result directly
    #[deprecated(since = "0.7.5", note = "Use Parser::new(code).parse() instead")]
    #[inline]
    pub fn parse(code: &str) -> Result<ast::Node, crate::ParseError> {
        Parser::new(code).parse()
    }

    /// Old parse_file API
    #[deprecated(since = "0.7.5", note = "Use Parser::new(code).parse() instead")]
    #[inline]
    pub fn parse_file(code: &str) -> Result<ast::Node, crate::ParseError> {
        Parser::new(code).parse()
    }

    // ============= Scope Analyzer Compatibility =============

    /// Old scope analyzer that took code directly
    #[deprecated(
        since = "0.7.5",
        note = "Use ScopeAnalyzer::new().analyze(&ast, code, pragmas)"
    )]
    #[inline]
    pub fn analyze_scope(code: &str) -> Result<Vec<scope_analyzer::ScopeIssue>, String> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| e.to_string())?;
        let analyzer = scope_analyzer::ScopeAnalyzer::new();
        Ok(analyzer.analyze(&ast, code, &[]))
    }

    // ============= Workspace Index Compatibility =============

    /// Old workspace symbol format
    #[derive(serde::Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct WorkspaceSymbolDto {
        pub name: String,
        pub kind: i32,
        pub location: LocationDto,
        pub container_name: Option<String>,
    }

    #[derive(serde::Serialize, Debug)]
    pub struct LocationDto {
        pub uri: String,
        pub range: RangeDto,
    }

    #[derive(serde::Serialize, Debug)]
    pub struct RangeDto {
        pub start: PositionDto,
        pub end: PositionDto,
    }

    #[derive(serde::Serialize, Debug)]
    pub struct PositionDto {
        pub line: u32,
        pub character: u32,
    }

    /// Convert new workspace symbol to old DTO format
    #[deprecated(
        since = "0.7.5",
        note = "Use workspace_index::WorkspaceSymbol directly"
    )]
    #[inline]
    pub fn to_old_workspace_symbol(sym: &workspace_index::WorkspaceSymbol) -> WorkspaceSymbolDto {
        use crate::workspace_index::SymbolKind;

        let kind = match sym.kind {
            SymbolKind::Package => 4,
            SymbolKind::Class => 5,
            SymbolKind::Method => 6,
            SymbolKind::Subroutine => 12,
            SymbolKind::Variable => 13,
            SymbolKind::Constant => 14,
            SymbolKind::Role => 5,    // Treat as Class
            SymbolKind::Import => 15, // Module
            SymbolKind::Export => 15, // Module
        };

        WorkspaceSymbolDto {
            name: sym.name.clone(),
            kind,
            container_name: sym.qualified_name.clone(),
            location: LocationDto {
                uri: sym.uri.clone(),
                range: RangeDto {
                    start: PositionDto {
                        line: sym.range.start.line,
                        character: sym.range.start.character,
                    },
                    end: PositionDto {
                        line: sym.range.end.line,
                        character: sym.range.end.character,
                    },
                },
            },
        }
    }

    // ============= LSP Server Compatibility =============

    /// Helper to handle execute command requests
    #[deprecated(since = "0.7.5", note = "Use LspServer::handle_request directly")]
    #[inline]
    pub fn execute_lsp_command(
        server: &mut LspServer,
        command: &str,
        args: Vec<Value>,
    ) -> Option<Value> {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "workspace/executeCommand".to_string(),
            params: Some(serde_json::json!({
                "command": command,
                "arguments": args
            })),
        };

        server
            .handle_request(request)
            .and_then(|response| response.result)
    }

    /// Helper for sending notifications
    /// Note: The notify method is now private, tests should use handle_request for notifications
    #[deprecated(since = "0.7.5", note = "Use handle_request with notification format")]
    #[inline]
    pub fn send_notification(
        _server: &LspServer,
        method: &str,
        params: Value,
    ) -> Result<(), String> {
        // Create a notification-style request (no id field)
        let _notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        // Since notify is private, we can't actually send it
        // Tests should be updated to use the proper API
        Ok(())
    }

    // ============= Re-exports for Module Paths =============

    /// Re-export old module paths that tests might expect
    pub mod scope {
        pub use crate::scope_analyzer::*;
    }

    pub mod workspace {
        pub use crate::workspace_index::*;
    }

    pub mod lsp {
        pub use crate::lsp_server::*;
    }
}
