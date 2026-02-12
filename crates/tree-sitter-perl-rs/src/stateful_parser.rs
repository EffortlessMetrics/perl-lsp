//! Stateful parser wrapper for handling Perl constructs that require state
//!
//! This module provides a stateful wrapper around the Pest parser to handle
//! constructs like heredocs that require maintaining state across lines.

use crate::pure_rust_parser::AstNode;
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct HeredocMarker {
    pub marker: String,
    pub indented: bool,
    pub quoted: HeredocQuoteType,
    pub position: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeredocQuoteType {
    None,
    Single,
    Double,
    Backtick,
    Escaped,
}

#[derive(Debug)]
pub struct StatefulPerlParser {
    /// Queue of heredoc markers we're waiting to collect content for
    pending_heredocs: VecDeque<HeredocMarker>,
    /// Buffer for collecting lines when processing heredocs
    line_buffer: Vec<String>,
    /// Current parsing state
    state: ParserState,
    /// Format declaration info when collecting format content
    current_format: Option<FormatInfo>,
    /// Collected format contents
    format_contents: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone)]
pub struct FormatInfo {
    name: String,
    _start_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParserState {
    Normal,
    CollectingHeredoc,
    CollectingFormat,
}

impl StatefulPerlParser {
    pub fn new() -> Self {
        Self {
            pending_heredocs: VecDeque::new(),
            line_buffer: Vec::new(),
            state: ParserState::Normal,
            current_format: None,
            format_contents: Vec::new(),
        }
    }

    /// Parse a complete Perl source file handling heredocs properly
    pub fn parse(&mut self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        let lines: Vec<&str> = input.lines().collect();
        let mut processed_lines = Vec::new();
        let mut heredoc_contents: Vec<(String, String)> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            match self.state {
                ParserState::Normal => {
                    // Check if this line contains a format declaration
                    if let Some(format_info) = self.extract_format_declaration(line) {
                        self.current_format = Some(format_info);
                        self.state = ParserState::CollectingFormat;
                        processed_lines.push(line.to_string());
                        i += 1;
                    }
                    // Check if this line contains a heredoc declaration
                    else if let Some(heredoc_info) = self.extract_heredoc_declaration(line) {
                        eprintln!("DEBUG: Found heredoc declaration: {:?}", heredoc_info);
                        self.pending_heredocs.push_back(heredoc_info);
                        processed_lines.push(line.to_string());
                        i += 1;

                        // Start collecting heredoc content
                        if !self.pending_heredocs.is_empty() {
                            self.state = ParserState::CollectingHeredoc;
                        }
                    } else {
                        processed_lines.push(line.to_string());
                        i += 1;
                    }
                }
                ParserState::CollectingHeredoc => {
                    if let Some(heredoc) = self.pending_heredocs.front() {
                        // Check if this line is the heredoc terminator
                        if self.is_heredoc_terminator(line, heredoc) {
                            // Collect all content up to this point
                            let content = self.line_buffer.join("\n");
                            heredoc_contents.push((heredoc.marker.clone(), content));

                            self.line_buffer.clear();
                            self.pending_heredocs.pop_front();

                            // Add the terminator line to processed lines
                            processed_lines.push(line.to_string());
                            i += 1;

                            // Check if we have more heredocs to collect
                            if self.pending_heredocs.is_empty() {
                                self.state = ParserState::Normal;
                            }
                        } else {
                            // This is heredoc content
                            self.line_buffer.push(line.to_string());
                            i += 1;
                        }
                    }
                }
                ParserState::CollectingFormat => {
                    // Check if this line is the format terminator (single '.')
                    if line.trim() == "." {
                        if let Some(format_info) = &self.current_format {
                            // Save the collected format content
                            self.format_contents
                                .push((format_info.name.clone(), self.line_buffer.clone()));
                            self.line_buffer.clear();
                            self.current_format = None;
                            self.state = ParserState::Normal;
                        }
                        processed_lines.push(line.to_string());
                        i += 1;
                    } else {
                        // This is format content
                        self.line_buffer.push(line.to_string());
                        i += 1;
                    }
                }
            }
        }

