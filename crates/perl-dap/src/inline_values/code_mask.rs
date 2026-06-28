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

// ============================================================================
// paired_delimiter_conformance — inline tests for matching_delimiter
//
// This test block is one of THREE that together form the conformance matrix
// for paired-delimiter implementations across the workspace (#1320).
//
// The same input set is tested in:
//   - crates/perl-lexer/src/quote_handler.rs                   (paired_close)
//   - crates/perl-parser-core/src/syntax/quote.rs              (get_closing_delimiter)
//
// Normalized contract: each impl maps to (close_char: char, is_paired: bool).
//   - This impl: matching_delimiter(c as u8) → (b, p); normalized as (b as char, p)
// ============================================================================
#[cfg(test)]
mod paired_delimiter_conformance {
    use super::matching_delimiter;

    /// Normalize `matching_delimiter` to the shared `(close_char, is_paired)` shape.
    fn normalize(open: char) -> (char, bool) {
        let (close_byte, is_paired) = matching_delimiter(open as u8);
        (close_byte as char, is_paired)
    }

    /// The shared conformance matrix.
    /// Each entry is `(open_char, expected_close, expected_is_paired)`.
    const MATRIX: &[(char, char, bool)] = &[
        // --- Paired openers -------------------------------------------
        ('(', ')', true),
        ('[', ']', true),
        ('{', '}', true),
        ('<', '>', true),
        // --- Self-delimiting: common punctuation ----------------------
        ('/', '/', false),
        ('#', '#', false),
        ('|', '|', false),
        ('!', '!', false),
        (',', ',', false),
        ('%', '%', false),
        ('~', '~', false),
        ('.', '.', false),
        (':', ':', false),
        (';', ';', false),
        // --- Self-delimiting: quote-adjacent chars --------------------
        ('\'', '\'', false),
        ('"', '"', false),
        // --- Self-delimiting: less-common punctuation ----------------
        ('@', '@', false),
        ('$', '$', false),
        ('^', '^', false),
        ('&', '&', false),
        ('*', '*', false),
        ('+', '+', false),
        ('-', '-', false),
        ('=', '=', false),
        ('?', '?', false),
        // --- Closing chars used as openers (not paired) --------------
        // Note: Perl does NOT auto-pair ) ] } > as openers.
        (')', ')', false),
        (']', ']', false),
        ('}', '}', false),
        ('>', '>', false),
    ];

    #[test]
    fn matching_delimiter_agrees_with_conformance_matrix() {
        // MATRIX is ASCII by construction; matching_delimiter operates on u8.
        for &(open, expected_close, expected_paired) in MATRIX {
            let (got_close, got_paired) = normalize(open);
            assert_eq!(
                got_close, expected_close,
                "perl-dap matching_delimiter({open:?}): close char mismatch \
                 (got {got_close:?}, expected {expected_close:?})"
            );
            assert_eq!(
                got_paired, expected_paired,
                "perl-dap matching_delimiter({open:?}): is_paired mismatch \
                 (got {got_paired}, expected {expected_paired})"
            );
        }
    }

    #[test]
    fn matching_delimiter_paired_openers_return_true() {
        // The four Perl auto-paired delimiters must all return is_paired = true.
        for (open, expected_close) in [(b'(', b')'), (b'[', b']'), (b'{', b'}'), (b'<', b'>')] {
            let (close, is_paired) = matching_delimiter(open);
            assert!(
                is_paired,
                "matching_delimiter({} = {:?}): is_paired should be true",
                open, open as char
            );
            assert_eq!(
                close, expected_close,
                "matching_delimiter({} = {:?}): close byte mismatch",
                open, open as char
            );
        }
    }

    #[test]
    fn matching_delimiter_self_delimiters_return_false() {
        // Self-delimiting ASCII chars must return (self, false).
        let self_delims: &[u8] = b"/#|!,%~.:;'\"@$^&*+=-?";
        for &open in self_delims {
            let (close, is_paired) = matching_delimiter(open);
            assert_eq!(
                close, open,
                "matching_delimiter({} = {:?}): close should equal open for self-delimiting char",
                open, open as char
            );
            assert!(
                !is_paired,
                "matching_delimiter({} = {:?}): is_paired should be false for self-delimiting char",
                open, open as char
            );
        }
    }

    #[test]
    fn matching_delimiter_closing_chars_used_as_openers_return_false() {
        // Perl does NOT auto-pair ) ] } > as openers.
        for &close in b")]}>" {
            let (result_byte, is_paired) = matching_delimiter(close);
            assert_eq!(
                result_byte, close,
                "matching_delimiter({} = {:?}): closing chars used as openers should return self",
                close, close as char
            );
            assert!(
                !is_paired,
                "matching_delimiter({} = {:?}): is_paired should be false for closing chars used as openers",
                close, close as char
            );
        }
    }
}
