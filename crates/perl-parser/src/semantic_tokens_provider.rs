//! Semantic tokens provider for enhanced syntax highlighting
//!
//! This module provides semantic token information to enable richer
//! syntax highlighting based on semantic understanding of the code.

use crate::ast::{Node, NodeKind};
use std::collections::HashMap;

/// Token types supported by the semantic tokens provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenType {
    Namespace, // package names
    Class,     // class names (modern Perl)
    Function,  // subroutine names
    Method,    // method names
    Variable,  // all variables
    Parameter, // function parameters
    Property,  // object properties/attributes
    Keyword,   // language keywords
    Comment,   // comments
    String,    // string literals
    Number,    // numeric literals
    Regexp,    // regular expressions
    Operator,  // operators
    Macro,     // constants/macros
}

impl SemanticTokenType {
    /// Get the string representation for LSP
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Namespace => "namespace",
            Self::Class => "class",
            Self::Function => "function",
            Self::Method => "method",
            Self::Variable => "variable",
            Self::Parameter => "parameter",
            Self::Property => "property",
            Self::Keyword => "keyword",
            Self::Comment => "comment",
            Self::String => "string",
            Self::Number => "number",
            Self::Regexp => "regexp",
            Self::Operator => "operator",
            Self::Macro => "macro",
        }
    }

    /// Get all token types in order
    pub fn all() -> Vec<Self> {
        vec![
            Self::Namespace,
            Self::Class,
            Self::Function,
            Self::Method,
            Self::Variable,
            Self::Parameter,
            Self::Property,
            Self::Keyword,
            Self::Comment,
            Self::String,
            Self::Number,
            Self::Regexp,
            Self::Operator,
            Self::Macro,
        ]
    }
}

/// Token modifiers that can be applied to token types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenModifier {
    Declaration,    // definition site
    Definition,     // same as declaration
    Reference,      // usage site
    Modification,   // being modified
    Static,         // package-level
    DefaultLibrary, // built-in
    Async,          // async operations
    Readonly,       // constants
    Deprecated,     // deprecated items
}

impl SemanticTokenModifier {
    /// Get the string representation for LSP
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Declaration => "declaration",
            Self::Definition => "definition",
            Self::Reference => "reference",
            Self::Modification => "modification",
            Self::Static => "static",
            Self::DefaultLibrary => "defaultLibrary",
            Self::Async => "async",
            Self::Readonly => "readonly",
            Self::Deprecated => "deprecated",
        }
    }

    /// Get all modifiers in order
    pub fn all() -> Vec<Self> {
        vec![
            Self::Declaration,
            Self::Definition,
            Self::Reference,
            Self::Modification,
            Self::Static,
            Self::DefaultLibrary,
            Self::Async,
            Self::Readonly,
            Self::Deprecated,
        ]
    }
}

/// A semantic token with position and type information
#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub line: u32,
    pub start_char: u32,
    pub length: u32,
    pub token_type: SemanticTokenType,
    pub modifiers: Vec<SemanticTokenModifier>,
}

/// Provider for semantic tokens
pub struct SemanticTokensProvider {
    source: String,
    /// Cache of variable declarations for scope tracking
    declared_vars: HashMap<String, Vec<(u32, u32)>>, // name -> [(line, col)]
}

impl SemanticTokensProvider {
    /// Create a new semantic tokens provider
    pub fn new(source: String) -> Self {
        Self {
            source,
            declared_vars: HashMap::new(),
        }
    }

    /// Extract semantic tokens from the AST
    pub fn extract(&mut self, ast: &Node) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();

        // Handle Program node specially
        if let NodeKind::Program { statements } = &ast.kind {
            for stmt in statements {
                self.visit_node(stmt, &mut tokens, false);
            }
        } else {
            self.visit_node(ast, &mut tokens, false);
        }

