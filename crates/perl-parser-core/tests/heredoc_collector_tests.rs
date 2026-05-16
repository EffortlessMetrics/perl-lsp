//! Unit tests for `crates/perl-parser-core/src/syntax/heredoc.rs`.
//!
//! Tests cover: basic heredoc collection (all QuoteKind variants), indented
//! heredocs (`<<~`), CRLF vs LF normalization, empty heredocs, unterminated
//! heredocs, mixed-indent (tabs + spaces) under `<<~`, and multiple pending
//! heredocs collected from one source string.

use perl_parser_core::heredoc_collector::{HeredocContent, PendingHeredoc, QuoteKind, collect_all};
use std::collections::VecDeque;
use std::sync::Arc;

// Helper: build a PendingHeredoc with LF-style termination.
fn pending(label: &str, allow_indent: bool, quote: QuoteKind) -> PendingHeredoc {
    PendingHeredoc {
        label: Arc::from(label),
        allow_indent,
        quote,
        decl_span: perl_parser_core::heredoc_collector::Span { start: 0, end: 0 },
    }
}

// Helper: extract the bytes for a segment from the source.
fn segment_text<'a>(src: &'a [u8], content: &HeredocContent, idx: usize) -> &'a [u8] {
    let seg = &content.segments[idx];
    &src[seg.start..seg.end]
}

// ---------------------------------------------------------------------------
// 1. Basic heredoc collection — QuoteKind variants
// ---------------------------------------------------------------------------

/// Unquoted heredoc (`<<EOF`) is collected correctly.
#[test]
fn test_basic_unquoted_heredoc() {
    // Source after the opening line: content lines then terminator.
    let src = b"hello world\nEOF\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("EOF", false, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    assert_eq!(result.contents.len(), 1);
    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(result.terminators_found, vec![true]);
    assert_eq!(content.segments.len(), 1);
    assert_eq!(segment_text(src, content, 0), b"hello world");
    // next_offset should be past the "EOF\n" terminator
    assert_eq!(result.next_offset, src.len());
}

/// Single-quoted heredoc (`<<'EOF'`) — QuoteKind value is stored and content
/// is collected identically (no interpolation happens in this collector layer).
#[test]
fn test_basic_single_quoted_heredoc() {
    let src = b"no interpolation here\nEOF\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("EOF", false, QuoteKind::Single));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(content.segments.len(), 1);
    assert_eq!(segment_text(src, content, 0), b"no interpolation here");
}

/// Double-quoted heredoc (`<<"EOF"`) — content collected normally.
#[test]
fn test_basic_double_quoted_heredoc() {
    let src = b"line one\nline two\nEOF\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("EOF", false, QuoteKind::Double));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(content.segments.len(), 2);
    assert_eq!(segment_text(src, content, 0), b"line one");
    assert_eq!(segment_text(src, content, 1), b"line two");
}

/// Backtick heredoc (``<<`EOF```) — QuoteKind stored, content collected normally.
#[test]
fn test_basic_backtick_heredoc() {
    let src = b"ls -la\nEOF\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("EOF", false, QuoteKind::Backtick));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(content.segments.len(), 1);
    assert_eq!(segment_text(src, content, 0), b"ls -la");
}

// ---------------------------------------------------------------------------
// 2. Indented heredoc (`<<~`) — leading whitespace stripping
// ---------------------------------------------------------------------------

/// Indented heredoc strips the common leading-space prefix from content lines.
#[test]
fn test_indented_heredoc_strips_common_prefix() {
    // The terminator "    END" has 4 spaces of indent.
    // Content lines each have 4 spaces of indent as well.
    let src = b"    line one\n    line two\n    END\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("END", true, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(content.segments.len(), 2);
    // The 4-space prefix from the terminator line is used as baseline.
    assert_eq!(segment_text(src, content, 0), b"line one");
    assert_eq!(segment_text(src, content, 1), b"line two");
}

/// With extra indent on content lines, only the common prefix (terminator indent)
/// is stripped; extra indent remains.
#[test]
fn test_indented_heredoc_extra_indent_preserved() {
    // Terminator has 2 spaces; content has 2 + 4 spaces.
    let src = b"      deeply indented\n  END\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("END", true, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(content.segments.len(), 1);
    // 2-space prefix stripped, leaving 4 spaces + "deeply indented".
    assert_eq!(segment_text(src, content, 0), b"    deeply indented");
}

// ---------------------------------------------------------------------------
// 3. CRLF vs LF line terminator normalization
// ---------------------------------------------------------------------------

/// CRLF-terminated source: the terminator is still matched and segments
/// exclude the CR and LF bytes.
#[test]
fn test_crlf_terminator_normalization() {
    let src = b"hello\r\nEOF\r\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("EOF", false, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated, "heredoc should be terminated even with CRLF");
    assert_eq!(content.segments.len(), 1);
    // Segment should not include the CR byte.
    assert_eq!(segment_text(src, content, 0), b"hello");
}

