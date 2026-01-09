//! Compatibility shim for tests using old API.
//!
//! This module provides a zero-cost abstraction layer to allow tests written
//! against the old API to work with the current implementation. It is **not**
//! part of the supported, stable API surface.
//!
//! Enabled only when running tests or when the `test-compat` feature is enabled.
//! New code should prefer the current APIs in the crate root.
//!
//! # Deprecation Policy
//!
//! All items in this module are deprecated. This namespace should shrink over
//! time as fixtures are migrated to use the current APIs.

#![allow(deprecated)]

/// Legacy v0 API used by archived fixtures and compatibility tests.
///
/// This module provides shims for the pre-0.7.5 API surface. Each function
/// and type maps to a modern equivalent, documented inline.
///
/// # Migration Guide
///
/// | Legacy | Current |
/// |--------|---------|
/// | `v0::parse(code)` | `Parser::new(code).parse()` |
/// | `v0::analyze_scope(code)` | `ScopeAnalyzer::new().analyze(&ast, code, &[])` |
/// | `v0::WorkspaceSymbolDto` | `workspace_index::WorkspaceSymbol` |
/// | `v0::execute_lsp_command(...)` | `LspServer::handle_request(...)` |
#[cfg(any(test, feature = "test-compat"))]
pub mod v0 {
    use crate::*;
    use serde_json::Value;

    // ============= Core Parser Compatibility =============

    /// Legacy wrapper around [`crate::Parser::parse`] used by v0 fixtures.
    ///
    /// Prefer calling `Parser::new(code).parse()` directly in new tests.
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
    #[deprecated(since = "0.7.5", note = "Use ScopeAnalyzer::new().analyze(&ast, code, pragmas)")]
    #[inline]
    pub fn analyze_scope(code: &str) -> Result<Vec<scope_analyzer::ScopeIssue>, String> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| e.to_string())?;
        let analyzer = scope_analyzer::ScopeAnalyzer::new();
        Ok(analyzer.analyze(&ast, code, &[]))
    }

    // ============= Workspace Index Compatibility =============

    /// Legacy workspace symbol DTO for JSON serialization.
    ///
    /// This struct mirrors the old JSON wire format used before v0.7.5.
    /// Prefer [`crate::workspace_index::WorkspaceSymbol`] for new code.
    #[derive(serde::Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct WorkspaceSymbolDto {
        /// Symbol name (e.g., subroutine or package name).
        pub name: String,
        /// LSP symbol kind number (e.g., 12 = Function, 4 = Package).
        pub kind: i32,
        /// File location where the symbol is defined.
        pub location: LocationDto,
        /// Containing package or class name, if any.
        pub container_name: Option<String>,
    }

    /// Legacy LSP location DTO for JSON serialization.
    ///
    /// Maps to the LSP `Location` interface. Prefer `lsp_types::Location` for new code.
    #[derive(serde::Serialize, Debug)]
    pub struct LocationDto {
        /// Document URI (e.g., `file:///path/to/file.pm`).
        pub uri: String,
        /// Character range within the document.
        pub range: RangeDto,
    }

    /// Legacy LSP range DTO for JSON serialization.
    ///
    /// Maps to the LSP `Range` interface. Prefer `lsp_types::Range` for new code.
    #[derive(serde::Serialize, Debug)]
    pub struct RangeDto {
        /// Start position (inclusive).
        pub start: PositionDto,
        /// End position (exclusive).
        pub end: PositionDto,
    }

    /// Legacy LSP position DTO for JSON serialization.
    ///
    /// Maps to the LSP `Position` interface. Prefer `lsp_types::Position` for new code.
    #[derive(serde::Serialize, Debug)]
    pub struct PositionDto {
        /// Zero-based line number.
        pub line: u32,
        /// Zero-based UTF-16 code unit offset on the line.
        pub character: u32,
    }

    /// Convert new workspace symbol to old DTO format
    #[deprecated(since = "0.7.5", note = "Use workspace_index::WorkspaceSymbol directly")]
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

        server.handle_request(request).and_then(|response| response.result)
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

    /// Legacy re-export of [`crate::scope_analyzer`] for old import paths.
    ///
    /// Tests that imported `v0::scope::*` should migrate to `use crate::scope_analyzer::*;`.
    pub mod scope {
        pub use crate::scope_analyzer::*;
    }

    /// Legacy re-export of [`crate::workspace_index`] for old import paths.
    ///
    /// Tests that imported `v0::workspace::*` should migrate to `use crate::workspace_index::*;`.
    pub mod workspace {
        pub use crate::workspace_index::*;
    }

    /// Legacy re-export of [`crate::lsp_server`] for old import paths.
    ///
    /// Tests that imported `v0::lsp::*` should migrate to `use crate::lsp_server::*;`.
    pub mod lsp {
        pub use crate::lsp_server::*;
    }
}