        // Sort tokens by position
        tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));

        tokens
    }

    /// Visit a node and extract semantic tokens
    fn visit_node(
        &mut self,
        node: &Node,
        tokens: &mut Vec<SemanticToken>,
        is_declaration_context: bool,
    ) {
        match &node.kind {
            NodeKind::Package { name, block } => {
                // Package name is a namespace
                self.add_token_from_string(
                    name,
                    SemanticTokenType::Namespace,
                    vec![SemanticTokenModifier::Declaration],
                    tokens,
                    node,
                );

                // Visit block
                if let Some(block) = block {
                    self.visit_node(block, tokens, false);
                }
            }

            NodeKind::Subroutine {
                name, params, body, ..
            } => {
                // Function name
                if let Some(name_str) = name {
                    let modifiers = vec![
                        SemanticTokenModifier::Declaration,
                        SemanticTokenModifier::Definition,
                    ];
                    self.add_token_from_string(
                        name_str,
                        SemanticTokenType::Function,
                        modifiers,
                        tokens,
                        node,
                    );
                }

                // Parameters
                for param in params {
                    self.visit_node(param, tokens, true);
                }

                // Body
                self.visit_node(body, tokens, false);
            }

            NodeKind::Variable { sigil: _, name: _ } => {
                let modifiers = if is_declaration_context {
                    vec![SemanticTokenModifier::Modification]
                } else {
                    vec![SemanticTokenModifier::Reference]
                };

                self.add_token(node, SemanticTokenType::Variable, modifiers, tokens);
            }

            NodeKind::VariableDeclaration { variable, .. } => {
                // Track declaration
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let pos = self.get_position(variable);
                    self.declared_vars
                        .entry(format!("{}{}", sigil, name))
                        .or_default()
                        .push((pos.0, pos.1));
                }

                // Mark as declaration
                self.add_token(
                    variable,
                    SemanticTokenType::Variable,
                    vec![SemanticTokenModifier::Declaration],
                    tokens,
                );
            }

            NodeKind::String { .. } => {
                self.add_token(node, SemanticTokenType::String, vec![], tokens);
            }

            NodeKind::Number { .. } => {
                self.add_token(node, SemanticTokenType::Number, vec![], tokens);
            }

            NodeKind::Regex { .. } => {
                self.add_token(node, SemanticTokenType::Regexp, vec![], tokens);
            }

            NodeKind::MethodCall {
                object,
                method,
                args,
            } => {
                // Object is a variable reference
                self.visit_node(object, tokens, false);

                // Method name
                self.add_token_from_string(
                    method,
                    SemanticTokenType::Method,
                    vec![SemanticTokenModifier::Reference],
                    tokens,
                    node,
                );

                // Arguments
                for arg in args {
                    self.visit_node(arg, tokens, false);
                }
            }

            NodeKind::FunctionCall { name, args } => {
                // Check if it's a built-in function
                let modifiers = if self.is_builtin_function(name) {
                    vec![
                        SemanticTokenModifier::DefaultLibrary,
                        SemanticTokenModifier::Reference,
                    ]
                } else {
                    vec![SemanticTokenModifier::Reference]
                };

                self.add_token_from_string(
                    name,
                    SemanticTokenType::Function,
                    modifiers,
                    tokens,
                    node,
                );

                // Arguments
                for arg in args {
                    self.visit_node(arg, tokens, false);
                }
            }

            // Comments are handled in trivia, not as nodes
            NodeKind::Use { module, .. } => {
                // Module name is a namespace
                self.add_token_from_string(
                    module,
                    SemanticTokenType::Namespace,
                    vec![SemanticTokenModifier::Reference],
                    tokens,
                    node,
                );
            }

            // Constants are handled differently in this AST
            NodeKind::Assignment { lhs, rhs, .. } => {
                // LHS is in modification context
                self.visit_node(lhs, tokens, true);

                // RHS is normal context
                self.visit_node(rhs, tokens, false);
            }

            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    self.visit_node(elem, tokens, is_declaration_context);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, tokens, false);
                }
            }

            _ => {
                // Visit children for other node types
                self.visit_children(node, tokens, is_declaration_context);
            }
        }
    }

    /// Add a token from a string with position from parent node
    fn add_token_from_string(
        &self,
        name: &str,
        token_type: SemanticTokenType,
        modifiers: Vec<SemanticTokenModifier>,
        tokens: &mut Vec<SemanticToken>,
        parent_node: &Node,
    ) {
        let (line, start_char) = self.get_position(parent_node);
        let length = name.len() as u32;

        tokens.push(SemanticToken {
            line,
            start_char,
            length,
            token_type,
            modifiers,
        });
    }

    /// Check if a function name is a built-in
    fn is_builtin_function(&self, name: &str) -> bool {
        // Common Perl built-in functions
        matches!(
            name,
            "print"
                | "say"
                | "open"
                | "close"
                | "read"
                | "write"
                | "push"
                | "pop"
                | "shift"
                | "unshift"
                | "grep"
                | "map"
                | "sort"
                | "reverse"
                | "join"
                | "split"
                | "substr"
                | "length"
                | "chomp"
                | "chop"
                | "lc"
                | "uc"
                | "index"
                | "rindex"
                | "die"
                | "warn"
                | "eval"
                | "require"
                | "use"
                | "package"
        )
    }

    /// Add a semantic token
    fn add_token(
        &self,
        node: &Node,
        token_type: SemanticTokenType,
        modifiers: Vec<SemanticTokenModifier>,
        tokens: &mut Vec<SemanticToken>,
    ) {
        let (line, start_char) = self.get_position(node);
        let length = self.get_length(node);

        tokens.push(SemanticToken {
            line,
            start_char,
            length,
            token_type,
            modifiers,
        });
    }

    /// Get the position of a node
    fn get_position(&self, node: &Node) -> (u32, u32) {
        let byte_offset = node.location.start;
        let mut line = 0;
        let mut col = 0;

        for (i, ch) in self.source.char_indices() {
            if i >= byte_offset {
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

    /// Get the length of a node in characters
    fn get_length(&self, node: &Node) -> u32 {
        let start = node.location.start;
        let end = node.location.end;

        self.source[start..end].chars().count() as u32
    }

    /// Visit all children generically
    fn visit_children(
        &mut self,
        node: &Node,
        tokens: &mut Vec<SemanticToken>,
        is_declaration_context: bool,
    ) {
        match &node.kind {
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, tokens, false);
                }
            }
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    self.visit_node(elem, tokens, is_declaration_context);
                }
            }
            _ => {}
        }
    }
}