/// Mixed CRLF and LF in one heredoc body — each line is handled correctly.
#[test]
fn test_mixed_crlf_lf_lines() {
    let src = b"first\r\nsecond\nEOF\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("EOF", false, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(content.segments.len(), 2);
    assert_eq!(segment_text(src, content, 0), b"first");
    assert_eq!(segment_text(src, content, 1), b"second");
}

// ---------------------------------------------------------------------------
// 4. Empty heredoc
// ---------------------------------------------------------------------------

/// An empty heredoc has no content lines — the terminator is on the very next
/// line after the declaration offset.
#[test]
fn test_empty_heredoc() {
    let src = b"EOF\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("EOF", false, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert!(content.segments.is_empty());
    assert_eq!(result.next_offset, src.len());
}

// ---------------------------------------------------------------------------
// 5. Unterminated heredoc
// ---------------------------------------------------------------------------

/// When the terminator label never appears, `terminated` is false and
/// `terminators_found` reports false.
#[test]
fn test_unterminated_heredoc() {
    let src = b"line one\nline two\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("MISSING", false, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    assert_eq!(result.contents.len(), 1);
    let content = &result.contents[0];
    assert!(!content.terminated, "heredoc should be unterminated");
    assert_eq!(result.terminators_found, vec![false]);
    // The collector should still have captured whatever lines it saw.
    assert_eq!(content.segments.len(), 2);
}

// ---------------------------------------------------------------------------
// 6. Mixed-indent (tabs + spaces) under `<<~`
// ---------------------------------------------------------------------------

/// When the indented terminator uses a tab, the tab is the baseline indent
/// and is stripped from content lines that start with a tab.
#[test]
fn test_indented_heredoc_tab_baseline() {
    // Terminator indented by one tab; content lines also indented by one tab.
    let src = b"\tline one\n\tEND\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("END", true, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert!(content.terminated);
    assert_eq!(content.segments.len(), 1);
    assert_eq!(segment_text(src, content, 0), b"line one");
}

// ---------------------------------------------------------------------------
// 7. Multiple pending heredocs from one source string
// ---------------------------------------------------------------------------

/// Two heredocs declared one after another are collected in FIFO order.
#[test]
fn test_multiple_heredocs_collected_in_order() {
    // Simulates the post-declaration byte slice containing both heredoc bodies
    // back-to-back, as the Perl parser would present them.
    let src = b"first body\nFIRST\nsecond body\nSECOND\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("FIRST", false, QuoteKind::Unquoted));
    pending_q.push_back(pending("SECOND", false, QuoteKind::Single));

    let result = collect_all(src, 0, pending_q);

    assert_eq!(result.contents.len(), 2);
    assert_eq!(result.terminators_found, vec![true, true]);

    let first = &result.contents[0];
    assert!(first.terminated);
    assert_eq!(first.segments.len(), 1);
    assert_eq!(segment_text(src, first, 0), b"first body");

    let second = &result.contents[1];
    assert!(second.terminated);
    assert_eq!(second.segments.len(), 1);
    assert_eq!(segment_text(src, second, 0), b"second body");

    // next_offset should be past the entire source.
    assert_eq!(result.next_offset, src.len());
}

/// Two heredocs — first terminates, second is missing its terminator (its
/// collector reaches EOF without finding "SECOND").
#[test]
fn test_multiple_heredocs_first_terminated_second_not() {
    let src = b"body one\nFIRST\nbody two\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("FIRST", false, QuoteKind::Unquoted));
    pending_q.push_back(pending("SECOND", false, QuoteKind::Unquoted));

    let result = collect_all(src, 0, pending_q);

    assert_eq!(result.contents.len(), 2);
    assert!(result.contents[0].terminated);
    assert!(!result.contents[1].terminated);
    assert_eq!(result.terminators_found, vec![true, false]);
}

// ---------------------------------------------------------------------------
// 8. full_span covers all content segments
// ---------------------------------------------------------------------------

/// `full_span` starts at the first segment and ends at the last segment,
/// providing a single span that encompasses all content.
#[test]
fn test_full_span_covers_segments() {
    let src = b"alpha\nbeta\ngamma\nEND\n";
    let mut pending_q = VecDeque::new();
    pending_q.push_back(pending("END", false, QuoteKind::Double));

    let result = collect_all(src, 0, pending_q);

    let content = &result.contents[0];
    assert_eq!(content.segments.len(), 3);

    // We already asserted segments.len() == 3 above, so indexing is safe here.
    // full_span.start == first segment start, full_span.end == last segment end.
    assert_eq!(content.full_span.start, content.segments[0].start);
    assert_eq!(content.full_span.end, content.segments[2].end);
}
