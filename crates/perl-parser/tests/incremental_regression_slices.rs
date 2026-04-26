#![cfg(feature = "incremental")]

use perl_parser::incremental::{Edit, IncrementalState, apply_edits};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn token_signatures(state: &IncrementalState) -> Vec<String> {
    state
        .tokens
        .iter()
        .map(|token| format!("{:?}|{}|{}|{}", token.token_type, token.start, token.end, token.text))
        .collect()
}

/// Assert that the incremental state's token stream matches a fresh full lex of the same source.
///
/// Note: this checks token-stream equivalence (type, byte span, text), not AST structural
/// equivalence. It is a sufficient regression guard for the incremental lexer sync path — if
/// tokens diverge, the AST will diverge too. For AST-level coverage see the proptest in
/// `perl_parser::incremental::tests::prop_incremental_apply_edits_matches_ground_truth`.
fn assert_equivalent_to_full_parse(state: &IncrementalState) {
    let full = IncrementalState::new(state.source.clone());
    assert_eq!(state.source, full.source, "source diverged from full parse state");
    assert_eq!(
        token_signatures(state),
        token_signatures(&full),
        "token stream diverged from full parse state"
    );
}

fn replace_first_edit(source: &str, from: &str, to: &str) -> Result<(Edit, String), String> {
    let start = source.find(from).ok_or_else(|| format!("missing pattern '{from}' in source"))?;
    let old_end = start + from.len();
    let updated = format!("{}{}{}", &source[..start], to, &source[old_end..]);
    let edit = Edit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: start + to.len(),
        new_text: to.to_string(),
    };
    Ok((edit, updated))
}

#[test]
fn large_deletion_falls_back_and_matches_full_parse() -> TestResult {
    let removed = "x".repeat(1500);
    let source = format!("my $prefix = 1;\n{removed}\nmy $suffix = 2;\n");
    let start = source.find(&removed).ok_or("deletion segment not found")?;
    let old_end = start + removed.len();

    let mut state = IncrementalState::new(source);
    let edit = Edit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: start,
        new_text: String::new(),
    };
    let result = apply_edits(&mut state, &[edit])?;

    // Verify full-reparse fallback fired (touched_bytes = 1500 > 1024 threshold).
    assert_eq!(result.reparsed_bytes, state.source.len(), "large deletion should use full reparse");
    // Verify the deleted segment is gone and the surrounding statements survive.
    assert!(!state.source.contains(&"x".repeat(100)), "deleted content must be gone");
    assert!(state.source.contains("$prefix"), "prefix statement must survive");
    assert!(state.source.contains("$suffix"), "suffix statement must survive");
    // Note: assert_equivalent_to_full_parse is intentionally omitted here because after
    // full_reparse the state tokens ARE the result of a fresh lex — comparing them against
    // another fresh lex on the same source would be trivially true (both call IncrementalState::new).
    // The substantive guards above verify the source mutation was correct.
    Ok(())
}

#[test]
fn insertion_invalidation_matches_full_parse() -> TestResult {
    let source = "my $left = 1;\nmy $right = 2;\nprint $left + $right;\n".to_string();
    let mut state = IncrementalState::new(source.clone());
    let insert_at = source.find("my $right").ok_or("insert anchor not found")?;
    let inserted = "my $middle = 99;\n";

    let edit = Edit {
        start_byte: insert_at,
        old_end_byte: insert_at,
        new_end_byte: insert_at + inserted.len(),
        new_text: inserted.to_string(),
    };

    apply_edits(&mut state, &[edit])?;
    assert!(state.source.contains("$middle"));
    assert_equivalent_to_full_parse(&state);
    Ok(())
}

#[test]
fn whitespace_only_edit_matches_full_parse() -> TestResult {
    let source = "my $x = 1;\nprint $x;\n";
    let mut state = IncrementalState::new(source.to_string());
    let (edit, expected) = replace_first_edit(source, "print $x;", "print    $x ;")?;

    apply_edits(&mut state, &[edit])?;

    assert_eq!(state.source, expected);
    assert_equivalent_to_full_parse(&state);
    Ok(())
}

