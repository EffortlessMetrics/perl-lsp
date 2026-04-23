/// Sprint A Day 3: Heredoc collector tests
/// Validates FIFO ordering, <<~ indent stripping, CRLF terminator matching,
/// and byte-accurate content spans.
///
/// Labels: tests:heredoc, sprint-a:day3, collector:comprehensive
use perl_parser::heredoc_collector::*;
use std::collections::VecDeque;
use std::sync::Arc;

fn sp(s: usize, e: usize) -> Span {
    Span { start: s, end: e }
}

#[test]
fn empty_unindented() {
    let src = b"<<EOT\nEOT\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: false,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 5),
    });
    // Offset 6 is right after "<<EOT\n" (the declaration line)
    let res = collect_all(src, 6, q);
    assert_eq!(res.contents.len(), 1);
    assert!(res.contents[0].segments.is_empty());
    assert_eq!(res.next_offset, src.len());
}

#[test]
fn indented_strip_uses_terminator_indent_mixed_ws() {
    // Baseline indent: "\t  " from the terminator line.
    let src = b"<<~EOT\n\t  one\n\t  \ttwo\n\t  EOT\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: true,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 6),
    });
    let res = collect_all(src, 7, q); // After "<<~EOT\n"
    let segs = &res.contents[0].segments;
    assert_eq!(segs.len(), 2);
    assert_eq!(&src[segs[0].start..segs[0].end], b"one");
    assert_eq!(&src[segs[1].start..segs[1].end], b"\ttwo"); // only baseline prefix stripped
}

#[test]
fn crlf_terminator_match_preserves_content_spans() {
    let src = b"<<~EOT\r\n  a\r\n  b\r\n  EOT\r\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: true,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 7),
    });
    let res = collect_all(src, 8, q); // After "<<~EOT\r\n"
    let segs = &res.contents[0].segments;
    assert_eq!(segs.len(), 2);
    assert_eq!(&src[segs[0].start..segs[0].end], b"a");
    assert_eq!(&src[segs[1].start..segs[1].end], b"b");
}

#[test]
fn label_like_lines_are_not_terminators() {
    let src = b"<<EOT\nnotEOT\n EOTX\nEOT\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: false,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 5),
    });
    let res = collect_all(src, 6, q); // After "<<EOT\n"
    assert_eq!(res.contents[0].segments.len(), 2); // "notEOT", " EOTX"
}

#[test]
fn two_heredocs_in_one_statement_fifo() {
    let src = b"<<A <<B\nA1\nA\nB1\nB\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("A"),
        allow_indent: false,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 4),
    });
    q.push_back(PendingHeredoc {
        label: Arc::from("B"),
        allow_indent: false,
        quote: QuoteKind::Unquoted,
        decl_span: sp(5, 9),
    });
    let res = collect_all(src, 8, q); // After "<<A <<B\n"
    assert_eq!(res.contents.len(), 2);
    let a = &res.contents[0].segments[0];
    let b = &res.contents[1].segments[0];
    assert_eq!(&src[a.start..a.end], b"A1");
    assert_eq!(&src[b.start..b.end], b"B1");
}

/// Additional test: indented heredoc with less indentation than terminator
#[test]
fn indented_content_less_than_terminator_indent() {
    // Terminator has "    " (4 spaces), content lines have "  " (2 spaces)
    let src = b"<<~EOT\n  line1\n  line2\n    EOT\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: true,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 6),
    });
    let res = collect_all(src, 7, q); // After "<<~EOT\n"
    let segs = &res.contents[0].segments;
    assert_eq!(segs.len(), 2);
    // Baseline is "    " (4 spaces), but content only has "  " (2 spaces)
    // So we strip the common prefix "  " (2 spaces)
    assert_eq!(&src[segs[0].start..segs[0].end], b"line1");
    assert_eq!(&src[segs[1].start..segs[1].end], b"line2");
}

/// Additional test: no indent stripping for non-<<~ heredocs
#[test]
fn non_indented_heredoc_preserves_all_whitespace() {
    let src = b"<<EOT\n  indented line\nno indent\n  EOT\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: false,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 5),
    });
    let res = collect_all(src, 6, q); // After "<<EOT\n"
    let segs = &res.contents[0].segments;
    assert_eq!(segs.len(), 2);
    assert_eq!(&src[segs[0].start..segs[0].end], b"  indented line");
    assert_eq!(&src[segs[1].start..segs[1].end], b"no indent");
}

/// Additional test: empty lines in heredoc content
#[test]
fn heredoc_with_empty_lines() {
    let src = b"<<EOT\nline1\n\nline3\nEOT\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: false,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 5),
    });
    let res = collect_all(src, 6, q); // After "<<EOT\n"
    let segs = &res.contents[0].segments;
    assert_eq!(segs.len(), 3);
    assert_eq!(&src[segs[0].start..segs[0].end], b"line1");
    assert_eq!(&src[segs[1].start..segs[1].end], b""); // empty line
    assert_eq!(&src[segs[2].start..segs[2].end], b"line3");
}

/// Additional test: full_span covers first to last segment
#[test]
fn full_span_covers_all_segments() {
    let src = b"<<EOT\nfirst\nmiddle\nlast\nEOT\n";
    let mut q = VecDeque::new();
    q.push_back(PendingHeredoc {
        label: Arc::from("EOT"),
        allow_indent: false,
        quote: QuoteKind::Unquoted,
        decl_span: sp(0, 5),
    });
    let res = collect_all(src, 6, q); // After "<<EOT\n"
    let content = &res.contents[0];
    let segs = &content.segments;

    // full_span should be from start of first segment to end of last segment
    assert_eq!(content.full_span.start, segs[0].start);
    assert_eq!(content.full_span.end, segs[segs.len() - 1].end);

    // Verify content
    assert_eq!(&src[content.full_span.start..content.full_span.end], b"first\nmiddle\nlast");
}
