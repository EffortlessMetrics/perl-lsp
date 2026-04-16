//! AST node analysis — the `analyze_node` traversal and source-text helpers.
//!
//! This module provides the core recursive descent over the Perl AST that
//! produces semantic tokens and hover information.  It is an `impl` block
//! extension of `SemanticAnalyzer`; Rust allows splitting `impl` blocks across
//! files within the same module, so the types defined in `mod.rs` are
//! directly accessible here.

use crate::SourceLocation;
use crate::ast::{Node, NodeKind};
use crate::symbol::{ScopeId, ScopeKind, SymbolKind};
use regex::Regex;
use std::sync::OnceLock;

use super::SemanticAnalyzer;
use super::builtins::{
    get_builtin_documentation, is_builtin_function, is_control_keyword, is_file_test_operator,
};
use super::hover::HoverInfo;
use super::tokens::{SemanticToken, SemanticTokenModifier, SemanticTokenType};

impl SemanticAnalyzer {
    /// Analyze a node and generate semantic information
    pub(super) fn analyze_node(&mut self, node: &Node, scope_id: ScopeId) {
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
                        documentation: self.extract_documentation(node.location.start),
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
                    "$" => SymbolKind::scalar(),
                    "@" => SymbolKind::array(),
                    "%" => SymbolKind::hash(),
                    _ => return,
                };

                // Find the symbol definition
                let symbols = self.symbol_table.find_symbol(name, scope_id, kind);

