//! Semantic analysis for IDE features
//!
//! This module provides semantic analysis on top of the symbol table,
//! including semantic tokens for syntax highlighting, hover information,
//! and code intelligence features.

use crate::SourceLocation;
use crate::ast::{Node, NodeKind};
use crate::symbol::{ScopeId, Symbol, SymbolExtractor, SymbolKind, SymbolTable};
use std::collections::HashMap;

/// Semantic token types for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenType {
    // Variables
    Variable,
    VariableDeclaration,
    VariableReadonly,
    Parameter,

    // Functions
    Function,
    FunctionDeclaration,
    Method,

    // Types
    Class,
    Namespace,
    Type,

    // Keywords
    Keyword,
    KeywordControl,
    Modifier,

    // Literals
    Number,
    String,
    Regex,

    // Comments
    Comment,
    CommentDoc,

    // Other
    Operator,
    Punctuation,
    Label,
}

/// Semantic token modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenModifier {
    Declaration,
    Definition,
    Readonly,
    Static,
    Deprecated,
    Abstract,
    Async,
    Modification,
    Documentation,
    DefaultLibrary,
}

/// A semantic token with type and modifiers
#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub location: SourceLocation,
    pub token_type: SemanticTokenType,
    pub modifiers: Vec<SemanticTokenModifier>,
}

/// Hover information for a symbol
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// Symbol signature or declaration
    pub signature: String,
    /// Documentation
    pub documentation: Option<String>,
    /// Additional details
    pub details: Vec<String>,
}

/// Semantic analyzer that provides IDE features
#[derive(Debug)]
pub struct SemanticAnalyzer {
    symbol_table: SymbolTable,
    semantic_tokens: Vec<SemanticToken>,
    hover_info: HashMap<SourceLocation, HoverInfo>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer from an AST
    pub fn analyze(ast: &Node) -> Self {
        // First extract symbols
        let symbol_table = SymbolExtractor::new().extract(ast);

        let mut analyzer = SemanticAnalyzer {
            symbol_table,
            semantic_tokens: Vec::new(),
            hover_info: HashMap::new(),
        };

        // Then perform semantic analysis
        analyzer.analyze_node(ast, 0);

        analyzer
    }

    /// Get the symbol table
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get semantic tokens for syntax highlighting
    pub fn semantic_tokens(&self) -> &[SemanticToken] {
        &self.semantic_tokens
    }

    /// Get hover information at a location
    pub fn hover_at(&self, location: SourceLocation) -> Option<&HoverInfo> {
        self.hover_info.get(&location)
    }

    /// Find the symbol at a given location
    pub fn symbol_at(&self, location: SourceLocation) -> Option<&Symbol> {
        // Search through all symbols for one at this location
        for symbols in self.symbol_table.symbols.values() {
            for symbol in symbols {
                if symbol.location.start <= location.start && symbol.location.end >= location.end {
                    return Some(symbol);
                }
            }
        }
        None
    }

    /// Find the definition of a symbol at a given position
    pub fn find_definition(&self, position: usize) -> Option<&Symbol> {
        // First, find if there's a reference at this position
        for refs in self.symbol_table.references.values() {
            for reference in refs {
                if reference.location.start <= position && reference.location.end >= position {
                    // Found a reference, now find its definition
                    let symbols = self.symbol_table.find_symbol(
                        &reference.name,
                        reference.scope_id,
                        reference.kind,
                    );

                    // Return the first matching definition
                    if !symbols.is_empty() {
                        return Some(symbols[0]);
                    }
                }
            }
        }

        // If no reference found, check if we're on a definition itself
        self.symbol_at(SourceLocation { start: position, end: position })
    }

