//! textDocument/selectionRange handler - smart selection expansion
//!
//! This module provides intelligent selection expansion that grows from
//! identifier -> expression -> statement -> block -> function -> file.

use lsp_types::{Position, Range, SelectionRange};

fn byte_offset(text: &str, pos: Position) -> usize {
    let mut off = 0usize;
    for (line, l) in text.split_inclusive('\n').enumerate() {
        if line as u32 == pos.line {
            // Count UTF-16 columns approximately as chars (good enough for ASCII perl code)
            let mut col = 0u32;
            for (i, ch) in l.char_indices() {
                if col == pos.character {
                    return off + i;
                }
                col += ch.len_utf16() as u32;
            }
            return off + l.len();
        }
        off += l.len();
    }
    off
}

fn make_range(text: &str, start: usize, end: usize) -> Range {
    // Simple byte->(line,col)
    let mut line = 0u32;
    let mut col = 0u32;
    let mut i = 0usize;
    let mut s = Position::new(0, 0);
    let mut e = Position::new(0, 0);
    for ch in text.chars() {
        if i == start {
            s = Position::new(line, col);
        }
        if i == end {
            e = Position::new(line, col);
            break;
        }
        i += ch.len_utf8();
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf16() as u32;
        }
    }
    if e == Position::new(0, 0) {
        e = Position::new(line, col);
    }
    Range::new(s, e)
}

pub fn selection_ranges(text: &str, positions: &[Position]) -> Vec<SelectionRange> {
    positions
        .iter()
        .map(|&pos| {
            let off = byte_offset(text, pos);
            let bytes = text.as_bytes();

            // Word span (identifier or variable)
            let start = (0..=off.min(bytes.len().saturating_sub(1)))
                .rev()
                .find(|&i| {
                    i == 0
                        || (!bytes[i - 1].is_ascii_alphanumeric()
                            && bytes[i - 1] != b'_'
                            && bytes[i - 1] != b'$'
                            && bytes[i - 1] != b'@'
                            && bytes[i - 1] != b'%')
                })
                .unwrap_or(off);
            let end = (off..bytes.len())
                .find(|&i| {
                    i == bytes.len() || (!bytes[i].is_ascii_alphanumeric() && bytes[i] != b'_')
                })
                .unwrap_or(off);
            let word = make_range(text, start, end);

            // Trimmed line
            let line_start = text[..off].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = text[off..]
                .find('\n')
                .map(|i| off + i)
                .unwrap_or(text.len());
            let line_text = &text[line_start..line_end];
            let trim_left = line_text.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            let trim_right = line_text
                .rfind(|c: char| !c.is_whitespace())
                .map(|i| i + 1)
                .unwrap_or(line_text.len());
            let trimmed = make_range(text, line_start + trim_left, line_start + trim_right);
            let whole_line = make_range(text, line_start, line_end);

            // Statement (find semicolon boundaries)
            let stmt_start = text[..off]
                .rfind(';')
                .map(|i| {
                    // Skip whitespace after semicolon
                    text[i + 1..]
                        .chars()
                        .position(|c| !c.is_whitespace())
                        .map(|j| i + 1 + j)
                        .unwrap_or(i + 1)
                })
                .unwrap_or(0);
            let stmt_end = text[off..]
                .find(';')
                .map(|i| off + i + 1)
                .unwrap_or_else(|| {
                    // If no semicolon, find end of line or block
                    text[off..]
                        .find('\n')
                        .map(|i| off + i)
                        .unwrap_or(text.len())
                });
            let statement = make_range(text, stmt_start, stmt_end);

            // Block (find brace boundaries)
            let block_start = text[..off].rfind('{').unwrap_or(0);
            let block_end = text[off..]
                .find('}')
                .map(|i| off + i + 1)
                .unwrap_or(text.len());
            let block = make_range(text, block_start, block_end);

            // Function (find sub boundaries)
            let func_start = text[..off].rfind("sub ").unwrap_or(0);
            let func_end = if func_start > 0 {
                // Find the closing brace for this sub
                let mut depth = 0;
                let mut in_sub = false;
                text[func_start..]
                    .char_indices()
                    .find(|(_, c)| {
                        if *c == '{' {
                            in_sub = true;
                            depth += 1;
                        } else if *c == '}' && in_sub {
                            depth -= 1;
                            if depth == 0 {
                                return true;
                            }
                        }
                        false
                    })
                    .map(|(i, _)| func_start + i + 1)
                    .unwrap_or(text.len())
            } else {
                text.len()
            };
            let function = if func_start > 0 && func_end > func_start {
                make_range(text, func_start, func_end)
            } else {
                make_range(text, 0, text.len())
            };

            // Build the selection hierarchy
            SelectionRange {
                range: word,
                parent: Some(Box::new(SelectionRange {
                    range: trimmed,
                    parent: Some(Box::new(SelectionRange {
                        range: whole_line,
                        parent: Some(Box::new(SelectionRange {
                            range: statement,
                            parent: Some(Box::new(SelectionRange {
                                range: block,
                                parent: Some(Box::new(SelectionRange {
                                    range: function,
                                    parent: None,
                                })),
                            })),
                        })),
                    })),
                })),
            }
        })
        .collect()
}
