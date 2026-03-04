//! Comprehensive unit tests for the `perl-heredoc` crate.
//!
//! Covers all public types (`QuoteKind`, `PendingHeredoc`, `HeredocContent`,
//! `CollectionResult`) and the `collect_all` function with edge cases.

use perl_heredoc::{CollectionResult, HeredocContent, PendingHeredoc, QuoteKind, collect_all};
use perl_position_tracking::ByteSpan;
use perl_tdd_support::must_some;
use std::collections::VecDeque;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pending(label: &str, allow_indent: bool, quote: QuoteKind) -> PendingHeredoc {
    PendingHeredoc { label: Arc::from(label), allow_indent, quote, decl_span: ByteSpan::default() }
}

fn collect_single(src: &str, label: &str, allow_indent: bool) -> CollectionResult {
    let mut q = VecDeque::new();
    q.push_back(pending(label, allow_indent, QuoteKind::Double));
    collect_all(src.as_bytes(), 0, q)
}

fn content_text<'a>(src: &'a str, hc: &HeredocContent) -> Vec<&'a str> {
    hc.segments.iter().map(|s| &src[s.start..s.end]).collect()
}

// ---------------------------------------------------------------------------
// QuoteKind — basic construction / Debug
// ---------------------------------------------------------------------------

#[test]
fn quote_kind_debug_representations() -> Result<(), Box<dyn std::error::Error>> {
    let _ = format!("{:?}", QuoteKind::Unquoted);
    let _ = format!("{:?}", QuoteKind::Single);
    let _ = format!("{:?}", QuoteKind::Double);
    let _ = format!("{:?}", QuoteKind::Backtick);
    Ok(())
}

#[test]
fn quote_kind_copy_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let a = QuoteKind::Single;
    let b = a; // Copy
    let _ = format!("{:?} {:?}", a, b);
    Ok(())
}

// ---------------------------------------------------------------------------
// PendingHeredoc — construction, Clone, Debug
// ---------------------------------------------------------------------------

#[test]
fn pending_heredoc_clone_and_debug() -> Result<(), Box<dyn std::error::Error>> {
    let hd = pending("EOF", false, QuoteKind::Double);
    let hd2 = hd.clone();
    assert_eq!(&*hd2.label, "EOF");
    assert!(!hd2.allow_indent);
    let _ = format!("{:?}", hd);
    Ok(())
}

#[test]
fn pending_heredoc_fields() -> Result<(), Box<dyn std::error::Error>> {
    let hd = pending("MARKER", true, QuoteKind::Single);
    assert_eq!(&*hd.label, "MARKER");
    assert!(hd.allow_indent);
    Ok(())
}

// ---------------------------------------------------------------------------
// HeredocContent — Debug
// ---------------------------------------------------------------------------

#[test]
fn heredoc_content_debug() -> Result<(), Box<dyn std::error::Error>> {
    let hc = HeredocContent { segments: vec![], full_span: ByteSpan::default(), terminated: true };
    let _ = format!("{:?}", hc);
    Ok(())
}

// ---------------------------------------------------------------------------
// CollectionResult — Debug
// ---------------------------------------------------------------------------

#[test]
fn collection_result_debug() -> Result<(), Box<dyn std::error::Error>> {
    let cr = CollectionResult { contents: vec![], terminators_found: vec![], next_offset: 0 };
    let _ = format!("{:?}", cr);
    Ok(())
}

// ---------------------------------------------------------------------------
// collect_all — basic heredoc scenarios
// ---------------------------------------------------------------------------

#[test]
fn single_line_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    let src = "hello world\nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["hello world"]);
    assert_eq!(r.next_offset, src.len());
    Ok(())
}

#[test]
fn multi_line_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    let src = "line1\nline2\nline3\nEND\n";
    let r = collect_single(src, "END", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["line1", "line2", "line3"]);
    Ok(())
}

#[test]
fn empty_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    let src = "EOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert!(hc.segments.is_empty());
    Ok(())
}

