//! BDD-style scenarios for `perl-heredoc`.
//!
//! These tests keep behavior expectations readable as product-level stories.

use perl_heredoc::{PendingHeredoc, QuoteKind, collect_all};
use perl_tdd_support::must_some;
use std::collections::VecDeque;
use std::sync::Arc;

fn pending(label: &str, allow_indent: bool, quote: QuoteKind) -> PendingHeredoc {
    PendingHeredoc { label: Arc::from(label), allow_indent, quote, decl_span: Default::default() }
}

fn text_segments<'a>(src: &'a str, ranges: &[perl_heredoc::Span]) -> Vec<&'a str> {
    ranges.iter().map(|span| &src[span.start..span.end]).collect()
}

#[test]
fn given_pending_heredoc_when_terminator_present_then_content_is_collected() {
    // Given
    let src = "alpha\nbeta\nEOF\n";
    let mut queue = VecDeque::new();
    queue.push_back(pending("EOF", false, QuoteKind::Double));

    // When
    let result = collect_all(src.as_bytes(), 0, queue);

    // Then
    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(text_segments(src, &content.segments), vec!["alpha", "beta"]);
    assert_eq!(result.terminators_found, vec![true]);
    assert_eq!(result.next_offset, src.len());
}

#[test]
fn given_indented_heredoc_when_terminator_is_indented_then_common_prefix_is_stripped() {
    // Given
    let src = "    one\n      two\n    END\n";
    let mut queue = VecDeque::new();
    queue.push_back(pending("END", true, QuoteKind::Unquoted));

    // When
    let result = collect_all(src.as_bytes(), 0, queue);

    // Then
    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(text_segments(src, &content.segments), vec!["one", "  two"]);
    assert_eq!(result.terminators_found, vec![true]);
}

#[test]
fn given_two_pending_heredocs_when_collected_then_fifo_order_is_preserved() {
    // Given
    let src = "first line\nONE\nsecond line\nTWO\n";
    let mut queue = VecDeque::new();
    queue.push_back(pending("ONE", false, QuoteKind::Single));
    queue.push_back(pending("TWO", false, QuoteKind::Backtick));

    // When
    let result = collect_all(src.as_bytes(), 0, queue);

    // Then
    assert_eq!(result.contents.len(), 2);
    assert_eq!(result.terminators_found, vec![true, true]);

    let first = must_some(result.contents.first());
    assert_eq!(text_segments(src, &first.segments), vec!["first line"]);

    let second = must_some(result.contents.get(1));
    assert_eq!(text_segments(src, &second.segments), vec!["second line"]);
    assert_eq!(result.next_offset, src.len());
}

#[test]
fn given_unterminated_heredoc_when_collecting_then_content_is_returned_with_failure_flag() {
    // Given
    let src = "left\nopen\n";
    let mut queue = VecDeque::new();
    queue.push_back(pending("END", false, QuoteKind::Double));

    // When
    let result = collect_all(src.as_bytes(), 0, queue);

    // Then
    let content = must_some(result.contents.first());
    assert!(!content.terminated);
    assert_eq!(text_segments(src, &content.segments), vec!["left", "open"]);
    assert_eq!(result.terminators_found, vec![false]);
    assert_eq!(result.next_offset, src.len());
}

#[test]
fn given_nonzero_start_offset_when_collecting_then_preceding_bytes_are_ignored() {
    // Given
    let src = "prefix\nkeep\ngoing\nSTOP\n";
    let start_at = must_some(src.find("keep"));
    let mut queue = VecDeque::new();
    queue.push_back(pending("STOP", false, QuoteKind::Double));

    // When
    let result = collect_all(src.as_bytes(), start_at, queue);

    // Then
    let content = must_some(result.contents.first());
    assert_eq!(text_segments(src, &content.segments), vec!["keep", "going"]);
    assert_eq!(result.terminators_found, vec![true]);
    assert_eq!(result.next_offset, src.len());
}

#[test]
fn given_no_pending_heredocs_when_collecting_then_result_is_empty_and_offset_is_unchanged() {
    // Given
    let src = "any text\n";
    let queue = VecDeque::new();

    // When
    let result = collect_all(src.as_bytes(), 3, queue);

    // Then
    assert!(result.contents.is_empty());
    assert!(result.terminators_found.is_empty());
    assert_eq!(result.next_offset, 3);
}
