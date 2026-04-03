//! Comprehensive unit tests for `perl-text-line` crate.
//!
//! Covers all public functions: `line_bounds_at`, `is_identifier_byte`,
//! `is_keyword_boundary`, and `skip_ascii_whitespace` with edge-case and
//! boundary testing.

use perl_tdd_support::must_some;
use perl_text_line::{
    is_identifier_byte, is_keyword_boundary, line_bounds_at, skip_ascii_whitespace,
};

// ───────────────────────────────────────────────────────────────────
// line_bounds_at
// ───────────────────────────────────────────────────────────────────

#[test]
fn line_bounds_empty_string() {
    let (start, end) = line_bounds_at("", 0);
    assert_eq!((start, end), (0, 0));
}

#[test]
fn line_bounds_empty_string_cursor_beyond() {
    let (start, end) = line_bounds_at("", 100);
    assert_eq!((start, end), (0, 0));
}

#[test]
fn line_bounds_single_char_no_newline() {
    let (start, end) = line_bounds_at("x", 0);
    assert_eq!((start, end), (0, 1));
}

#[test]
fn line_bounds_cursor_at_end_single_line() {
    let text = "hello";
    let (start, end) = line_bounds_at(text, text.len());
    assert_eq!((start, end), (0, 5));
}

#[test]
fn line_bounds_cursor_beyond_text_clamps() {
    let text = "abc";
    let (start, end) = line_bounds_at(text, 1000);
    assert_eq!((start, end), (0, 3));
}

#[test]
fn line_bounds_first_line_of_multiline() {
    let text = "first\nsecond\nthird";
    let (start, end) = line_bounds_at(text, 0);
    assert_eq!(&text[start..end], "first");
}

#[test]
fn line_bounds_middle_line_of_multiline() {
    let text = "first\nsecond\nthird";
    let cursor = 8; // inside "second"
    let (start, end) = line_bounds_at(text, cursor);
    assert_eq!(&text[start..end], "second");
}

#[test]
fn line_bounds_last_line_of_multiline() {
    let text = "first\nsecond\nthird";
    let cursor = 15; // inside "third"
    let (start, end) = line_bounds_at(text, cursor);
    assert_eq!(&text[start..end], "third");
}

#[test]
fn line_bounds_cursor_exactly_on_newline() {
    let text = "abc\ndef";
    let (start, end) = line_bounds_at(text, 3);
    assert_eq!(&text[start..end], "abc");
}

#[test]
fn line_bounds_cursor_just_after_newline() {
    let text = "abc\ndef";
    let (start, end) = line_bounds_at(text, 4);
    assert_eq!(&text[start..end], "def");
}

#[test]
fn line_bounds_trailing_newline() {
    let text = "line1\n";
    let (start, end) = line_bounds_at(text, 6);
    assert_eq!((start, end), (6, 6));
    assert_eq!(&text[start..end], "");
}

#[test]
fn line_bounds_multiple_empty_lines() {
    let text = "\n\n\n";
    assert_eq!(line_bounds_at(text, 0), (0, 0));
    assert_eq!(line_bounds_at(text, 1), (1, 1));
    assert_eq!(line_bounds_at(text, 3), (3, 3));
}

#[test]
fn line_bounds_crlf_line_endings() {
    let text = "line1\r\nline2\r\n";
    let (s1, e1) = line_bounds_at(text, 0);
    assert_eq!(&text[s1..e1], "line1\r");

    let (s2, e2) = line_bounds_at(text, 7);
    assert_eq!(&text[s2..e2], "line2\r");
}

#[test]
fn line_bounds_unicode_content() {
    let text = "café\nnaïve\n日本語";
    // "café" = 5 bytes (é=2), \n at 5, "naïve" at 6..12 (ï=2), \n at 12, "日本語" at 13..22
    let (s0, e0) = line_bounds_at(text, 0);
    assert_eq!(&text[s0..e0], "café");

    let (s1, e1) = line_bounds_at(text, 8);
    assert_eq!(&text[s1..e1], "naïve");

    // Use byte 13 — start of "日本語" (valid char boundary)
    let (s2, e2) = line_bounds_at(text, 13);
    assert_eq!(&text[s2..e2], "日本語");
}

#[test]
fn line_bounds_only_newline() {
    let text = "\n";
    assert_eq!(line_bounds_at(text, 0), (0, 0));
    assert_eq!(line_bounds_at(text, 1), (1, 1));
}

#[test]
fn line_bounds_long_line() {
    let long = "a".repeat(10_000);
    let text = format!("before\n{long}\nafter");
    let cursor = 7 + 5000;
    let (start, end) = line_bounds_at(&text, cursor);
    assert_eq!(start, 7);
    assert_eq!(end, 7 + 10_000);
}

