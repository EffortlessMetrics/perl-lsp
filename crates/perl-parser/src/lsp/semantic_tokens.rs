//! Semantic tokens provider for enhanced syntax highlighting
//!
//! Provides semantic token information to enable richer syntax highlighting
//! based on semantic understanding rather than just lexical analysis.

use crate::{
    ast::{Node, NodeKind},
    lsp::{DocumentFeatureProvider, FeatureProvider},
};
use std::collections::HashMap;

/// Semantic token types (indices match the legend order)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenType {
    Namespace = 0,     // package
    Class = 1,         // class (Perl 5.38+)
    Enum = 2,          // not used in Perl
    Interface = 3,     // role
    Struct = 4,        // not used in Perl
    TypeParameter = 5, // not used in Perl
    Type = 6,          // type constraints
    Parameter = 7,     // subroutine parameters
    Variable = 8,      // $var, @array, %hash
    Property = 9,      // hash keys, object properties
    EnumMember = 10,   // not used in Perl
    Decorator = 11,    // attributes
    Event = 12,        // not used in Perl
    Function = 13,     // subroutines
    Method = 14,       // method (Perl 5.38+)
    Macro = 15,        // not used in Perl
    Label = 16,        // labels for goto/last/next
    Comment = 17,      // comments and POD
    String = 18,       // string literals
    Keyword = 19,      // my, our, sub, etc.
    Number = 20,       // numeric literals
    Regexp = 21,       // regular expressions
    Operator = 22,     // operators
}

/// Semantic token modifiers (bit flags)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenModifier {
    Declaration = 1 << 0,     // Variable declaration
    Definition = 1 << 1,      // Subroutine definition
    Readonly = 1 << 2,        // Constants
    Static = 1 << 3,          // Package variables
    Deprecated = 1 << 4,      // Deprecated features
    Abstract = 1 << 5,        // Abstract methods
    Async = 1 << 6,           // Async subroutines
    Modification = 1 << 7,    // Variable modification
    Documentation = 1 << 8,   // POD documentation
    DefaultLibrary = 1 << 9,  // Built-in functions
}

/// A semantic token
#[derive(Debug, Clone)]
pub struct SemanticToken {
    /// Line number (0-based)
    pub line: u32,
    /// Start character (0-based)
    pub start_char: u32,
    /// Length in characters
    pub length: u32,
    /// Token type
    pub token_type: SemanticTokenType,
    /// Token modifiers (bit flags)
    pub token_modifiers: u32,
}

/// Semantic tokens builder
pub struct SemanticTokensBuilder {
    tokens: Vec<SemanticToken>,
    source: String,
}

impl SemanticTokensBuilder {
    /// Create a new builder
    pub fn new(source: String) -> Self {
        Self {
            tokens: Vec::new(),
            source,
        }
    }
    
    /// Add a token
    pub fn push(&mut self, token: SemanticToken) {
        self.tokens.push(token);
    }
    
    /// Build the tokens into LSP format
    pub fn build(mut self) -> Vec<u32> {
        // Sort tokens by position
        self.tokens.sort_by(|a, b| {
            a.line.cmp(&b.line)
                .then_with(|| a.start_char.cmp(&b.start_char))
        });
        
        // Encode as deltas
        let mut data = Vec::new();
        let mut prev_line = 0;
        let mut prev_char = 0;
        
        for token in self.tokens {
            let delta_line = token.line - prev_line;
            let delta_start = if delta_line == 0 {
                token.start_char - prev_char
            } else {
                token.start_char
            };
            
            data.push(delta_line);
            data.push(delta_start);
            data.push(token.length);
            data.push(token.token_type as u32);
            data.push(token.token_modifiers);
            
            prev_line = token.line;
            prev_char = token.start_char;
        }
        
        data
    }
    
    /// Extract semantic tokens from AST
    pub fn extract_from_ast(&mut self, ast: &Node) {
        self.visit_node(ast, &mut Context::new());
    }
    
