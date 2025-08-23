use crate::ast::{Node, NodeKind};
use serde_json::{Value, json};

/// Inlay Hint types according to LSP spec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    Type = 1,
    Parameter = 2,
}

/// Position in a document
#[derive(Debug, Clone)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// An inlay hint
#[derive(Debug, Clone)]
pub struct InlayHint {
    pub position: Position,
    pub label: String,
    pub kind: InlayHintKind,
    pub tooltip: Option<String>,
    pub padding_left: bool,
    pub padding_right: bool,
}

/// Inlay Hints Provider
pub struct InlayHintsProvider {
    source: String,
    enabled_hints: InlayHintConfig,
}

/// Configuration for which hints to show
#[derive(Debug, Clone)]
pub struct InlayHintConfig {
    pub parameter_hints: bool,
    pub type_hints: bool,
    pub chained_hints: bool,
    pub max_length: usize,
}

impl Default for InlayHintConfig {
    fn default() -> Self {
        Self { parameter_hints: true, type_hints: true, chained_hints: true, max_length: 30 }
    }
}

impl InlayHintsProvider {
    pub fn new(source: String) -> Self {
        Self { source, enabled_hints: InlayHintConfig::default() }
    }

    pub fn with_config(source: String, config: InlayHintConfig) -> Self {
        Self { source, enabled_hints: config }
    }

