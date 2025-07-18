//! Multi-phase heredoc parser for Perl
//! 
//! This module implements a three-phase approach to handle Perl's heredocs:
//! 1. Detection - Identify heredoc declarations and mark boundaries
//! 2. Collection - Extract heredoc content from subsequent lines
//! 3. Integration - Replace markers with content for PEG parsing

use std::collections::HashMap;
use std::sync::Arc;

/// Represents a heredoc declaration found during Phase 1
#[derive(Debug, Clone)]
pub struct HeredocDeclaration {
    /// The terminator string (e.g., "EOF", "DATA")
    pub terminator: String,
    /// Position in input where the heredoc was declared
    pub declaration_pos: usize,
    /// Line number of declaration
    pub declaration_line: usize,
    /// Whether the heredoc is interpolated (<<EOF vs <<'EOF')
    pub interpolated: bool,
    /// Whether the heredoc is indented (<<~EOF)
    pub indented: bool,
    /// Unique placeholder token for this heredoc
    pub placeholder_id: String,
    /// The collected content (filled in Phase 2)
    pub content: Option<Arc<str>>,
}

/// Phase 1: Heredoc Detection Scanner
pub struct HeredocScanner<'a> {
    input: &'a str,
    position: usize,
    line_number: usize,
    heredoc_counter: usize,
}

impl<'a> HeredocScanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            line_number: 1,
            heredoc_counter: 0,
        }
    }

    /// Scan input for heredoc declarations and return processed input + declarations
    pub fn scan(mut self) -> (String, Vec<HeredocDeclaration>) {
        let mut output = String::with_capacity(self.input.len());
        let mut declarations = Vec::new();
        let chars: Vec<char> = self.input.chars().collect();
        let lines: Vec<&str> = self.input.lines().collect();

        while self.position < chars.len() {
            if self.check_heredoc_start(&chars) {
                let start_line = self.line_number;
                if let Some(mut decl) = self.parse_heredoc_declaration(&chars) {
                    // Replace heredoc with placeholder
                    output.push_str(&decl.placeholder_id);
                    
                    // Skip to end of line
                    while self.position < chars.len() && chars[self.position] != '\n' {
                        self.position += 1;
                    }
                    if self.position < chars.len() {
                        output.push('\n');
                        self.position += 1;
                        self.line_number += 1;
                    }
                    
                    // Now collect the heredoc content and skip it
                    let mut content = String::new();
                    let mut found_terminator = false;
                    
                    while self.line_number <= lines.len() {
                        if self.line_number > lines.len() {
                            break;
                        }
                        
                        let line = lines[self.line_number - 1]; // Convert to 0-based
                        
                        // Check if this line is the terminator
                        let is_terminator = if decl.indented {
                            line.trim() == decl.terminator
                        } else {
                            line == decl.terminator
                        };
                        
                        if is_terminator {
                            found_terminator = true;
                            // Skip the terminator line
                            while self.position < chars.len() && chars[self.position] != '\n' {
                                self.position += 1;
                            }
                            if self.position < chars.len() {
                                self.position += 1;
                                self.line_number += 1;
                            }
                            break;
                        }
                        
                        // Add line to content
                        if !content.is_empty() {
                            content.push('\n');
                        }
                        
                        if decl.indented {
                            content.push_str(line.trim_start());
                        } else {
                            content.push_str(line);
                        }
                        
                        // Skip this line in the output
                        while self.position < chars.len() && chars[self.position] != '\n' {
                            self.position += 1;
                        }
                        if self.position < chars.len() {
                            self.position += 1;
                            self.line_number += 1;
                        }
                    }
                    
                    if found_terminator {
                        decl.content = Some(Arc::from(content));
                    }
                    
                    declarations.push(decl);
                } else {
                    // Not a heredoc, just copy the <<
                    output.push_str("<<");
                    self.position += 2;
                }
            } else {
                // Regular character
                let ch = chars[self.position];
                output.push(ch);
                if ch == '\n' {
                    self.line_number += 1;
                }
                self.position += 1;
            }
        }

        (output, declarations)
    }

    fn check_heredoc_start(&self, chars: &[char]) -> bool {
        self.position + 1 < chars.len() 
            && chars[self.position] == '<' 
            && chars[self.position + 1] == '<'
    }

    fn parse_heredoc_declaration(&mut self, chars: &[char]) -> Option<HeredocDeclaration> {
        let start_pos = self.position;
        self.position += 2; // Skip <<

        // Check for indented heredoc (<<~)
        let indented = if self.position < chars.len() && chars[self.position] == '~' {
            self.position += 1;
            true
        } else {
            false
        };

        // Skip whitespace
        while self.position < chars.len() && chars[self.position].is_whitespace() && chars[self.position] != '\n' {
            self.position += 1;
        }

        // Determine if quoted
        let (interpolated, terminator) = if self.position < chars.len() {
            match chars[self.position] {
                '\'' => {
                    self.position += 1;
                    let term = self.read_until_quote('\'', chars)?;
                    (false, term)
                }
                '"' => {
                    self.position += 1;
                    let term = self.read_until_quote('"', chars)?;
                    (true, term)
                }
                '`' => {
                    self.position += 1;
                    let term = self.read_until_quote('`', chars)?;
                    (true, term) // Backticks are interpolated
                }
                _ => {
                    let term = self.read_bareword(chars)?;
                    (true, term) // Bare terminators are interpolated
                }
            }
        } else {
            return None;
        };

        self.heredoc_counter += 1;
        let placeholder_id = format!("__HEREDOC_{}__", self.heredoc_counter);

        Some(HeredocDeclaration {
            terminator,
            declaration_pos: start_pos,
            declaration_line: self.line_number,
            interpolated,
            indented,
            placeholder_id,
            content: None,
        })
    }

    fn read_until_quote(&mut self, quote: char, chars: &[char]) -> Option<String> {
        let mut result = String::new();
        while self.position < chars.len() && chars[self.position] != quote {
            if chars[self.position] == '\\' && self.position + 1 < chars.len() {
                // Handle escaped quotes
                self.position += 1;
            }
            result.push(chars[self.position]);
            self.position += 1;
        }
        if self.position < chars.len() {
            self.position += 1; // Skip closing quote
            Some(result)
        } else {
            None
        }
    }

    fn read_bareword(&mut self, chars: &[char]) -> Option<String> {
        let mut result = String::new();
        while self.position < chars.len() 
            && (chars[self.position].is_alphanumeric() || chars[self.position] == '_') {
            result.push(chars[self.position]);
            self.position += 1;
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

/// Phase 2: Heredoc Content Collector
pub struct HeredocCollector<'a> {
    input: &'a str,
    lines: Vec<&'a str>,
}

impl<'a> HeredocCollector<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            lines: input.lines().collect(),
        }
    }

    /// Collect content for all heredoc declarations
    pub fn collect(&self, declarations: &mut Vec<HeredocDeclaration>) {
        // Map declaration line to heredocs declared on that line
        let mut line_to_heredocs: HashMap<usize, Vec<usize>> = HashMap::new();
        
        for (idx, decl) in declarations.iter().enumerate() {
            line_to_heredocs.entry(decl.declaration_line)
                .or_insert_with(Vec::new)
                .push(idx);
        }

        // For each line with heredocs, collect content
        for (line_num, heredoc_indices) in line_to_heredocs {
            // Heredoc content starts on the line after declaration
            let mut content_line = line_num; // line_num is 1-based, lines array is 0-based
            
            for &idx in &heredoc_indices {
                let decl = &declarations[idx];
                let mut content = String::new();
                let mut found_terminator = false;

                // Scan lines for terminator
                while content_line < self.lines.len() {
                    let line = self.lines[content_line];
                    
                    // Check if this line is the terminator
                    let is_terminator = if decl.indented {
                        line.trim() == decl.terminator
                    } else {
                        line == decl.terminator
                    };

                    if is_terminator {
                        found_terminator = true;
                        content_line += 1;
                        break;
                    }

                    // Add line to content
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    
                    if decl.indented {
                        // Remove common leading whitespace for indented heredocs
                        content.push_str(line.trim_start());
                    } else {
                        content.push_str(line);
                    }
                    
                    content_line += 1;
                }

                if found_terminator {
                    declarations[idx].content = Some(Arc::from(content));
                }
            }
        }
    }
}