    /// Visit a node and extract tokens
    fn visit_node(&mut self, node: &Node, ctx: &mut Context) {
        match &node.kind {
            NodeKind::Package { name, block } => {
                // Package name is a namespace
                self.add_token_from_span(
                    node.span.start + 8, // Skip "package "
                    name.len(),
                    SemanticTokenType::Namespace,
                    SemanticTokenModifier::Declaration as u32,
                );
                
                ctx.current_package = Some(name.clone());
                if let Some(block) = block {
                    self.visit_node(block, ctx);
                }
            }
            
            NodeKind::Subroutine { name, params, body, .. } => {
                // Subroutine name
                let is_method = ctx.in_class || name.starts_with('_');
                self.add_token_from_span(
                    node.span.start + 4, // Skip "sub "
                    name.len(),
                    if is_method { SemanticTokenType::Method } else { SemanticTokenType::Function },
                    SemanticTokenModifier::Definition as u32,
                );
                
                // Parameters
                if let Some(params) = params {
                    for param in params {
                        self.visit_node(param, ctx);
                    }
                }
                
                // Body
                if let Some(body) = body {
                    self.visit_node(body, ctx);
                }
            }
            
            NodeKind::Variable { name, sigil } => {
                let token_type = SemanticTokenType::Variable;
                let mut modifiers = 0u32;
                
                // Check if it's a declaration
                if self.is_declaration(node, ctx) {
                    modifiers |= SemanticTokenModifier::Declaration as u32;
                }
                
                // Check if it's a package variable
                if ctx.is_package_var(name) {
                    modifiers |= SemanticTokenModifier::Static as u32;
                }
                
                self.add_token_from_span(
                    node.span.start,
                    sigil.len() + name.len(),
                    token_type,
                    modifiers,
                );
            }
            
            NodeKind::FunctionCall { name, args } => {
                // Check if it's a built-in function
                let modifiers = if is_builtin_function(name) {
                    SemanticTokenModifier::DefaultLibrary as u32
                } else {
                    0
                };
                
                self.add_token_from_span(
                    node.span.start,
                    name.len(),
                    SemanticTokenType::Function,
                    modifiers,
                );
                
                // Visit arguments
                for arg in args {
                    self.visit_node(arg, ctx);
                }
            }
            
            NodeKind::StringLiteral { .. } => {
                self.add_token_from_span(
                    node.span.start,
                    node.span.end - node.span.start,
                    SemanticTokenType::String,
                    0,
                );
            }
            
            NodeKind::NumberLiteral { .. } => {
                self.add_token_from_span(
                    node.span.start,
                    node.span.end - node.span.start,
                    SemanticTokenType::Number,
                    0,
                );
            }
            
            NodeKind::Regex { .. } => {
                self.add_token_from_span(
                    node.span.start,
                    node.span.end - node.span.start,
                    SemanticTokenType::Regexp,
                    0,
                );
            }
            
            NodeKind::Comment { .. } => {
                self.add_token_from_span(
                    node.span.start,
                    node.span.end - node.span.start,
                    SemanticTokenType::Comment,
                    0,
                );
            }
            
            // Handle other node types...
            _ => {
                // Visit children
                self.visit_children(node, ctx);
            }
        }
    }
    
    /// Visit all children of a node
    fn visit_children(&mut self, node: &Node, ctx: &mut Context) {
        match &node.kind {
            NodeKind::Program { statements } |
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, ctx);
                }
            }
            NodeKind::If { condition, then_branch, else_branch } => {
                self.visit_node(condition, ctx);
                self.visit_node(then_branch, ctx);
                if let Some(else_branch) = else_branch {
                    self.visit_node(else_branch, ctx);
                }
            }
            // Add more cases as needed...
            _ => {}
        }
    }
    
    /// Add a token from a span
    fn add_token_from_span(
        &mut self,
        start: usize,
        length: usize,
        token_type: SemanticTokenType,
        modifiers: u32,
    ) {
        let (line, start_char) = self.offset_to_position(start);
        
        self.tokens.push(SemanticToken {
            line,
            start_char,
            length: length as u32,
            token_type,
            token_modifiers: modifiers,
        });
    }
    
    /// Convert byte offset to line/character position
    fn offset_to_position(&self, offset: usize) -> (u32, u32) {
        let mut line = 0;
        let mut char_pos = 0;
        
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                char_pos = 0;
            } else {
                char_pos += 1;
            }
        }
        
        (line, char_pos)
    }
    
    /// Check if this is a variable declaration
    fn is_declaration(&self, node: &Node, ctx: &Context) -> bool {
        // Simple heuristic: check if preceded by my/our/local/state
        if node.span.start < 4 {
            return false;
        }
        
        let prefix_start = node.span.start.saturating_sub(4);
        let prefix = &self.source[prefix_start..node.span.start];
        
        prefix.ends_with("my ") || 
        prefix.ends_with("our ") || 
        prefix.ends_with("local ") || 
        prefix.ends_with("state ")
    }
}

/// Context for semantic analysis
struct Context {
    current_package: Option<String>,
    in_class: bool,
    package_vars: HashMap<String, bool>,
}

impl Context {
    fn new() -> Self {
        Self {
            current_package: None,
            in_class: false,
            package_vars: HashMap::new(),
        }
    }
    
    fn is_package_var(&self, name: &str) -> bool {
        self.package_vars.get(name).copied().unwrap_or(false)
    }
}

/// Check if a function is built-in
fn is_builtin_function(name: &str) -> bool {
    matches!(name,
        "print" | "say" | "die" | "warn" | "open" | "close" | "read" | "write" |
        "push" | "pop" | "shift" | "unshift" | "splice" | "sort" | "grep" | "map" |
        "join" | "split" | "substr" | "length" | "index" | "rindex" |
        "ref" | "defined" | "undef" | "exists" | "delete" | "keys" | "values" |
        "bless" | "tie" | "untie" | "tied" | "require" | "use" | "do" | "eval"
        // ... many more
    )
}

/// Semantic tokens provider
pub struct SemanticTokensProvider;

impl SemanticTokensProvider {
    pub fn new() -> Self {
        Self
    }
    
    /// Get full semantic tokens for a document
    pub fn get_tokens(&self, source: &str, ast: &Node) -> Vec<u32> {
        let mut builder = SemanticTokensBuilder::new(source.to_string());
        builder.extract_from_ast(ast);
        builder.build()
    }
}

impl FeatureProvider for SemanticTokensProvider {
    fn name(&self) -> &'static str {
        "semanticTokens"
    }
}

impl DocumentFeatureProvider for SemanticTokensProvider {
    fn process_document(&self, _uri: &str, _content: &str, _ast: &Node) -> Result<(), Box<dyn std::error::Error>> {
        // Semantic tokens are computed on demand, not cached
        Ok(())
    }
}