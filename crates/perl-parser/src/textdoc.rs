//! UTF-16 aware text document handling for LSP
//!
//! Provides proper position mapping between LSP's UTF-16 encoding and Rust's UTF-8 strings

use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use ropey::Rope;

#[derive(Clone)]
pub struct Doc {
    pub rope: Rope,
    pub version: i32,
}

pub enum PosEnc {
    Utf16,
    Utf8,
}

/// Convert LSP position to byte offset with proper encoding support
pub fn lsp_pos_to_byte(rope: &Rope, pos: Position, enc: PosEnc) -> usize {
    let line = pos.line as usize;
    let col = pos.character as usize;
    
    if line >= rope.len_lines() {
        return rope.len_bytes();
    }
    
    let line_char0 = rope.line_to_char(line);
    
    match enc {
        PosEnc::Utf8 => {
            let line_chars = rope.line(line).len_chars();
            let char_idx = col.min(line_chars);
            rope.char_to_byte(line_char0 + char_idx)
        }
        PosEnc::Utf16 => {
            let line_slice = rope.line(line);
            let mut cu = 0usize;
            for (i, ch) in line_slice.chars().enumerate() {
                if cu >= col {
                    return rope.char_to_byte(line_char0 + i);
                }
                cu += ch.len_utf16();
            }
            // Clamp to end of line
            rope.char_to_byte(line_char0 + line_slice.len_chars())
        }
    }
}

/// Convert byte offset to LSP position with proper encoding support
pub fn byte_to_lsp_pos(rope: &Rope, byte: usize, enc: PosEnc) -> Position {
    let byte = byte.min(rope.len_bytes());
    let char_idx = rope.byte_to_char(byte);
    let line = rope.char_to_line(char_idx);
    let line_char0 = rope.line_to_char(line);
    let col_chars = char_idx - line_char0;
    
    let character = match enc {
        PosEnc::Utf8 => col_chars as u32,
        PosEnc::Utf16 => {
            let line_slice = rope.line(line);
            let mut cu = 0u32;
            for (i, ch) in line_slice.chars().enumerate() {
                if i >= col_chars {
                    break;
                }
                cu += ch.len_utf16() as u32;
            }
            cu
        }
    };
    
    Position {
        line: line as u32,
        character,
    }
}

/// Convert LSP range to byte offsets
pub fn range_to_bytes(rope: &Rope, range: &Range, enc: PosEnc) -> (usize, usize) {
    let s = lsp_pos_to_byte(rope, range.start, enc.clone());
    let e = lsp_pos_to_byte(rope, range.end, enc);
    (s.min(rope.len_bytes()), e.min(rope.len_bytes()))
}

/// Apply incremental changes to a document
pub fn apply_changes(doc: &mut Doc, changes: &[TextDocumentContentChangeEvent], enc: PosEnc) {
    for ch in changes {
        if let Some(r) = &ch.range {
            let (s, e) = range_to_bytes(&doc.rope, r, enc.clone());
            if s <= doc.rope.len_bytes() && e <= doc.rope.len_bytes() && s <= e {
                doc.rope.remove(s..e);
                doc.rope.insert(s, &ch.text);
            }
        } else {
            // Full document replace
            doc.rope = Rope::from_str(&ch.text);
        }
    }
}

// Make PosEnc Clone
impl Clone for PosEnc {
    fn clone(&self) -> Self {
        match self {
            PosEnc::Utf16 => PosEnc::Utf16,
            PosEnc::Utf8 => PosEnc::Utf8,
        }
    }
}