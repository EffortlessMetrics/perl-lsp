//! Statement boundary tracker for proper heredoc content collection
//!
//! This module provides a simple statement boundary detector that helps
//! the heredoc scanner know when a statement containing heredocs actually ends.

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq)]
enum BracketType {
    Paren,    // ()
    Square,   // []
    Curly,    // {}
}

/// Tracks statement boundaries by monitoring brackets and semicolons
pub struct StatementTracker {
    bracket_stack: Vec<BracketType>,
    in_string: Option<char>,  // None, Some('"'), Some('\''), Some('`')
    escape_next: bool,
}

impl StatementTracker {
    pub fn new() -> Self {
        Self {
            bracket_stack: Vec::new(),
            in_string: None,
            escape_next: false,
        }
    }
    
    /// Process a character and return true if we're at a statement boundary
    pub fn process_char(&mut self, ch: char, prev_char: Option<char>) -> bool {
        // Handle escape sequences
        if self.escape_next {
            self.escape_next = false;
            return false;
        }
        
        if ch == '\\' {
            self.escape_next = true;
            return false;
        }
        
        // Handle string state
        if let Some(quote) = self.in_string {
            if ch == quote {
                self.in_string = None;
            }
            return false;
        }
        
        // Check for string starts
        match ch {
            '"' | '\'' | '`' => {
                self.in_string = Some(ch);
                return false;
            }
            _ => {}
        }
        
        // Track brackets
        match ch {
            '(' => self.bracket_stack.push(BracketType::Paren),
            '[' => self.bracket_stack.push(BracketType::Square),
            '{' => self.bracket_stack.push(BracketType::Curly),
            ')' => {
                if self.bracket_stack.last() == Some(&BracketType::Paren) {
                    self.bracket_stack.pop();
                }
            }
            ']' => {
                if self.bracket_stack.last() == Some(&BracketType::Square) {
                    self.bracket_stack.pop();
                }
            }
            '}' => {
                if self.bracket_stack.last() == Some(&BracketType::Curly) {
                    self.bracket_stack.pop();
                }
            }
            _ => {}
        }
        
        // Statement ends at semicolon or newline when brackets are balanced
        if self.bracket_stack.is_empty() {
            match ch {
                ';' => return true,
                '\n' => {
                    // Check if the line ended with a comma (continuation)
                    if prev_char != Some(',') {
                        return true;
                    }
                }
                _ => {}
            }
        }
        
        false
    }
    
    /// Check if we're currently inside a balanced construct
    pub fn is_balanced(&self) -> bool {
        self.bracket_stack.is_empty() && self.in_string.is_none()
    }
    
    /// Reset the tracker state
    pub fn reset(&mut self) {
        self.bracket_stack.clear();
        self.in_string = None;
        self.escape_next = false;
    }
}

/// Find the line where a statement containing a heredoc actually ends
pub fn find_statement_end_line(input: &str, heredoc_line: usize) -> usize {
    let lines: Vec<&str> = input.lines().collect();
    let mut tracker = StatementTracker::new();
    let mut prev_char = None;
    let mut statement_start_line = heredoc_line;
    
    // First, scan backwards to find where the statement starts
    for line_idx in (0..heredoc_line).rev() {
        let line = lines[line_idx];
        // Simple heuristic: statement likely starts after a semicolon or at start of block
        if line.trim().ends_with(';') || line.trim().ends_with('{') || line_idx == 0 {
            statement_start_line = line_idx + 1;
            if line_idx > 0 && (line.trim().ends_with(';') || line.trim().ends_with('{')) {
                statement_start_line = line_idx + 2; // Next line after semicolon/brace
            }
            break;
        }
    }
    
    // Now scan forward from statement start to find where it ends
    for (idx, line) in lines.iter().enumerate() {
        let current_line = idx + 1;
        
        // Skip lines before statement start
        if current_line < statement_start_line {
            continue;
        }
        
        for ch in line.chars() {
            if tracker.process_char(ch, prev_char) {
                return current_line;
            }
            prev_char = Some(ch);
        }
        
        // Check end of line
        if tracker.process_char('\n', prev_char) {
            return current_line;
        }
        prev_char = Some('\n');
    }
    
    // If we didn't find an end, assume it's at the last line
    lines.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_statement() {
        let input = "my $x = 42;";
        let end = find_statement_end_line(input, 1);
        assert_eq!(end, 1);
    }
    
    #[test]
    fn test_multi_line_hash() {
        let input = r#"my %hash = (
    key => <<'EOF'
);
content
EOF"#;
        let end = find_statement_end_line(input, 2);
        assert_eq!(end, 3); // Statement ends at line 3 with );
    }
    
    #[test]
    fn test_nested_parens() {
        let input = r#"func(
    arg1,
    func2(
        <<'EOF'
    ),
    arg3
);
content
EOF"#;
        let end = find_statement_end_line(input, 4);
        assert_eq!(end, 7); // Statement ends at line 7 with );
    }
}