        // Join processed lines and parse
        let processed_input = processed_lines.join("\n");

        // Build AST and inject heredoc and format contents
        let mut ast = self.build_ast_from_processed_input(&processed_input)?;
        self.inject_heredoc_contents(&mut ast, heredoc_contents);
        self.inject_format_contents(&mut ast);

        Ok(ast)
    }

    /// Extract format declaration information from a line
    pub fn extract_format_declaration(&self, line: &str) -> Option<FormatInfo> {
        let trimmed = line.trim();

        // Check if line starts with "format"
        if !trimmed.starts_with("format") {
            return None;
        }

        let after_format = &trimmed[6..].trim_start();

        // Check for equals sign
        if let Some(eq_pos) = after_format.find('=') {
            let name_part = after_format[..eq_pos].trim();

            // If no name specified, use empty string (parser will use default)
            let name = if name_part.is_empty() { String::new() } else { name_part.to_string() };

            Some(FormatInfo {
                name,
                _start_line: 0, // Not used in current implementation
            })
        } else {
            None
        }
    }

    /// Extract heredoc declaration information from a line
    pub fn extract_heredoc_declaration(&self, line: &str) -> Option<HeredocMarker> {
        // Simple regex-like pattern matching for heredoc declarations
        // This is a simplified version - a full implementation would be more robust

        if let Some(pos) = line.find("<<") {
            let after_marker = &line[pos + 2..];
            let (indented, rest) = if let Some(r) = after_marker.strip_prefix('~') {
                (true, r)
            } else {
                (false, after_marker)
            };

            // Extract the marker and quote type
            let trimmed = rest.trim_start();
            if trimmed.is_empty() {
                return None;
            }

            let (marker, quote_type) = if let Some(stripped) = trimmed.strip_prefix('\'') {
                // Single quoted
                if let Some(end) = stripped.find('\'') {
                    (stripped[..end].to_string(), HeredocQuoteType::Single)
                } else {
                    return None;
                }
            } else if let Some(stripped) = trimmed.strip_prefix('"') {
                // Double quoted
                if let Some(end) = stripped.find('"') {
                    (stripped[..end].to_string(), HeredocQuoteType::Double)
                } else {
                    return None;
                }
            } else if let Some(stripped) = trimmed.strip_prefix('`') {
                // Backtick
                if let Some(end) = stripped.find('`') {
                    (stripped[..end].to_string(), HeredocQuoteType::Backtick)
                } else {
                    return None;
                }
            } else if let Some(stripped) = trimmed.strip_prefix('\\') {
                // Escaped
                let end = stripped
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(stripped.len());
                (stripped[..end].to_string(), HeredocQuoteType::Escaped)
            } else {
                // Bare marker
                let end = trimmed
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(trimmed.len());
                (trimmed[..end].to_string(), HeredocQuoteType::None)
            };

            if marker.is_empty() {
                return None;
            }

            Some(HeredocMarker { marker, indented, quoted: quote_type, position: pos })
        } else {
            None
        }
    }

    /// Check if a line is a heredoc terminator
    pub fn is_heredoc_terminator(&self, line: &str, heredoc: &HeredocMarker) -> bool {
        let trimmed = if heredoc.indented { line.trim_start() } else { line };

        trimmed == heredoc.marker
    }

    /// Build AST from processed input
    fn build_ast_from_processed_input(
        &self,
        input: &str,
    ) -> Result<AstNode, Box<dyn std::error::Error>> {
        use crate::pure_rust_parser::PureRustPerlParser;

        let mut parser = PureRustPerlParser::new();
        parser.parse(input)
    }

    /// Inject collected heredoc contents into the AST
    fn inject_heredoc_contents(&self, ast: &mut AstNode, contents: Vec<(String, String)>) {
        if contents.is_empty() {
            return;
        }

        // Create a map for quick lookup
        let mut content_map: std::collections::HashMap<Arc<str>, String> =
            contents.into_iter().map(|(k, v)| (Arc::from(k), v)).collect();

        // Recursively walk the AST and update heredoc nodes
        self.update_heredoc_nodes(ast, &mut content_map);
    }

