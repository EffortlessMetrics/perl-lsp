//! Property-based tests for `perl-line-index` invariants.
//!
//! Invariants tested:
//! - **Round-trip**: every byte offset in `0..=text.len()` produced by
//!   `byte_to_position` maps back via `position_to_byte` to the original byte.
//! - **Monotonicity**: if `b1 < b2` then the (line, col) position of `b1` is
//!   lexicographically less than or equal to that of `b2`.
//! - **line 0 always exists**: `position_to_byte(0, 0)` is always `Some(0)`.
//! - **Out-of-range line returns `None`**: line index >= number of newlines + 1.
//! - **`checked` and `unchecked` agree on the final line** of a document.
//! - **Empty input**: a single line at (0,0) with byte 0.
//! - **Single `\n`**: two lines, with correct byte assignments.
//! - **Trailing `\n`**: creates an empty final line.
//! - **No trailing `\n`**: final line's last column is `text_len - last_line_start`.

use perl_line_index::LineIndex;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Text generation strategies
// ---------------------------------------------------------------------------

/// Strategy that builds a text string from "fragments" that are either
/// printable ASCII runs or newlines, keeping the total length bounded.
/// This avoids raw `any::<String>()` while still generating diverse input.
fn text_with_newlines(max_len: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            // Printable ASCII fragment (a-z, A-Z, digits, spaces)
            "[a-zA-Z0-9 ]{0,16}".prop_map(|s| s),
            // Newline fragment
            Just("\n".to_string()),
            // CR+LF fragment (CRLF: \r is treated as a regular byte)
            Just("\r\n".to_string()),
            // Single carriage return (regular byte, not a line terminator)
            Just("\r".to_string()),
        ],
        0..=(max_len / 2 + 1),
    )
    .prop_map(move |fragments| {
        let joined = fragments.concat();
        // Clamp to max_len bytes (fragments may individually be short, so this is
        // only a safety net against edge cases).
        if joined.len() > max_len { joined[..max_len].to_string() } else { joined }
    })
}

