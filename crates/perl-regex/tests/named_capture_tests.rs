//! Tests for named capture group extraction and hover text generation.
//!
//! Tests `extract_named_captures` and `hover_text_for_regex` added as part of
//! issue #2339 (feat(perl): Regex named capture group extraction and hover).

use perl_regex::{CaptureGroup, RegexAnalyzer};

// ── extract_named_captures — basic cases ────────────────────────────────

#[test]
fn test_extract_no_captures_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("\\d+");
    assert!(captures.is_empty());
    Ok(())
}

#[test]
fn test_extract_single_named_capture() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("(?<id>\\d+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 1);
    Ok(())
}

#[test]
fn test_extract_multiple_named_captures() -> Result<(), Box<dyn std::error::Error>> {
    let captures =
        RegexAnalyzer::extract_named_captures("(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})");
    assert_eq!(captures.len(), 3);
    assert_eq!(captures[0].name, "year");
    assert_eq!(captures[0].index, 1);
    assert_eq!(captures[1].name, "month");
    assert_eq!(captures[1].index, 2);
    assert_eq!(captures[2].name, "day");
    assert_eq!(captures[2].index, 3);
    Ok(())
}

#[test]
fn test_extract_mixed_named_and_unnamed_captures() -> Result<(), Box<dyn std::error::Error>> {
    // unnamed group counts toward index but produces no CaptureGroup entry
    let captures = RegexAnalyzer::extract_named_captures("(prefix)(?<id>\\d+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 2);
    Ok(())
}

#[test]
fn test_extract_non_capturing_group_not_counted() -> Result<(), Box<dyn std::error::Error>> {
    // (?:...) does not increment capture index
    let captures = RegexAnalyzer::extract_named_captures("(?:prefix)(?<id>\\d+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 1);
    Ok(())
}

#[test]
fn test_extract_lookahead_not_counted_as_capture() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("(?=foo)(?<id>\\d+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 1);
    Ok(())
}

#[test]
fn test_extract_lookbehind_not_counted_as_capture() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("(?<=foo)(?<id>\\d+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 1);
    Ok(())
}

#[test]
fn test_extract_empty_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("");
    assert!(captures.is_empty());
    Ok(())
}

#[test]
fn test_extract_escaped_paren_not_a_group() -> Result<(), Box<dyn std::error::Error>> {
    // \( is not a real group opener
    let captures = RegexAnalyzer::extract_named_captures(r"\(not_a_group\)(?<real>\d+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "real");
    assert_eq!(captures[0].index, 1);
    Ok(())
}

#[test]
fn test_extract_name_with_underscore() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("(?<first_name>\\w+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "first_name");
    Ok(())
}

#[test]
fn test_extract_single_named_capture_with_quote_syntax() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("(?'id'\\d+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 1);
    Ok(())
}

#[test]
fn test_extract_mixed_named_capture_syntaxes() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("(?'prefix'\\w+)-(?<id>\\d+)");
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].name, "prefix");
    assert_eq!(captures[0].index, 1);
    assert_eq!(captures[1].name, "id");
    assert_eq!(captures[1].index, 2);
    Ok(())
}

#[test]
fn test_capture_group_has_pattern_field() -> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures("(?<id>\\d+)");
    assert_eq!(captures[0].pattern, "\\d+");
    Ok(())
}

#[test]
fn test_quote_syntax_empty_name_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    // (?''...) has an empty name — parse_named_capture_name returns None, so it
    // falls through to the "any other (?...)" branch and is not counted as a capture.
    let captures = RegexAnalyzer::extract_named_captures("(?''\\d+)");
    assert!(captures.is_empty(), "empty-name quote capture must not produce a CaptureGroup");
    Ok(())
}

#[test]
fn test_quote_syntax_unclosed_quote_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    // (?'unclosed — no closing quote — must not panic or produce a capture.
    let captures = RegexAnalyzer::extract_named_captures("(?'unclosed\\d+)");
    assert!(captures.is_empty(), "unclosed quote capture must not produce a CaptureGroup");
    Ok(())
}

#[test]
fn test_quote_syntax_sub_pattern_extracted() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that sub-pattern extraction works for (?'name'...) just like (?<name>...).
    let captures = RegexAnalyzer::extract_named_captures("(?'word'\\w+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "word");
    assert_eq!(captures[0].pattern, "\\w+");
    Ok(())
}