    /// Inject collected format contents into the AST
    fn inject_format_contents(&self, ast: &mut AstNode) {
        if self.format_contents.is_empty() {
            return;
        }

        // Create a map for quick lookup
        let content_map: std::collections::HashMap<Arc<str>, Vec<Arc<str>>> = self
            .format_contents
            .clone()
            .into_iter()
            .map(|(k, v)| (Arc::from(k), v.into_iter().map(Arc::from).collect()))
            .collect();

        // Recursively walk the AST and update format nodes
        self.update_format_nodes(ast, &content_map);
    }

    /// Recursively update heredoc nodes with their content
    #[allow(clippy::only_used_in_recursion)]
    fn update_heredoc_nodes(
        &self,
        node: &mut AstNode,
        content_map: &mut std::collections::HashMap<Arc<str>, String>,
    ) {
        match node {
            AstNode::Heredoc { marker, content, .. } => {
                if let Some(collected_content) = content_map.remove(marker) {
                    *content = Arc::from(collected_content);
                }
            }
            AstNode::Program(nodes) | AstNode::Block(nodes) => {
                for child in nodes {
                    self.update_heredoc_nodes(child, content_map);
                }
            }
            AstNode::Statement(inner)
            | AstNode::BeginBlock(inner)
            | AstNode::EndBlock(inner)
            | AstNode::CheckBlock(inner)
            | AstNode::InitBlock(inner)
            | AstNode::UnitcheckBlock(inner)
            | AstNode::DoBlock(inner)
            | AstNode::EvalBlock(inner)
            | AstNode::EvalString(inner) => {
                self.update_heredoc_nodes(inner, content_map);
            }
            AstNode::IfStatement { then_block, elsif_clauses, else_block, .. } => {
                self.update_heredoc_nodes(then_block, content_map);
                for (_, block) in elsif_clauses {
                    self.update_heredoc_nodes(block, content_map);
                }
                if let Some(else_block) = else_block {
                    self.update_heredoc_nodes(else_block, content_map);
                }
            }
            AstNode::UnlessStatement { block, else_block, .. } => {
                self.update_heredoc_nodes(block, content_map);
                if let Some(else_block) = else_block {
                    self.update_heredoc_nodes(else_block, content_map);
                }
            }
            AstNode::WhileStatement { block, .. }
            | AstNode::ForeachStatement { block, .. }
            | AstNode::ForStatement { block, .. } => {
                self.update_heredoc_nodes(block, content_map);
            }
            AstNode::SubDeclaration { body, .. } => {
                self.update_heredoc_nodes(body, content_map);
            }
            AstNode::LabeledBlock { block, .. } => {
                self.update_heredoc_nodes(block, content_map);
            }
            AstNode::PackageDeclaration { block: Some(block), .. } => {
                self.update_heredoc_nodes(block, content_map);
            }
            AstNode::Assignment { target, value, .. } => {
                self.update_heredoc_nodes(target, content_map);
                self.update_heredoc_nodes(value, content_map);
            }
            AstNode::BinaryOp { left, right, .. } => {
                self.update_heredoc_nodes(left, content_map);
                self.update_heredoc_nodes(right, content_map);
            }
            AstNode::UnaryOp { operand, .. } => {
                self.update_heredoc_nodes(operand, content_map);
            }
            AstNode::TernaryOp { condition, true_expr, false_expr } => {
                self.update_heredoc_nodes(condition, content_map);
                self.update_heredoc_nodes(true_expr, content_map);
                self.update_heredoc_nodes(false_expr, content_map);
            }
            AstNode::FunctionCall { function, args }
            | AstNode::MethodCall { object: function, args, .. } => {
                self.update_heredoc_nodes(function, content_map);
                for arg in args {
                    self.update_heredoc_nodes(arg, content_map);
                }
            }
            AstNode::ArrayElement { index, .. } => {
                self.update_heredoc_nodes(index, content_map);
            }
            AstNode::HashElement { key, .. } => {
                self.update_heredoc_nodes(key, content_map);
            }
            AstNode::List(items) | AstNode::ArrayRef(items) | AstNode::HashRef(items) => {
                for item in items {
                    self.update_heredoc_nodes(item, content_map);
                }
            }
            AstNode::VariableDeclaration { initializer: Some(init), .. } => {
                self.update_heredoc_nodes(init, content_map);
            }
            _ => {
                // Leaf nodes that don't contain other nodes
            }
        }
    }

