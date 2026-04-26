#![cfg(feature = "incremental")]

use perl_parser::incremental::{Edit, IncrementalState, apply_edits};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn range(&mut self, max_exclusive: usize) -> usize {
        if max_exclusive == 0 {
            return 0;
        }
        (self.next_u64() as usize) % max_exclusive
    }
}

fn random_ascii_text(rng: &mut XorShift64, max_len: usize) -> String {
    const ALPHABET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_$;=() \n\t";
    let len = rng.range(max_len + 1);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let idx = rng.range(ALPHABET.len());
        out.push(ALPHABET[idx] as char);
    }
    out
}

fn random_edit_for_source(rng: &mut XorShift64, source: &str) -> Edit {
    let start = rng.range(source.len() + 1);
    let delete_len = rng.range(source.len().saturating_sub(start) + 1).min(32);
    let old_end = start + delete_len;
    let new_text = random_ascii_text(rng, 32);
    Edit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: start + new_text.len(),
        new_text,
    }
}

fn apply_edit_to_string(source: &mut String, edit: &Edit) {
    source.replace_range(edit.start_byte..edit.old_end_byte, &edit.new_text);
}

#[test]
fn test_incremental_state_creation() {
    let source = "my $x = 42;\nprint $x;".to_string();
    let state = IncrementalState::new(source.clone());

    assert_eq!(state.source, source);
    assert!(!state.lex_checkpoints.is_empty());
    assert!(!state.tokens.is_empty());
}

#[test]
fn test_single_character_edit() -> TestResult {
    let source = "my $x = 1;".to_string();
    let mut state = IncrementalState::new(source);

    // Change 1 to 2
    let edit = Edit { start_byte: 8, old_end_byte: 9, new_end_byte: 9, new_text: "2".to_string() };

    let result = apply_edits(&mut state, &[edit])?;

    assert_eq!(state.source, "my $x = 2;");
    assert!(result.reparsed_bytes > 0);
    assert!(!result.changed_ranges.is_empty());
    Ok(())
}

#[test]
fn test_multi_character_insertion() -> TestResult {
    let source = "my $x = ;".to_string();
    let mut state = IncrementalState::new(source);

    // Insert "42"
    let edit =
        Edit { start_byte: 8, old_end_byte: 8, new_end_byte: 10, new_text: "42".to_string() };

    let result = apply_edits(&mut state, &[edit])?;

    assert_eq!(state.source, "my $x = 42;");
    assert!(result.reparsed_bytes > 0);
    Ok(())
}

#[test]
fn test_line_deletion() -> TestResult {
    let source = "my $x = 1;\nmy $y = 2;\nprint $x;".to_string();
    let mut state = IncrementalState::new(source);

    // Delete second line
    let edit =
        Edit { start_byte: 11, old_end_byte: 22, new_end_byte: 11, new_text: "".to_string() };

    let result = apply_edits(&mut state, &[edit])?;

    assert_eq!(state.source, "my $x = 1;\nprint $x;");
    assert!(result.reparsed_bytes > 0);
    Ok(())
}

#[test]
fn test_checkpoint_creation() -> TestResult {
    let source = "sub foo {\n    return 1;\n}\n\nsub bar {\n    return 2;\n}".to_string();
    let state = IncrementalState::new(source);

    // Should have checkpoints at sub boundaries
    assert!(state.lex_checkpoints.len() > 2);

    // Find checkpoint before "sub bar"
    let bar_pos = state.source.find("sub bar").ok_or("expected 'sub bar' in source")?;
    let checkpoint = state.find_lex_checkpoint(bar_pos);
    assert!(checkpoint.is_some());
    Ok(())
}

#[test]
fn test_large_edit_fallback() -> TestResult {
    let source = "my $x = 1;".to_string();
    let mut state = IncrementalState::new(source);

    // Large insertion (>1KB) should trigger full reparse
    let large_text = "x".repeat(2000);
    let edit = Edit {
        start_byte: 10,
        old_end_byte: 10,
        new_end_byte: 10 + large_text.len(),
        new_text: large_text,
    };

    let result = apply_edits(&mut state, &[edit])?;

    // Should have reparsed entire document
    assert_eq!(result.reparsed_bytes, state.source.len());
    Ok(())
}

