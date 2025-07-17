//! Stateful parser wrapper for handling context-sensitive Perl constructs like heredocs

use std::collections::{HashMap, VecDeque};
use crate::pure_rust_parser::{PureRustPerlParser, AstNode};

/// A pending heredoc declaration waiting for its content
#[derive(Debug, Clone)]
struct PendingHeredoc {
    /// The heredoc marker (e.g., "EOF", "END", etc.)
    marker: String,
    /// Whether this is an indented heredoc (<<~)
    indented: bool,
    /// Whether the marker was quoted (<<'EOF' or <<"EOF")
    quoted: bool,
    /// Whether this is a command heredoc (<<`EOF`)
    command: bool,
    /// The line number where the heredoc was declared
    declaration_line: usize,
    /// Position in the original source for AST mapping
    source_position: usize,
}

/// Collected heredoc content
#[derive(Debug, Clone)]
struct HeredocContent {
    content: String,
    indented: bool,
    quoted: bool,
    command: bool,
}

/// A stateful wrapper around PureRustPerlParser that handles heredocs
pub struct StatefulPerlParser {
    inner: PureRustPerlParser,
    pending_heredocs: VecDeque<PendingHeredoc>,
    collected_heredocs: HashMap<String, HeredocContent>,
}

impl StatefulPerlParser {
    pub fn new() -> Self {
        Self {
            inner: PureRustPerlParser::new(),
            pending_heredocs: VecDeque::new(),
            collected_heredocs: HashMap::new(),
        }
    }

    /// Parse Perl source with full heredoc content support
    pub fn parse(&mut self, source: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        // Step 1: Scan for heredoc declarations and collect content
        let processed_source = self.preprocess_heredocs(source)?;
        
        // Step 2: Parse the processed source
        let mut ast = self.inner.parse(&processed_source)?;
        
        // Step 3: Inject heredoc content into the AST
        self.inject_heredoc_content(&mut ast)?;
        
        Ok(ast)
    }

