//! Diagnostics provider for Perl code
//!
//! This module provides the core diagnostic generation functionality.

use lsp_types::Diagnostic as LspDiagnostic;
use serde::{Deserialize, Serialize};

/// Diagnostic severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A diagnostic message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The range at which the message applies
    pub range: lsp_types::Range,
    /// The diagnostic's severity
    pub severity: DiagnosticSeverity,
    /// The diagnostic's code
    pub code: Option<String>,
    /// A human-readable string describing the source of this diagnostic
    pub source: Option<String>,
    /// The diagnostic's message
    pub message: String,
}

/// Diagnostics provider
pub struct DiagnosticsProvider;

impl DiagnosticsProvider {
    /// Create a new diagnostics provider
    pub fn new() -> Self {
        Self
    }

    /// Generate diagnostics for the given AST
    pub fn generate_diagnostics(
        &self,
        _ast: &perl_parser_core::Node,
        _source: &str,
        _workspace_index: Option<&perl_workspace_index::workspace_index::WorkspaceIndex>,
    ) -> Result<Vec<LspDiagnostic>, String> {
        Ok(Vec::new())
    }
}

impl Default for DiagnosticsProvider {
    fn default() -> Self {
        Self::new()
    }
}
