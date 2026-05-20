//! Whole-source code actions that are not tied to diagnostics or selections.

use super::quick_fixes;
use super::types::CodeAction;

pub(super) fn get_source_actions(source: &str, range: (usize, usize)) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Only suggest shebang fixes when the requested range includes the first line.
    if range.0 == 0 || !source[..range.0].contains('\n') {
        actions.extend(quick_fixes::fix_hardcoded_shebang(source));
    }

    actions
}
