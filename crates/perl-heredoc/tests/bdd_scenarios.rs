//! Behavior-driven scenarios for `perl-heredoc`.
//!
//! These tests focus on user-visible behavior using Given/When/Then structure.

use perl_heredoc::{PendingHeredoc, QuoteKind, collect_all};
use perl_position_tracking::ByteSpan;
use perl_tdd_support::must_some;
use std::collections::VecDeque;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn pending(label: &str, allow_indent: bool, quote: QuoteKind) -> PendingHeredoc {
    PendingHeredoc { label: Arc::from(label), allow_indent, quote, decl_span: ByteSpan::default() }
}

fn collect_with(
    src: &str,
    pending_docs: VecDeque<PendingHeredoc>,
) -> perl_heredoc::CollectionResult {
    collect_all(src.as_bytes(), 0, pending_docs)
}

fn segment_texts<'a>(src: &'a str, spans: &[ByteSpan]) -> Vec<&'a str> {
    spans.iter().map(|span| &src[span.start..span.end]).collect()
}

#[test]
fn scenario_collects_single_terminated_heredoc() -> TestResult {
    // Given a source with one heredoc body and matching terminator
    let src = "hello from heredoc\nEOF\n";
    let mut pending_docs = VecDeque::new();
    pending_docs.push_back(pending("EOF", false, QuoteKind::Double));

    // When heredoc content is collected
    let result = collect_with(src, pending_docs);

    // Then the body is returned, marked terminated, and offset advances past terminator
    assert_eq!(result.contents.len(), 1);
    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(segment_texts(src, &content.segments), vec!["hello from heredoc"]);
    assert_eq!(result.next_offset, src.len());
    Ok(())
}

#[test]
fn scenario_preserves_content_when_terminator_is_missing() -> TestResult {
    // Given a source where the requested terminator does not exist
    let src = "line one\nline two\n";
    let mut pending_docs = VecDeque::new();
    pending_docs.push_back(pending("MISSING", false, QuoteKind::Double));

    // When collection runs to EOF
    let result = collect_with(src, pending_docs);

    // Then all lines are captured and the heredoc is marked unterminated
    let content = must_some(result.contents.first());
    assert!(!content.terminated);
    assert_eq!(segment_texts(src, &content.segments), vec!["line one", "line two"]);
    assert_eq!(result.terminators_found, vec![false]);
    Ok(())
}

#[test]
fn scenario_strips_indent_for_tilde_heredoc() -> TestResult {
    // Given a <<~ style heredoc with an indented terminator
    let src = "    alpha\n    beta\n    END\n";
    let mut pending_docs = VecDeque::new();
    pending_docs.push_back(pending("END", true, QuoteKind::Unquoted));

    // When collection uses the terminator indent baseline
    let result = collect_with(src, pending_docs);

    // Then the common baseline indent is stripped from content lines
    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(segment_texts(src, &content.segments), vec!["alpha", "beta"]);
    Ok(())
}

#[test]
fn scenario_normalizes_crlf_for_terminator_matching() -> TestResult {
    // Given Windows CRLF text
    let src = "payload\r\nDONE\r\n";
    let mut pending_docs = VecDeque::new();
    pending_docs.push_back(pending("DONE", false, QuoteKind::Single));

    // When lines are scanned for terminator matching
    let result = collect_with(src, pending_docs);

    // Then terminator detection succeeds and content excludes line endings
    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(segment_texts(src, &content.segments), vec!["payload"]);
    Ok(())
}

#[test]
fn scenario_processes_multiple_pending_heredocs_fifo() -> TestResult {
    // Given two pending heredoc declarations queued in lexical order
    let src = "first body\nEOF1\nsecond body\nEOF2\n";
    let mut pending_docs = VecDeque::new();
    pending_docs.push_back(pending("EOF1", false, QuoteKind::Double));
    pending_docs.push_back(pending("EOF2", false, QuoteKind::Backtick));

    // When collection runs across the source once
    let result = collect_with(src, pending_docs);

    // Then results align with FIFO order of declarations
    assert_eq!(result.contents.len(), 2);
    let first = must_some(result.contents.first());
    let second = must_some(result.contents.get(1));
    assert_eq!(segment_texts(src, &first.segments), vec!["first body"]);
    assert_eq!(segment_texts(src, &second.segments), vec!["second body"]);
    assert_eq!(result.terminators_found, vec![true, true]);
    Ok(())
}

#[test]
fn scenario_ignores_similar_but_non_exact_terminator_lines() -> TestResult {
    // Given body lines that look close to the label but are not exact matches
    let src = "data\nEOFX\nXEOF\nEOF\n";
    let mut pending_docs = VecDeque::new();
    pending_docs.push_back(pending("EOF", false, QuoteKind::Double));

    // When the collector evaluates potential terminator lines
    let result = collect_with(src, pending_docs);

    // Then only the exact label line terminates the heredoc
    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(segment_texts(src, &content.segments), vec!["data", "EOFX", "XEOF"]);
    Ok(())
}
