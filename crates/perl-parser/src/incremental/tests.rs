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

#[test]
fn test_incremental_state_small_edit_uses_checkpoint() -> Result<()> {
    let source = (0..30usize).map(|i| format!("my $var_{i} = {i};")).collect::<Vec<_>>().join("\n");
    let doc_len = source.len();
    let mut state = IncrementalState::new(source.clone());
    assert!(state.lex_checkpoints.len() > 1);
    let edit_start = source.rfind("29;").unwrap_or(source.len() - 3);
    let edit = Edit {
        start_byte: edit_start,
        old_end_byte: edit_start + 2,
        new_end_byte: edit_start + 3,
        new_text: "999".to_string(),
    };
    let result = apply_edits(&mut state, &[edit])?;
    assert!(result.reparsed_bytes < doc_len);
    Ok(())
}

proptest! {
    #[test]
    fn prop_incremental_apply_edits_matches_ground_truth(
        edits in prop::collection::vec(
            (0usize..240usize, 0usize..24usize, "[a-zA-Z0-9_ ]{0,24}"),
            1..20,
        )
    ) {
        let mut state = IncrementalState::new("my $seed = 0;\n".repeat(80));
        let mut expected = state.source.clone();

        for (start, delete_len, insert_text) in edits {
            let fuzz = FuzzEdit { start, delete_len, insert_text };
            let start_byte = fuzz.start.min(state.source.len());
            let old_end = (start_byte + fuzz.delete_len).min(state.source.len());
            apply_edit_to_ground_truth(&mut expected, &fuzz);
            let edit = Edit {
                start_byte,
                old_end_byte: old_end,
                new_end_byte: start_byte + fuzz.insert_text.len(),
                new_text: fuzz.insert_text,
            };
            prop_assert!(apply_edits(&mut state, &[edit]).is_ok());
            prop_assert_eq!(&state.source, &expected);
        }
    }
}
