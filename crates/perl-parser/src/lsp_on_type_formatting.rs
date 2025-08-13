//! textDocument/onTypeFormatting handler - automatic indentation
//!
//! This module provides automatic indentation when typing trigger characters
//! like {, }, ;, ), and newline.

use lsp_types::{Position, Range, TextEdit};

fn byte_offset(text: &str, pos: Position) -> usize {
    let mut off = 0usize;
    for (line, l) in text.split_inclusive('\n').enumerate() {
        if line as u32 == pos.line {
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

fn lsp_range_for_line_start(text: &str, line: u32) -> Range {
    for (cur, l) in text.split_inclusive('\n').enumerate() {
        if cur as u32 == line {
            // Leading whitespace span
            let ws = l
                .chars()
                .take_while(|c| c.is_whitespace() && *c != '\n')
                .count();
            // Build range [line:0 .. line:ws]
            return Range {
                start: Position::new(line, 0),
                end: Position::new(line, ws as u32),
            };
        }
    }
    Range::new(Position::new(line, 0), Position::new(line, 0))
}

fn compute_indent(text: &str, up_to: usize, tab_size: usize) -> usize {
    // Naive: count '{' and '}' before current position
    // Also handle common Perl constructs like if/elsif/else
    let mut depth = 0i32;
    let mut last_line_start = 0;
    let mut in_comment = false;

    for (i, ch) in text.char_indices() {
        if i >= up_to {
            break;
        }

        match ch {
            '#' if !in_comment => in_comment = true,
            '\n' => {
                in_comment = false;
                last_line_start = i + 1;
            }
            '{' if !in_comment => depth += 1,
            '}' if !in_comment => depth = (depth - 1).max(0),
            _ => {}
        }
    }

    // Check if previous line ends with certain keywords that increase indent
    if last_line_start > 0 && last_line_start < up_to {
        let prev_line = &text[last_line_start..up_to];
        let trimmed = prev_line.trim();
        if trimmed.ends_with("if")
            || trimmed.ends_with("elsif")
            || trimmed.ends_with("else")
            || trimmed.ends_with("unless")
            || trimmed.ends_with("while")
            || trimmed.ends_with("until")
            || trimmed.ends_with("for")
            || trimmed.ends_with("foreach")
            || trimmed.ends_with("given")
            || trimmed.ends_with("when")
            || trimmed.ends_with("sub")
        {
            depth += 1;
        }
    }

    (depth.max(0) as usize) * tab_size
}

pub fn format_on_type(
    text: &str,
    _uri: lsp_types::Uri,
    ch: String,
    position: Position,
    tab_size: usize,
    insert_spaces: bool,
) -> Vec<TextEdit> {
    // Only re-indent the current line start
    let line_start_range = lsp_range_for_line_start(text, position.line);

    let off = byte_offset(text, Position::new(position.line, 0));

    // If the char itself is '}' reduce indent before placing it
    let indent = if ch == "}" {
        compute_indent(text, off.saturating_sub(1), tab_size).saturating_sub(tab_size)
    } else if ch == "\n" {
        // For newline, compute indent based on previous content
        let prev_line_end = byte_offset(
            text,
            Position::new(position.line.saturating_sub(1), u32::MAX),
        );
        compute_indent(text, prev_line_end, tab_size)
    } else {
        compute_indent(text, off, tab_size)
    };

    let indent_str = if insert_spaces {
        " ".repeat(indent)
    } else {
        // Use tabs: 1 tab per indent level
        let level = indent / tab_size.max(1);
        "\t".repeat(level)
    };

    vec![TextEdit {
        range: line_start_range,
        new_text: indent_str,
    }]
}