#[test]
fn test_incremental_vs_full_parse_equivalence() -> TestResult {
    let initial = "my $x = 1;\nmy $y = 2;".to_string();
    let mut incremental_state = IncrementalState::new(initial.clone());

    // Apply edit incrementally
    let edit =
        Edit { start_byte: 8, old_end_byte: 9, new_end_byte: 10, new_text: "10".to_string() };
    apply_edits(&mut incremental_state, &[edit])?;

    // Full parse of the result
    let expected = "my $x = 10;\nmy $y = 2;".to_string();
    let full_state = IncrementalState::new(expected.clone());

    // ASTs should be equivalent
    assert_eq!(incremental_state.source, full_state.source);
    // Note: Deep AST comparison would require PartialEq on Node
    Ok(())
}

#[test]
fn test_edit_at_statement_boundary() -> TestResult {
    let source = "my $x = 1;\nmy $y = 2;\nmy $z = 3;".to_string();
    let mut state = IncrementalState::new(source);

    // Edit at semicolon boundary
    let edit = Edit {
        start_byte: 10,   // After first semicolon
        old_end_byte: 11, // Newline
        new_end_byte: 34,
        new_text: "\n# Comment\nmy $w = 0;\n".to_string(),
    };

    let result = apply_edits(&mut state, &[edit])?;

    assert!(state.source.contains("# Comment"));
    assert!(state.source.contains("my $w = 0"));
    // Should have used checkpoint at semicolon
    assert!(result.reparsed_bytes < state.source.len());
    Ok(())
}

#[test]
fn test_multiple_edits_fallback() -> TestResult {
    let source = "my $x = 1;\nmy $y = 2;".to_string();
    let mut state = IncrementalState::new(source);

    // Multiple edits trigger full reparse (MVP limitation)
    let edits = vec![
        Edit { start_byte: 8, old_end_byte: 9, new_end_byte: 9, new_text: "5".to_string() },
        Edit { start_byte: 19, old_end_byte: 20, new_end_byte: 20, new_text: "6".to_string() },
    ];

    let result = apply_edits(&mut state, &edits)?;

    // Should fallback to full parse
    assert_eq!(result.reparsed_bytes, state.source.len());
    Ok(())
}

#[test]
fn test_edit_in_subroutine() -> TestResult {
    let source = "sub foo {\n    my $x = 1;\n    return $x;\n}".to_string();
    let mut state = IncrementalState::new(source);

    // Edit inside subroutine
    let edit = Edit {
        start_byte: 22, // The "1" in "$x = 1"
        old_end_byte: 23,
        new_end_byte: 24,
        new_text: "42".to_string(),
    };

    let result = apply_edits(&mut state, &[edit])?;

    assert_eq!(state.source, "sub foo {\n    my $x = 42;\n    return $x;\n}");
    // Should have checkpoint at sub start
    assert!(result.reparsed_bytes > 0);
    Ok(())
}

#[test]
fn fuzz_incremental_random_edit_sequences_match_ground_truth() -> TestResult {
    let mut rng = XorShift64::seeded(0x4999_FEED_CAFE_BABE);
    let mut initial = String::from("my $x = 1;\nmy $y = 2;\nsub f { return $x + $y; }\n");
    initial.push_str(&random_ascii_text(&mut rng, 64));

    for _case in 0..200 {
        let mut incremental_state = IncrementalState::new(initial.clone());
        let mut expected = initial.clone();

        for _step in 0..100 {
            let edit = random_edit_for_source(&mut rng, &expected);
            apply_edit_to_string(&mut expected, &edit);
            apply_edits(&mut incremental_state, &[edit])?;
        }

        assert_eq!(
            incremental_state.source, expected,
            "incremental apply_edits source mismatch after random edit sequence"
        );

        let full_state = IncrementalState::new(expected);
        assert_eq!(
            incremental_state.source, full_state.source,
            "incremental final document diverged from full parse source"
        );
    }

    Ok(())
}
