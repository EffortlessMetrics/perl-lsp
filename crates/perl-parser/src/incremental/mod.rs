#![allow(missing_docs)]



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
mod reparse;
mod state;
mod types;

pub use reparse::{Edit, ReparseResult, apply_edits};
pub use state::IncrementalState;
pub use types::{LexCheckpoint, ParseCheckpoint, ScopeSnapshot};

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use proptest::prelude::*;

    #[derive(Clone, Debug)]
    struct FuzzEdit {
        start: usize,
        delete_len: usize,
        insert_text: String,
    }

    fn apply_edit_to_ground_truth(source: &mut String, edit: &FuzzEdit) {
        let start = edit.start.min(source.len());
        let old_end = (start + edit.delete_len).min(source.len());
        source.replace_range(start..old_end, &edit.insert_text);
    }

    /// Verify that a small ranged edit re-lexes substantially fewer bytes than
    /// the total document length, confirming the checkpoint fast-path fires.
    #[test]
    fn test_incremental_state_small_edit_uses_checkpoint() -> Result<()> {
        // Build a document large enough that a checkpoint will exist before the edit.
        // We need at least one statement-boundary token (semicolon / brace) to create
        // a checkpoint entry that sits before the edit site.
        let mut lines = Vec::with_capacity(30);
        for i in 0..30usize {
            lines.push(format!("my $var_{i} = {i};"));
        }
        let source = lines.join("\n");
        let doc_len = source.len();

        let mut state = IncrementalState::new(source.clone());

        // Sanity: we should have at least one lex checkpoint from all those semicolons.
        assert!(
            state.lex_checkpoints.len() > 1,
            "expected multiple lex checkpoints, got {}",
            state.lex_checkpoints.len()
        );

        // Edit the LAST line: change `my $var_29 = 29;` -> `my $var_29 = 999;`
        // The checkpoint at some semicolon earlier in the doc should be used, meaning
        // we re-lex far less than the full document.
        let edit_text = "999";
        let edit_start = source.rfind("29;").unwrap_or(source.len() - 3);
        let edit_end = edit_start + 2; // replace "29"

        let edit = Edit {
            start_byte: edit_start,
            old_end_byte: edit_end,
            new_end_byte: edit_start + edit_text.len(),
            new_text: edit_text.to_string(),
        };

        let result = apply_edits(&mut state, &[edit])?;

        // The key assertion: reparsed_bytes must be strictly less than the full
        // document size, proving the incremental path was taken.
        assert!(
            result.reparsed_bytes < doc_len,
            "incremental reparse should cover less than the full document ({} bytes reparsed, {} doc len)",
            result.reparsed_bytes,
            doc_len
        );

        // Source must reflect the edit.
        assert!(state.source.contains("999"), "source must contain the new value after edit");
        assert!(
            !state.source.contains("= 29;"),
            "source must not contain the old value after edit"
        );

        Ok(())
    }

    /// Verify that the full-reparse fallback is triggered for an edit >64KB.
    #[test]
    fn test_incremental_state_large_edit_falls_back_to_full_reparse() -> Result<()> {
        let source = "my $x = 1;\n".repeat(10);
        let mut state = IncrementalState::new(source.clone());

        // A new_text larger than 64KB must trigger full reparse
        let big_text = "x".repeat(65 * 1024);
        let edit = Edit {
            start_byte: 0,
            old_end_byte: 0,
            new_end_byte: big_text.len(),
            new_text: big_text.clone(),
        };

        let result = apply_edits(&mut state, &[edit])?;

        // Full reparse: reparsed_bytes == full (new) source length
        assert_eq!(
            result.reparsed_bytes,
            state.source.len(),
            "large edit must trigger full reparse, reparsed_bytes should equal doc length"
        );

        Ok(())
    }

    /// Verify large deletions trigger full-reparse fallback the same as large insertions.
    #[test]
    fn test_incremental_state_large_deletion_falls_back_to_full_reparse() -> Result<()> {
        let source = "my $x = 1;\n".repeat(10_000);
        let mut state = IncrementalState::new(source);

        // Delete almost the entire document; this should use full-reparse fallback
        // to keep incremental behavior predictable for large edits.
        let old_end_byte = state.source.len().saturating_sub(1);
        let edit = Edit { start_byte: 0, old_end_byte, new_end_byte: 0, new_text: String::new() };

        let result = apply_edits(&mut state, &[edit])?;

        assert_eq!(
            result.reparsed_bytes,
            state.source.len(),
            "large deletion must trigger full reparse, reparsed_bytes should equal doc length"
        );

        Ok(())
    }

    proptest! {
        #[test]
        fn prop_incremental_apply_edits_matches_ground_truth(
            edits in prop::collection::vec(
                (
                    0usize..900usize,
                    0usize..80usize,
                    "[a-zA-Z0-9_ \\n;\\$\\(\\)\\{\\}\\[\\],]{0,40}"
                ),
                1..60
            )
        ) {
            let mut state = IncrementalState::new("my $seed = 0;\n".repeat(80));
            let mut expected_source = state.source.clone();

            for (start, delete_len, insert_text) in edits {
                let fuzz_edit = FuzzEdit { start, delete_len, insert_text };
                let start_byte = fuzz_edit.start.min(state.source.len());
                let old_end_byte = (start_byte + fuzz_edit.delete_len).min(state.source.len());

                apply_edit_to_ground_truth(&mut expected_source, &fuzz_edit);

                let incremental_edit = Edit {
                    start_byte,
                    old_end_byte,
                    new_end_byte: start_byte + fuzz_edit.insert_text.len(),
                    new_text: fuzz_edit.insert_text,
                };

                let result = apply_edits(&mut state, &[incremental_edit]);
                prop_assert!(result.is_ok());
                prop_assert_eq!(&state.source, &expected_source);
            }

            let reparsed = IncrementalState::new(expected_source);
            prop_assert_eq!(&state.source, &reparsed.source);
        }
    }
}