#[test]
fn test_quote_syntax_mixed_with_unnamed_group() -> Result<(), Box<dyn std::error::Error>> {
    // An unnamed group before (?'name'...) must increment the capture index.
    let captures = RegexAnalyzer::extract_named_captures("(\\d+)(?'id'\\w+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 2);
    Ok(())
}

#[test]
fn test_capture_pattern_with_char_class_containing_rparen() -> Result<(), Box<dyn std::error::Error>>
{
    // A closing paren inside [...] is literal and must not terminate the group pattern scan.
    let captures = RegexAnalyzer::extract_named_captures(r"(?<tok>[^)]+)");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].name, "tok");
    assert_eq!(captures[0].index, 1);
    assert_eq!(captures[0].pattern, "[^)]+");
    Ok(())
}

// ── CaptureGroup struct ──────────────────────────────────────────────────

#[test]
fn test_capture_group_debug_impl() -> Result<(), Box<dyn std::error::Error>> {
    let cg = CaptureGroup { name: "foo".to_string(), index: 1, pattern: "\\w+".to_string() };
    let s = format!("{cg:?}");
    assert!(s.contains("foo"));
    Ok(())
}

#[test]
fn test_capture_group_clone() -> Result<(), Box<dyn std::error::Error>> {
    let cg = CaptureGroup { name: "bar".to_string(), index: 2, pattern: "\\d+".to_string() };
    let cg2 = cg.clone();
    assert_eq!(cg2.name, "bar");
    assert_eq!(cg2.index, 2);
    Ok(())
}

// ── hover_text_for_regex ─────────────────────────────────────────────────

#[test]
fn test_hover_text_no_captures_no_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("\\d+", "");
    assert!(text.contains("\\d+"));
    Ok(())
}

#[test]
fn test_hover_text_single_named_capture_listed() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("(?<id>\\d+)", "");
    assert!(text.contains("id"));
    Ok(())
}

#[test]
fn test_hover_text_multiple_captures_all_listed() -> Result<(), Box<dyn std::error::Error>> {
    let text =
        RegexAnalyzer::hover_text_for_regex("(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})", "");
    assert!(text.contains("year"));
    assert!(text.contains("month"));
    assert!(text.contains("day"));
    Ok(())
}

#[test]
fn test_hover_text_modifier_i_explained() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("hello", "i");
    assert!(text.to_lowercase().contains("case"));
    Ok(())
}

#[test]
fn test_hover_text_modifier_m_explained() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("^hello$", "m");
    assert!(
        text.to_lowercase().contains("multiline") || text.to_lowercase().contains("multi-line")
    );
    Ok(())
}

#[test]
fn test_hover_text_modifier_s_explained() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex(".*", "s");
    // /s makes . match newlines
    assert!(text.to_lowercase().contains("newline") || text.to_lowercase().contains("dot"));
    Ok(())
}

#[test]
fn test_hover_text_modifier_x_explained() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("# comment\n\\d+", "x");
    assert!(text.to_lowercase().contains("comment") || text.to_lowercase().contains("whitespace"));
    Ok(())
}

#[test]
fn test_hover_text_multiple_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("hello", "gi");
    // Both global and case-insensitive should appear
    assert!(text.to_lowercase().contains("case") || text.to_lowercase().contains("global"));
    Ok(())
}

#[test]
fn test_hover_text_capture_includes_group_number() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("(unnamed)(?<named>\\w+)", "");
    // named capture at index 2 — text should mention it
    assert!(text.contains("named"));
    assert!(text.contains('2'));
    Ok(())
}

#[test]
fn test_hover_text_empty_pattern() -> Result<(), Box<dyn std::error::Error>> {
    // Should not panic on empty input
    let text = RegexAnalyzer::hover_text_for_regex("", "");
    // Returns some string (may be empty or a note)
    let _ = text;
    Ok(())
}

#[test]
fn test_hover_text_modifier_u_explained() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex(r"\w+", "u");
    assert!(text.to_lowercase().contains("unicode"));
    Ok(())
}

#[test]
fn test_hover_text_modifier_n_explained() -> Result<(), Box<dyn std::error::Error>> {
    let text = RegexAnalyzer::hover_text_for_regex("(foo)(?<bar>baz)", "n");
    assert!(text.to_lowercase().contains("non-capturing"));
    Ok(())
}

#[test]
fn test_hover_text_duplicate_modifiers_are_deduplicated() -> Result<(), Box<dyn std::error::Error>>
{
    let text = RegexAnalyzer::hover_text_for_regex("hello", "iiigg");
    let count = text.match_indices("case-insensitive matching").count();
    assert_eq!(count, 1, "duplicate modifiers should produce one note");
    Ok(())
}
#[test]
fn test_hover_text_unknown_modifier_ignored_gracefully() -> Result<(), Box<dyn std::error::Error>> {
    // Unknown modifier letters should not panic
    let text = RegexAnalyzer::hover_text_for_regex("\\d+", "z");
    let _ = text;
    Ok(())
}
