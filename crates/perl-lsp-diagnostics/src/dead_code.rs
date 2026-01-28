//! Dead code detection using workspace-wide symbol analysis

use crate::types::{Diagnostic, DiagnosticSeverity, DiagnosticTag};

/// Detect dead code using workspace-wide symbol analysis
///
/// Identifies unused symbols (subroutines, variables, constants, packages)
/// that have no references in the workspace. Returns diagnostics for symbols
/// in the specified document.
///
/// # Arguments
///
/// * `workspace_index` - Workspace-wide symbol index
/// * `document_uri` - URI of the document to generate diagnostics for
/// * `source_text` - The source text of the document (for position conversion)
/// * `line_index` - Line index helper for position conversion
///
/// # Returns
///
/// Dead code diagnostics for symbols in the specified document
#[cfg(not(target_arch = "wasm32"))]
pub fn detect_dead_code(
    workspace_index: &perl_workspace_index::workspace_index::WorkspaceIndex,
    document_uri: &str,
    source_text: &str,
    line_index: &perl_parser_core::position::LineStartsCache,
) -> Vec<Diagnostic> {
    use perl_workspace_index::workspace_index::SymbolKind;

    let unused_symbols = workspace_index.find_unused_symbols();
    let mut diagnostics = Vec::new();

    for symbol in unused_symbols {
        // Only report diagnostics for symbols in the current document
        if symbol.uri != document_uri {
            continue;
        }

        // Determine diagnostic code and message based on symbol kind
        let (code, message_prefix) = match symbol.kind {
            SymbolKind::Subroutine => ("dead-code-subroutine", "Unused subroutine"),
            SymbolKind::Variable(_) => ("dead-code-variable", "Unused variable"),
            SymbolKind::Constant => ("dead-code-constant", "Unused constant"),
            SymbolKind::Package => ("dead-code-package", "Unused package"),
            _ => continue, // Skip other symbol kinds
        };

        let message = format!("{}: '{}'", message_prefix, symbol.name);

        // Convert line/column to byte offsets using the line index
        let start_byte = line_index.position_to_offset(
            source_text,
            symbol.range.start.line,
            symbol.range.start.column,
        );
        let end_byte =
            line_index.position_to_offset(source_text, symbol.range.end.line, symbol.range.end.column);

        diagnostics.push(Diagnostic {
            range: (start_byte, end_byte),
            severity: DiagnosticSeverity::Hint,
            code: Some(code.to_string()),
            message,
            related_information: Vec::new(),
            tags: vec![DiagnosticTag::Unnecessary],
        });
    }

    diagnostics
}