#[test]
fn heredoc_with_blank_lines() -> Result<(), Box<dyn std::error::Error>> {
    let src = "first\n\n\nlast\nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["first", "", "", "last"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// Unterminated heredocs
// ---------------------------------------------------------------------------

#[test]
fn unterminated_heredoc_returns_all_lines() -> Result<(), Box<dyn std::error::Error>> {
    let src = "line1\nline2\n";
    let r = collect_single(src, "NOPE", false);
    let hc = must_some(r.contents.first());
    assert!(!hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["line1", "line2"]);
    assert!(!*must_some(r.terminators_found.first()));
    Ok(())
}

#[test]
fn unterminated_on_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let src = "";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(!hc.terminated);
    assert!(hc.segments.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Indented heredocs (<<~)
// ---------------------------------------------------------------------------

#[test]
fn indented_heredoc_strips_common_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let src = "    hello\n    world\n    EOF\n";
    let r = collect_single(src, "EOF", true);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["hello", "world"]);
    Ok(())
}

#[test]
fn indented_heredoc_mixed_indent() -> Result<(), Box<dyn std::error::Error>> {
    // Terminator has 2 spaces; content has 2+ spaces
    let src = "    deep\n  shallow\n  EOF\n";
    let r = collect_single(src, "EOF", true);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["  deep", "shallow"]);
    Ok(())
}

#[test]
fn indented_heredoc_tab_indent() -> Result<(), Box<dyn std::error::Error>> {
    let src = "\t\thello\n\t\tEOF\n";
    let r = collect_single(src, "EOF", true);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["hello"]);
    Ok(())
}

#[test]
fn indented_heredoc_no_indent_on_terminator() -> Result<(), Box<dyn std::error::Error>> {
    // Terminator at column 0 => no stripping
    let src = "    indented\nEOF\n";
    let r = collect_single(src, "EOF", true);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["    indented"]);
    Ok(())
}

