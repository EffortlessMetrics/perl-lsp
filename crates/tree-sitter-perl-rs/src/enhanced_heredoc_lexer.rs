//! Enhanced heredoc lexer with improved edge case handling
//!
//! This module provides a more robust heredoc lexer that handles:
//! - Backtick heredocs (<<`CMD`)
//! - Escaped delimiter heredocs (<<\EOF)
//! - Whitespace around operators (<< 'EOF')
//! - Multiple heredocs in lists
//! - Heredocs in complex contexts (hashes, arrays, returns)

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum HeredocQuoteType {
    Bare,     // <<EOF
    Single,   // <<'EOF'
    Double,   // <<"EOF"
    Backtick, // <<`CMD`
    Escaped,  // <<\EOF
}

#[derive(Debug, Clone)]
pub struct HeredocDeclaration {
    pub terminator: String,
    pub quote_type: HeredocQuoteType,
    pub indented: bool,
    pub declaration_start: usize,
    pub declaration_end: usize,
    pub line_number: usize,
    pub placeholder_id: String,
    pub content: Option<Arc<str>>,
}

#[derive(Debug, Clone)]
pub struct HeredocToken {
    pub kind: HeredocTokenKind,
    pub text: String,
    pub position: usize,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeredocTokenKind {
    Text,
    HeredocStart,
    HeredocTerminator,
    HeredocContent,
    Newline,
    Other,
}

pub struct EnhancedHeredocLexer<'a> {
    #[allow(dead_code)]
    input: &'a str,
    chars: Vec<char>,
    position: usize,
    line: usize,
    pending_heredocs: VecDeque<HeredocDeclaration>,
    active_heredoc: Option<HeredocDeclaration>,
    heredoc_counter: usize,
}