    /// Find all references to a symbol at a given position
    pub fn find_all_references(
        &self,
        position: usize,
        include_declaration: bool,
    ) -> Vec<SourceLocation> {
        // First find the symbol at this position (either definition or reference)
        let symbol = if let Some(def) = self.find_definition(position) {
            Some(def)
        } else {
            // Check if we're on a reference
            for refs in self.symbol_table.references.values() {
                for reference in refs {
                    if reference.location.start <= position && reference.location.end >= position {
                        // Found a reference, get its definition to get the symbol ID
                        let symbols = self.symbol_table.find_symbol(
                            &reference.name,
                            reference.scope_id,
                            reference.kind,
                        );
                        if !symbols.is_empty() {
                            return self
                                .find_all_references_for_symbol(symbols[0], include_declaration);
                        }
                    }
                }
            }
            None
        };

        if let Some(symbol) = symbol {
            return self.find_all_references_for_symbol(symbol, include_declaration);
        }

        Vec::new()
    }

    /// Find all references for a specific symbol
    fn find_all_references_for_symbol(
        &self,
        symbol: &Symbol,
        include_declaration: bool,
    ) -> Vec<SourceLocation> {
        let mut locations = Vec::new();

        // Include the declaration if requested
        if include_declaration {
            locations.push(symbol.location);
        }

        // Find all references to this symbol by name
        if let Some(refs) = self.symbol_table.references.get(&symbol.name) {
            for reference in refs {
                // Only include references of the same kind and in scope where the symbol is visible
                if reference.kind == symbol.kind {
                    // Check if the symbol is visible from this reference's scope
                    if self.is_symbol_visible(symbol, reference.scope_id) {
                        locations.push(reference.location);
                    }
                }
            }
        }

        locations
    }

    /// Check if a symbol is visible from a given scope
    fn is_symbol_visible(&self, symbol: &Symbol, scope_id: ScopeId) -> bool {
        // For now, simple visibility check:
        // - Symbols in the same scope are visible
        // - Symbols in parent scopes are visible
        // - Package-level symbols are visible from package scopes

        if symbol.scope_id == scope_id {
            return true;
        }

        // Check if scope_id is a descendant of symbol.scope_id
        let mut current_scope = scope_id;
        while let Some(scope) = self.symbol_table.scopes.get(&current_scope) {
            if scope.parent == Some(symbol.scope_id) {
                return true;
            }
            if let Some(parent) = scope.parent {
                current_scope = parent;
            } else {
                break;
            }
        }

        // For package-level symbols (scope_id 0), always visible
        symbol.scope_id == 0
    }