// ───────────────────────────────────────────────────────────────────
// is_identifier_byte
// ───────────────────────────────────────────────────────────────────

#[test]
fn identifier_byte_lowercase_letters() {
    for b in b'a'..=b'z' {
        assert!(is_identifier_byte(b), "lowercase {b} should be identifier");
    }
}

#[test]
fn identifier_byte_uppercase_letters() {
    for b in b'A'..=b'Z' {
        assert!(is_identifier_byte(b), "uppercase {b} should be identifier");
    }
}

#[test]
fn identifier_byte_digits() {
    for b in b'0'..=b'9' {
        assert!(is_identifier_byte(b), "digit {b} should be identifier");
    }
}

#[test]
fn identifier_byte_underscore() {
    assert!(is_identifier_byte(b'_'));
}

#[test]
fn identifier_byte_rejects_punctuation() {
    let non_ident: &[u8] = b" \t\n\r!@#$%^&*()-+=[]{}|;:'\",.<>?/\\`~";
    for &b in non_ident {
        assert!(!is_identifier_byte(b), "byte {b:#04x} should not be identifier");
    }
}

#[test]
fn identifier_byte_null_and_high_bytes() {
    assert!(!is_identifier_byte(0));
    assert!(!is_identifier_byte(0x80));
    assert!(!is_identifier_byte(0xFF));
}

#[test]
fn identifier_byte_exhaustive_ascii_count() {
    let count = (0u8..=127).filter(|&b| is_identifier_byte(b)).count();
    // 26 lowercase + 26 uppercase + 10 digits + 1 underscore = 63
    assert_eq!(count, 63);
}

// ───────────────────────────────────────────────────────────────────
// is_keyword_boundary
// ───────────────────────────────────────────────────────────────────

#[test]
fn keyword_boundary_at_start_of_bytes() {
    assert!(is_keyword_boundary(b"use Foo;", 0, 3));
}

#[test]
fn keyword_boundary_at_end_of_bytes() {
    // "var" at index 4, length 3, end=7 == bytes.len()
    assert!(is_keyword_boundary(b"my $var", 4, 3));
}

#[test]
fn keyword_boundary_surrounded_by_spaces() {
    assert!(is_keyword_boundary(b"  sub  ", 2, 3));
}

#[test]
fn keyword_boundary_preceded_by_identifier() {
    // "sub" at index 2 preceded by 'o'
    assert!(!is_keyword_boundary(b"nosub foo", 2, 3));
}

#[test]
fn keyword_boundary_followed_by_identifier() {
    // "sub" at 0, followed by '_'
    assert!(!is_keyword_boundary(b"sub_routine", 0, 3));
}

#[test]
fn keyword_boundary_both_sides_identifier() {
    // "use" at 1, 'x' on both sides
    assert!(!is_keyword_boundary(b"xusex", 1, 3));
}

#[test]
fn keyword_boundary_empty_bytes_zero_len() {
    let bytes: &[u8] = b"";
    assert!(is_keyword_boundary(bytes, 0, 0));
    assert!(!is_keyword_boundary(bytes, 1, 0));
}

#[test]
fn keyword_boundary_start_beyond_length() {
    assert!(!is_keyword_boundary(b"abc", 10, 3));
}

#[test]
fn keyword_boundary_end_beyond_length() {
    assert!(!is_keyword_boundary(b"abc", 1, 10));
}

#[test]
fn keyword_boundary_zero_length_between_non_ident() {
    // Zero-length at start with following identifier byte
    assert!(!is_keyword_boundary(b"abc", 0, 0));
    // Zero-length between punctuation
    assert!(is_keyword_boundary(b" ; ", 1, 0));
}

#[test]
fn keyword_boundary_entire_bytes() {
    assert!(is_keyword_boundary(b"sub", 0, 3));
}

#[test]
fn keyword_boundary_punctuation_delimiters() {
    assert!(is_keyword_boundary(b"(use)", 1, 3));
    assert!(is_keyword_boundary(b";sub;", 1, 3));
}

#[test]
fn keyword_boundary_len_overflow_saturates() {
    assert!(!is_keyword_boundary(b"abc", 1, usize::MAX));
}

#[test]
fn keyword_boundary_digit_before() {
    assert!(!is_keyword_boundary(b"9sub ", 1, 3));
}

#[test]
fn keyword_boundary_digit_after() {
    assert!(!is_keyword_boundary(b" sub9", 1, 3));
}

#[test]
fn keyword_boundary_newline_delimiter() {
    assert!(is_keyword_boundary(b"\nsub\n", 1, 3));
}

#[test]
fn keyword_boundary_tab_delimiter() {
    assert!(is_keyword_boundary(b"\tsub\t", 1, 3));
}

// ───────────────────────────────────────────────────────────────────
// skip_ascii_whitespace
// ───────────────────────────────────────────────────────────────────

