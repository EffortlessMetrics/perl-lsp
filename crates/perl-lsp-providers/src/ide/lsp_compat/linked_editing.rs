//! Linked editing ranges for bracket pairs in Perl code.
//!
//! Provides support for simultaneous editing of matching brackets, quotes,
//! and other paired delimiters.

use lsp_types::{LinkedEditingRanges, Position, Range};
use perl_parser_core::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};

const OPEN: &[char] = &['(', '[', '{', '<', '\'', '"'];
const CLOSE: &[char] = &[')', ']', '}', '>', '\'', '"'];

fn char_at(text: &str, byte: usize) -> Option<char> {
    text.get(byte..)?.chars().next()
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
    // Safety: prev_start is a valid char boundary from char_indices
    let ch = text.get(prev_start..)?.chars().next()?;
    Some((prev_start, ch))
}

/// Find a matching bracket/quote from a byte position that sits on, or just
/// after, a bracket/quote.
///
/// Returns `(opener_byte, closer_byte)`.
fn find_pair(text: &str, start_byte: usize) -> Option<(usize, usize)> {
    // Prefer the char at cursor; otherwise the previous char (cursor after token)
    let (pos, ch) = char_at(text, start_byte)
        .map(|c| (start_byte, c))
        .or_else(|| prev_char_pos(text, start_byte))?;

    // If it's a closer, scan backward; if opener, scan forward; if quote treat symmetric.
    if let Some(open_idx) = OPEN.iter().position(|&c| c == ch) {
        let close = CLOSE[open_idx];
        if ch == close {
            // quotes: scan forward for same quote with escape handling
            let mut i = pos + ch.len_utf8();
            while i < text.len() {
                if let Some(c) = char_at(text, i) {
                    if c == '\\' {
                        // skip the backslash and the following character
                        i += c.len_utf8();
                        if let Some(next) = char_at(text, i) {
                            i += next.len_utf8();
                        }
                        continue;
                    }
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
        // bracket close: scan backward; start at 1 to count the close bracket we're sitting on
        let mut depth = 1usize;
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

/// Try to find a heredoc pair at `start_byte`.
///
/// Detects `<<LABEL`, `<<"LABEL"`, `<<'LABEL'`, `` <<`LABEL` ``, and `<<~LABEL`
/// forms.  When a heredoc operator is found, scans forward line-by-line for the
/// terminator line.
///
/// Returns `(label_start_byte, terminator_label_start_byte, label_len)` or
/// `None` if no heredoc is detected at this position.
fn find_heredoc_pair(text: &str, start_byte: usize) -> Option<(usize, usize, usize)> {
    // Find all `<<` occurrences on the same line up to and including start_byte.
    // A cursor on the first `<`, second `<`, optional `~`/quote prefix, or any
    // character of the label itself all resolve to the same heredoc pair.
    let line_start = text[..start_byte].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_text = text.get(line_start..)?;
    let cursor_offset_in_line = start_byte.saturating_sub(line_start);

    let mut search_pos = 0usize;
    let mut found_heredoc: Option<(usize, usize)> = None; // (label_start_byte, label_len)

    while let Some(rel) = line_text[search_pos..].find("<<") {
        let abs_rel = search_pos + rel;
        if abs_rel > cursor_offset_in_line + 1 {
            // This `<<` is entirely after the cursor
            break;
        }
        let after_chevrons = abs_rel + 2; // byte offset in line_text after `<<`

        // Skip optional `~` (indented heredoc)
        let mut label_pos = after_chevrons;
        if line_text.as_bytes().get(label_pos) == Some(&b'~') {
            label_pos += 1;
        }

        // Skip optional surrounding quote char
        let quote_char: Option<u8> = match line_text.as_bytes().get(label_pos) {
            Some(&b'"') | Some(&b'\'') | Some(&b'`') => {
                let q = line_text.as_bytes()[label_pos];
                label_pos += 1;
                Some(q)
            }
            _ => None,
        };

        // Read identifier: [A-Za-z_][A-Za-z0-9_]*
        let label_start_in_line = label_pos;
        let ident_bytes = &line_text.as_bytes()[label_pos..];
        let ident_len = ident_bytes
            .iter()
            .take_while(|&&b| b.is_ascii_alphanumeric() || b == b'_')
            .count();

        if ident_len == 0 {
            search_pos = abs_rel + 1;
            continue;
        }

        let label_end_in_line = label_start_in_line + ident_len;

        // Check if there is an expected closing quote
        if let Some(q) = quote_char {
            if line_text.as_bytes().get(label_end_in_line) != Some(&q) {
                search_pos = abs_rel + 1;
                continue;
            }
        }

        // The cursor must be within the label (or on the `<<` / `~` / quote prefix)
        let token_start_in_line = abs_rel; // start of `<<`
        let token_end_in_line = label_end_in_line + quote_char.map_or(0, |_| 1); // past closing quote
        if cursor_offset_in_line >= token_start_in_line
            && cursor_offset_in_line <= token_end_in_line
        {
            found_heredoc = Some((line_start + label_start_in_line, ident_len));
        }

        search_pos = abs_rel + 1;
    }

    let (label_start_byte, label_len) = found_heredoc?;
    let label = text.get(label_start_byte..label_start_byte + label_len)?;

    // Scan forward line-by-line from the line after the `<<` opener.
    let heredoc_line_end = text[line_start..].find('\n').map(|p| line_start + p + 1)?;
    let rest = text.get(heredoc_line_end..)?;

    let mut scan_byte = heredoc_line_end;
    for scan_line in rest.split('\n') {
        let trimmed = scan_line.trim_start();
        if trimmed == label || trimmed.strip_suffix('\r').unwrap_or(trimmed) == label {
            // Found the terminator — compute its byte start
            let leading_ws = scan_line.len() - trimmed.len();
            let terminator_start = scan_byte + leading_ws;
            return Some((label_start_byte, terminator_start, label_len));
        }
        scan_byte += scan_line.len() + 1; // +1 for the '\n' consumed by split
    }

    None
}

/// Regex/substitution operators that take a single non-bracket delimiter.
///
/// The bracket forms (`m(...)`, `s{...}{...}`, etc.) are already handled by
/// the generic bracket scanner above.  This function handles non-bracket
/// single-char delimiters like `m/foo/`, `m|foo|`, `s/pat/rep/`,
/// `tr/a/b/`, `y/a/b/`.
///
/// Returns `(opener_byte, closer_byte)` for the delimiter *pair* containing
/// `start_byte`, or `None`.
fn find_regex_delimiter_pair(text: &str, start_byte: usize) -> Option<(usize, usize)> {
    let (start_byte, ch) = char_at(text, start_byte)
        .map(|c| (start_byte, c))
        .or_else(|| prev_char_pos(text, start_byte))?;

    // The character at cursor must be a non-bracket, non-quote punctuation char
    // that is plausibly a regex delimiter.
    let is_bracket = OPEN.contains(&ch) || CLOSE.contains(&ch);
    if is_bracket || ch.is_alphanumeric() || ch == '_' || ch == '$' || ch == '@' || ch == '%' {
        return None;
    }

    // Scan from line start to find a regex operator immediately followed by this
    // delimiter character anywhere on the line.  We match against the cursor
    // byte to identify which delimiter position the cursor occupies.
    let line_start = text[..start_byte].rfind('\n').map(|p| p + 1).unwrap_or(0);

    let line_end = text[line_start..]
        .find('\n')
        .map(|p| line_start + p)
        .unwrap_or(text.len());

    let line_text = text.get(line_start..line_end)?;

    // Walk through the line looking for a regex operator + delimiter pattern
    // whose delimiter byte matches start_byte.
    //
    // Operators are checked longest-first to avoid `q` matching `qr`, etc.
    // two_pair = true for s, tr, y (three delimiters total).
    let all_ops: &[(&str, bool)] = &[
        ("tr", true),
        ("qr", false),
        ("qq", false),
        ("qw", false),
        ("s", true),
        ("y", true),
        ("m", false),
        ("q", false),
    ];

    let mut pos = 0usize;
    while pos < line_text.len() {
        let remaining = &line_text[pos..];

        // Check word boundary before operator (must not follow alphanumeric or `_`)
        let word_boundary = pos == 0 || {
            let prev = line_text.as_bytes()[pos - 1];
            !prev.is_ascii_alphanumeric() && prev != b'_'
        };

        let matched = if word_boundary {
            all_ops.iter().find(|(op, _)| {
                if !remaining.starts_with(op) {
                    return false;
                }
                // After the operator keyword, allow optional spaces then the delimiter char
                let after = remaining[op.len()..].trim_start_matches(' ');
                after.starts_with(ch)
            })
        } else {
            None
        };

        let Some(&(op, two_pair)) = matched else {
            pos += 1;
            continue;
        };

        // Found operator at `pos`. Locate the first delimiter byte.
        let after_op = &line_text[pos + op.len()..];
        let ws_len = after_op.len() - after_op.trim_start_matches(' ').len();
        let first_delim_rel = pos + op.len() + ws_len;
        let first_delim_byte = line_start + first_delim_rel;

        if !two_pair {
            if let Some(second_delim_byte) = find_non_bracket_close(text, first_delim_byte, ch) {
                if start_byte == first_delim_byte || start_byte == second_delim_byte {
                    return Some((first_delim_byte, second_delim_byte));
                }
            }
            pos += op.len() + 1;
            continue;
        }

        // Two-pair: D pattern D replacement D
        if let (Some(second), Some(third)) = (
            find_non_bracket_close(text, first_delim_byte, ch),
            find_non_bracket_close(text, first_delim_byte, ch)
                .and_then(|s| find_non_bracket_close(text, s, ch)),
        ) {
            if start_byte == first_delim_byte {
                return Some((first_delim_byte, second));
            }
            if start_byte == second {
                return Some((second, third));
            }
            if start_byte == third {
                return Some((second, third));
            }
        }

        pos += op.len() + 1;
    }

    None
}

/// Scan forward from `after_byte` (exclusive) for the next occurrence of `delim`.
/// Used for non-bracket single-char delimiter matching (no depth tracking needed).
fn find_non_bracket_close(text: &str, from_byte: usize, delim: char) -> Option<usize> {
    let start = from_byte + delim.len_utf8();
    let mut i = start;
    while i < text.len() {
        if let Some(c) = char_at(text, i) {
            if c == '\\' {
                // skip escaped character
                i += c.len_utf8();
                if let Some(next) = char_at(text, i) {
                    i += next.len_utf8();
                }
                continue;
            }
            if c == delim {
                return Some(i);
            }
            if c == '\n' {
                // don't cross line boundaries for inline regex
                break;
            }
            i += c.len_utf8();
        } else {
            break;
        }
    }
    None
}

/// Handles the `textDocument/linkedEditingRange` request.
///
/// This function finds a matching bracket, quote, heredoc label, or regex
/// delimiter for the character at the given position and returns a
/// `LinkedEditingRanges` object containing the ranges of the two matching
/// tokens.
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

    // Try heredoc pair first (multi-character label, variable-width ranges)
    if let Some((a, b, label_len)) = find_heredoc_pair(text, byte) {
        let (a_line, a_char) = offset_to_utf16_line_col(text, a);
        let (b_line, b_char) = offset_to_utf16_line_col(text, b);
        let len = label_len as u32;
        let ranges = vec![
            Range::new(
                Position::new(a_line, a_char),
                Position::new(a_line, a_char + len),
            ),
            Range::new(
                Position::new(b_line, b_char),
                Position::new(b_line, b_char + len),
            ),
        ];
        return Some(LinkedEditingRanges {
            ranges,
            word_pattern: None,
        });
    }

    // Try regex/substitution non-bracket delimiter pair
    if let Some((a, b)) = find_regex_delimiter_pair(text, byte) {
        let (a_line, a_char) = offset_to_utf16_line_col(text, a);
        let (b_line, b_char) = offset_to_utf16_line_col(text, b);
        let ranges = vec![
            Range::new(
                Position::new(a_line, a_char),
                Position::new(a_line, a_char + 1),
            ),
            Range::new(
                Position::new(b_line, b_char),
                Position::new(b_line, b_char + 1),
            ),
        ];
        return Some(LinkedEditingRanges {
            ranges,
            word_pattern: None,
        });
    }

    // Fall through to bracket/quote pair matching
    let (a, b) = find_pair(text, byte)?;
    let (a_line, a_char) = offset_to_utf16_line_col(text, a);
    let (b_line, b_char) = offset_to_utf16_line_col(text, b);

    let ranges = vec![
        Range::new(
            Position::new(a_line, a_char),
            Position::new(a_line, a_char + 1),
        ),
        Range::new(
            Position::new(b_line, b_char),
            Position::new(b_line, b_char + 1),
        ),
    ];
    Some(LinkedEditingRanges {
        ranges,
        word_pattern: None,
    })
}