    /// Analyze a node and generate semantic information
    fn analyze_node(&mut self, node: &Node, scope_id: ScopeId) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.analyze_node(stmt, scope_id);
                }
            }

            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                // Add semantic token for declaration
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let token_type = match declarator.as_str() {
                        "my" | "state" => SemanticTokenType::VariableDeclaration,
                        "our" => SemanticTokenType::Variable,
                        "local" => SemanticTokenType::Variable,
                        _ => SemanticTokenType::Variable,
                    };

                    let mut modifiers = vec![SemanticTokenModifier::Declaration];
                    if declarator == "state" || attributes.iter().any(|a| a == ":shared") {
                        modifiers.push(SemanticTokenModifier::Static);
                    }

                    self.semantic_tokens.push(SemanticToken {
                        location: variable.location,
                        token_type,
                        modifiers,
                    });

                    // Add hover info
                    let hover = HoverInfo {
                        signature: format!("{} {}{}", declarator, sigil, name),
                        documentation: None,
                        details: if attributes.is_empty() {
                            vec![]
                        } else {
                            vec![format!("Attributes: {}", attributes.join(", "))]
                        },
                    };

                    self.hover_info.insert(variable.location, hover);
                }

                if let Some(init) = initializer {
                    self.analyze_node(init, scope_id);
                }
            }

            NodeKind::Variable { sigil, name } => {
                let kind = match sigil.as_str() {
                    "$" => SymbolKind::ScalarVariable,
                    "@" => SymbolKind::ArrayVariable,
                    "%" => SymbolKind::HashVariable,
                    _ => return,
                };

                // Find the symbol definition
                let symbols = self.symbol_table.find_symbol(name, scope_id, kind);

                let token_type = if symbols.is_empty() {
                    // Undefined variable
                    SemanticTokenType::Variable
                } else {
                    let symbol = symbols[0];
                    match symbol.declaration.as_deref() {
                        Some("my") | Some("state") => SemanticTokenType::Variable,
                        Some("our") => SemanticTokenType::Variable,
                        _ => SemanticTokenType::Variable,
                    }
                };

                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type,
                    modifiers: vec![],
                });

                // Add hover info if we found the symbol
                if let Some(symbol) = symbols.first() {
                    let hover = HoverInfo {
                        signature: format!(
                            "{} {}{}",
                            symbol.declaration.as_deref().unwrap_or(""),
                            sigil,
                            name
                        )
                        .trim()
                        .to_string(),
                        documentation: symbol.documentation.clone(),
                        details: vec![format!(
                            "Defined at line {}",
                            self.line_number(symbol.location.start)
                        )],
                    };

                    self.hover_info.insert(node.location, hover);
                }
            }

            NodeKind::Subroutine { name, params, attributes, body } => {
                if let Some(sub_name) = name {
                    let token = SemanticToken {
                        location: node.location,
                        token_type: SemanticTokenType::FunctionDeclaration,
                        modifiers: vec![SemanticTokenModifier::Declaration],
                    };

                    self.semantic_tokens.push(token);

                    // Add hover info
                    let mut signature = format!("sub {}", sub_name);
                    if !params.is_empty() {
                        signature.push_str("(...)");
                    }

                    let hover = HoverInfo {
                        signature,
                        documentation: None, // TODO: Extract from POD or comments
                        details: if attributes.is_empty() {
                            vec![]
                        } else {
                            vec![format!("Attributes: {}", attributes.join(", "))]
                        },
                    };

                    self.hover_info.insert(node.location, hover);
                }

                {
                    // Get the subroutine scope
                    let sub_scope = scope_id; // TODO: Get actual sub scope
                    self.analyze_node(body, sub_scope);
                }
            }

            NodeKind::FunctionCall { name, args } => {
                // Check if this is a built-in function
                {
                    let token_type = if is_builtin_function(name) {
                        SemanticTokenType::Function
                    } else {
                        // Check if it's a user-defined function
                        let symbols =
                            self.symbol_table.find_symbol(name, scope_id, SymbolKind::Subroutine);
                        if symbols.is_empty() {
                            SemanticTokenType::Function
                        } else {
                            SemanticTokenType::Function
                        }
                    };

                    self.semantic_tokens.push(SemanticToken {
                        location: node.location,
                        token_type,
                        modifiers: if is_builtin_function(name) {
                            vec![SemanticTokenModifier::DefaultLibrary]
                        } else {
                            vec![]
                        },
                    });

                    // Add hover for built-ins
                    if let Some(doc) = get_builtin_documentation(name) {
                        let hover = HoverInfo {
                            signature: doc.signature.to_string(),
                            documentation: Some(doc.description.to_string()),
                            details: vec![],
                        };

                        self.hover_info.insert(node.location, hover);
                    }
                }

                // Name is already a string, not a node
                for arg in args {
                    self.analyze_node(arg, scope_id);
                }
            }

            NodeKind::Package { name, block } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Namespace,
                    modifiers: vec![SemanticTokenModifier::Declaration],
                });

                let hover = HoverInfo {
                    signature: format!("package {}", name),
                    documentation: None,
                    details: vec![],
                };

                self.hover_info.insert(node.location, hover);

                if let Some(block_node) = block {
                    let package_scope = scope_id; // TODO: Get actual package scope
                    self.analyze_node(block_node, package_scope);
                }
            }

            NodeKind::String { value: _, interpolated: _ } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::String,
                    modifiers: vec![],
                });
            }

            NodeKind::Number { value: _ } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Number,
                    modifiers: vec![],
                });
            }

            NodeKind::Regex { .. } | NodeKind::Match { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Regex,
                    modifiers: vec![],
                });
            }

            NodeKind::LabeledStatement { label: _, statement } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Label,
                    modifiers: vec![],
                });

                {
                    self.analyze_node(statement, scope_id);
                }
            }

            // Control flow keywords
            NodeKind::If { condition, then_branch, elsif_branches: _, else_branch } => {
                self.analyze_node(condition, scope_id);
                self.analyze_node(then_branch, scope_id);
                if let Some(else_node) = else_branch {
                    self.analyze_node(else_node, scope_id);
                }
            }

            NodeKind::While { condition, body, continue_block: _ } => {
                self.analyze_node(condition, scope_id);
                self.analyze_node(body, scope_id);
            }

            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(init_node) = init {
                    self.analyze_node(init_node, scope_id);
                }
                if let Some(cond_node) = condition {
                    self.analyze_node(cond_node, scope_id);
                }
                if let Some(update_node) = update {
                    self.analyze_node(update_node, scope_id);
                }
                self.analyze_node(body, scope_id);
            }

            NodeKind::Foreach { variable, list, body } => {
                self.analyze_node(variable, scope_id);
                self.analyze_node(list, scope_id);
                self.analyze_node(body, scope_id);
            }

            // Recursively analyze other nodes
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.analyze_node(stmt, scope_id);
                }
            }

            NodeKind::Binary { left, right, .. } => {
                self.analyze_node(left, scope_id);
                self.analyze_node(right, scope_id);
            }

            NodeKind::Assignment { lhs, rhs, .. } => {
                self.analyze_node(lhs, scope_id);
                self.analyze_node(rhs, scope_id);
            }

            _ => {
                // Handle other node types as needed
            }
        }
    }

    /// Get line number from byte offset (simplified version)
    fn line_number(&self, _offset: usize) -> usize {
        // In a real implementation, we'd need the source text to calculate this
        // For now, return a placeholder
        1
    }
}

