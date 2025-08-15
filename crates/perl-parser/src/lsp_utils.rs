//! Common utilities for LSP handlers

use lsp_types::Position;

/// Convert an LSP position to a byte offset in the text
pub fn position_to_offset(content: &str, line: u32, character: u32) -> Option<usize> {
    let mut current_line = 0;
    let mut current_char = 0;

    for (i, ch) in content.char_indices() {
        if current_line == line && current_char == character {
            return Some(i);
        }

        if ch == '\n' {
            current_line += 1;
            current_char = 0;
        } else if current_line == line {
            current_char += 1;
        }
    }

    // Handle end of file
    if current_line == line && current_char == character { Some(content.len()) } else { None }
}

/// Convert a byte offset to an LSP position
pub fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position { line, character }
}