                let token_type = if let Some(symbol) = symbols.first() {
                    match symbol.declaration.as_deref() {
                        Some("my") | Some("state") => SemanticTokenType::Variable,
                        Some("our") => SemanticTokenType::Variable,
                        _ => SemanticTokenType::Variable,
                    }
                } else {
                    // Undefined variable
                    SemanticTokenType::Variable
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

            NodeKind::Subroutine { name, prototype, signature, attributes, body, name_span: _ } => {
                if let Some(sub_name) = name {
                    // Named subroutine
                    let token = SemanticToken {
                        location: node.location,
                        token_type: SemanticTokenType::FunctionDeclaration,
                        modifiers: vec![SemanticTokenModifier::Declaration],
                    };

                    self.semantic_tokens.push(token);

                    // Add hover info
                    let mut signature_str = format!("sub {}", sub_name);
                    if let Some(sig_node) = signature {
                        signature_str.push_str(&format_signature_params(sig_node));
                    }

                    let hover = HoverInfo {
                        signature: signature_str,
                        documentation: self.extract_sub_documentation(node.location.start, body),
                        details: if attributes.is_empty() {
                            vec![]
                        } else {
                            vec![format!("Attributes: {}", attributes.join(", "))]
                        },
                    };

                    self.hover_info.insert(node.location, hover);
                } else {
                    // Anonymous subroutine (closure)
                    // Add semantic token for the 'sub' keyword
                    self.semantic_tokens.push(SemanticToken {
                        location: SourceLocation {
                            start: node.location.start,
                            end: node.location.start + 3, // "sub"
                        },
                        token_type: SemanticTokenType::Keyword,
                        modifiers: vec![],
                    });

                    // Add hover info for anonymous subs
                    let mut signature_str = "sub".to_string();
                    if let Some(sig_node) = signature {
                        signature_str.push_str(&format_signature_params(sig_node));
                    }
                    signature_str.push_str(" { ... }");

                    let mut details = vec!["Anonymous subroutine (closure)".to_string()];
                    if !attributes.is_empty() {
                        details.push(format!("Attributes: {}", attributes.join(", ")));
                    }

                    let hover = HoverInfo {
                        signature: signature_str,
                        documentation: self.extract_sub_documentation(node.location.start, body),
                        details,
                    };

                    self.hover_info.insert(node.location, hover);
                }

                {
                    // Get the subroutine scope from the symbol table
                    let sub_scope = self.get_scope_for(node, ScopeKind::Subroutine);

                    if let Some(proto) = prototype {
                        self.analyze_node(proto, sub_scope);
                    }
                    if let Some(sig) = signature {
                        self.analyze_node(sig, sub_scope);
                    }

                    self.analyze_node(body, sub_scope);
                }
            }

            NodeKind::Method { name, signature, attributes, body } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location, // Approximate, ideally name span
                    token_type: SemanticTokenType::FunctionDeclaration,
                    modifiers: vec![SemanticTokenModifier::Declaration],
                });

                // Add hover info
                let hover = HoverInfo {
                    signature: format!("method {}", name),
                    documentation: self.extract_sub_documentation(node.location.start, body),
                    details: if attributes.is_empty() {
                        vec![]
                    } else {
                        vec![format!("Attributes: {}", attributes.join(", "))]
                    },
                };
                self.hover_info.insert(node.location, hover);

                // Analyze body in new scope (assumed same as Subroutine scope kind for now)
                let sub_scope = self.get_scope_for(node, ScopeKind::Subroutine);
                if let Some(sig) = signature {
                    self.analyze_node(sig, sub_scope);
                }
                self.analyze_node(body, sub_scope);
            }

            NodeKind::FunctionCall { name, args } => {
                // Check if this is a built-in function
                {
                    let token_type = if is_control_keyword(name) {
                        SemanticTokenType::KeywordControl
                    } else if is_builtin_function(name) {
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
                        modifiers: if is_builtin_function(name) && !is_control_keyword(name) {
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

            NodeKind::Package { name, block, name_span: _ } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Namespace,
                    modifiers: vec![SemanticTokenModifier::Declaration],
                });

                // Try POD docs first, then fall back to leading comments
                let documentation = self
                    .extract_pod_name_section(name)
                    .or_else(|| self.extract_documentation(node.location.start));

                let hover = HoverInfo {
                    signature: format!("package {}", name),
                    documentation,
                    details: vec![],
                };

                self.hover_info.insert(node.location, hover);

                if let Some(block_node) = block {
                    let package_scope = self.get_scope_for(node, ScopeKind::Package);
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

            NodeKind::Regex { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Regex,
                    modifiers: vec![],
                });
            }

            NodeKind::Match { expr, .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Regex,
                    modifiers: vec![],
                });
                self.analyze_node(expr, scope_id);
            }
            NodeKind::Substitution { expr, .. } => {
                // Substitution operator: s/// - add semantic token for the operator
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Operator,
                    modifiers: vec![],
                });
                self.analyze_node(expr, scope_id);
            }
            NodeKind::Transliteration { expr, .. } => {
                // Transliteration operator: tr/// or y/// - add semantic token for the operator
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Operator,
                    modifiers: vec![],
                });
                self.analyze_node(expr, scope_id);
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
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.analyze_node(condition, scope_id);
                self.analyze_node(then_branch, scope_id);
                for (elsif_cond, elsif_branch) in elsif_branches {
                    self.analyze_node(elsif_cond, scope_id);
                    self.analyze_node(elsif_branch, scope_id);
                }
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

            NodeKind::Foreach { variable, list, body, continue_block } => {
                self.analyze_node(variable, scope_id);
                self.analyze_node(list, scope_id);
                self.analyze_node(body, scope_id);
                if let Some(cb) = continue_block {
                    self.analyze_node(cb, scope_id);
                }
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

            // Phase 1: Critical LSP Features (Issue #188)
            NodeKind::VariableListDeclaration {
                declarator,
                variables,
                attributes,
                initializer,
            } => {
                // Handle multi-variable declarations like: my ($x, $y, $z) = (1, 2, 3);
                for var in variables {
                    if let NodeKind::Variable { sigil, name } = &var.kind {
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
                            location: var.location,
                            token_type,
                            modifiers,
                        });

                        // Add hover info
                        let hover = HoverInfo {
                            signature: format!("{} {}{}", declarator, sigil, name),
                            documentation: self.extract_documentation(var.location.start),
                            details: if attributes.is_empty() {
                                vec![]
                            } else {
                                vec![format!("Attributes: {}", attributes.join(", "))]
                            },
                        };

                        self.hover_info.insert(var.location, hover);
                    }
                }

                if let Some(init) = initializer {
                    self.analyze_node(init, scope_id);
                }
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                // Handle conditional expressions: $x ? $y : $z
                self.analyze_node(condition, scope_id);
                self.analyze_node(then_expr, scope_id);
                self.analyze_node(else_expr, scope_id);
            }

            NodeKind::ArrayLiteral { elements } => {
                // Handle array constructors: [1, 2, 3, 4]
                for elem in elements {
                    self.analyze_node(elem, scope_id);
                }
            }

            NodeKind::HashLiteral { pairs } => {
                // Handle hash constructors: { key1 => "value1", key2 => "value2" }
                for (key, value) in pairs {
                    self.analyze_node(key, scope_id);
                    self.analyze_node(value, scope_id);
                }
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                // Handle try/catch error handling
                self.analyze_node(body, scope_id);

                for (_var, catch_body) in catch_blocks {
                    // Note: var is just a String (variable name), not a Node
                    self.analyze_node(catch_body, scope_id);
                }

                if let Some(finally) = finally_block {
                    self.analyze_node(finally, scope_id);
                }
            }

            NodeKind::PhaseBlock { phase: _, phase_span: _, block } => {
                // Handle BEGIN/END/INIT/CHECK/UNITCHECK blocks
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Keyword,
                    modifiers: vec![],
                });

                self.analyze_node(block, scope_id);
            }

            NodeKind::ExpressionStatement { expression } => {
                // Handle expression statements: $x + 10;
                // Just delegate to the wrapped expression
                self.analyze_node(expression, scope_id);
            }

            NodeKind::Do { block } => {
                // Handle do blocks: do { ... }
                // Do blocks create expression context but maintain scope
                self.analyze_node(block, scope_id);
            }

            NodeKind::Eval { block } => {
                // Handle eval blocks: eval { dangerous_operation(); }
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Keyword,
                    modifiers: vec![],
                });

                // Eval blocks should create a new scope for error isolation
                self.analyze_node(block, scope_id);
            }

            NodeKind::Defer { block } => {
                // defer { } blocks run on scope exit; analyze the block for symbol resolution
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Keyword,
                    modifiers: vec![],
                });
                self.analyze_node(block, scope_id);
            }

            NodeKind::VariableWithAttributes { variable, attributes } => {
                // Handle attributed variables: my $x :shared = 42;
                // Analyze the base variable node
                self.analyze_node(variable, scope_id);

                // Add modifier tokens for special attributes
                if attributes.iter().any(|a| a == ":shared" || a == ":lvalue") {
                    // The variable node was already processed, so we just note the attributes
                    // in the hover info (if we need to enhance it later)
                }
            }

            NodeKind::Unary { op, operand } => {
                // Handle unary operators: -$x, !$x, ++$x, $x++
                // Add token for the operator itself (if needed for highlighting)
                if matches!(op.as_str(), "++" | "--" | "!" | "-" | "~" | "\\") {
                    self.semantic_tokens.push(SemanticToken {
                        location: node.location,
                        token_type: SemanticTokenType::Operator,
                        modifiers: vec![],
                    });
                }

                // Handle file test operators: -e, -d, -f, -r, -w, -x, -s, -z, -T, -B, etc.
                if is_file_test_operator(op) {
                    self.semantic_tokens.push(SemanticToken {
                        location: node.location,
                        token_type: SemanticTokenType::Operator,
                        modifiers: vec![],
                    });
                }

                self.analyze_node(operand, scope_id);
            }

            NodeKind::Readline { filehandle } => {
                // Handle readline/diamond operator: <STDIN>, <$fh>, <>
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Operator, // diamond operator is an I/O operator
                    modifiers: vec![],
                });

                // Add hover info for common filehandles
                if let Some(fh) = filehandle {
                    let hover = HoverInfo {
                        signature: format!("<{}>", fh),
                        documentation: match fh.as_str() {
                            "STDIN" => Some("Standard input filehandle".to_string()),
                            "STDOUT" => Some("Standard output filehandle".to_string()),
                            "STDERR" => Some("Standard error filehandle".to_string()),
                            _ => Some(format!("Read from filehandle {}", fh)),
                        },
                        details: vec![],
                    };
                    self.hover_info.insert(node.location, hover);
                } else {
                    // Bare <> reads from ARGV or STDIN
                    let hover = HoverInfo {
                        signature: "<>".to_string(),
                        documentation: Some("Read from command-line files or STDIN".to_string()),
                        details: vec![],
                    };
                    self.hover_info.insert(node.location, hover);
                }
            }

            // Phase 2/3 Handlers
            NodeKind::MethodCall { object, method, args } => {
                self.analyze_node(object, scope_id);

                if let Some(offset) =
                    self.find_substring_in_source_after(node, method, object.location.end)
                {
                    self.semantic_tokens.push(SemanticToken {
                        location: SourceLocation { start: offset, end: offset + method.len() },
                        token_type: SemanticTokenType::Method,
                        modifiers: vec![],
                    });
                }

                for arg in args {
                    self.analyze_node(arg, scope_id);
                }
            }

            NodeKind::IndirectCall { method, object, args } => {
                if let Some(offset) = self.find_method_name_in_source(node, method) {
                    self.semantic_tokens.push(SemanticToken {
                        location: SourceLocation { start: offset, end: offset + method.len() },
                        token_type: SemanticTokenType::Method,
                        modifiers: vec![],
                    });
                }
                self.analyze_node(object, scope_id);
                for arg in args {
                    self.analyze_node(arg, scope_id);
                }
            }

            NodeKind::Use { module, args, .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation {
                        start: node.location.start,
                        end: node.location.start + 3,
                    },
                    token_type: SemanticTokenType::Keyword,
                    modifiers: vec![],
                });

                let mut args_start = node.location.start + 3;
                if let Some(offset) = self.find_substring_in_source(node, module) {
                    self.semantic_tokens.push(SemanticToken {
                        location: SourceLocation { start: offset, end: offset + module.len() },
                        token_type: SemanticTokenType::Namespace,
                        modifiers: vec![],
                    });
                    args_start = offset + module.len();
                }

                self.analyze_string_args(node, args, args_start);
            }

            NodeKind::No { module, args, .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation {
                        start: node.location.start,
                        end: node.location.start + 2,
                    },
                    token_type: SemanticTokenType::Keyword,
                    modifiers: vec![],
                });

                let mut args_start = node.location.start + 2;
                if let Some(offset) = self.find_substring_in_source(node, module) {
                    self.semantic_tokens.push(SemanticToken {
                        location: SourceLocation { start: offset, end: offset + module.len() },
                        token_type: SemanticTokenType::Namespace,
                        modifiers: vec![],
                    });
                    args_start = offset + module.len();
                }

                self.analyze_string_args(node, args, args_start);
            }

            NodeKind::Given { expr, body } => {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation {
                        start: node.location.start,
                        end: node.location.start + 5,
                    }, // given
                    token_type: SemanticTokenType::KeywordControl,
                    modifiers: vec![],
                });
                self.analyze_node(expr, scope_id);
                self.analyze_node(body, scope_id);
            }

            NodeKind::When { condition, body } => {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation {
                        start: node.location.start,
                        end: node.location.start + 4,
                    }, // when
                    token_type: SemanticTokenType::KeywordControl,
                    modifiers: vec![],
                });
                self.analyze_node(condition, scope_id);
                self.analyze_node(body, scope_id);
            }

            NodeKind::Default { body } => {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation {
                        start: node.location.start,
                        end: node.location.start + 7,
                    }, // default
                    token_type: SemanticTokenType::KeywordControl,
                    modifiers: vec![],
                });
                self.analyze_node(body, scope_id);
            }

            NodeKind::Return { value } => {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation {
                        start: node.location.start,
                        end: node.location.start + 6,
                    }, // return
                    token_type: SemanticTokenType::KeywordControl,
                    modifiers: vec![],
                });
                if let Some(v) = value {
                    self.analyze_node(v, scope_id);
                }
            }

            NodeKind::Class { name, body, .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation {
                        start: node.location.start,
                        end: node.location.start + 5,
                    }, // class
                    token_type: SemanticTokenType::Keyword,
                    modifiers: vec![],
                });

                if let Some(offset) = self.find_substring_in_source(node, name) {
                    self.semantic_tokens.push(SemanticToken {
                        location: SourceLocation { start: offset, end: offset + name.len() },
                        token_type: SemanticTokenType::Class,
                        modifiers: vec![SemanticTokenModifier::Declaration],
                    });
                }

                let class_scope = self.get_scope_for(node, ScopeKind::Package);
                self.analyze_node(body, class_scope);
            }

            NodeKind::Signature { parameters } => {
                for param in parameters {
                    self.analyze_node(param, scope_id);
                }
            }

            NodeKind::MandatoryParameter { variable }
            | NodeKind::OptionalParameter { variable, .. }
            | NodeKind::SlurpyParameter { variable }
            | NodeKind::NamedParameter { variable } => {
                self.analyze_node(variable, scope_id);
            }

            NodeKind::Diamond | NodeKind::Ellipsis => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Operator,
                    modifiers: vec![],
                });
            }

            NodeKind::Undef => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Keyword,
                    modifiers: vec![],
                });
            }

            NodeKind::Identifier { .. } => {
                // Bareword identifiers, usually left to lexical highlighting
                // but we handle them to avoid the default case.
            }

            NodeKind::Heredoc { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::String,
                    modifiers: vec![],
                });
            }

            NodeKind::Glob { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Operator,
                    modifiers: vec![],
                });
            }

            NodeKind::DataSection { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Comment,
                    modifiers: vec![],
                });
            }

            NodeKind::Prototype { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Punctuation,
                    modifiers: vec![],
                });
            }

            NodeKind::Typeglob { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::Variable,
                    modifiers: vec![],
                });
            }

            NodeKind::Untie { variable } => {
                self.analyze_node(variable, scope_id);
            }

            NodeKind::LoopControl { .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::KeywordControl,
                    modifiers: vec![],
                });
            }

            NodeKind::Goto { target } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::KeywordControl,
                    modifiers: vec![],
                });
                self.analyze_node(target, scope_id);
            }

            NodeKind::MissingExpression
            | NodeKind::MissingStatement
            | NodeKind::MissingIdentifier
            | NodeKind::MissingBlock => {
                // No tokens for missing constructs
            }

            NodeKind::Tie { variable, package, args } => {
                self.analyze_node(variable, scope_id);
                self.analyze_node(package, scope_id);
                for arg in args {
                    self.analyze_node(arg, scope_id);
                }
            }

            NodeKind::StatementModifier { statement, condition, modifier } => {
                // Handle postfix loop modifiers: for, while, until, foreach
                // e.g., print $_ for @list; or $x++ while $x < 10;
                if matches!(modifier.as_str(), "for" | "foreach" | "while" | "until") {
                    self.semantic_tokens.push(SemanticToken {
                        location: node.location,
                        token_type: SemanticTokenType::KeywordControl,
                        modifiers: vec![],
                    });
                }
                self.analyze_node(statement, scope_id);
                self.analyze_node(condition, scope_id);
            }

            NodeKind::Format { name, .. } => {
                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type: SemanticTokenType::FunctionDeclaration,
                    modifiers: vec![SemanticTokenModifier::Declaration],
                });

                let hover = HoverInfo {
                    signature: format!("format {} =", name),
                    documentation: None,
                    details: vec![],
                };
                self.hover_info.insert(node.location, hover);
            }

            NodeKind::Error { .. } | NodeKind::UnknownRest => {
                // No semantic tokens for error nodes
            }
        }
    }

    /// Extract documentation (POD or comments) preceding a position
    pub(super) fn extract_documentation(&self, start: usize) -> Option<String> {
        static POD_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
        static COMMENT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

        if self.source.is_empty() {
            return None;
        }
        let before = &self.source[..start];

        // Check for POD blocks ending with =cut
        let pod_re = POD_RE
            .get_or_init(|| Regex::new(r"(?ms)(=[a-zA-Z0-9].*?\n=cut\n?)\s*$"))
            .as_ref()
            .ok()?;
        if let Some(caps) = pod_re.captures(before) {
            if let Some(pod_text) = caps.get(1) {
                return Some(pod_text.as_str().trim().to_string());
            }
        }

        // Check for consecutive comment lines
        let comment_re =
            COMMENT_RE.get_or_init(|| Regex::new(r"(?m)(#.*\n)+\s*$")).as_ref().ok()?;
        if let Some(caps) = comment_re.captures(before) {
            if let Some(comment_match) = caps.get(0) {
                // Strip the # prefix from each comment line
                let doc = comment_match
                    .as_str()
                    .lines()
                    .map(|line| line.trim_start_matches('#').trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                return Some(doc);
            }
        }

        None
    }

    /// Extract POD documentation from inside a subroutine body.
    ///
    /// Perl allows POD blocks anywhere in source, including inside subroutine
    /// bodies. This method searches within the body's byte range for the first
    /// POD block (`=pod`/`=head`/etc. through `=cut`) and returns its content.
    /// Falls back to leading comment blocks inside the body if no POD is found.
    pub(super) fn extract_body_inline_pod(&self, body: &Node) -> Option<String> {
        static INLINE_POD_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
        static BODY_COMMENT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

        let body_start = body.location.start;
        let body_end = body.location.end.min(self.source.len());
        if body_start >= body_end {
            return None;
        }
        let body_text = self.source.get(body_start..body_end)?;

        // Search for the first POD block inside the body
        let pod_re = INLINE_POD_RE
            .get_or_init(|| Regex::new(r"(?ms)(=[a-zA-Z][a-zA-Z0-9]*\s.*?\n=cut\n?)"))
            .as_ref()
            .ok()?;
        if let Some(caps) = pod_re.captures(body_text) {
            if let Some(pod_text) = caps.get(1) {
                return Some(pod_text.as_str().trim().to_string());
            }
        }

        // Fall back to leading comment block at the start of the body
        let body_comment_re = BODY_COMMENT_RE
            .get_or_init(|| Regex::new(r"(?m)\A\s*\{?\s*\n((?:\s*#.*\n)+)"))
            .as_ref()
            .ok()?;
        if let Some(caps) = body_comment_re.captures(body_text) {
            if let Some(comment_match) = caps.get(1) {
                let doc = comment_match
                    .as_str()
                    .lines()
                    .map(|line| line.trim().trim_start_matches('#').trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !doc.is_empty() {
                    return Some(doc);
                }
            }
        }

        None
    }

    /// Extract subroutine documentation, searching both before the `sub`
    /// keyword and inside the body.
    ///
    /// Tries preceding POD/comments first (the common case), then falls back
    /// to inline POD or leading comments within the body.
    pub(super) fn extract_sub_documentation(
        &self,
        sub_start: usize,
        body: &Node,
    ) -> Option<String> {
        self.extract_documentation(sub_start).or_else(|| self.extract_body_inline_pod(body))
    }

    /// Extract the POD `=head1 NAME` section for a package.
    ///
    /// Scans the entire source for a `=head1 NAME` POD section and returns
    /// its content if it mentions the given package name.
    pub(super) fn extract_pod_name_section(&self, package_name: &str) -> Option<String> {
        if self.source.is_empty() {
            return None;
        }

        let mut in_name_section = false;
        let mut name_lines: Vec<&str> = Vec::new();

        for line in self.source.lines() {
            let trimmed: &str = line.trim();
            if trimmed.starts_with("=head1") {
                if in_name_section {
                    break;
                }
                let heading = trimmed.strip_prefix("=head1").map(|s: &str| s.trim());
                if heading == Some("NAME") {
                    in_name_section = true;
                    continue;
                }
            } else if trimmed.starts_with("=cut") && in_name_section {
                break;
            } else if trimmed.starts_with('=') && in_name_section {
                break;
            } else if in_name_section && !trimmed.is_empty() {
                name_lines.push(trimmed);
            }
        }

        if !name_lines.is_empty() {
            let name_doc = name_lines.join(" ");
            if name_doc.contains(package_name)
                || name_doc.contains(&package_name.replace("::", "-"))
            {
                return Some(name_doc);
            }
        }

        None
    }

    /// Get scope id for a node by consulting the symbol table
    pub(super) fn get_scope_for(&self, node: &Node, kind: ScopeKind) -> ScopeId {
        for scope in self.symbol_table.scopes.values() {
            if scope.kind == kind
                && scope.location.start == node.location.start
                && scope.location.end == node.location.end
            {
                return scope.id;
            }
        }
        0
    }

    /// Get line number from byte offset (simplified version)
    pub(super) fn line_number(&self, offset: usize) -> usize {
        if self.source.is_empty() { 1 } else { self.source[..offset].lines().count() + 1 }
    }

    /// Find substring in source within node's range
    pub(super) fn find_substring_in_source(&self, node: &Node, substring: &str) -> Option<usize> {
        if self.source.len() < node.location.end {
            return None;
        }
        let node_text = &self.source[node.location.start..node.location.end];
        if let Some(pos) = node_text.find(substring) {
            return Some(node.location.start + pos);
        }
        None
    }

    /// Find method name in source within node's range
    pub(super) fn find_method_name_in_source(
        &self,
        node: &Node,
        method_name: &str,
    ) -> Option<usize> {
        self.find_substring_in_source(node, method_name)
    }

    /// Find substring in source within node's range, starting search after a specific absolute offset
    pub(super) fn find_substring_in_source_after(
        &self,
        node: &Node,
        substring: &str,
        after: usize,
    ) -> Option<usize> {
        if self.source.len() < node.location.end || after >= node.location.end {
            return None;
        }

        let start_rel = after.saturating_sub(node.location.start);

        let node_text = &self.source[node.location.start..node.location.end];
        if start_rel >= node_text.len() {
            return None;
        }

        let text_to_search = &node_text[start_rel..];
        if let Some(pos) = text_to_search.find(substring) {
            return Some(node.location.start + start_rel + pos);
        }
        None
    }

    /// Analyze string arguments for highlighting (e.g. in use/no statements)
    pub(super) fn analyze_string_args(
        &mut self,
        node: &Node,
        args: &[String],
        start_offset: usize,
    ) {
        let mut current_offset = start_offset;
        for arg in args {
            if let Some(offset) = self.find_substring_in_source_after(node, arg, current_offset) {
                self.semantic_tokens.push(SemanticToken {
                    location: SourceLocation { start: offset, end: offset + arg.len() },
                    token_type: SemanticTokenType::String,
                    modifiers: vec![],
                });
                current_offset = offset + arg.len();
            }
        }
    }

    /// Infer the type of a node based on its context and initialization.
    ///
    /// Provides basic type inference for Perl expressions to enhance hover
    /// information with derived type information. Supports common patterns:
    /// - Literal values (numbers, strings, arrays, hashes)
    /// - Variable references (looks up declaration)
    /// - Function calls (basic return type hints)
    ///
    /// In the semantic workflow (Parse -> Index -> Analyze), this method runs
    /// during the Analyze stage and consumes symbols produced during Index.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node to infer type for
    ///
    /// # Returns
    ///
    /// A string describing the inferred type, or None if type cannot be determined
    pub fn infer_type(&self, node: &Node) -> Option<String> {
        match &node.kind {
            NodeKind::Number { .. } => Some("number".to_string()),
            NodeKind::String { .. } => Some("string".to_string()),
            NodeKind::ArrayLiteral { .. } => Some("array".to_string()),
            NodeKind::HashLiteral { .. } => Some("hash".to_string()),

            NodeKind::Variable { sigil, name } => {
                // Look up the variable in the symbol table
                let kind = match sigil.as_str() {
                    "$" => SymbolKind::scalar(),
                    "@" => SymbolKind::array(),
                    "%" => SymbolKind::hash(),
                    _ => return None,
                };

                let symbols = self.symbol_table.find_symbol(name, 0, kind);
                symbols.first()?;

                // Return the basic type based on sigil
                match sigil.as_str() {
                    "$" => Some("scalar".to_string()),
                    "@" => Some("array".to_string()),
                    "%" => Some("hash".to_string()),
                    _ => None,
                }
            }

            NodeKind::FunctionCall { name, .. } => {
                // Basic return type inference for built-in functions
                match name.as_str() {
                    "scalar" => Some("scalar".to_string()),
                    "ref" => Some("string".to_string()),
                    "length" | "index" | "rindex" => Some("number".to_string()),
                    "split" => Some("array".to_string()),
                    "keys" | "values" => Some("array".to_string()),
                    _ => None,
                }
            }

            NodeKind::Binary { op, .. } => {
                // Infer based on operator
                match op.as_str() {
                    "+" | "-" | "*" | "/" | "%" | "**" => Some("number".to_string()),
                    "." | "x" => Some("string".to_string()),
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "eq" | "ne" | "lt" | "gt" | "le"
                    | "ge" => Some("boolean".to_string()),
                    _ => None,
                }
            }

            _ => None,
        }
    }
}

/// Build a parenthesised parameter list string from a `Signature` AST node.
///
/// Extracts each parameter variable's sigil and name and joins them with
/// ", ".  Returns `"(...)"` as a safe fallback for any unrecognised structure.
fn format_signature_params(sig_node: &Node) -> String {
    let NodeKind::Signature { parameters } = &sig_node.kind else {
        return "(...)".to_string();
    };

    let labels: Vec<String> = parameters
        .iter()
        .filter_map(|param| {
            let var = match &param.kind {
                NodeKind::MandatoryParameter { variable }
                | NodeKind::OptionalParameter { variable, .. }
                | NodeKind::SlurpyParameter { variable }
                | NodeKind::NamedParameter { variable } => variable.as_ref(),
                NodeKind::Variable { .. } => param,
                _ => return None,
            };
            if let NodeKind::Variable { sigil, name } = &var.kind {
                Some(format!("{}{}", sigil, name))
            } else {
                None
            }
        })
        .collect();

    format!("({})", labels.join(", "))
}
