//! Behavior-driven scenarios for `perl-heredoc`.
//!
//! These tests intentionally use `Given/When/Then` naming so the crate's
//! externally visible behavior remains easy to audit as acceptance criteria.

use perl_heredoc::{PendingHeredoc, QuoteKind, collect_all};
use perl_position_tracking::ByteSpan;
use perl_tdd_support::must_some;
use std::collections::VecDeque;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn pending(label: &str, allow_indent: bool, quote: QuoteKind) -> PendingHeredoc {
    PendingHeredoc { label: Arc::from(label), allow_indent, quote, decl_span: ByteSpan::default() }
}

fn collect_single(src: &str, label: &str, allow_indent: bool) -> perl_heredoc::CollectionResult {
    let mut queue = VecDeque::new();
    queue.push_back(pending(label, allow_indent, QuoteKind::Double));
    collect_all(src.as_bytes(), 0, queue)
}

fn segment_text<'a>(src: &'a str, result: &'a perl_heredoc::HeredocContent) -> Vec<&'a str> {
    result.segments.iter().map(|span| &src[span.start..span.end]).collect()
}

#[test]
fn given_terminated_non_indented_heredoc_when_collecting_then_body_is_preserved() -> TestResult {
    // Given
    let src = "first line\nsecond line\nEND\n";

    // When
    let result = collect_single(src, "END", false);
    let content = must_some(result.contents.first());

    // Then
    assert!(content.terminated);
    assert_eq!(segment_text(src, content), vec!["first line", "second line"]);
    assert_eq!(result.next_offset, src.len());
    Ok(())
}

#[test]
fn given_indented_heredoc_when_terminator_has_baseline_indent_then_common_prefix_is_stripped()
-> TestResult {
    // Given
    let src = "    one\n    two\n    EOF\n";

    // When
    let result = collect_single(src, "EOF", true);
    let content = must_some(result.contents.first());

    // Then
    assert!(content.terminated);
    assert_eq!(segment_text(src, content), vec!["one", "two"]);
    Ok(())
}

#[test]
fn given_unterminated_heredoc_when_collecting_then_result_marks_not_terminated() -> TestResult {
    // Given
    let src = "line a\nline b\n";

    // When
    let result = collect_single(src, "MISSING", false);
    let content = must_some(result.contents.first());

    // Then
    assert!(!content.terminated);
    assert_eq!(segment_text(src, content), vec!["line a", "line b"]);
    assert_eq!(result.terminators_found, vec![false]);
    Ok(())
}

#[test]
fn given_crlf_source_when_collecting_then_terminator_match_and_segments_are_normalized()
-> TestResult {
    // Given
    let src = "alpha\r\nbeta\r\nEOF\r\n";

    // When
    let result = collect_single(src, "EOF", false);
    let content = must_some(result.contents.first());

    // Then
    assert!(content.terminated);
    assert_eq!(segment_text(src, content), vec!["alpha", "beta"]);
    Ok(())
}

#[test]
fn given_multiple_pending_heredocs_when_collecting_then_fifo_order_is_respected() -> TestResult {
    // Given
    let src = "a\nEOF1\nb\nEOF2\n";
    let mut queue = VecDeque::new();
    queue.push_back(pending("EOF1", false, QuoteKind::Unquoted));
    queue.push_back(pending("EOF2", false, QuoteKind::Single));

    // When
    let result = collect_all(src.as_bytes(), 0, queue);

    // Then
    assert_eq!(result.contents.len(), 2);
    assert_eq!(segment_text(src, must_some(result.contents.first())), vec!["a"]);
    assert_eq!(segment_text(src, must_some(result.contents.get(1))), vec!["b"]);
    assert_eq!(result.terminators_found, vec![true, true]);
    assert_eq!(result.next_offset, src.len());
    Ok(())
}

#[test]
fn given_empty_pending_queue_when_collecting_then_no_content_is_produced() -> TestResult {
    // Given
    let src = "anything\n";

    // When
    let result = collect_all(src.as_bytes(), 0, VecDeque::new());

    // Then
    assert!(result.contents.is_empty());
    assert!(result.terminators_found.is_empty());
    assert_eq!(result.next_offset, 0);
    Ok(())
}

#[test]
fn given_heredoc_with_trailing_label_text_when_collecting_then_only_exact_label_matches()
-> TestResult {
    // Given
    let src = "data\nEOF extra\nEOF\n";

    // When
    let result = collect_single(src, "EOF", false);
    let content = must_some(result.contents.first());

    // Then
    assert!(content.terminated);
    assert_eq!(segment_text(src, content), vec!["data", "EOF extra"]);
    Ok(())
}