/// Built-in function documentation
struct BuiltinDoc {
    signature: &'static str,
    description: &'static str,
}

/// Check if a function is a Perl built-in
fn is_builtin_function(name: &str) -> bool {
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
            | "chomp"
            | "chop"
            | "split"
            | "join"
            | "push"
            | "pop"
            | "shift"
            | "unshift"
            | "sort"
            | "reverse"
            | "map"
            | "grep"
            | "length"
            | "substr"
            | "index"
            | "rindex"
            | "lc"
            | "uc"
            | "lcfirst"
            | "ucfirst"
            | "defined"
            | "undef"
            | "ref"
            | "blessed"
            | "die"
            | "warn"
            | "eval"
            | "require"
            | "use"
            | "return"
            | "next"
            | "last"
            | "redo"
            | "goto" // ... many more
    )
}

/// Get documentation for built-in functions
fn get_builtin_documentation(name: &str) -> Option<BuiltinDoc> {
    match name {
        "print" => Some(BuiltinDoc {
            signature: "print FILEHANDLE LIST\nprint FILEHANDLE\nprint LIST\nprint",
            description: "Prints a string or list of strings to a filehandle",
        }),
        "push" => Some(BuiltinDoc {
            signature: "push ARRAY, LIST",
            description: "Appends one or more elements to an array",
        }),
        "split" => Some(BuiltinDoc {
            signature: "split /PATTERN/, EXPR, LIMIT\nsplit /PATTERN/, EXPR\nsplit /PATTERN/",
            description: "Splits a string into a list of strings",
        }),
        "map" => Some(BuiltinDoc {
            signature: "map BLOCK LIST\nmap EXPR, LIST",
            description: "Evaluates the BLOCK or EXPR for each element of LIST",
        }),
        // Add more built-ins as needed
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_semantic_tokens() {
        let code = r#"
my $x = 42;
print $x;
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();

        let analyzer = SemanticAnalyzer::analyze(&ast);
        let tokens = analyzer.semantic_tokens();

        // Should have tokens for: my (keyword), $x (variable declaration), 42 (number), print (function), $x (variable)
        assert!(tokens.len() >= 3);

        // Check first $x is a declaration
        let x_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| {
                matches!(
                    t.token_type,
                    SemanticTokenType::Variable | SemanticTokenType::VariableDeclaration
                )
            })
            .collect();
        assert!(!x_tokens.is_empty());
        assert!(x_tokens[0].modifiers.contains(&SemanticTokenModifier::Declaration));
    }

    #[test]
    fn test_hover_info() {
        let code = r#"
sub foo {
    return 42;
}

my $result = foo();
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();

        let analyzer = SemanticAnalyzer::analyze(&ast);

        // The hover info would be at specific locations
        // In practice, we'd look up by position
        assert!(!analyzer.hover_info.is_empty());
    }
}
