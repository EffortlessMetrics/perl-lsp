// crates/perl-parser/src/linked_editing.rs
use crate::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};
use lsp_types::{LinkedEditingRanges, Position, Range};

const OPEN: &[char] = &['(', '[', '{', '<', '\'', '"'];
const CLOSE: &[char] = &[')', ']', '}', '>', '\'', '"'];

fn char_at(text: &str, byte: usize) -> Option<char> {
    text[byte..].chars().next()
}

fn prev_char_pos(text: &str, mut byte: usize) -> Option<(usize, char)> {
    if byte == 0 {
        return None;
    }
    // step back to the previous char boundary
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    let prev_start = text[..byte].char_indices().last()?.0;
    Some((prev_start, text[prev_start..].chars().next().unwrap()))
}

/// Find a matching bracket/quote from a byte position that sits on, or just
/// after, a bracket/quote.
fn find_pair(text: &str, start_byte: usize) -> Option<(usize, usize)> {
    // Prefer the char at cursor; otherwise the previous char (cursor after token)
    let (pos, ch) = char_at(text, start_byte)
        .map(|c| (start_byte, c))
        .or_else(|| prev_char_pos(text, start_byte))?;

    // If it's a closer, scan backward; if opener, scan forward; if quote treat symmetric.
    if let Some(open_idx) = OPEN.iter().position(|&c| c == ch) {
        let close = CLOSE[open_idx];
        if ch == close {
            // quotes: scan forward for same quote (no escape handling for now)
            let mut i = pos + ch.len_utf8();
            while i < text.len() {
                if let Some(c) = char_at(text, i) {
                    if c == ch {
                        return Some((pos, i));
                    }
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            return None;
        } else {
            // bracket open: scan forward with depth
            let mut depth = 0usize;
            let mut i = pos;
            while i < text.len() {
                if let Some(c) = char_at(text, i) {
                    if c == ch {
                        depth += 1;
                    }
                    if c == close {
                        depth -= 1;
                        if depth == 0 {
                            return Some((pos, i));
                        }
                    }
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
        }
    } else if let Some(close_idx) = CLOSE.iter().position(|&c| c == ch) {
        let open = OPEN[close_idx];
        if ch == open {
            // quotes handled above; this branch covers the case we landed on a closer
        }
        // bracket close: scan backward
        let mut depth = 0usize;
        let mut i = pos;
        loop {
            if let Some((j, c)) = prev_char_pos(text, i) {
                if c == ch {
                    depth += 1;
                }
                if c == open {
                    depth -= 1;
                    if depth == 0 {
                        return Some((j, pos));
                    }
                }
                i = j;
            } else {
                break;
            }
        }
    }
    None
}

/// Handles the `textDocument/linkedEditingRange` request.
///
/// This function finds a matching bracket or quote for the character at the given
/// position and returns a `LinkedEditingRanges` object containing the ranges of
/// the two matching characters.
///
/// # Arguments
///
/// * `text` - The content of the document.
/// * `line` - The line number of the character.
/// * `character` - The character offset on that line.
///
/// # Returns
///
/// An `Option<LinkedEditingRanges>` object.
pub fn handle_linked_editing(text: &str, line: u32, character: u32) -> Option<LinkedEditingRanges> {
    let byte = utf16_line_col_to_offset(text, line, character);
    let (a, b) = find_pair(text, byte)?;
    let (a_line, a_char) = offset_to_utf16_line_col(text, a);
    let (b_line, b_char) = offset_to_utf16_line_col(text, b);

    let ranges = vec![
        Range::new(Position::new(a_line, a_char), Position::new(a_line, a_char + 1)),
        Range::new(Position::new(b_line, b_char), Position::new(b_line, b_char + 1)),
    ];
    Some(LinkedEditingRanges { ranges, word_pattern: None })
}