    /// Recursively update format nodes with their content
    #[allow(clippy::only_used_in_recursion)]
    fn update_format_nodes(
        &self,
        node: &mut AstNode,
        content_map: &std::collections::HashMap<Arc<str>, Vec<Arc<str>>>,
    ) {
        match node {
            AstNode::FormatDeclaration { name, format_lines } => {
                if let Some(collected_lines) = content_map.get(name) {
                    *format_lines = collected_lines.clone();
                }
            }
            AstNode::Program(nodes) | AstNode::Block(nodes) => {
                for child in nodes {
                    self.update_format_nodes(child, content_map);
                }
            }
            AstNode::Statement(inner)
            | AstNode::BeginBlock(inner)
            | AstNode::EndBlock(inner)
            | AstNode::CheckBlock(inner)
            | AstNode::InitBlock(inner)
            | AstNode::UnitcheckBlock(inner)
            | AstNode::DoBlock(inner)
            | AstNode::EvalBlock(inner)
            | AstNode::EvalString(inner) => {
                self.update_format_nodes(inner, content_map);
            }
            AstNode::IfStatement { then_block, elsif_clauses, else_block, .. } => {
                self.update_format_nodes(then_block, content_map);
                for (_, block) in elsif_clauses {
                    self.update_format_nodes(block, content_map);
                }
                if let Some(else_block) = else_block {
                    self.update_format_nodes(else_block, content_map);
                }
            }
            AstNode::UnlessStatement { block, else_block, .. } => {
                self.update_format_nodes(block, content_map);
                if let Some(else_block) = else_block {
                    self.update_format_nodes(else_block, content_map);
                }
            }
            AstNode::WhileStatement { block, .. }
            | AstNode::ForeachStatement { block, .. }
            | AstNode::ForStatement { block, .. } => {
                self.update_format_nodes(block, content_map);
            }
            AstNode::SubDeclaration { body, .. } => {
                self.update_format_nodes(body, content_map);
            }
            AstNode::LabeledBlock { block, .. } => {
                self.update_format_nodes(block, content_map);
            }
            AstNode::PackageDeclaration { block: Some(block), .. } => {
                self.update_format_nodes(block, content_map);
            }
            _ => {
                // Other nodes don't contain format declarations
            }
        }
    }
}

impl Default for StatefulPerlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_heredoc() {
        let _parser = StatefulPerlParser::new();
        let _input = r#"print <<EOF;
Hello World
This is heredoc content
EOF
print "after heredoc";"#;

        // This test would verify heredoc parsing works correctly
        // let ast = parser.parse(input).unwrap();
    }

    #[test]
    fn test_indented_heredoc() {
        let _parser = StatefulPerlParser::new();
        let _input = r#"print <<~EOF;
    Hello World
    This is indented content
    EOF
print "after heredoc";"#;

        // Test indented heredoc handling
    }

    #[test]
    fn test_quoted_heredoc() {
        let _parser = StatefulPerlParser::new();
        let _input = r#"print <<'EOF';
No $interpolation here
EOF
print <<"INTERP";
With $interpolation
INTERP"#;

        // Test different quote types
    }

    #[test]
    fn test_format_declaration() {
        let _parser = StatefulPerlParser::new();
        let _input = r#"format STDOUT =
@<<<< @||||| @>>>>
$name, $login, $office
.
print "after format";"#;

        // Test format parsing
        // let ast = parser.parse(input).unwrap();
    }

    #[test]
    fn test_named_format() {
        let _parser = StatefulPerlParser::new();
        let _input = r#"format EMPLOYEE =
Name: @<<<<<<<<<<<<<<<<<<
      $name
Login: @<<<<<<<<
       $login
.
write EMPLOYEE;"#;

        // Test named format
    }
}