#[test]
fn skip_whitespace_no_whitespace() {
    assert_eq!(skip_ascii_whitespace(b"abc", 0), 0);
}

#[test]
fn skip_whitespace_all_spaces() {
    assert_eq!(skip_ascii_whitespace(b"   ", 0), 3);
}

#[test]
fn skip_whitespace_tabs() {
    assert_eq!(skip_ascii_whitespace(b"\t\tabc", 0), 2);
}

#[test]
fn skip_whitespace_mixed() {
    assert_eq!(skip_ascii_whitespace(b" \t \t x", 0), 5);
}

#[test]
fn skip_whitespace_from_middle() {
    assert_eq!(skip_ascii_whitespace(b"abc   def", 3), 6);
}

#[test]
fn skip_whitespace_empty_bytes() {
    assert_eq!(skip_ascii_whitespace(b"", 0), 0);
}

#[test]
fn skip_whitespace_idx_beyond_length() {
    assert_eq!(skip_ascii_whitespace(b"abc", 100), 100);
}

#[test]
fn skip_whitespace_idx_at_length() {
    assert_eq!(skip_ascii_whitespace(b"abc", 3), 3);
}

#[test]
fn skip_whitespace_newline_is_whitespace() {
    assert_eq!(skip_ascii_whitespace(b"\n \t", 0), 3);
}

#[test]
fn skip_whitespace_carriage_return() {
    assert_eq!(skip_ascii_whitespace(b"\r\nfoo", 0), 2);
}

#[test]
fn skip_whitespace_form_feed_skipped_vertical_tab_not() {
    // Rust's is_ascii_whitespace includes \x0C (form feed) but NOT \x0B (vertical tab)
    assert_eq!(skip_ascii_whitespace(b"\x0Cx", 0), 1);
    assert_eq!(skip_ascii_whitespace(b"\x0Bx", 0), 0);
}

#[test]
fn skip_whitespace_non_ascii_high_byte() {
    let bytes: &[u8] = &[0xA0, b' ', b'x'];
    assert_eq!(skip_ascii_whitespace(bytes, 0), 0);
}

#[test]
fn skip_whitespace_stops_at_non_whitespace_then_more_whitespace() {
    assert_eq!(skip_ascii_whitespace(b"  a  b", 0), 2);
}

// ───────────────────────────────────────────────────────────────────
// Combined / integration-style scenarios
// ───────────────────────────────────────────────────────────────────

#[test]
fn combined_extract_keyword_from_line() {
    let source = "my $x = 1;\nuse strict;\nmy $y = 2;";
    let cursor = 15; // inside "use strict;"
    let (start, end) = line_bounds_at(source, cursor);
    let line = &source[start..end];
    let bytes = line.as_bytes();

    let use_pos = must_some(line.find("use"));
    assert!(is_keyword_boundary(bytes, use_pos, 3));

    let after_use = skip_ascii_whitespace(bytes, use_pos + 3);
    assert!(line[after_use..].starts_with("strict"));
}

#[test]
fn combined_perl_package_declaration() {
    let source = "#!/usr/bin/perl\npackage My::Module;\nuse warnings;\n1;";
    let cursor = 20; // inside "package My::Module;"
    let (start, end) = line_bounds_at(source, cursor);
    let line = &source[start..end];
    let bytes = line.as_bytes();

    let pkg_pos = must_some(line.find("package"));
    assert!(is_keyword_boundary(bytes, pkg_pos, 7));

    let after_pkg = skip_ascii_whitespace(bytes, pkg_pos + 7);
    assert!(line[after_pkg..].starts_with("My::Module"));
}

#[test]
fn combined_all_lines_extractable() {
    let lines_input = ["alpha", "beta", "gamma", "delta"];
    let source = lines_input.join("\n");

    for (i, expected) in lines_input.iter().enumerate() {
        let offset: usize = lines_input[..i].iter().map(|l| l.len() + 1).sum();
        let (start, end) = line_bounds_at(&source, offset);
        assert_eq!(&source[start..end], *expected);
    }
}

#[test]
fn combined_whitespace_then_keyword_boundary() {
    let bytes = b"    sub foo";
    let pos = skip_ascii_whitespace(bytes, 0);
    assert_eq!(pos, 4);
    assert!(is_keyword_boundary(bytes, pos, 3));
}

#[test]
fn combined_line_bounds_preserves_indentation() {
    let source = "    my $x = 1;\n        my $y = 2;\n";
    let (start, end) = line_bounds_at(source, 20);
    let line = &source[start..end];
    assert!(line.starts_with("        "));
    let trimmed_start = skip_ascii_whitespace(line.as_bytes(), 0);
    assert_eq!(&line[trimmed_start..trimmed_start + 2], "my");
}
