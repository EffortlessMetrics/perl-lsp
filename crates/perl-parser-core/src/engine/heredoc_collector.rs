use std::collections::VecDeque;
use std::sync::Arc;

/// Half-open byte offsets into the source buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

/// Quoting style used in a heredoc declaration.
#[derive(Debug, Copy, Clone)]
pub enum QuoteKind {
    /// Bare identifier (e.g., `<<EOF`), interpolates like double-quoted.
    Unquoted,
    /// Single-quoted (e.g., `<<'EOF'`), no interpolation.
    Single,
    /// Double-quoted (e.g., `<<"EOF"`), interpolates variables and escapes.
    Double,
}

/// Declaration info captured at parse time.
#[derive(Debug, Clone)]
pub struct PendingHeredoc {
    /// Exact terminator token that ends this heredoc.
    pub label: Arc<str>,
    /// True for indented heredocs (`<<~`), allows leading whitespace before terminator.
    pub allow_indent: bool,
    /// Quoting style determining interpolation behavior.
    pub quote: QuoteKind,
    /// Source span of the heredoc declaration (e.g., `<<EOF`).
    pub decl_span: Span,
    // Optional: add your node id here if convenient for AST attachment.
    // pub node_id: NodeId,
}

/// Collected content. Each segment is a line after indent stripping (no CR/LF).
#[derive(Debug)]
pub struct HeredocContent {
    /// Individual line spans after indent stripping, excluding line terminators.
    pub segments: Vec<Span>,
    /// Span from start of first segment to end of last segment (empty span if no content).
    pub full_span: Span,
}

/// Result of collecting one or more heredocs from source.
#[derive(Debug)]
pub struct CollectionResult {
    /// Collected heredoc contents in FIFO order, aligned to pending declarations.
    pub contents: Vec<HeredocContent>,
    /// Byte offset immediately after the final terminator newline.
    pub next_offset: usize,
}

/// Collects all pending heredocs from source starting at the given offset.
///
/// Processes heredocs in FIFO order, returning their contents and the byte offset
/// after the final terminator.
pub fn collect_all(
    src: &[u8],
    mut offset: usize,
    mut pending: VecDeque<PendingHeredoc>,
) -> CollectionResult {
    let mut results = Vec::with_capacity(pending.len());
    while let Some(hd) = pending.pop_front() {
        let (content, off2) = collect_one(src, offset, &hd);
        results.push(content);
        offset = off2;
    }
    CollectionResult { contents: results, next_offset: offset }
}

/// Reads content lines until `label` matches after optional leading whitespace.
/// For `<<~`, capture the terminator's leading whitespace as the indent baseline
/// and strip the longest common BYTE prefix on each content line.
/// CRLF is normalized **only** for terminator comparison; content spans exclude
/// CR and LF bytes by construction.
fn collect_one(src: &[u8], mut off: usize, hd: &PendingHeredoc) -> (HeredocContent, usize) {
    #[derive(Debug)]
    struct Line {
        start: usize,
        end_no_eol: usize,
    } // [start, end_no_eol)

    let mut raw_lines: Vec<Line> = Vec::new();
    let mut baseline_indent: Vec<u8> = Vec::new();
    let mut after_terminator_off = off;
    let mut found = false;

    while off <= src.len() {
        let (ls, le, next) = next_line_bounds(src, off);
        let line = &src[ls..le];

        // For terminator: ignore leading spaces/tabs; ignore trailing CR.
        let (lead_ws, rest) = split_leading_ws(line);
        let rest_no_cr = strip_trailing_cr(rest);

        if rest_no_cr == hd.label.as_bytes() {
            if hd.allow_indent {
                baseline_indent.clear();
                baseline_indent.extend_from_slice(&line[..lead_ws]);
            } else {
                baseline_indent.clear();
            }
            after_terminator_off = next;
            found = true;
            break;
        }

        raw_lines.push(Line { start: ls, end_no_eol: le });
        off = next;
    }

    let segments: Vec<Span> = raw_lines
        .iter()
        .map(|ln| {
            if baseline_indent.is_empty() {
                Span { start: ln.start, end: ln.end_no_eol }
            } else {
                let bytes = &src[ln.start..ln.end_no_eol];
                let strip = common_prefix_len(bytes, &baseline_indent);
                Span { start: ln.start + strip, end: ln.end_no_eol }
            }
        })
        .collect();

    let full_span = match (segments.first(), segments.last()) {
        (Some(f), Some(l)) => Span { start: f.start, end: l.end },
        _ => Span { start: off, end: off }, // empty heredoc
    };

    if !found {
        // Unterminated; return what we have (upstream should report a syntax error)
        return (HeredocContent { segments, full_span }, off);
    }

    (HeredocContent { segments, full_span }, after_terminator_off)
}

/// (line_start, line_end_excluding_newline, next_offset_after_newline)
/// Treats "\r\n" as one newline; "\n" also supported. EOF without newline ok.
fn next_line_bounds(src: &[u8], mut off: usize) -> (usize, usize, usize) {
    let start = off;
    while off < src.len() && src[off] != b'\n' && src[off] != b'\r' {
        off += 1;
    }
    let end_no_eol = off;
    if off < src.len() {
        if src[off] == b'\r' {
            off += 1;
            if off < src.len() && src[off] == b'\n' {
                off += 1;
            }
        } else if src[off] == b'\n' {
            off += 1;
        }
    }
    (start, end_no_eol, off)
}

/// Splits a byte slice into leading whitespace length and the remainder.
fn split_leading_ws(s: &[u8]) -> (usize, &[u8]) {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t') {
        i += 1;
    }
    (i, &s[i..])
}

/// For label comparison only, drop a trailing '\r' (CRLF normalization).
fn strip_trailing_cr(s: &[u8]) -> &[u8] {
    if s.last().copied() == Some(b'\r') { &s[..s.len() - 1] } else { s }
}

/// Returns the length of the common byte prefix between two slices.
fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let mut i = 0;
    while i < n && a[i] == b[i] {
        i += 1;
    }
    i
}
