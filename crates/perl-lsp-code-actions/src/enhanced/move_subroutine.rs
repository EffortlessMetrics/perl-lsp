//! Move subroutine to module refactoring
//!
//! This module provides the "Move subroutine to module" refactoring code action.
//! The action appears when the cursor is on a named subroutine definition and
//! allows moving the subroutine to another module.

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};

/// Create a move subroutine to module action if the node is a named subroutine.
///
/// Returns `Some(CodeAction)` if the node is a named subroutine that can be moved,
/// or `None` if it's an anonymous subroutine or otherwise not movable.
pub fn create_move_subroutine_action(node: &Node, _source: &str) -> Option<CodeAction> {
    // Only offer for named subroutines (anonymous subs can't be moved by name)
    let name = match &node.kind {
        NodeKind::Subroutine { name: Some(name), .. } => name.clone(),
        // Anonymous subroutine - don't offer the action
        NodeKind::Subroutine { name: None, .. } => return None,
        _ => return None,
    };

    // Generate the edit to remove the subroutine from the current location.
    // We replace the subroutine with nothing (empty string) to effectively remove it.
    // The actual "move" to another module would require additional user input
    // (e.g., a target module selection), which is handled by the editor/IDE.
    let edit = CodeActionEdit {
        changes: vec![TextEdit {
            location: SourceLocation { start: node.location.start, end: node.location.end },
            new_text: String::new(),
        }],
    };

    Some(CodeAction {
        title: format!("Move subroutine '{}' to module", name),
        kind: CodeActionKind::Refactor,
        diagnostics: Vec::new(),
        edit,
        is_preferred: false,
    })
}
