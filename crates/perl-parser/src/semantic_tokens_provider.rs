//! Semantic tokens provider for enhanced syntax highlighting
//!
//! This module provides semantic token information to enable richer
//! syntax highlighting based on semantic understanding of the code.
//!
//! ## Features
//!
//! - **Comprehensive token types**: Support for packages, classes, functions, methods,
//!   variables, parameters, built-ins, constants, and more
//! - **Advanced error recovery**: Continues processing even with syntax errors
//! - **Performance optimizations**: Pre-computed line starts and efficient position calculation
//! - **Unicode support**: Proper handling of UTF-8 text and Unicode identifiers
//! - **Modifier support**: Distinguishes between declarations, references, modifications
//! - **Built-in detection**: Recognizes Perl built-in functions and pragmas
//!
//! ## Usage
//!
//! ```rust
//! use perl_parser::{Parser, semantic_tokens_provider::SemanticTokensProvider};
//!
//! let code = "package MyPkg; my $var = 42; sub func { return $var; }";
//! let mut parser = Parser::new(code);
//! 
//! if let Ok(ast) = parser.parse() {
//!     let mut provider = SemanticTokensProvider::new(code.to_string());
//!     let tokens = provider.extract(&ast);
//!     println!("Generated {} tokens", tokens.len());
//! }
//! ```

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
    /// Pre-computed line starts for faster position calculation
    line_starts: Vec<usize>,
    /// Performance metrics
    pub stats: ProviderStats,
}

/// Performance statistics for the semantic tokens provider
#[derive(Debug, Default)]
pub struct ProviderStats {
    pub nodes_processed: usize,
    pub tokens_generated: usize,
    pub processing_time_ms: u64,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl SemanticTokensProvider {
    /// Create a new semantic tokens provider
    pub fn new(source: String) -> Self {
        let line_starts = Self::compute_line_starts(&source);
        Self { 
            source, 
            declared_vars: HashMap::new(),
            line_starts,
            stats: ProviderStats::default(),
        }
    }

    /// Pre-compute line start positions for O(1) position lookup
    fn compute_line_starts(source: &str) -> Vec<usize> {
        let mut line_starts = vec![0]; // First line starts at 0
        
        for (pos, ch) in source.char_indices() {
            if ch == '\n' {
                line_starts.push(pos + 1);
            }
        }
        
        line_starts
    }

    /// Extract semantic tokens from the AST with error recovery
    pub fn extract(&mut self, ast: &Node) -> Vec<SemanticToken> {
        let start_time = std::time::Instant::now();
        let mut tokens = Vec::new();

        // Reset stats and caches
        self.stats = ProviderStats::default();
        self.declared_vars.clear();

        // Extract tokens with error handling
        match self.extract_with_recovery(ast, &mut tokens) {
            Ok(_) => {
                // Sort tokens by position for proper LSP encoding
                tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));
                
                // Update stats
                self.stats.processing_time_ms = start_time.elapsed().as_millis() as u64;
                self.stats.tokens_generated = tokens.len();
                
                tokens
            }
            Err(e) => {
                eprintln!("Warning: Error during semantic token extraction: {}", e);
                // Return whatever tokens we managed to extract
                tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));
                
                // Update stats even on error
                self.stats.processing_time_ms = start_time.elapsed().as_millis() as u64;
                self.stats.tokens_generated = tokens.len();
                