/// Phase 3: Heredoc Integration
pub struct HeredocIntegrator;

impl HeredocIntegrator {
    /// Replace heredoc placeholders with actual content for final parsing
    pub fn integrate(processed_input: &str, declarations: &[HeredocDeclaration]) -> String {
        let mut result = processed_input.to_string();
        
        // Replace placeholders with quoted content
        for decl in declarations {
            if let Some(ref content) = decl.content {
                // For PEG parsing, we'll represent heredocs as special string literals
                let replacement = if decl.interpolated {
                    format!("qq{{__HEREDOC__{}__HEREDOC__}}", escape_for_qq(content))
                } else {
                    format!("q{{__HEREDOC__{}__HEREDOC__}}", content)
                };
                result = result.replace(&decl.placeholder_id, &replacement);
            }
        }
        
        result
    }
}

fn escape_for_qq(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

/// High-level API for multi-phase heredoc parsing
pub fn parse_with_heredocs(input: &str) -> (String, Vec<HeredocDeclaration>) {
    // Phase 1: Detect heredocs
    let scanner = HeredocScanner::new(input);
    let (processed_input, mut declarations) = scanner.scan();
    
    // Phase 2: Collect content
    let collector = HeredocCollector::new(input);
    collector.collect(&mut declarations);
    
    // Phase 3: Integrate for parsing
    let final_input = HeredocIntegrator::integrate(&processed_input, &declarations);
    
    (final_input, declarations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_heredoc() {
        let input = r#"my $text = <<'EOF';
Hello, World!
This is a heredoc.
EOF
print $text;"#;

        let (processed, declarations) = parse_with_heredocs(input);
        
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(!declarations[0].interpolated);
        assert_eq!(declarations[0].content.as_deref(), Some("Hello, World!\nThis is a heredoc."));
        assert!(processed.contains("__HEREDOC__"));
    }

    #[test]
    fn test_multiple_heredocs() {
        let input = r#"print <<A, <<B;
Content A
A
Content B
B"#;

        let (_, declarations) = parse_with_heredocs(input);
        
        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].content.as_deref(), Some("Content A"));
        assert_eq!(declarations[1].content.as_deref(), Some("Content B"));
    }

    #[test]
    fn test_indented_heredoc() {
        let input = r#"if (1) {
    my $text = <<~'EOF';
        Indented content
        More content
        EOF
}"#;

        let (_, declarations) = parse_with_heredocs(input);
        
        assert_eq!(declarations.len(), 1);
        assert!(declarations[0].indented);
        assert_eq!(declarations[0].content.as_deref(), Some("Indented content\nMore content"));
    }
}