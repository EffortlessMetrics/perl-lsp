// Incremental parser internals are being split into private modules; facade
// documentation stays on the exported types and entry points.
#![allow(missing_docs)]

mod checkpoint;
mod diagnostics;
mod edit;
mod lex;
mod reparse;
mod state;
mod strategy;

use anyhow::Result;

pub use perl_line_index::LineIndex;

pub use checkpoint::{LexCheckpoint, ParseCheckpoint, ScopeSnapshot};
pub use diagnostics::ReparseResult;
pub use edit::Edit;
use reparse::{apply_single_edit, apply_text_edit_to_state, full_reparse};
pub use state::IncrementalState;
pub use strategy::MAX_EDIT_SIZE;

pub mod incremental_advanced_reuse;
#[cfg(test)]
mod incremental_boundary_regressions;
pub mod incremental_checkpoint;
pub mod incremental_document;
pub mod incremental_edit;
pub mod incremental_handler_v2;
pub mod incremental_integration;
pub mod incremental_simple;
pub mod incremental_v2;

/// Apply edits incrementally
pub fn apply_edits(state: &mut IncrementalState, edits: &[Edit]) -> Result<ReparseResult> {
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by_key(|e| e.start_byte);
    sorted_edits.reverse();

    let total_changed = sorted_edits.iter().map(Edit::touched_bytes).sum::<usize>();

    if total_changed > MAX_EDIT_SIZE {
        return full_reparse(state);
    }

    if sorted_edits.len() == 1 {
        let edit = &sorted_edits[0];

        if edit.touched_bytes() > 1024 || edit.new_text.matches('\n').count() > 10 {
            apply_text_edit_to_state(state, edit)?;
            return full_reparse(state);
        }

        let reparse = match apply_single_edit(state, edit) {
            Ok(reparse) => reparse,
            Err(_) => return full_reparse(state),
        };
        let reparsed_bytes = reparse.range.end - reparse.range.start;

        Ok(ReparseResult {
            changed_ranges: vec![reparse.range],
            diagnostics: vec![],
            reparsed_bytes,
            reused_tokens: reparse.reused_tokens,
            token_count: reparse.token_count,
        })
    } else {
        for edit in sorted_edits {
            if apply_single_edit(state, &edit).is_err() {
                return full_reparse(state);
            }
        }
        full_reparse(state)
    }
}

#[cfg(test)]
mod tests;
