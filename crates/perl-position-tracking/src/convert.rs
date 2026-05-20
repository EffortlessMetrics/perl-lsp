//! UTF-8/UTF-16 position conversion functions.
//!
//! The helpers in this module follow Language Server Protocol (LSP) semantics,
//! where lines and columns are zero-based and columns are measured in UTF-16
//! code units.

fn line_content_end(line: &str) -> usize {
    let without_lf = line.strip_suffix('\n').unwrap_or(line);
    without_lf.strip_suffix('\r').unwrap_or(without_lf).len()
}

fn text_end_utf16_line_col(text: &str) -> (u32, u32) {
    if text.is_empty() {
        return (0, 0);
    }
    if text.ends_with('\n') {
        return (text.split_inclusive('\n').count() as u32, 0);
    }

    let mut last_line = 0u32;
    let mut last_col = 0u32;
    for (idx, line) in text.split_inclusive('\n').enumerate() {
        last_line = idx as u32;
        last_col = line[..line_content_end(line)].encode_utf16().count() as u32;
    }
    (last_line, last_col)
}

/// Converts a byte offset into `(line, column_utf16)` coordinates.
///
/// Offsets beyond the end of the document are clamped to the last valid
/// position.
pub fn offset_to_utf16_line_col(text: &str, offset: usize) -> (u32, u32) {
    if offset >= text.len() {
        return text_end_utf16_line_col(text);
    }
    let mut acc = 0usize;
    for (line_idx, line) in text.split_inclusive('\n').enumerate() {
        let next = acc + line.len();
        if offset < next {
            let rel = offset - acc;
            if rel == 0 {
                return (line_idx as u32, 0);
            }

            let content_end = line_content_end(line);
            if rel >= content_end {
                return (line_idx as u32, line[..content_end].encode_utf16().count() as u32);
            }
            if line.is_char_boundary(rel) {
                return (line_idx as u32, line[..rel].encode_utf16().count() as u32);
            }
            let mut cs = rel;
            while cs > 0 && !line.is_char_boundary(cs) {
                cs -= 1;
            }
            // Clamp to the previous Unicode scalar boundary.
            // Returning a half-surrogate UTF-16 column would violate LSP invariants.
            return (line_idx as u32, line[..cs].encode_utf16().count() as u32);
        }
        acc = next;
    }
    text_end_utf16_line_col(text)
}

/// Converts `(line, column_utf16)` coordinates into a byte offset.
///
/// If the provided line or column is out of bounds, the result is clamped to
/// the nearest valid byte position in `text`.
pub fn utf16_line_col_to_offset(text: &str, line: u32, col: u32) -> usize {
    let mut offset = 0;
    for (curr, lt) in text.split_inclusive('\n').enumerate() {
        if curr as u32 == line {
            if col == 0 {
                return offset;
            }
            let line_end = line_content_end(lt);
            let line_content = &lt[..line_end];
            let mut up = 0u32;
            for (bi, ch) in line_content.char_indices() {
                if up == col {
                    return offset + bi;
                }
                if up < col && col < up + ch.len_utf16() as u32 {
                    return offset + bi;
                }
                up += ch.len_utf16() as u32;
                if up > col {
                    return offset + bi;
                }
            }
            return offset + line_end.min(text.len() - offset);
        }
        offset += lt.len();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::{offset_to_utf16_line_col, utf16_line_col_to_offset};

    #[test]
    fn offset_to_utf16_clamps_mid_codepoint_offsets_to_previous_boundary() {
        let text = "💖z";

        // Byte offset 1 sits inside the first UTF-8 codepoint (the emoji).
        // The reported UTF-16 column must stay on a valid boundary (0 or 2).
        assert_eq!(offset_to_utf16_line_col(text, 1), (0, 0));
        assert_eq!(offset_to_utf16_line_col(text, 2), (0, 0));
        assert_eq!(offset_to_utf16_line_col(text, 3), (0, 0));
    }

    #[test]
    fn offset_to_utf16_handles_multibyte_and_surrogate_pairs() {
        let text = "aé💖z";

        assert_eq!(offset_to_utf16_line_col(text, 0), (0, 0));
        assert_eq!(offset_to_utf16_line_col(text, 1), (0, 1));
        assert_eq!(offset_to_utf16_line_col(text, 3), (0, 2));
        assert_eq!(offset_to_utf16_line_col(text, 7), (0, 4));
        assert_eq!(offset_to_utf16_line_col(text, text.len()), (0, 5));
    }

    #[test]
    fn offset_to_utf16_clamps_out_of_bounds_to_last_position() {
        let text = "alpha\nbeta";
        assert_eq!(offset_to_utf16_line_col(text, text.len() + 25), (1, 4));
    }

    #[test]
    fn offset_to_utf16_reports_new_empty_line_for_terminal_newline() {
        let text = "one\ntwo\n";
        assert_eq!(offset_to_utf16_line_col(text, text.len()), (2, 0));
        assert_eq!(offset_to_utf16_line_col(text, text.len() + 25), (2, 0));

        let crlf_text = "one\r\ntwo\r\n";
        assert_eq!(offset_to_utf16_line_col(crlf_text, crlf_text.len()), (2, 0));
        assert_eq!(offset_to_utf16_line_col(crlf_text, crlf_text.len() + 25), (2, 0));
    }

    #[test]
    fn utf16_line_col_to_offset_handles_split_surrogate_column() {
        let text = "x💖y";
        assert_eq!(utf16_line_col_to_offset(text, 0, 0), 0);
        assert_eq!(utf16_line_col_to_offset(text, 0, 1), 1);
        assert_eq!(utf16_line_col_to_offset(text, 0, 2), 1);
        assert_eq!(utf16_line_col_to_offset(text, 0, 3), 5);
        assert_eq!(utf16_line_col_to_offset(text, 0, 4), 6);
    }

    #[test]
    fn utf16_line_col_to_offset_clamps_when_column_or_line_is_too_large() {
        let text = "abc\nw💡";
        assert_eq!(utf16_line_col_to_offset(text, 1, 99), text.len());
        assert_eq!(utf16_line_col_to_offset(text, 99, 0), text.len());
    }

    #[test]
    fn utf16_helpers_handle_crlf_and_surrogates_together() {
        let text = "a\r\nb💖c\r\n";

        assert_eq!(offset_to_utf16_line_col(text, 3), (1, 0));
        assert_eq!(offset_to_utf16_line_col(text, 8), (1, 3));
        assert_eq!(offset_to_utf16_line_col(text, 9), (1, 4));
        assert_eq!(offset_to_utf16_line_col(text, 10), (1, 4));
        assert_eq!(utf16_line_col_to_offset(text, 1, 2), 4);
        assert_eq!(utf16_line_col_to_offset(text, 1, 3), 8);
        assert_eq!(utf16_line_col_to_offset(text, 1, 4), 9);
        assert_eq!(utf16_line_col_to_offset(text, 1, 99), 9);
    }

    #[test]
    fn utf16_helpers_exclude_crlf_from_line_columns() {
        let text = "ab\r\ncd";

        assert_eq!(offset_to_utf16_line_col(text, 2), (0, 2));
        assert_eq!(offset_to_utf16_line_col(text, 3), (0, 2));
        assert_eq!(utf16_line_col_to_offset(text, 0, 2), 2);
        assert_eq!(utf16_line_col_to_offset(text, 0, 99), 2);
    }
}
