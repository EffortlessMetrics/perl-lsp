//! Statement boundary tracker for proper heredoc content collection
//!
//! This module provides a simple statement boundary detector that helps
//! the heredoc scanner know when a statement containing heredocs actually ends.


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
    
    // For heredocs, we only care about the immediate statement boundary,
    // not enclosing blocks. Just scan the heredoc line itself.
    if heredoc_line > 0 && heredoc_line <= lines.len() {
        let line = lines[heredoc_line - 1];
        
        // Simple heuristic: if the line ends with a semicolon, the statement ends here
        if line.trim_end().ends_with(';') {
            return heredoc_line;
        }
        
        // If it's inside a parenthesized expression, find the closing paren
        let mut paren_depth: i32 = 0;
        let mut in_string = false;
        let mut string_char = ' ';
        
        // Count unclosed parens on the heredoc line
        for ch in line.chars() {
            if in_string {
                if ch == string_char && line.chars().nth(line.find(ch).unwrap_or(0).saturating_sub(1)) != Some('\\') {
                    in_string = false;
                }
            } else {
                match ch {
                    '"' | '\'' => {
                        in_string = true;
                        string_char = ch;
                    }
                    '(' => paren_depth += 1,
                    ')' => paren_depth = paren_depth.saturating_sub(1),
                    _ => {}
                }
            }
        }
        
        // If we have unclosed parens, scan forward to find the closing
        if paren_depth > 0 {
            for (idx, scan_line) in lines.iter().enumerate().skip(heredoc_line) {
                for ch in scan_line.chars() {
                    if in_string {
                        if ch == string_char {
                            in_string = false;
                        }
                    } else {
                        match ch {
                            '"' | '\'' => {
                                in_string = true;
                                string_char = ch;
                            }
                            '(' => paren_depth += 1,
                            ')' => {
                                paren_depth = paren_depth.saturating_sub(1);
                                if paren_depth == 0 && scan_line.trim_end().ends_with(");") {
                                    return idx + 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    
    // Default: statement ends on the same line as the heredoc
    heredoc_line
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