// ---------------------------------------------------------------------------
// Round-trip: byte_to_position then position_to_byte
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// Every byte in `0..text.len()` round-trips through `byte_to_position` and back.
    #[test]
    fn byte_to_position_and_back_roundtrip(text in text_with_newlines(256)) {
        let idx = LineIndex::new(&text);
        for byte in 0..=text.len() {
            let (line, col) = idx.byte_to_position(byte);
            let back = idx.position_to_byte(line, col);
            prop_assert_eq!(
                back,
                Some(byte),
                "roundtrip failed: byte {} -> ({}, {}) -> {:?} in text {:?}",
                byte,
                line,
                col,
                back,
                text
            );
        }
    }

    /// `byte_to_position` is monotonically non-decreasing (lexicographic).
    ///
    /// For `b1 < b2`, either `line(b1) < line(b2)`, or `line(b1) == line(b2)` and
    /// `col(b1) <= col(b2)`.
    #[test]
    fn byte_to_position_is_monotone(text in text_with_newlines(256)) {
        let idx = LineIndex::new(&text);
        let len = text.len();
        if len == 0 {
            return Ok(());
        }
        let mut prev_pos = idx.byte_to_position(0);
        for byte in 1..=len {
            let pos = idx.byte_to_position(byte);
            let (pl, pc) = prev_pos;
            let (cl, cc) = pos;
            // lexicographic: (pl, pc) <= (cl, cc)
            prop_assert!(
                (pl, pc) <= (cl, cc),
                "monotonicity violated: byte {} gives ({}, {}) but byte {} gave ({}, {})",
                byte,
                cl,
                cc,
                byte - 1,
                pl,
                pc
            );
            prev_pos = pos;
        }
    }

    /// Line 0, col 0 is always byte 0.
    #[test]
    fn position_zero_zero_is_always_byte_zero(text in text_with_newlines(256)) {
        let idx = LineIndex::new(&text);
        prop_assert_eq!(idx.position_to_byte(0, 0), Some(0));
    }

    /// A line index beyond the last line returns `None`.
    #[test]
    fn out_of_range_line_returns_none(text in text_with_newlines(128)) {
        let newline_count = text.chars().filter(|&c| c == '\n').count();
        let line_count = newline_count + 1;
        let idx = LineIndex::new(&text);
        // line_count is one past the last valid line
        prop_assert_eq!(
            idx.position_to_byte(line_count, 0),
            None,
            "expected None for line {} (line_count={})",
            line_count,
            line_count
        );
    }

    /// `position_to_byte` and `position_to_byte_checked` agree on the final line.
    ///
    /// The two methods have the same semantics for the final line (no following
    /// `next_line_start` to subtract one from).
    #[test]
    fn checked_and_unchecked_agree_on_final_line(text in text_with_newlines(128)) {
        let newline_count = text.chars().filter(|&c| c == '\n').count();
        let final_line = newline_count;
        let idx = LineIndex::new(&text);
        // Probe final line for a range of columns
        for col in 0..=(text.len() + 2) {
            let a = idx.position_to_byte(final_line, col);
            let b = idx.position_to_byte_checked(final_line, col);
            prop_assert_eq!(
                a,
                b,
                "methods diverged on final line {} col {}: unchecked={:?} checked={:?}",
                final_line,
                col,
                a,
                b
            );
        }
    }

    /// Newlines split the text into lines: the byte at each `\n` is on the line
    /// before it, and the next byte starts a new line.
    #[test]
    fn newline_byte_is_on_current_line_next_byte_starts_next_line(
        text in text_with_newlines(256),
    ) {
        let idx = LineIndex::new(&text);
        for (byte, ch) in text.char_indices() {
            if ch == '\n' {
                let (nl_line, _nl_col) = idx.byte_to_position(byte);
                // byte immediately after the newline (if within text)
                let next_byte = byte + 1;
                if next_byte <= text.len() {
                    let (next_line, next_col) = idx.byte_to_position(next_byte);
                    prop_assert_eq!(
                        next_line,
                        nl_line + 1,
                        "byte after newline at {} should be on line {} not {}",
                        byte,
                        nl_line + 1,
                        next_line
                    );
                    prop_assert_eq!(
                        next_col,
                        0,
                        "byte after newline at {} should be col 0 not {}",
                        byte,
                        next_col
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Targeted regression / boundary cases
// ---------------------------------------------------------------------------

/// Empty input: line 0 starts at byte 0, no further lines.
#[test]
fn empty_input_single_line_at_origin() {
    let idx = LineIndex::new("");
    assert_eq!(idx.byte_to_position(0), (0, 0));
    assert_eq!(idx.position_to_byte(0, 0), Some(0));
    assert_eq!(idx.position_to_byte(1, 0), None);
}

/// Single `\n`: two lines, byte 0 is the newline on line 0, byte 1 is line 1 col 0.
#[test]
fn single_newline_is_two_lines() {
    let idx = LineIndex::new("\n");
    assert_eq!(idx.byte_to_position(0), (0, 0));
    assert_eq!(idx.byte_to_position(1), (1, 0));
    assert_eq!(idx.position_to_byte(0, 0), Some(0));
    assert_eq!(idx.position_to_byte(1, 0), Some(1));
    assert_eq!(idx.position_to_byte(2, 0), None);
}

/// Trailing `\n` creates an empty final line.
#[test]
fn trailing_newline_creates_empty_final_line() {
    let idx = LineIndex::new("foo\n");
    // byte 4 is the start of the empty final line
    assert_eq!(idx.byte_to_position(4), (1, 0));
    assert_eq!(idx.position_to_byte(1, 0), Some(4));
    assert_eq!(idx.position_to_byte(1, 1), None);
}

/// No trailing `\n`: final line column runs to the last byte.
#[test]
fn no_trailing_newline_final_line_extends_to_text_len() {
    let text = "ab\ncd";
    let idx = LineIndex::new(text);
    // line 1: "cd" starts at byte 3, so col 0..2 are valid
    assert_eq!(idx.position_to_byte(1, 0), Some(3));
    assert_eq!(idx.position_to_byte(1, 1), Some(4));
    assert_eq!(idx.position_to_byte(1, 2), Some(5)); // one past last byte is allowed
    assert_eq!(idx.position_to_byte(1, 3), None);
}

/// Consecutive newlines create empty intermediate lines.
#[test]
fn consecutive_newlines_produce_empty_lines() {
    let idx = LineIndex::new("\n\n\n");
    assert_eq!(idx.byte_to_position(0), (0, 0));
    assert_eq!(idx.byte_to_position(1), (1, 0));
    assert_eq!(idx.byte_to_position(2), (2, 0));
    assert_eq!(idx.byte_to_position(3), (3, 0));
    assert_eq!(idx.position_to_byte(3, 0), Some(3));
    assert_eq!(idx.position_to_byte(3, 1), None);
    assert_eq!(idx.position_to_byte(4, 0), None);
}

/// CRLF: `\r` is a regular byte, only `\n` terminates a line.
#[test]
fn crlf_carriage_return_is_not_a_line_terminator() {
    let text = "a\r\nb";
    let idx = LineIndex::new(text);
    // \r at byte 1 stays on line 0; \n at byte 2 also on line 0
    assert_eq!(idx.byte_to_position(1), (0, 1)); // \r
    assert_eq!(idx.byte_to_position(2), (0, 2)); // \n
    assert_eq!(idx.byte_to_position(3), (1, 0)); // b starts line 1
    assert_eq!(idx.position_to_byte_checked(0, 2), Some(2)); // \n is addressable
    assert_eq!(idx.position_to_byte_checked(0, 3), None); // start of next line
}

/// EOF offset (== text.len()) round-trips correctly for a multi-line text.
#[test]
fn eof_offset_roundtrip() {
    let text = "hello\nworld";
    let idx = LineIndex::new(text);
    let eof = text.len();
    let (line, col) = idx.byte_to_position(eof);
    assert_eq!(idx.position_to_byte(line, col), Some(eof));
}