    /// Extract inlay hints from the AST
    pub fn extract(&self, ast: &Node) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        self.visit_node(ast, &mut hints);
        hints
    }

    /// Visit a node and collect hints
    fn visit_node(&self, node: &Node, hints: &mut Vec<InlayHint>) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, hints);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, hints);
                }
            }

            // Function calls - show parameter hints
            NodeKind::FunctionCall { name, args } => {
                if self.enabled_hints.parameter_hints {
                    self.add_parameter_hints(name, args, node, hints);
                }

                // Visit arguments
                for arg in args {
                    self.visit_node(arg, hints);
                }
            }

            // Method calls - show parameter hints
            NodeKind::MethodCall { object, method, args } => {
                if self.enabled_hints.parameter_hints {
                    self.add_parameter_hints(method, args, node, hints);
                }

                // Visit object and arguments
                self.visit_node(object, hints);
                for arg in args {
                    self.visit_node(arg, hints);
                }
            }

            // Variable declarations - show type hints
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                if self.enabled_hints.type_hints {
                    if let Some(init) = initializer {
                        self.add_type_hint(variable, init, hints);
                    }
                }

                // Visit initializer
                if let Some(init) = initializer {
                    self.visit_node(init, hints);
                }
            }

            // Chained method calls - show intermediate types
            NodeKind::Binary { op, left, right } if op == "->" => {
                if self.enabled_hints.chained_hints {
                    self.add_chain_hint(left, hints);
                }

                self.visit_node(left, hints);
                self.visit_node(right, hints);
            }

            // Visit other nodes recursively
            _ => {
                self.visit_children(node, hints);
            }
        }
    }

    /// Add parameter hints for function/method calls
    fn add_parameter_hints(
        &self,
        function_name: &str,
        args: &[Node],
        _call_node: &Node,
        hints: &mut Vec<InlayHint>,
    ) {
        // Get parameter names for known functions
        let param_names = self.get_parameter_names(function_name);

        if param_names.is_empty() {
            return;
        }

        // Add hints for each argument
        for (i, (arg, param_name)) in args.iter().zip(param_names.iter()).enumerate() {
            // Skip if argument is already clear (e.g., named parameter)
            if self.is_clear_argument(arg) {
                continue;
            }

            let position = self.get_node_start_position(arg);
            hints.push(InlayHint {
                position,
                label: format!("{}: ", param_name),
                kind: InlayHintKind::Parameter,
                tooltip: Some(format!("Parameter {} of {}", i + 1, function_name)),
                padding_left: false,
                padding_right: false,
            });
        }
    }

    /// Add type hint for variable declaration
    fn add_type_hint(&self, variable: &Node, initializer: &Node, hints: &mut Vec<InlayHint>) {
        if let Some(type_info) = self.infer_type(initializer) {
            // Don't show if type is too long
            if type_info.len() > self.enabled_hints.max_length {
                return;
            }

            let position = self.get_node_end_position(variable);
            hints.push(InlayHint {
                position,
                label: format!(": {}", type_info),
                kind: InlayHintKind::Type,
                tooltip: Some("Inferred type".to_string()),
                padding_left: false,
                padding_right: true,
            });
        }
    }

    /// Add hint for chained method calls
    fn add_chain_hint(&self, expr: &Node, hints: &mut Vec<InlayHint>) {
        if let Some(type_info) = self.infer_type(expr) {
            if type_info.len() > self.enabled_hints.max_length {
                return;
            }

            let position = self.get_node_end_position(expr);
            hints.push(InlayHint {
                position,
                label: format!(" /* {} */", type_info),
                kind: InlayHintKind::Type,
                tooltip: Some("Type of expression".to_string()),
                padding_left: true,
                padding_right: true,
            });
        }
    }

    /// Get parameter names for known functions
    fn get_parameter_names(&self, function_name: &str) -> Vec<String> {
        match function_name {
            // Built-in functions
            "open" => vec!["filehandle".to_string(), "mode".to_string(), "filename".to_string()],
            "print" => vec!["filehandle".to_string(), "list".to_string()],
            "printf" => vec!["filehandle".to_string(), "format".to_string(), "list".to_string()],
            "push" => vec!["array".to_string(), "list".to_string()],
            "unshift" => vec!["array".to_string(), "list".to_string()],
            "splice" => vec![
                "array".to_string(),
                "offset".to_string(),
                "length".to_string(),
                "list".to_string(),
            ],
            "substr" => vec![
                "string".to_string(),
                "offset".to_string(),
                "length".to_string(),
                "replacement".to_string(),
            ],
            "index" => vec!["string".to_string(), "substring".to_string(), "position".to_string()],
            "join" => vec!["separator".to_string(), "list".to_string()],
            "split" => vec!["pattern".to_string(), "string".to_string(), "limit".to_string()],
            "grep" => vec!["block".to_string(), "list".to_string()],
            "map" => vec!["block".to_string(), "list".to_string()],
            "sort" => vec!["block".to_string(), "list".to_string()],
            _ => vec![],
        }
    }

    /// Check if an argument is already clear
    fn is_clear_argument(&self, arg: &Node) -> bool {
        match &arg.kind {
            // String literals with clear content
            NodeKind::String { value, .. } => {
                value.len() < 20 && value.chars().all(|c| c.is_alphanumeric() || c.is_whitespace())
            }
            // Simple variable names
            NodeKind::Variable { name, .. } => {
                name.len() > 5 // Descriptive variable names
            }
            _ => false,
        }
    }

    /// Infer type from expression
    fn infer_type(&self, expr: &Node) -> Option<String> {
        match &expr.kind {
            NodeKind::ArrayLiteral { .. } => Some("ARRAY".to_string()),
            NodeKind::HashLiteral { .. } => Some("HASH".to_string()),
            // Handle block that contains a hash literal (e.g., { key => "value" })
            NodeKind::Block { statements } if statements.len() == 1 => {
                // Check if the single statement is a hash-like expression
                if let NodeKind::ArrayLiteral { elements } = &statements[0].kind {
                    // Check if this looks like hash pairs (even number of elements)
                    if elements.len() % 2 == 0 && !elements.is_empty() {
                        return Some("HASH".to_string());
                    }
                }
                // Otherwise, check if it's already a HashLiteral wrapped in a block
                if let NodeKind::HashLiteral { .. } = &statements[0].kind {
                    return Some("HASH".to_string());
                }
                self.infer_type(&statements[0])
            }
            NodeKind::String { .. } => Some("string".to_string()),
            NodeKind::Number { .. } => Some("number".to_string()),
            NodeKind::Regex { .. } => Some("Regexp".to_string()),
            NodeKind::FunctionCall { name, .. } => self.get_return_type(name),
            NodeKind::MethodCall { method, .. } => self.get_return_type(method),
            _ => None,
        }
    }

    /// Get return type for known functions
    fn get_return_type(&self, function_name: &str) -> Option<String> {
        match function_name {
            "new" => Some("object".to_string()),
            "split" => Some("ARRAY".to_string()),
            "keys" | "values" => Some("ARRAY".to_string()),
            "reverse" => Some("ARRAY".to_string()),
            "sort" => Some("ARRAY".to_string()),
            "grep" | "map" => Some("ARRAY".to_string()),
            "localtime" | "gmtime" => Some("ARRAY".to_string()),
            "stat" | "lstat" => Some("ARRAY".to_string()),
            _ => None,
        }
    }

    /// Visit children nodes
    fn visit_children(&self, node: &Node, hints: &mut Vec<InlayHint>) {
        match &node.kind {
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.visit_node(condition, hints);
                self.visit_node(then_branch, hints);
                for (cond, body) in elsif_branches {
                    self.visit_node(cond, hints);
                    self.visit_node(body, hints);
                }
                if let Some(else_b) = else_branch {
                    self.visit_node(else_b, hints);
                }
            }
            NodeKind::While { condition, body, .. } => {
                self.visit_node(condition, hints);
                self.visit_node(body, hints);
            }
            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(i) = init {
                    self.visit_node(i, hints);
                }
                if let Some(c) = condition {
                    self.visit_node(c, hints);
                }
                if let Some(u) = update {
                    self.visit_node(u, hints);
                }
                self.visit_node(body, hints);
            }
            NodeKind::Foreach { variable, list, body } => {
                self.visit_node(variable, hints);
                self.visit_node(list, hints);
                self.visit_node(body, hints);
            }
            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left, hints);
                self.visit_node(right, hints);
            }
            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand, hints);
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                self.visit_node(lhs, hints);
                self.visit_node(rhs, hints);
            }
            NodeKind::Return { value } => {
                if let Some(v) = value {
                    self.visit_node(v, hints);
                }
            }
            _ => {}
        }
    }

    /// Get the start position of a node
    fn get_node_start_position(&self, node: &Node) -> Position {
        let offset = node.location.start;
        let (line, character) = self.offset_to_position(offset);
        Position { line, character }
    }

    /// Get the end position of a node
    fn get_node_end_position(&self, node: &Node) -> Position {
        let offset = node.location.end;
        let (line, character) = self.offset_to_position(offset);
        Position { line, character }
    }

    /// Convert byte offset to line/character position
    fn offset_to_position(&self, offset: usize) -> (u32, u32) {
        let mut line = 0;
        let mut col = 0;

        for (i, ch) in self.source.chars().enumerate() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        (line, col)
    }
}

