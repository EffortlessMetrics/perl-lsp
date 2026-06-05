/// Regression tests for char-boundary-safe slicing in ErrorClassifier::classify.
///
/// Confirmed panic (Finding 1 from issue #750): source `"1234567😀z"` with an
/// error node at start=0 → `end = (0 + 10).min(source.len()) = 10`, which lands
/// inside the 4-byte emoji (bytes 7..11), causing:
///   byte index 10 is not a char boundary; it is inside '😀' (bytes 7..11)
///
/// The fix must ensure both `start` and `end` are snapped to valid char
/// boundaries before indexing into `source`.
use perl_parser_core::error_classifier::{ErrorClassifier, ParseErrorKind};
use perl_parser_core::{Node as V1Node, NodeKind as V1NodeKind, SourceLocation};

/// Build a minimal Error node at the given byte offset.
fn error_node_at(start: usize, end: usize) -> V1Node {
    V1Node::new(
        V1NodeKind::Error {
            message: "test error".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        SourceLocation::new(start, end),
    )
}

// ── 4-byte emoji (😀 = U+1F600 → 4 bytes) ────────────────────────────────────

/// "1234567😀z" = bytes [49,50,51,52,53,54,55, F0,9F,98,80, 7A]
///                indices 0  1  2  3  4  5  6  7  8  9  10  11
/// start=0, end=(0+10).min(12)=10 → byte 10 is inside the emoji → PANIC before fix.
#[test]
fn test_classify_emoji_start0_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let source = "1234567\u{1F600}z"; // 7 ASCII + 4-byte emoji + 1 ASCII = 12 bytes
    assert_eq!(source.len(), 12, "precondition: byte length");

    // byte 10 is inside the emoji – was the panic site
    assert!(!source.is_char_boundary(10), "precondition: byte 10 is mid-char");

    let classifier = ErrorClassifier::new();
    let node = error_node_at(0, 7);
    // Must not panic; return value must be a valid ParseErrorKind variant.
    let _kind = classifier.classify(&node, source);
    Ok(())
}

/// Error node placed so start itself is mid-emoji (should not panic either).
#[test]
fn test_classify_emoji_start_mid_char_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let source = "1234567\u{1F600}z";
    // Place error node at byte 8 (inside the emoji)
    let node = error_node_at(8, 9);
    let classifier = ErrorClassifier::new();
    let _kind = classifier.classify(&node, source);
    Ok(())
}

// ── 2-byte latin (é = U+00E9 → 2 bytes) ─────────────────────────────────────

/// "123456789é" = 9 ASCII bytes + 2-byte é = 11 bytes total
/// start=0, end=(0+10).min(11)=10 → byte 10 is the second byte of é → PANIC.
#[test]
fn test_classify_two_byte_char_boundary_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let source = "123456789\u{00E9}"; // 9 ASCII + 2-byte é = 11 bytes
    assert_eq!(source.len(), 11, "precondition: byte length");
    assert!(!source.is_char_boundary(10), "precondition: byte 10 is mid é");

    let classifier = ErrorClassifier::new();
    let node = error_node_at(0, 9);
    let _kind = classifier.classify(&node, source);
    Ok(())
}

// ── 3-byte CJK (中 = U+4E2D → 3 bytes) ──────────────────────────────────────

/// "12345678中z" = 8 ASCII + 3-byte 中 + 1 ASCII = 12 bytes
/// start=0, end=10 → byte 10 is the third byte of 中 (bytes 8..11) → PANIC.
#[test]
fn test_classify_three_byte_char_boundary_does_not_panic() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "12345678\u{4E2D}z"; // 8 ASCII + 3-byte CJK + 1 ASCII = 12 bytes
    assert_eq!(source.len(), 12, "precondition: byte length");
    assert!(!source.is_char_boundary(10), "precondition: byte 10 is mid 中");

    let classifier = ErrorClassifier::new();
    let node = error_node_at(0, 8);
    let _kind = classifier.classify(&node, source);
    Ok(())
}

/// Error node whose `start` offset sits mid-char in a 3-byte CJK character.
/// The line-context slicing `source[..pos]` and `source[pos..]` must not panic.
#[test]
fn test_classify_pos_mid_cjk_char_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let source = "12345678\u{4E2D}z";
    // byte 9 is inside 中 (bytes 8..11)
    assert!(!source.is_char_boundary(9), "precondition: byte 9 is mid 中");

    let classifier = ErrorClassifier::new();
    let node = error_node_at(9, 10); // start mid-char
    let _kind = classifier.classify(&node, source);
    Ok(())
}

// ── ASCII regression — correct snippet and classification still works ─────────

/// Pure-ASCII input must still classify correctly after the fix.
#[test]
fn test_classify_ascii_unclosed_string_regression() -> Result<(), Box<dyn std::error::Error>> {
    let classifier = ErrorClassifier::new();
    let source = r#"my $x = "hello"#; // odd double-quote count → UnclosedString
    let node = error_node_at(9, 15);
    let kind = classifier.classify(&node, source);
    assert_eq!(
        kind,
        ParseErrorKind::UnclosedString,
        "ASCII unclosed-string detection must still work"
    );
    Ok(())
}

/// Pure-ASCII missing-semicolon detection must still work after the fix.
#[test]
fn test_classify_ascii_missing_semicolon_regression() -> Result<(), Box<dyn std::error::Error>> {
    let classifier = ErrorClassifier::new();
    let source = "my $x = 42\nmy $y = 10;";
    let node = error_node_at(10, 11); // newline at position 10
    let kind = classifier.classify(&node, source);
    assert_eq!(
        kind,
        ParseErrorKind::MissingSemicolon,
        "ASCII missing-semicolon detection must still work"
    );
    Ok(())
}

/// Short ASCII source where start+10 > source.len() — boundary clamping edge case.
#[test]
fn test_classify_short_ascii_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let classifier = ErrorClassifier::new();
    let source = "abc";
    let node = error_node_at(0, 3);
    let _kind = classifier.classify(&node, source);
    Ok(())
}