    /// Preprocess source to handle heredocs
    fn preprocess_heredocs(&mut self, source: &str) -> Result<String, Box<dyn std::error::Error>> {
        let lines: Vec<&str> = source.lines().collect();
        let mut processed_lines = Vec::new();
        let mut skip_lines = std::collections::HashSet::new();
        
        // First pass: detect heredoc declarations
        for (line_no, line) in lines.iter().enumerate() {
            if skip_lines.contains(&line_no) {
                continue;
            }
            
            // Look for heredoc declarations
            if let Some(heredocs) = self.detect_heredocs(line, line_no) {
                for heredoc in heredocs {
                    self.pending_heredocs.push_back(heredoc);
                }
            }
            
            processed_lines.push(line.to_string());
        }
        
        // Second pass: collect heredoc content
        let mut i = 0;
        while i < lines.len() {
            if !self.pending_heredocs.is_empty() {
                // Check if we're past a heredoc declaration line
                if let Some(heredoc) = self.pending_heredocs.front() {
                    if i > heredoc.declaration_line {
                        // Start collecting content for this heredoc
                        let (content, end_line) = self.collect_heredoc_content(&lines, i, heredoc)?;
                        
                        // Store the collected content
                        let heredoc = self.pending_heredocs.pop_front().unwrap();
                        let key = self.make_heredoc_key(&heredoc);
                        self.collected_heredocs.insert(key, HeredocContent {
                            content,
                            indented: heredoc.indented,
                            quoted: heredoc.quoted,
                            command: heredoc.command,
                        });
                        
                        // Mark lines as consumed
                        for line_no in i..=end_line {
                            skip_lines.insert(line_no);
                        }
                        
                        i = end_line + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        
        // Check for unterminated heredocs
        if !self.pending_heredocs.is_empty() {
            let heredoc = self.pending_heredocs.front().unwrap();
            return Err(format!(
                "Unterminated heredoc '{}' starting at line {}",
                heredoc.marker,
                heredoc.declaration_line + 1
            ).into());
        }
        
        // Rebuild source without consumed lines
        let mut final_lines = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if !skip_lines.contains(&i) {
                final_lines.push(line.to_string());
            }
        }
        
        Ok(final_lines.join("\n"))
    }

    /// Detect heredoc declarations in a line
    fn detect_heredocs(&self, line: &str, line_no: usize) -> Option<Vec<PendingHeredoc>> {
        let mut heredocs = Vec::new();
        
        // More comprehensive regex to match various heredoc forms
        // This matches: <<EOF, <<'EOF', <<"EOF", <<~EOF, <<`EOF`, <<\EOF
        // Note: We can't use backreferences in Rust regex, so we'll handle quotes differently
        let heredoc_pattern = regex::Regex::new(
            r#"<<(~)?\s*(?:(['"`])([^'"`]+)(['"`])|\\(\w+)|(\w+))"#
        ).unwrap();
        
        for cap in heredoc_pattern.captures_iter(line) {
            let indented = cap.get(1).is_some();
            let open_quote = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let close_quote = cap.get(4).map(|m| m.as_str()).unwrap_or("");
            
            // Extract marker from different capture groups
            let (marker, quoted, command) = if let Some(m) = cap.get(3) {
                // Quoted marker - check if quotes match
                if open_quote == close_quote && !open_quote.is_empty() {
                    (m.as_str().to_string(), open_quote == "'", open_quote == "`")
                } else {
                    continue; // Mismatched quotes
                }
            } else if let Some(m) = cap.get(5) {
                // Escaped marker (\EOF)
                (m.as_str().to_string(), true, false)
            } else if let Some(m) = cap.get(6) {
                // Unquoted marker
                (m.as_str().to_string(), false, false)
            } else {
                continue;
            };
            
            heredocs.push(PendingHeredoc {
                marker,
                indented,
                quoted,
                command,
                declaration_line: line_no,
                source_position: cap.get(0).unwrap().start(),
            });
        }
        
        if heredocs.is_empty() {
            None
        } else {
            Some(heredocs)
        }
    }

    /// Collect content for a heredoc until its terminator
    fn collect_heredoc_content(
        &self,
        lines: &[&str],
        start_line: usize,
        heredoc: &PendingHeredoc,
    ) -> Result<(String, usize), Box<dyn std::error::Error>> {
        let mut content = String::new();
        let mut end_line = start_line;
        let mut found_terminator = false;
        
        for (i, line) in lines[start_line..].iter().enumerate() {
            let line_no = start_line + i;
            
            // Check if this line is the terminator
            let trimmed = if heredoc.indented {
                line.trim_start()
            } else {
                *line
            };
            
            if trimmed == heredoc.marker {
                end_line = line_no;
                found_terminator = true;
                break;
            }
            
            // Add content
            if heredoc.indented {
                // Strip common leading whitespace
                content.push_str(&self.strip_indent(line));
            } else {
                content.push_str(line);
            }
            content.push('\n');
            
            // Safety check to prevent infinite loops
            if i > 10000 {
                return Err(format!(
                    "Heredoc '{}' content exceeds maximum length",
                    heredoc.marker
                ).into());
            }
        }
        
        // Check if we found the terminator
        if !found_terminator {
            return Err(format!(
                "Unterminated heredoc '{}' starting at line {}",
                heredoc.marker,
                heredoc.declaration_line + 1
            ).into());
        }
        
        // Remove trailing newline if present
        if content.ends_with('\n') {
            content.pop();
        }
        
        Ok((content, end_line))
    }

    /// Strip common leading whitespace for indented heredocs
    fn strip_indent(&self, line: &str) -> String {
        // For now, just strip leading tabs/spaces
        // A full implementation would calculate common indent
        line.trim_start().to_string()
    }

    /// Create a unique key for storing heredoc content
    fn make_heredoc_key(&self, heredoc: &PendingHeredoc) -> String {
        format!("{}:{}:{}", heredoc.marker, heredoc.declaration_line, heredoc.source_position)
    }

    /// Walk the AST and inject heredoc content
    fn inject_heredoc_content(&mut self, ast: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        self.walk_and_inject(ast, 0)
    }

    /// Recursively walk AST nodes and inject content
    fn walk_and_inject(&mut self, node: &mut AstNode, depth: usize) -> Result<(), Box<dyn std::error::Error>> {
        match node {
            AstNode::Heredoc { marker, content, indented, quoted, .. } => {
                // Try to find content for this heredoc
                // For now, we'll use a simple lookup based on marker
                // A real implementation would track line numbers through parsing
                for (key, heredoc_content) in &self.collected_heredocs {
                    if key.starts_with(&format!("{}:", marker)) {
                        *content = heredoc_content.content.clone();
                        *indented = heredoc_content.indented;
                        *quoted = heredoc_content.quoted;
                        break;
                    }
                }
            }
            // Recursively process child nodes
            AstNode::Program(statements) |
            AstNode::Block(statements) => {
                for stmt in statements {
                    self.walk_and_inject(stmt, depth + 1)?;
                }
            }
            AstNode::DoBlock(stmt) => {
                self.walk_and_inject(stmt, depth + 1)?;
            }
            AstNode::Statement(stmt) => {
                self.walk_and_inject(stmt, depth + 1)?;
            }
            AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
                self.walk_and_inject(condition, depth + 1)?;
                self.walk_and_inject(then_block, depth + 1)?;
                for (cond, block) in elsif_clauses {
                    self.walk_and_inject(cond, depth + 1)?;
                    self.walk_and_inject(block, depth + 1)?;
                }
                if let Some(else_b) = else_block {
                    self.walk_and_inject(else_b, depth + 1)?;
                }
            }
            AstNode::UnlessStatement { condition, block, else_block } => {
                self.walk_and_inject(condition, depth + 1)?;
                self.walk_and_inject(block, depth + 1)?;
                if let Some(else_b) = else_block {
                    self.walk_and_inject(else_b, depth + 1)?;
                }
            }
            AstNode::WhileStatement { condition, block, .. } => {
                self.walk_and_inject(condition, depth + 1)?;
                self.walk_and_inject(block, depth + 1)?;
            }
            AstNode::ForStatement { init, condition, update, block, .. } => {
                if let Some(i) = init {
                    self.walk_and_inject(i, depth + 1)?;
                }
                if let Some(c) = condition {
                    self.walk_and_inject(c, depth + 1)?;
                }
                if let Some(u) = update {
                    self.walk_and_inject(u, depth + 1)?;
                }
                self.walk_and_inject(block, depth + 1)?;
            }
            AstNode::ForeachStatement { variable, list, block, .. } => {
                if let Some(var) = variable {
                    self.walk_and_inject(var, depth + 1)?;
                }
                self.walk_and_inject(list, depth + 1)?;
                self.walk_and_inject(block, depth + 1)?;
            }
            AstNode::FunctionCall { function, args } => {
                self.walk_and_inject(function, depth + 1)?;
                for arg in args {
                    self.walk_and_inject(arg, depth + 1)?;
                }
            }
            AstNode::MethodCall { object, method, args } => {
                self.walk_and_inject(object, depth + 1)?;
                for arg in args {
                    self.walk_and_inject(arg, depth + 1)?;
                }
            }
            AstNode::BinaryOp { left, right, .. } => {
                self.walk_and_inject(left, depth + 1)?;
                self.walk_and_inject(right, depth + 1)?;
            }
            AstNode::UnaryOp { operand, .. } => {
                self.walk_and_inject(operand, depth + 1)?;
            }
            AstNode::Assignment { target, value, .. } => {
                self.walk_and_inject(target, depth + 1)?;
                self.walk_and_inject(value, depth + 1)?;
            }
            AstNode::List(elements) => {
                for elem in elements {
                    self.walk_and_inject(elem, depth + 1)?;
                }
            }
            AstNode::VariableDeclaration { variables, initializer, .. } => {
                for var in variables {
                    self.walk_and_inject(var, depth + 1)?;
                }
                if let Some(init) = initializer {
                    self.walk_and_inject(init, depth + 1)?;
                }
            }
            AstNode::SubDeclaration { body, .. } => {
                self.walk_and_inject(body, depth + 1)?;
            }
            AstNode::PackageDeclaration { block, .. } => {
                if let Some(b) = block {
                    self.walk_and_inject(b, depth + 1)?;
                }
            }
            // Leaf nodes - no recursion needed
            _ => {}
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_heredoc() {
        let source = r#"my $text = <<EOF;
Hello
World
EOF
print $text;"#;

        let mut parser = StatefulPerlParser::new();
        let ast = parser.parse(source).unwrap();
        
        // Convert to S-expression to check
        let sexp = parser.inner.to_sexp(&ast);
        println!("S-expression: {}", sexp);
        assert!(sexp.contains("heredoc EOF"));
        assert!(sexp.contains("Hello\\nWorld"));
    }

    #[test]
    fn test_quoted_heredoc() {
        let source = r#"my $text = <<'END';
No $interpolation here
END
"#;

        let mut parser = StatefulPerlParser::new();
        let ast = parser.parse(source).unwrap();
        
        let sexp = parser.inner.to_sexp(&ast);
        println!("Quoted heredoc S-expression: {}", sexp);
        assert!(sexp.contains("heredoc END"));
        assert!(sexp.contains("No $interpolation here"));
    }

    #[test]
    fn test_indented_heredoc() {
        let source = r#"my $code = <<~CODE;
    if (1) {
        print "indented";
    }
CODE
"#;

        let mut parser = StatefulPerlParser::new();
        let ast = parser.parse(source).unwrap();
        
        let sexp = parser.inner.to_sexp(&ast);
        println!("Indented heredoc S-expression: {}", sexp);
        assert!(sexp.contains("heredoc CODE"));
        // The content should have leading whitespace stripped
        assert!(sexp.contains("if (1)"));
    }

    #[test]
    fn test_unterminated_heredoc() {
        let source = r#"my $text = <<EOF;
This heredoc
never ends"#;

        let mut parser = StatefulPerlParser::new();
        let result = parser.parse(source);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Unterminated heredoc"));
    }

    #[test]
    fn test_multiple_heredocs() {
        let source = r#"print <<ONE, <<TWO;
First heredoc
ONE
Second heredoc
TWO
"#;

        let mut parser = StatefulPerlParser::new();
        let ast = parser.parse(source).unwrap();
        
        let sexp = parser.inner.to_sexp(&ast);
        println!("Multiple heredocs S-expression: {}", sexp);
        assert!(sexp.contains("First heredoc"));
        assert!(sexp.contains("Second heredoc"));
    }

    #[test]
    fn test_heredoc_with_code_after() {
        let source = r#"my $text = <<EOF;
Content here
EOF
print "After heredoc";
my $x = 42;"#;

        let mut parser = StatefulPerlParser::new();
        let ast = parser.parse(source).unwrap();
        
        let sexp = parser.inner.to_sexp(&ast);
        println!("Heredoc with code after S-expression: {}", sexp);
        assert!(sexp.contains("Content here"));
        assert!(sexp.contains("After heredoc"));
        assert!(sexp.contains("42"));
    }

    #[test]
    fn test_command_heredoc() {
        let source = r#"my $output = <<`CMD`;
echo "Hello from shell"
CMD
"#;

        let mut parser = StatefulPerlParser::new();
        let ast = parser.parse(source).unwrap();
        
        let sexp = parser.inner.to_sexp(&ast);
        println!("Command heredoc S-expression: {}", sexp);
        assert!(sexp.contains("echo"));
        assert!(sexp.contains("Hello from shell"));
    }

    #[test]
    fn test_escaped_heredoc() {
        let source = r#"my $text = <<\EOF;
No interpolation with \EOF
EOF
"#;

        let mut parser = StatefulPerlParser::new();
        let ast = parser.parse(source).unwrap();
        
        let sexp = parser.inner.to_sexp(&ast);
        println!("Escaped heredoc S-expression: {}", sexp);
        assert!(sexp.contains("No interpolation"));
    }
}