impl<'a> EnhancedHeredocLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            position: 0,
            line: 1,
            pending_heredocs: VecDeque::new(),
            active_heredoc: None,
            heredoc_counter: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<HeredocToken> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            // Check if we're collecting heredoc content
            if let Some(ref heredoc) = self.active_heredoc.clone()
                && let Some(token) = self.collect_heredoc_content(heredoc)
            {
                tokens.push(token);
                continue;
            }

            // Check for end of line to activate pending heredocs
            if self.current_char() == Some('\n') {
                tokens.push(self.make_token(HeredocTokenKind::Newline, "\n"));
                self.advance();

                // Activate the next pending heredoc if any
                if let Some(heredoc) = self.pending_heredocs.pop_front() {
                    self.active_heredoc = Some(heredoc);
                }
                continue;
            }

            // Check for heredoc start
            if let Some(token) = self.check_heredoc_start() {
                tokens.push(token);
                continue;
            }

            // Regular character
            let ch = self.current_char().unwrap_or('\0');
            tokens.push(self.make_token(HeredocTokenKind::Text, &ch.to_string()));
            self.advance();
        }

        tokens
    }

    fn check_heredoc_start(&mut self) -> Option<HeredocToken> {
        if self.current_char() != Some('<') || self.peek_char() != Some('<') {
            return None;
        }

        let start_pos = self.position;
        let start_line = self.line;

        self.advance(); // First <
        self.advance(); // Second <

        // Check for indented heredoc (<<~)
        let indented = if self.current_char() == Some('~') {
            self.advance();
            true
        } else {
            false
        };

        // Skip optional whitespace
        while self.current_char() == Some(' ') || self.current_char() == Some('\t') {
            self.advance();
        }

        // Determine quote type and extract terminator
        let (quote_type, terminator) = self.parse_heredoc_terminator()?;

        self.heredoc_counter += 1;
        let placeholder_id = format!("HEREDOC_PLACEHOLDER_{}", self.heredoc_counter);

        let declaration = HeredocDeclaration {
            terminator,
            quote_type,
            indented,
            declaration_start: start_pos,
            declaration_end: self.position,
            line_number: start_line,
            placeholder_id: placeholder_id.clone(),
            content: None,
        };

        self.pending_heredocs.push_back(declaration);

        Some(HeredocToken {
            kind: HeredocTokenKind::HeredocStart,
            text: placeholder_id,
            position: start_pos,
            line: start_line,
        })
    }

    fn parse_heredoc_terminator(&mut self) -> Option<(HeredocQuoteType, String)> {
        match self.current_char()? {
            '\'' => {
                self.advance();
                let terminator = self.read_until('\'');
                self.advance(); // consume closing '
                Some((HeredocQuoteType::Single, terminator))
            }
            '"' => {
                self.advance();
                let terminator = self.read_until('"');
                self.advance(); // consume closing "
                Some((HeredocQuoteType::Double, terminator))
            }
            '`' => {
                self.advance();
                let terminator = self.read_until('`');
                self.advance(); // consume closing `
                Some((HeredocQuoteType::Backtick, terminator))
            }
            '\\' => {
                self.advance();
                let terminator = self.read_identifier();
                Some((HeredocQuoteType::Escaped, terminator))
            }
            _ if self.is_identifier_start(self.current_char()?) => {
                let terminator = self.read_identifier();
                Some((HeredocQuoteType::Bare, terminator))
            }
            _ => None,
        }
    }

    fn collect_heredoc_content(&mut self, heredoc: &HeredocDeclaration) -> Option<HeredocToken> {
        let start_pos = self.position;
        let start_line = self.line;
        let mut content = String::new();
        let mut lines = Vec::new();

        // Collect lines until we find the terminator
        loop {
            if self.is_at_end() {
                break;
            }

            let _line_start = self.position;
            let mut line_content = String::new();

            // Read the line
            while !self.is_at_end() && self.current_char() != Some('\n') {
                line_content.push(self.current_char()?);
                self.advance();
            }

            // Check if this is the terminator
            let trimmed = if heredoc.indented { line_content.trim() } else { &line_content };

            if trimmed == heredoc.terminator {
                // Found terminator
                if self.current_char() == Some('\n') {
                    self.advance();
                }

                // Create content token
                let token = HeredocToken {
                    kind: HeredocTokenKind::HeredocContent,
                    text: content,
                    position: start_pos,
                    line: start_line,
                };

                self.active_heredoc = None;
                return Some(token);
            }

            // Add line to content
            lines.push(line_content);

            // Handle newline
            if self.current_char() == Some('\n') {
                self.advance();
                if !lines.is_empty() {
                    content.push_str(&lines.join("\n"));
                    content.push('\n');
                    lines.clear();
                }
            }
        }

        // If we get here, we didn't find the terminator
        if !content.is_empty() || !lines.is_empty() {
            if !lines.is_empty() {
                content.push_str(&lines.join("\n"));
            }

            Some(HeredocToken {
                kind: HeredocTokenKind::HeredocContent,
                text: content,
                position: start_pos,
                line: start_line,
            })
        } else {
            None
        }
    }

    fn read_until(&mut self, delimiter: char) -> String {
        let mut result = String::new();
        while !self.is_at_end() && self.current_char() != Some(delimiter) {
            result.push(self.current_char().unwrap_or('\0'));
            self.advance();
        }
        result
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        while !self.is_at_end() {
            let ch = self.current_char().unwrap_or('\0');
            if self.is_identifier_char(ch) {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn is_identifier_start(&self, ch: char) -> bool {
        ch.is_alphabetic() || ch == '_'
    }

    fn is_identifier_char(&self, ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    fn current_char(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.position + 1).copied()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            if ch == '\n' {
                self.line += 1;
            }
            self.position += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.chars.len()
    }

    fn make_token(&self, kind: HeredocTokenKind, text: &str) -> HeredocToken {
        HeredocToken { kind, text: text.to_string(), position: self.position, line: self.line }
    }
}

/// Process input with enhanced heredoc handling
pub fn process_with_enhanced_heredocs(input: &str) -> (String, Vec<HeredocDeclaration>) {
    let mut lexer = EnhancedHeredocLexer::new(input);
    let tokens = lexer.tokenize();

    let mut output = String::new();
    let mut declarations = Vec::new();
    let mut content_map = HashMap::new();

    // Build output and collect heredoc info
    for token in tokens {
        match token.kind {
            HeredocTokenKind::HeredocStart => {
                output.push_str(&token.text);
            }
            HeredocTokenKind::HeredocContent => {
                // Store content for later
                if let Some(mut heredoc) = lexer.pending_heredocs.pop_front() {
                    heredoc.content = Some(Arc::from(token.text.as_str()));
                    content_map.insert(heredoc.placeholder_id.clone(), heredoc.clone());
                    declarations.push(heredoc);
                }
            }
            _ => {
                output.push_str(&token.text);
            }
        }
    }

    (output, declarations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backtick_heredoc() {
        let input = r#"my $cmd = <<`EOF`;
echo hello
EOF
"#;
        let (_output, decls) = process_with_enhanced_heredocs(input);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].quote_type, HeredocQuoteType::Backtick);
    }

    #[test]
    fn test_escaped_heredoc() {
        let input = r#"my $text = <<\EOF;
literal $text
EOF
"#;
        let (_output, decls) = process_with_enhanced_heredocs(input);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].quote_type, HeredocQuoteType::Escaped);
    }

    #[test]
    fn test_whitespace_around_operator() {
        let input = r#"my $text = << 'EOF';
content
EOF
"#;
        let (_output, decls) = process_with_enhanced_heredocs(input);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].terminator, "EOF");
    }
}