#[test]
fn multibyte_boundary_edit_matches_full_parse() -> TestResult {
    let source = "my $emoji = \"😀\";\nprint $emoji;\n";
    let mut state = IncrementalState::new(source.to_string());
    let (edit, expected) = replace_first_edit(source, "😀", "🐪")?;

    apply_edits(&mut state, &[edit])?;

    assert_eq!(state.source, expected);
    assert_equivalent_to_full_parse(&state);
    Ok(())
}

#[test]
fn batch_edits_with_independent_shifts_match_full_parse() -> TestResult {
    let source = "my $alpha = 1;\nmy $beta = 2;\nmy $gamma = 3;\n".to_string();
    let mut state = IncrementalState::new(source.clone());

    let alpha_start = source.find("1").ok_or("missing alpha value")?;
    let gamma_start = source.find("3").ok_or("missing gamma value")?;

    let edits = vec![
        Edit {
            start_byte: alpha_start,
            old_end_byte: alpha_start + 1,
            new_end_byte: alpha_start + 2,
            new_text: "10".to_string(),
        },
        Edit {
            start_byte: gamma_start,
            old_end_byte: gamma_start + 1,
            new_end_byte: gamma_start + 2,
            new_text: "30".to_string(),
        },
    ];

    let result = apply_edits(&mut state, &edits)?;

    // Multiple edits always trigger full reparse (the MVP handles only single edits incrementally).
    assert_eq!(result.reparsed_bytes, state.source.len(), "multiple edits should use full reparse");
    // Verify both edits were applied in the correct non-overlapping positions.
    assert!(state.source.contains("$alpha = 10"), "alpha edit must be applied");
    assert!(state.source.contains("$gamma = 30"), "gamma edit must be applied");
    // Verify beta was not accidentally mutated by adjacent edits.
    assert!(state.source.contains("$beta = 2"), "beta must be unchanged");
    // Note: assert_equivalent_to_full_parse is intentionally omitted here because the
    // multi-edit path goes through full_reparse — comparing its tokens against another
    // fresh lex of the same source would be trivially identical.
    Ok(())
}

#[test]
fn out_of_range_edit_does_not_panic() -> TestResult {
    // An edit whose start_byte and old_end_byte are both beyond the end of the
    // source must be handled gracefully — no panic, no data loss.  Both byte
    // values are clamped to source.len() inside apply_single_edit, turning this
    // into an effective append.  The gap between start_byte and old_end_byte
    // (1025 bytes) exceeds the 1024-byte single-edit fallback threshold, so the
    // code path taken is: apply_single_edit (with clamped offsets) followed by
    // full_reparse.
    let source = "my $x = 1;\n".to_string();
    let mut state = IncrementalState::new(source.clone());
    let append = "print $x;\n";

    // start_byte = source.len(), old_end_byte = source.len() + 1025 so that
    // touched_bytes = max(1025, append.len()) = 1025 > 1024, triggering the
    // large-edit fallback in apply_edits.
    let edit = Edit {
        start_byte: source.len(),
        old_end_byte: source.len() + 1025,
        new_end_byte: source.len() + append.len(),
        new_text: append.to_string(),
    };

    let result = apply_edits(&mut state, &[edit])?;

    // Full reparse must have fired (touched_bytes = 1025 > 1024 threshold).
    assert_eq!(
        result.reparsed_bytes,
        state.source.len(),
        "large out-of-range edit should trigger full reparse fallback"
    );
    // The clamped append must produce the correct combined source — no data corruption,
    // no truncation, no duplication.
    assert_eq!(state.source, format!("{source}{append}"));
    // Token count must be non-zero: the combined source has real Perl statements.
    assert!(!state.tokens.is_empty(), "token stream must be populated after out-of-range append");
    // Note: assert_equivalent_to_full_parse is intentionally omitted because the large-edit
    // path goes through full_reparse — its token stream is already a fresh lex, so a
    // second fresh lex comparison would be trivially identical.
    Ok(())
}