                tokens
            }
        }
    }

    /// Extract tokens with error recovery
    fn extract_with_recovery(&mut self, ast: &Node, tokens: &mut Vec<SemanticToken>) -> Result<(), String> {
        // Handle Program node specially
        if let NodeKind::Program { statements } = &ast.kind {
            for stmt in statements {
                if let Err(e) = self.visit_node_safe(stmt, tokens, false) {
                    eprintln!("Warning: Failed to process statement: {}", e);
                    // Continue with next statement
                }
            }
        } else {
            self.visit_node_safe(ast, tokens, false)?;
        }

        Ok(())
    }

    /// Safe node visiting with error handling
    fn visit_node_safe(
        &mut self,
        node: &Node,
        tokens: &mut Vec<SemanticToken>,
        is_declaration_context: bool,
    ) -> Result<(), String> {
        // Validate node bounds
        if node.location.start > node.location.end {
            return Err(format!("Invalid node bounds: {} > {}", node.location.start, node.location.end));
        }

        if node.location.end > self.source.len() {
            return Err(format!("Node bounds exceed source length: {} > {}", node.location.end, self.source.len()));
        }

        // Process the node
        self.visit_node(node, tokens, is_declaration_context);
        Ok(())
    }

    /// Visit a node and extract semantic tokens
    fn visit_node(
        &mut self,
        node: &Node,
        tokens: &mut Vec<SemanticToken>,
        is_declaration_context: bool,
    ) {
        // Update statistics
        self.stats.nodes_processed += 1;
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

            NodeKind::Subroutine { name, params, body, .. } => {
                // Function name
                if let Some(name_str) = name {
                    let modifiers =
                        vec![SemanticTokenModifier::Declaration, SemanticTokenModifier::Definition];
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

            NodeKind::MethodCall { object, method, args } => {
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
                // Check if it's a built-in function or pragma
                let (token_type, modifiers) = if self.is_pragma(name) {
                    (SemanticTokenType::Keyword, vec![SemanticTokenModifier::DefaultLibrary])
                } else if self.is_builtin_function(name) {
                    (SemanticTokenType::Function, vec![SemanticTokenModifier::DefaultLibrary, SemanticTokenModifier::Reference])
                } else if self.is_constant(name) {
                    (SemanticTokenType::Macro, vec![SemanticTokenModifier::Readonly, SemanticTokenModifier::Reference])
                } else {
                    (SemanticTokenType::Function, vec![SemanticTokenModifier::Reference])
                };

                self.add_token_from_string(
                    name,
                    token_type,
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

            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    // Visit both key and value
                    self.visit_node(key, tokens, false);
                    self.visit_node(value, tokens, false);
                }
            }

            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left, tokens, false);
                self.visit_node(right, tokens, false);
            }

            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand, tokens, false);
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                self.visit_node(condition, tokens, false);
                self.visit_node(then_expr, tokens, false);
                self.visit_node(else_expr, tokens, false);
            }

            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.visit_node(condition, tokens, false);
                self.visit_node(then_branch, tokens, false);
                for (elsif_condition, elsif_body) in elsif_branches {
                    self.visit_node(elsif_condition, tokens, false);
                    self.visit_node(elsif_body, tokens, false);
                }
                if let Some(else_branch) = else_branch {
                    self.visit_node(else_branch, tokens, false);
                }
            }

            NodeKind::While { condition, body, .. } => {
                self.visit_node(condition, tokens, false);
                self.visit_node(body, tokens, false);
            }

            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(init) = init {
                    self.visit_node(init, tokens, false);
                }
                if let Some(condition) = condition {
                    self.visit_node(condition, tokens, false);
                }
                if let Some(update) = update {
                    self.visit_node(update, tokens, false);
                }
                self.visit_node(body, tokens, false);
            }

            NodeKind::Foreach { variable, list, body, .. } => {
                self.visit_node(variable, tokens, true); // Variable in foreach is implicitly declared
                self.visit_node(list, tokens, false);
                self.visit_node(body, tokens, false);
            }

            NodeKind::Given { expr, body } => {
                self.visit_node(expr, tokens, false);
                self.visit_node(body, tokens, false);
            }

            NodeKind::When { condition, body } => {
                self.visit_node(condition, tokens, false);
                self.visit_node(body, tokens, false);
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                self.visit_node(body, tokens, false);
                for (_, catch_body) in catch_blocks {
                    self.visit_node(catch_body, tokens, false);
                }
                if let Some(finally) = finally_block {
                    self.visit_node(finally, tokens, false);
                }
            }

            NodeKind::Class { name, body } => {
                // Class name
                self.add_token_from_string(
                    name,
                    SemanticTokenType::Class,
                    vec![SemanticTokenModifier::Declaration, SemanticTokenModifier::Definition],
                    tokens,
                    node,
                );

                // Class body
                self.visit_node(body, tokens, false);
            }

            NodeKind::Method { name, params, body } => {
                // Method name
                self.add_token_from_string(
                    name,
                    SemanticTokenType::Method,
                    vec![SemanticTokenModifier::Declaration, SemanticTokenModifier::Definition],
                    tokens,
                    node,
                );

                // Parameters
                for param in params {
                    self.visit_node(param, tokens, true);
                }

                // Body
                self.visit_node(body, tokens, false);
            }

            NodeKind::PhaseBlock { block, .. } => {
                self.visit_node(block, tokens, false);
            }

            NodeKind::Return { value } => {
                if let Some(value) = value {
                    self.visit_node(value, tokens, false);
                }
            }

            NodeKind::Match { expr, .. } | NodeKind::Substitution { expr, .. } | NodeKind::Transliteration { expr, .. } => {
                self.visit_node(expr, tokens, false);
            }

            NodeKind::IndirectCall { object, args, .. } => {
                self.visit_node(object, tokens, false);
                for arg in args {
                    self.visit_node(arg, tokens, false);
                }
            }

            NodeKind::StatementModifier { statement, condition, .. } => {
                self.visit_node(statement, tokens, false);
                self.visit_node(condition, tokens, false);
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

        tokens.push(SemanticToken { line, start_char, length, token_type, modifiers });
    }

    /// Check if a function name is a built-in
    fn is_builtin_function(&self, name: &str) -> bool {
        // Common Perl built-in functions
        matches!(
            name,
            "print"
                | "say"
                | "printf"
                | "sprintf"
                | "open"
                | "close"
                | "read"
                | "write"
                | "readline"
                | "getline"
                | "push"
                | "pop"
                | "shift"
                | "unshift"
                | "splice"
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
                | "lcfirst"
                | "ucfirst"
                | "index"
                | "rindex"
                | "die"
                | "warn"
                | "carp"
                | "croak"
                | "confess"
                | "cluck"
                | "eval"
                | "require"
                | "use"
                | "package"
                | "defined"
                | "exists"
                | "delete"
                | "keys"
                | "values"
                | "each"
                | "scalar"
                | "ref"
                | "blessed"
                | "isa"
                | "can"
                | "UNIVERSAL"
                | "bless"
                | "tie"
                | "tied"
                | "untie"
                | "caller"
                | "wantarray"
                | "return"
                | "goto"
                | "last"
                | "next"
                | "redo"
                | "exit"
                | "exec"
                | "system"
                | "fork"
                | "wait"
                | "waitpid"
                | "kill"
                | "sleep"
                | "alarm"
                | "time"
                | "localtime"
                | "gmtime"
                | "mktime"
                | "times"
                | "stat"
                | "lstat"
                | "filetest"
                | "glob"
                | "unlink"
                | "rename"
                | "link"
                | "symlink"
                | "readlink"
                | "mkdir"
                | "rmdir"
                | "opendir"
                | "readdir"
                | "closedir"
                | "seekdir"
                | "telldir"
                | "rewinddir"
                | "chmod"
                | "chown"
                | "chroot"
                | "umask"
                | "rand"
                | "srand"
                | "int"
                | "hex"
                | "oct"
                | "abs"
                | "atan2"
                | "cos"
                | "sin"
                | "exp"
                | "log"
                | "sqrt"
        )
    }

    /// Check if a name is a pragma
    fn is_pragma(&self, name: &str) -> bool {
        matches!(
            name,
            "strict" | "warnings" | "utf8" | "feature" | "autodie" | "constant" 
            | "base" | "parent" | "lib" | "vars" | "subs" | "integer" | "bytes"
            | "locale" | "encoding" | "open" | "charnames" | "diagnostics"
            | "sigtrap" | "sort" | "threads" | "threads::shared" | "version"
        )
    }

    /// Check if a name looks like a constant
    fn is_constant(&self, name: &str) -> bool {
        // Constants are typically ALL_CAPS or start with uppercase
        name.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
            && name.chars().any(|c| c.is_ascii_uppercase())
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

        tokens.push(SemanticToken { line, start_char, length, token_type, modifiers });
    }

    /// Get the position of a node with optimized line lookup
    fn get_position(&self, node: &Node) -> (u32, u32) {
        let byte_offset = node.location.start;
        
        // Handle boundary cases
        if byte_offset >= self.source.len() {
            return (0, 0);
        }

        // Binary search to find the line containing this offset
        let line = match self.line_starts.binary_search(&byte_offset) {
            Ok(line) => line,
            Err(line) => line.saturating_sub(1),
        };

        // Calculate column position from line start
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        let col = if line_start <= byte_offset {
            // Count characters from line start to byte offset
            let slice = &self.source[line_start..byte_offset];
            slice.chars().count()
        } else {
            0
        };

        (line as u32, col as u32)
    }

    /// Get the length of a node in characters with Unicode support
    fn get_length(&self, node: &Node) -> u32 {
        let start = node.location.start;
        let end = node.location.end.min(self.source.len());

        if start >= end {
            return 0;
        }

        // Safe byte slicing with UTF-8 boundary validation
        match self.source.get(start..end) {
            Some(slice) => slice.chars().count() as u32,
            None => {
                // Fallback: find valid UTF-8 boundaries
                let valid_start = self.find_utf8_boundary(start, true);
                let valid_end = self.find_utf8_boundary(end, false);
                
                if valid_start < valid_end {
                    self.source[valid_start..valid_end].chars().count() as u32
                } else {
                    0
                }
            }
        }
    }

    /// Find the nearest UTF-8 character boundary
    fn find_utf8_boundary(&self, pos: usize, search_backward: bool) -> usize {
        let bytes = self.source.as_bytes();
        let mut search_pos = pos.min(bytes.len());

        if search_backward {
            while search_pos > 0 && !self.source.is_char_boundary(search_pos) {
                search_pos -= 1;
            }
        } else {
            while search_pos < bytes.len() && !self.source.is_char_boundary(search_pos) {
                search_pos += 1;
            }
        }

        search_pos
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
        let delta_start =
            if delta_line == 0 { token.start_char - prev_start } else { token.start_char };

        // Encode token type index
        let token_type_index =
            SemanticTokenType::all().iter().position(|&t| t == token.token_type).unwrap() as u32;

        // Encode modifiers as bit flags
        let mut modifier_bits = 0u32;
        for modifier in &token.modifiers {
            let modifier_index =
                SemanticTokenModifier::all().iter().position(|&m| m == *modifier).unwrap();
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
            let pkg_token = tokens.iter().find(|t| t.token_type == SemanticTokenType::Namespace);
            assert!(pkg_token.is_some());

            // Check function token
            let func_token = tokens.iter().find(|t| t.token_type == SemanticTokenType::Function);
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
