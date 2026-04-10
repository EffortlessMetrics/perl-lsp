//! Behavior-driven scenarios for `perl-heredoc` collection.
//!
//! These tests use Given/When/Then style names so the expected behavior
//! reads like an executable specification.

use perl_heredoc::{PendingHeredoc, QuoteKind, collect_all};
use perl_position_tracking::ByteSpan;
use perl_tdd_support::must_some;
use std::collections::VecDeque;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone)]
struct PendingSpec {
    label: &'static str,
    allow_indent: bool,
    quote: QuoteKind,
}

fn pending_queue(specs: &[PendingSpec]) -> VecDeque<PendingHeredoc> {
    specs
        .iter()
        .map(|spec| PendingHeredoc {
            label: Arc::from(spec.label),
            allow_indent: spec.allow_indent,
            quote: spec.quote,
            decl_span: ByteSpan::default(),
        })
        .collect()
}

fn line_texts<'a>(src: &'a str, spans: &[ByteSpan]) -> Vec<&'a str> {
    spans.iter().map(|span| &src[span.start..span.end]).collect()
}

#[test]
fn given_standard_heredoc_when_collected_then_body_and_offset_are_reported() -> TestResult {
    let src = "alpha\nbeta\nEOF\ntrailing\n";

    let result = collect_all(
        src.as_bytes(),
        0,
        pending_queue(&[PendingSpec {
            label: "EOF",
            allow_indent: false,
            quote: QuoteKind::Unquoted,
        }]),
    );

    assert_eq!(result.contents.len(), 1);
    assert_eq!(result.terminators_found, vec![true]);
    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(line_texts(src, &content.segments), vec!["alpha", "beta"]);
    assert_eq!(result.next_offset, "alpha\nbeta\nEOF\n".len());
    Ok(())
}

#[test]
fn given_indented_heredoc_when_terminator_is_indented_then_common_prefix_is_stripped() -> TestResult
{
    let src = "    one\n      two\n    END\n";

    let result = collect_all(
        src.as_bytes(),
        0,
        pending_queue(&[PendingSpec {
            label: "END",
            allow_indent: true,
            quote: QuoteKind::Double,
        }]),
    );

    let content = must_some(result.contents.first());
    assert!(content.terminated);
    assert_eq!(line_texts(src, &content.segments), vec!["one", "  two"]);
    Ok(())
}

#[test]
fn given_multiple_pending_heredocs_when_collected_then_they_resolve_in_fifo_order() -> TestResult {
    let src = "left\nA\nright\nB\n";
    let result = collect_all(
        src.as_bytes(),
        0,
        pending_queue(&[
            PendingSpec { label: "A", allow_indent: false, quote: QuoteKind::Single },
            PendingSpec { label: "B", allow_indent: false, quote: QuoteKind::Backtick },
        ]),
    );

    assert_eq!(result.contents.len(), 2);
    assert_eq!(result.terminators_found, vec![true, true]);
    assert_eq!(line_texts(src, &must_some(result.contents.first()).segments), vec!["left"]);
    assert_eq!(line_texts(src, &result.contents[1].segments), vec!["right"]);
    assert_eq!(result.next_offset, src.len());
    Ok(())
}

#[test]
fn given_missing_terminator_when_collected_then_content_is_marked_unterminated() -> TestResult {
    let src = "line1\nline2\n";
    let result = collect_all(
        src.as_bytes(),
        0,
        pending_queue(&[PendingSpec {
            label: "DONE",
            allow_indent: false,
            quote: QuoteKind::Double,
        }]),
    );

    let content = must_some(result.contents.first());
    assert!(!content.terminated);
    assert_eq!(result.terminators_found, vec![false]);
    assert_eq!(line_texts(src, &content.segments), vec!["line1", "line2"]);
    assert_eq!(result.next_offset, src.len());
    Ok(())
}

#[test]
fn given_nonzero_offset_when_collecting_then_preceding_source_is_ignored() -> TestResult {
    let src = "ignore\nkeep\nEND\n";
    let offset = "ignore\n".len();
    let result = collect_all(
        src.as_bytes(),
        offset,
        pending_queue(&[PendingSpec {
            label: "END",
            allow_indent: false,
            quote: QuoteKind::Double,
        }]),
    );

    let content = must_some(result.contents.first());
    assert_eq!(line_texts(src, &content.segments), vec!["keep"]);
    assert!(content.full_span.start >= offset);
    Ok(())
}

#[test]
fn given_each_quote_kind_when_collecting_then_quote_kind_does_not_change_collection() -> TestResult
{
    let src = "payload\nTAG\n";
    let expected = vec!["payload"];

    for quote in [QuoteKind::Unquoted, QuoteKind::Single, QuoteKind::Double, QuoteKind::Backtick] {
        let result = collect_all(
            src.as_bytes(),
            0,
            pending_queue(&[PendingSpec { label: "TAG", allow_indent: false, quote }]),
        );

        let content = must_some(result.contents.first());
        assert!(content.terminated, "expected termination for {quote:?}");
        assert_eq!(line_texts(src, &content.segments), expected);
    }

    Ok(())
}