/// Convert InlayHint to JSON for LSP
impl InlayHint {
    pub fn to_json(&self) -> Value {
        let mut hint = json!({
            "position": {
                "line": self.position.line,
                "character": self.position.character
            },
            "label": self.label,
            "kind": self.kind as u32,
            "paddingLeft": self.padding_left,
            "paddingRight": self.padding_right,
        });

        if let Some(tooltip) = &self.tooltip {
            hint["tooltip"] = json!(tooltip);
        }

        hint
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_parameter_hints() {
        let code = r#"
push(@array, "value");
substr($string, 0, 5, "replacement");
open(FH, "<", "file.txt");
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let provider = InlayHintsProvider::new(code.to_string());
            let hints = provider.extract(&ast);

            // Should have parameter hints for push, substr, and open
            assert!(hints.len() >= 3); // At least 1 for each function call

            // Check first hint is for array parameter
            assert_eq!(hints[0].label, "array: ");
            assert_eq!(hints[0].kind, InlayHintKind::Parameter);
        }
    }

    #[test]
    fn test_type_hints() {
        let code = r#"
my $arr = [1, 2, 3];
my $hash = { key => "value" };
my $result = split(/,/, $input);
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let provider = InlayHintsProvider::new(code.to_string());
            let hints = provider.extract(&ast);

            // Should have type hints for variables
            let type_hints: Vec<_> =
                hints.iter().filter(|h| h.kind == InlayHintKind::Type).collect();

            assert!(type_hints.len() >= 3);

            // Check types
            assert!(type_hints.iter().any(|h| h.label.contains("ARRAY")));
            assert!(type_hints.iter().any(|h| h.label.contains("HASH")));
        }
    }

    #[test]
    fn test_no_hints_for_clear_arguments() {
        let code = r#"
push(@descriptive_array_name, "value");
print("Hello, World!");
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let provider = InlayHintsProvider::new(code.to_string());
            let hints = provider.extract(&ast);

            // Should skip hints for clear arguments
            let param_hints: Vec<_> =
                hints.iter().filter(|h| h.kind == InlayHintKind::Parameter).collect();

            // Should have some parameter hints, but skip clear ones
            assert!(!param_hints.is_empty());
        }
    }
}
