//! Position tracking for lexer tokens

/// Position information with line and column
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Byte offset in the source (0-based)
    pub byte: usize,
    /// Line number (1-based)
    pub line: u32,
    /// Column number (1-based)
    pub column: u32,
}

impl Position {
    /// Create a position at the start of input
    pub fn start() -> Self {
        Position { byte: 0, line: 1, column: 1 }
    }

    /// Advance position by a character
    pub fn advance(&mut self, ch: char) {
        self.byte += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }

    /// Advance position by a string slice
    pub fn advance_str(&mut self, text: &str) {
        for ch in text.chars() {
            self.advance(ch);
        }
    }
}