#[test]
fn non_indented_heredoc_leading_ws_still_matches_terminator()
-> Result<(), Box<dyn std::error::Error>> {
    // allow_indent=false: terminator comparison still strips leading ws,
    // but no content indent stripping occurs.
    let src = "content\n  EOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["content"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// CRLF handling
// ---------------------------------------------------------------------------

#[test]
fn crlf_line_endings() -> Result<(), Box<dyn std::error::Error>> {
    let src = "hello\r\nworld\r\nEOF\r\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["hello", "world"]);
    Ok(())
}

#[test]
fn mixed_crlf_and_lf() -> Result<(), Box<dyn std::error::Error>> {
    let src = "alpha\r\nbeta\ngamma\r\nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["alpha", "beta", "gamma"]);
    Ok(())
}

#[test]
fn terminator_with_cr_only() -> Result<(), Box<dyn std::error::Error>> {
    // Bare \r not followed by \n — treated as line ending
    let src = "data\rEOF\r";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["data"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// Offset parameter
// ---------------------------------------------------------------------------

#[test]
fn collect_with_nonzero_offset() -> Result<(), Box<dyn std::error::Error>> {
    let src = "SKIP\nhello\nEOF\n";
    let offset = 5; // skip "SKIP\n"
    let mut q = VecDeque::new();
    q.push_back(pending("EOF", false, QuoteKind::Double));
    let r = collect_all(src.as_bytes(), offset, q);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["hello"]);
    assert_eq!(r.next_offset, src.len());
    Ok(())
}

#[test]
fn collect_at_end_of_source() -> Result<(), Box<dyn std::error::Error>> {
    let src = "done";
    let r = collect_single(src, "EOF", false);
    // No terminator found
    let hc = must_some(r.contents.first());
    assert!(!hc.terminated);
    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple heredocs (FIFO ordering)
// ---------------------------------------------------------------------------

#[test]
fn two_heredocs_fifo_order() -> Result<(), Box<dyn std::error::Error>> {
    let src = "first body\nEOF1\nsecond body\nEOF2\n";
    let mut q = VecDeque::new();
    q.push_back(pending("EOF1", false, QuoteKind::Double));
    q.push_back(pending("EOF2", false, QuoteKind::Single));
    let r = collect_all(src.as_bytes(), 0, q);
    assert_eq!(r.contents.len(), 2);

    let h1 = must_some(r.contents.first());
    assert!(h1.terminated);
    assert_eq!(content_text(src, h1), vec!["first body"]);

    let h2 = must_some(r.contents.get(1));
    assert!(h2.terminated);
    assert_eq!(content_text(src, h2), vec!["second body"]);

    assert_eq!(r.next_offset, src.len());
    Ok(())
}

#[test]
fn three_heredocs_mixed_indent() -> Result<(), Box<dyn std::error::Error>> {
    let src = "a\nEOF1\n  b\n  EOF2\nc\nEOF3\n";
    let mut q = VecDeque::new();
    q.push_back(pending("EOF1", false, QuoteKind::Unquoted));
    q.push_back(pending("EOF2", true, QuoteKind::Double));
    q.push_back(pending("EOF3", false, QuoteKind::Backtick));
    let r = collect_all(src.as_bytes(), 0, q);
    assert_eq!(r.contents.len(), 3);
    assert!(r.contents.iter().all(|c| c.terminated));
    Ok(())
}

#[test]
fn second_heredoc_unterminated() -> Result<(), Box<dyn std::error::Error>> {
    let src = "body1\nEOF1\nbody2\n";
    let mut q = VecDeque::new();
    q.push_back(pending("EOF1", false, QuoteKind::Double));
    q.push_back(pending("MISSING", false, QuoteKind::Double));
    let r = collect_all(src.as_bytes(), 0, q);
    assert_eq!(r.contents.len(), 2);
    assert!(must_some(r.contents.first()).terminated);
    assert!(!must_some(r.contents.get(1)).terminated);
    Ok(())
}

// ---------------------------------------------------------------------------
// Empty pending queue
// ---------------------------------------------------------------------------

#[test]
fn empty_pending_queue() -> Result<(), Box<dyn std::error::Error>> {
    let src = "anything\n";
    let q = VecDeque::new();
    let r = collect_all(src.as_bytes(), 0, q);
    assert!(r.contents.is_empty());
    assert!(r.terminators_found.is_empty());
    assert_eq!(r.next_offset, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Label edge cases
// ---------------------------------------------------------------------------

#[test]
fn label_with_special_characters() -> Result<(), Box<dyn std::error::Error>> {
    let src = "data\n__END_OF_DATA__\n";
    let r = collect_single(src, "__END_OF_DATA__", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert_eq!(content_text(src, hc), vec!["data"]);
    Ok(())
}

#[test]
fn label_is_single_char() -> Result<(), Box<dyn std::error::Error>> {
    let src = "x\nE\n";
    let r = collect_single(src, "E", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert_eq!(content_text(src, hc), vec!["x"]);
    Ok(())
}

#[test]
fn label_not_prefix_matched() -> Result<(), Box<dyn std::error::Error>> {
    // "EOFX" should not match "EOF"
    let src = "data\nEOFX\nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert_eq!(content_text(src, hc), vec!["data", "EOFX"]);
    Ok(())
}

#[test]
fn label_not_suffix_matched() -> Result<(), Box<dyn std::error::Error>> {
    // "XEOF" should not match "EOF"
    let src = "data\nXEOF\nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert_eq!(content_text(src, hc), vec!["data", "XEOF"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// full_span correctness
// ---------------------------------------------------------------------------

#[test]
fn full_span_covers_all_content() -> Result<(), Box<dyn std::error::Error>> {
    let src = "alpha\nbeta\nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    let span = hc.full_span;
    assert_eq!(span.start, 0);
    // "alpha" ends at 5, "beta" ends at 10, full_span.end should be 10
    assert_eq!(span.end, 10);
    Ok(())
}

#[test]
fn full_span_empty_heredoc_is_zero_length() -> Result<(), Box<dyn std::error::Error>> {
    let src = "EOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.full_span.start == hc.full_span.end);
    Ok(())
}

// ---------------------------------------------------------------------------
// terminators_found alignment
// ---------------------------------------------------------------------------

#[test]
fn terminators_found_aligns_with_contents() -> Result<(), Box<dyn std::error::Error>> {
    let src = "a\nEOF1\nb\n";
    let mut q = VecDeque::new();
    q.push_back(pending("EOF1", false, QuoteKind::Double));
    q.push_back(pending("MISSING", false, QuoteKind::Double));
    let r = collect_all(src.as_bytes(), 0, q);
    assert_eq!(r.contents.len(), r.terminators_found.len());
    assert!(*must_some(r.terminators_found.first()));
    assert!(!*must_some(r.terminators_found.get(1)));
    Ok(())
}

// ---------------------------------------------------------------------------
// No trailing newline after terminator
// ---------------------------------------------------------------------------

#[test]
fn terminator_at_eof_without_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    let src = "data\nEOF";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    // The terminator line has no trailing newline — should still match
    assert!(hc.terminated);
    assert_eq!(content_text(src, hc), vec!["data"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// Content that looks like terminator but has trailing text
// ---------------------------------------------------------------------------

#[test]
fn label_with_trailing_text_does_not_match() -> Result<(), Box<dyn std::error::Error>> {
    let src = "data\nEOF extra\nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert_eq!(content_text(src, hc), vec!["data", "EOF extra"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// All QuoteKind variants pass through collect_all
// ---------------------------------------------------------------------------

#[test]
fn all_quote_kinds_collect_identically() -> Result<(), Box<dyn std::error::Error>> {
    let src = "body\nEOF\n";
    for quote in [QuoteKind::Unquoted, QuoteKind::Single, QuoteKind::Double, QuoteKind::Backtick] {
        let mut q = VecDeque::new();
        q.push_back(pending("EOF", false, quote));
        let r = collect_all(src.as_bytes(), 0, q);
        let hc = must_some(r.contents.first());
        assert!(hc.terminated, "failed for {:?}", quote);
        assert_eq!(content_text(src, hc), vec!["body"]);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ByteSpan re-export
// ---------------------------------------------------------------------------

#[test]
fn span_reexport_is_bytespan() -> Result<(), Box<dyn std::error::Error>> {
    let _s: perl_heredoc::Span = ByteSpan::default();
    Ok(())
}

// ---------------------------------------------------------------------------
// Indented heredoc edge: empty body with indent
// ---------------------------------------------------------------------------

#[test]
fn indented_empty_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    let src = "    EOF\n";
    let r = collect_single(src, "EOF", true);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert!(hc.segments.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Content with only whitespace lines
// ---------------------------------------------------------------------------

#[test]
fn whitespace_only_content_lines() -> Result<(), Box<dyn std::error::Error>> {
    let src = "   \n  \nEOF\n";
    let r = collect_single(src, "EOF", false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert_eq!(hc.segments.len(), 2);
    Ok(())
}

// ---------------------------------------------------------------------------
// Large label
// ---------------------------------------------------------------------------

#[test]
fn long_label() -> Result<(), Box<dyn std::error::Error>> {
    let label = "A".repeat(200);
    let src = format!("content\n{}\n", label);
    let r = collect_single(&src, &label, false);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    assert_eq!(content_text(&src, hc), vec!["content"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// next_offset is correct after multiple heredocs
// ---------------------------------------------------------------------------

#[test]
fn next_offset_after_two_heredocs() -> Result<(), Box<dyn std::error::Error>> {
    let src = "a\nEOF1\nb\nEOF2\ntrailing";
    let mut q = VecDeque::new();
    q.push_back(pending("EOF1", false, QuoteKind::Double));
    q.push_back(pending("EOF2", false, QuoteKind::Double));
    let r = collect_all(src.as_bytes(), 0, q);
    // next_offset should be right after "EOF2\n"
    let expected = "a\nEOF1\nb\nEOF2\n".len();
    assert_eq!(r.next_offset, expected);
    Ok(())
}

// ---------------------------------------------------------------------------
// Arc<str> label — ensure Rc-like sharing works
// ---------------------------------------------------------------------------

#[test]
fn arc_label_sharing() -> Result<(), Box<dyn std::error::Error>> {
    let label: Arc<str> = Arc::from("SHARED");
    let hd = PendingHeredoc {
        label: Arc::clone(&label),
        allow_indent: false,
        quote: QuoteKind::Double,
        decl_span: ByteSpan::default(),
    };
    assert_eq!(&*hd.label, "SHARED");
    assert_eq!(&*label, "SHARED");
    Ok(())
}

// ---------------------------------------------------------------------------
// Indented heredoc: content less indented than terminator
// ---------------------------------------------------------------------------

#[test]
fn indented_content_less_than_terminator() -> Result<(), Box<dyn std::error::Error>> {
    // Content at col 0, terminator at col 4 — baseline is 4 spaces
    // Content has 0-length common prefix with baseline → no stripping
    let src = "hello\n    EOF\n";
    let r = collect_single(src, "EOF", true);
    let hc = must_some(r.contents.first());
    assert!(hc.terminated);
    let lines = content_text(src, hc);
    assert_eq!(lines, vec!["hello"]);
    Ok(())
}

// ---------------------------------------------------------------------------
// decl_span is independent of collection
// ---------------------------------------------------------------------------

#[test]
fn decl_span_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let hd = PendingHeredoc {
        label: Arc::from("EOF"),
        allow_indent: false,
        quote: QuoteKind::Double,
        decl_span: ByteSpan::new(10, 15),
    };
    assert_eq!(hd.decl_span.start, 10);
    assert_eq!(hd.decl_span.end, 15);
    Ok(())
}