/// Convert semantic tokens to LSP format (delta encoding)
pub fn encode_semantic_tokens(tokens: &[SemanticToken]) -> Vec<u32> {
    let mut encoded = Vec::new();
    let mut prev_line = 0;
    let mut prev_start = 0;

    for token in tokens {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start_char - prev_start
        } else {
            token.start_char
        };

        // Encode token type index
        let token_type_index = SemanticTokenType::all()
            .iter()
            .position(|&t| t == token.token_type)
            .unwrap() as u32;

        // Encode modifiers as bit flags
        let mut modifier_bits = 0u32;
        for modifier in &token.modifiers {
            let modifier_index = SemanticTokenModifier::all()
                .iter()
                .position(|&m| m == *modifier)
                .unwrap();
            modifier_bits |= 1 << modifier_index;
        }

        // Delta line
        encoded.push(delta_line);
        // Delta start character
        encoded.push(delta_start);
        // Token length
        encoded.push(token.length);
        // Token type
        encoded.push(token_type_index);
        // Token modifiers
        encoded.push(modifier_bits);

        prev_line = token.line;
        prev_start = token.start_char;
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_semantic_tokens_basic() {
        let code = r#"
package MyPackage;

my $var = 42;
sub test_function {
    my ($param) = @_;
    print $param;
}
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let mut provider = SemanticTokensProvider::new(code.to_string());
            let tokens = provider.extract(&ast);

            // Should have tokens for package, variable, function, parameter
            assert!(tokens.len() >= 5);

            // Check package token
            let pkg_token = tokens
                .iter()
                .find(|t| t.token_type == SemanticTokenType::Namespace);
            assert!(pkg_token.is_some());

            // Check function token
            let func_token = tokens
                .iter()
                .find(|t| t.token_type == SemanticTokenType::Function);
            assert!(func_token.is_some());
        }
    }

    #[test]
    fn test_semantic_token_encoding() {
        let tokens = vec![
            SemanticToken {
                line: 1,
                start_char: 0,
                length: 7,
                token_type: SemanticTokenType::Namespace,
                modifiers: vec![SemanticTokenModifier::Declaration],
            },
            SemanticToken {
                line: 3,
                start_char: 3,
                length: 4,
                token_type: SemanticTokenType::Variable,
                modifiers: vec![SemanticTokenModifier::Declaration],
            },
        ];

        let encoded = encode_semantic_tokens(&tokens);

        // First token: line 1, char 0, length 7, type 0 (Namespace), modifier 1 (Declaration)
        assert_eq!(encoded[0], 1); // delta line
        assert_eq!(encoded[1], 0); // delta start
        assert_eq!(encoded[2], 7); // length
        assert_eq!(encoded[3], 0); // type index
        assert_eq!(encoded[4], 1); // modifier bits

        // Second token: line 3, char 3, length 4, type 4 (Variable), modifier 1 (Declaration)
        assert_eq!(encoded[5], 2); // delta line (3-1)
        assert_eq!(encoded[6], 3); // start (new line, so absolute)
        assert_eq!(encoded[7], 4); // length
        assert_eq!(encoded[8], 4); // type index
        assert_eq!(encoded[9], 1); // modifier bits
    }
}
