/// Build a byte-level mask for regions that should be considered executable code.
///
/// Bytes that belong to Perl comments (`# ...`) or single/double-quoted string
/// literals are marked `false` and should be ignored by lightweight regex scans.
pub(super) fn code_byte_mask(line: &str) -> Vec<bool> {
    fn is_ident_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    let bytes = line.as_bytes();
    let mut mask = vec![true; bytes.len()];
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        if in_single {
            mask[i] = false;
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }

        if in_double {
            mask[i] = false;
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        if let Some(end_idx) = parse_quote_like_operator(bytes, i) {
            for byte in mask.iter_mut().take(end_idx).skip(i) {
                *byte = false;
            }
            i = end_idx;
            continue;
        }

        match b {
            b'#' => {
                if is_perl_array_length_marker(bytes, i) {
                    i += 1;
                    continue;
                }
                for byte in mask.iter_mut().take(bytes.len()).skip(i) {
                    *byte = false;
                }
                break;
            }
            b'\'' => {
                let prev_is_ident = i > 0 && is_ident_byte(bytes[i - 1]);
                let next_is_ident = (i + 1) < bytes.len() && is_ident_byte(bytes[i + 1]);

                if prev_is_ident && next_is_ident {
                    // Legacy Perl namespace separator (e.g. `$Foo'bar`).
                    i += 1;
                    continue;
                }

                mask[i] = false;
                in_single = true;
            }
            b'"' => {
                mask[i] = false;
                in_double = true;
            }
            _ => {}
        }

        i += 1;
    }

    mask
}

fn is_perl_array_length_marker(bytes: &[u8], idx: usize) -> bool {
    if idx > 0 && bytes[idx - 1] == b'$' {
        return true;
    }
    idx > 1 && bytes[idx - 1] == b'{' && bytes[idx - 2] == b'$'
}

/// Parse Perl quote-like operators (`q`, `qq`, `qw`, `qr`, `qx`) at `start`.
///
/// Returns the end index (exclusive) of the full quote-like segment when found.
fn parse_quote_like_operator(bytes: &[u8], start: usize) -> Option<usize> {
    let prev_is_sigil = start > 0 && matches!(bytes[start - 1], b'$' | b'@' | b'%');
    if prev_is_sigil {
        return None;
    }

    let prev_is_ident =
        start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
    if prev_is_ident {
        return None;
    }

    let operators = [
        (b"qq".as_slice(), QuoteLikeKind::SingleSegment),
        (b"qw".as_slice(), QuoteLikeKind::SingleSegment),
        (b"qr".as_slice(), QuoteLikeKind::SingleSegment),
        (b"qx".as_slice(), QuoteLikeKind::SingleSegment),
        (b"q".as_slice(), QuoteLikeKind::SingleSegment),
        (b"tr".as_slice(), QuoteLikeKind::DoubleSegment),
        (b"y".as_slice(), QuoteLikeKind::DoubleSegment),
        (b"s".as_slice(), QuoteLikeKind::DoubleSegment),
        (b"m".as_slice(), QuoteLikeKind::SingleSegment),
    ];

    for (op, kind) in operators {
        let Some(op_end) = start.checked_add(op.len()) else {
            continue;
        };
        if op_end > bytes.len() || bytes.get(start..op_end) != Some(op) {
            continue;
        }

        if !is_operator_boundary(bytes, op_end) {
            continue;
        }

        let mut idx = op_end;
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= bytes.len() {
            return None;
        }

        let Some(after_first_segment) = consume_delimited_segment(bytes, idx) else {
            continue;
        };

        idx = after_first_segment;
        if matches!(kind, QuoteLikeKind::DoubleSegment) {
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            let Some(after_second_segment) = consume_delimited_segment(bytes, idx) else {
                continue;
            };
            idx = after_second_segment;
        }

        while idx < bytes.len() && bytes[idx].is_ascii_alphabetic() {
            idx += 1;
        }

        return Some(idx);
    }

    None
}

#[derive(Clone, Copy)]
enum QuoteLikeKind {
    SingleSegment,
    DoubleSegment,
}

fn consume_delimited_segment(bytes: &[u8], start: usize) -> Option<usize> {
    if start >= bytes.len() {
        return None;
    }

    let open = bytes[start];
    if open.is_ascii_alphanumeric() || open == b'_' {
        return None;
    }
    let (close, paired) = matching_delimiter(open);
    let mut idx = start + 1;
    let mut depth = if paired { 1usize } else { 0usize };
    let mut escaped = false;

    while idx < bytes.len() {
        let b = bytes[idx];
        if escaped {
            escaped = false;
            idx += 1;
            continue;
        }

        if b == b'\\' {
            escaped = true;
            idx += 1;
            continue;
        }

        if paired && b == open {
            depth += 1;
            idx += 1;
            continue;
        }

        if b == close {
            if paired {
                depth = depth.saturating_sub(1);
                idx += 1;
                if depth == 0 {
                    return Some(idx);
                }
                continue;
            }
            return Some(idx + 1);
        }

        idx += 1;
    }

    Some(bytes.len())
}

fn is_identifier_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_operator_boundary(bytes: &[u8], op_end: usize) -> bool {
    // Require a non-identifier boundary after multi-character operators so
    // identifiers like `qqx` aren't mistaken for `qq`.
    if op_end < bytes.len() && is_identifier_byte(bytes[op_end]) {
        return false;
    }

    true
}

fn matching_delimiter(open: u8) -> (u8, bool) {
    match open {
        b'(' => (b')', true),
        b'[' => (b']', true),
        b'{' => (b'}', true),
        b'<' => (b'>', true),
        _ => (open, false),
    }
}
