//! Abstract Syntax Tree definitions for Perl within the parsing and LSP workflow.
//!
//! This module defines the comprehensive AST node types that represent parsed Perl code
//! during the Parse → Index → Navigate → Complete → Analyze stages. The design is optimized
//! for both direct use in Rust analysis and for generating tree-sitter compatible
//! S-expressions during large workspace processing operations.
//!
//! # LSP Workflow Integration
//!
//! The AST structures support Perl tooling workflows by:
//! - **Parse**: Produced by the parser as the canonical syntax tree
//! - **Index**: Traversed to build symbol and reference tables
//! - **Navigate**: Provides locations for definition and reference lookups
//! - **Complete**: Supplies context for completion, hover, and signature help
//! - **Analyze**: Feeds semantic analysis, diagnostics, and refactoring
//!
//! # Performance Characteristics
//!
//! AST structures are optimized for large codebases with:
//! - Memory-efficient node representation using `Box<Node>` for recursive structures
//! - Fast pattern matching via enum variants for common Perl constructs
//! - Location tracking for precise error reporting in large files
//! - Cheap cloning for parallel analysis tasks
//!
//! # Usage Examples
//!
//! ## Basic AST Construction
//!
//! ```ignore
//! use perl_ast::{Node, NodeKind, SourceLocation};
//!
//! // Create a simple variable declaration node
//! let location = SourceLocation { start: 0, end: 10 };
//! let node = Node::new(
//!     NodeKind::VariableDeclaration {
//!         declarator: "my".to_string(),
//!         variable: Box::new(Node::new(
//!             NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
//!             location
//!         )),
//!         attributes: vec![],
//!         initializer: None,
//!     },
//!     location
//! );
//! ```
//!
//! ## Tree-sitter S-expression Generation
//!
//! ```ignore
//! use crate::{Node, NodeKind};
//!
//! // Example assuming a parser exists
//! // let code = "my $x = 42;";
//! // let mut parser = Parser::new(code);
//! // let ast = parser.parse()?;
//!
//! // // Convert to tree-sitter compatible format
//! // let sexp = ast.to_sexp();
//! // println!("S-expression: {}", sexp);
//! ```
//!
//! ## AST Traversal and Analysis
//!
//! ```ignore
//! use perl_ast::{Node, NodeKind};
//!
//! fn count_variables(node: &Node) -> usize {
//!     let mut count = 0;
//!     match &node.kind {
//!         NodeKind::Variable { .. } => count += 1,
//!         NodeKind::Program { statements } => {
//!             for stmt in statements {
//!                 count += count_variables(stmt);
//!             }
//!         }
//!         _ => {} // Handle other node types as needed
//!     }
//!     count
//! }
//! ```
//!
//! ## LSP Integration Example
//!
//! ```ignore
//! use crate::Node;
//!
//! // Parse Perl code and extract symbols for LSP
//! // let code = "sub hello { my $name = shift; print \"Hello, $name!\\n\"; }";
//! // let mut parser = Parser::new(code);
//! // let ast = parser.parse()?;
//!
//! // Extract symbols for workspace indexing
//! // let extractor = SymbolExtractor::new();
//! // let symbol_table = extractor.extract(&ast);
//!
//! // Use symbols for LSP features like go-to-definition
//! for (name, symbols) in &symbol_table.symbols {
//!     for symbol in symbols {
//!         println!("Found symbol: {} at {:?}", symbol.name, symbol.location);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

// Re-export SourceLocation from perl-position-tracking for unified span handling
pub use perl_position_tracking::SourceLocation;
// Re-export Token and TokenKind from perl-token for AST error nodes
pub use perl_token::{Token, TokenKind};

/// Core AST node representing any Perl language construct within parsing workflows.
///
/// This is the fundamental building block for representing parsed Perl code. Each node
/// contains both the semantic information (kind) and positional information (location)
/// necessary for comprehensive script analysis.
///
/// # LSP Workflow Role
///
/// Nodes flow through tooling stages:
/// - **Parse**: Created by the parser as it builds the syntax tree
/// - **Index**: Visited to build symbol and reference tables
/// - **Navigate**: Used to resolve definitions, references, and call hierarchy
/// - **Complete**: Provides contextual information for completion and hover
/// - **Analyze**: Drives semantic analysis and diagnostics
///
/// # Memory Optimization
///
/// The structure is designed for efficient memory usage during large-scale parsing:
/// - `SourceLocation` uses compact position encoding for large files
/// - `NodeKind` enum variants minimize memory overhead for common constructs
/// - Clone operations are optimized for shared analysis workflows
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The specific type and semantic content of this AST node
    pub kind: NodeKind,
    /// Source position information for error reporting and code navigation
    pub location: SourceLocation,
}

impl Node {
    /// Create a new AST node
    pub fn new(kind: NodeKind, location: SourceLocation) -> Self {
        Node { kind, location }
    }

    /// Convert the AST to a tree-sitter compatible S-expression
    pub fn to_sexp(&self) -> String {
        match &self.kind {
            NodeKind::Program { statements } => {
                let stmts =
                    statements.iter().map(|s| s.to_sexp_inner()).collect::<Vec<_>>().join(" ");
                format!("(source_file {})", stmts)
            }

            NodeKind::ExpressionStatement { expression } => {
                format!("(expression_statement {})", expression.to_sexp())
            }

            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                let attrs_str = if attributes.is_empty() {
                    String::new()
                } else {
                    format!(" (attributes {})", attributes.join(" "))
                };
                if let Some(init) = initializer {
                    format!(
                        "({}_declaration {}{}{})",
                        declarator,
                        variable.to_sexp(),
                        attrs_str,
                        init.to_sexp()
                    )
                } else {
                    format!("({}_declaration {}{})", declarator, variable.to_sexp(), attrs_str)
                }
            }

            NodeKind::VariableListDeclaration {
                declarator,
                variables,
                attributes,
                initializer,
            } => {
                let vars = variables.iter().map(|v| v.to_sexp()).collect::<Vec<_>>().join(" ");
                let attrs_str = if attributes.is_empty() {
                    String::new()
                } else {
                    format!(" (attributes {})", attributes.join(" "))
                };
                if let Some(init) = initializer {
                    format!(
                        "({}_declaration ({}){}{})",
                        declarator,
                        vars,
                        attrs_str,
                        init.to_sexp()
                    )
                } else {
                    format!("({}_declaration ({}){})", declarator, vars, attrs_str)
                }
            }

            NodeKind::Variable { sigil, name } => {
                // Format expected by bless parsing tests: (variable $ name)
                format!("(variable {} {})", sigil, name)
            }

            NodeKind::VariableWithAttributes { variable, attributes } => {
                let attrs = attributes.join(" ");
                format!("({} (attributes {}))", variable.to_sexp(), attrs)
            }

            NodeKind::Assignment { lhs, rhs, op } => {
                format!(
                    "(assignment_{} {} {})",
                    op.replace("=", "assign"),
                    lhs.to_sexp(),
                    rhs.to_sexp()
                )
            }

            NodeKind::Binary { op, left, right } => {
                // Tree-sitter format: (binary_op left right)
                let op_name = format_binary_operator(op);
                format!("({} {} {})", op_name, left.to_sexp(), right.to_sexp())
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                format!(
                    "(ternary {} {} {})",
                    condition.to_sexp(),
                    then_expr.to_sexp(),
                    else_expr.to_sexp()
                )
            }

            NodeKind::Unary { op, operand } => {
                // Tree-sitter format: (unary_op operand)
                let op_name = format_unary_operator(op);
                format!("({} {})", op_name, operand.to_sexp())
            }

            NodeKind::Diamond => "(diamond)".to_string(),

            NodeKind::Ellipsis => "(ellipsis)".to_string(),

            NodeKind::Undef => "(undef)".to_string(),

            NodeKind::Readline { filehandle } => {
                if let Some(fh) = filehandle {
                    format!("(readline {})", fh)
                } else {
                    "(readline)".to_string()
                }
            }

            NodeKind::Glob { pattern } => {
                format!("(glob {})", pattern)
            }
            NodeKind::Typeglob { name } => {
                format!("(typeglob {})", name)
            }

            NodeKind::Number { value } => {
                // Format expected by bless parsing tests: (number value)
                format!("(number {})", value)
            }

            NodeKind::String { value, interpolated } => {
                // Escape quotes in string value to prevent S-expression parsing issues
                let escaped_value = value.replace('\\', "\\\\").replace('"', "\\\"");

                // Format based on interpolation status
                if *interpolated {
                    format!("(string_interpolated \"{}\")", escaped_value)
                } else {
                    format!("(string \"{}\")", escaped_value)
                }
            }

            NodeKind::Heredoc { delimiter, content, interpolated, indented, command, .. } => {
                let type_str = if *command {
                    "heredoc_command"
                } else if *indented {
                    if *interpolated { "heredoc_indented_interpolated" } else { "heredoc_indented" }
                } else if *interpolated {
                    "heredoc_interpolated"
                } else {
                    "heredoc"
                };
                format!("({} {:?} {:?})", type_str, delimiter, content)
            }

            NodeKind::ArrayLiteral { elements } => {
                let elems = elements.iter().map(|e| e.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(array {})", elems)
            }

            NodeKind::HashLiteral { pairs } => {
                let kvs = pairs
                    .iter()
                    .map(|(k, v)| format!("({} {})", k.to_sexp(), v.to_sexp()))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(hash {})", kvs)
            }

            NodeKind::Block { statements } => {
                let stmts = statements.iter().map(|s| s.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(block {})", stmts)
            }

            NodeKind::Eval { block } => {
                format!("(eval {})", block.to_sexp())
            }

            NodeKind::Do { block } => {
                format!("(do {})", block.to_sexp())
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                let mut parts = vec![format!("(try {})", body.to_sexp())];

                for (var, block) in catch_blocks {
                    if let Some(v) = var {
                        parts.push(format!("(catch {} {})", v, block.to_sexp()));
                    } else {
                        parts.push(format!("(catch {})", block.to_sexp()));
                    }
                }

                if let Some(finally) = finally_block {
                    parts.push(format!("(finally {})", finally.to_sexp()));
                }

                parts.join(" ")
            }

            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let mut parts =
                    vec![format!("(if {} {})", condition.to_sexp(), then_branch.to_sexp())];

                for (cond, block) in elsif_branches {
                    parts.push(format!("(elsif {} {})", cond.to_sexp(), block.to_sexp()));
                }

                if let Some(else_block) = else_branch {
                    parts.push(format!("(else {})", else_block.to_sexp()));
                }

                parts.join(" ")
            }

            NodeKind::LabeledStatement { label, statement } => {
                format!("(labeled_statement {} {})", label, statement.to_sexp())
            }

            NodeKind::While { condition, body, continue_block } => {
                let mut s = format!("(while {} {})", condition.to_sexp(), body.to_sexp());
                if let Some(cont) = continue_block {
                    s.push_str(&format!(" (continue {})", cont.to_sexp()));
                }
                s
            }
            NodeKind::Tie { variable, package, args } => {
                let mut s = format!("(tie {} {}", variable.to_sexp(), package.to_sexp());
                for arg in args {
                    s.push_str(&format!(" {}", arg.to_sexp()));
                }
                s.push(')');
                s
            }
            NodeKind::Untie { variable } => {
                format!("(untie {})", variable.to_sexp())
            }
            NodeKind::For { init, condition, update, body, continue_block } => {
                let init_str =
                    init.as_ref().map(|i| i.to_sexp()).unwrap_or_else(|| "()".to_string());
                let cond_str =
                    condition.as_ref().map(|c| c.to_sexp()).unwrap_or_else(|| "()".to_string());
                let update_str =
                    update.as_ref().map(|u| u.to_sexp()).unwrap_or_else(|| "()".to_string());
                let mut result =
                    format!("(for {} {} {} {})", init_str, cond_str, update_str, body.to_sexp());
                if let Some(cont) = continue_block {
                    result.push_str(&format!(" (continue {})", cont.to_sexp()));
                }
                result
            }

            NodeKind::Foreach { variable, list, body, continue_block } => {
                let cont = if let Some(cb) = continue_block {
                    format!(" {}", cb.to_sexp())
                } else {
                    String::new()
                };
                format!(
                    "(foreach {} {} {}{})",
                    variable.to_sexp(),
                    list.to_sexp(),
                    body.to_sexp(),
                    cont
                )
            }

            NodeKind::Given { expr, body } => {
                format!("(given {} {})", expr.to_sexp(), body.to_sexp())
            }

            NodeKind::When { condition, body } => {
                format!("(when {} {})", condition.to_sexp(), body.to_sexp())
            }

            NodeKind::Default { body } => {
                format!("(default {})", body.to_sexp())
            }

            NodeKind::StatementModifier { statement, modifier, condition } => {
                format!(
                    "(statement_modifier_{} {} {})",
                    modifier,
                    statement.to_sexp(),
                    condition.to_sexp()
                )
            }

            NodeKind::Subroutine { name, prototype, signature, attributes, body, name_span: _ } => {
                if let Some(sub_name) = name {
                    // Named subroutine - bless test expected format: (sub name () block)
                    let mut parts = vec![sub_name.clone()];

                    // Add attributes if present (before prototype/signature)
                    if !attributes.is_empty() {
                        for attr in attributes {
                            parts.push(format!(":{}", attr));
                        }
                    }

                    // Add prototype/signature - use () for empty prototype
                    if let Some(proto) = prototype {
                        parts.push(format!("({})", proto.to_sexp()));
                    } else if signature.is_some() {
                        // If there's a signature but no prototype, still show ()
                        parts.push("()".to_string());
                    } else {
                        parts.push("()".to_string());
                    }

                    // Add body
                    parts.push(body.to_sexp());

                    // Format: (sub name [attrs...] ()(block ...)) - space between name and (), no space between () and block
                    if parts.len() >= 3 && parts[parts.len() - 2] == "()" {
                        let name_and_attrs = parts[0..parts.len() - 2].join(" ");
                        let proto = &parts[parts.len() - 2];
                        let body = &parts[parts.len() - 1];
                        format!("(sub {} {}{})", name_and_attrs, proto, body)
                    } else {
                        format!("(sub {})", parts.join(" "))
                    }
                } else {
                    // Anonymous subroutine - tree-sitter format
                    let mut parts = Vec::new();

                    // Add attributes if present
                    if !attributes.is_empty() {
                        let attrs: Vec<String> = attributes
                            .iter()
                            .map(|_attr| "(attribute (attribute_name))".to_string())
                            .collect();
                        parts.push(format!("(attrlist {})", attrs.join("")));
                    }

                    // Add prototype if present
                    if let Some(proto) = prototype {
                        parts.push(proto.to_sexp());
                    }

                    // Add signature if present
                    if let Some(sig) = signature {
                        parts.push(sig.to_sexp());
                    }

                    // Add body
                    parts.push(body.to_sexp());

                    format!("(anonymous_subroutine_expression {})", parts.join(""))
                }
            }

            NodeKind::Prototype { content: _ } => "(prototype)".to_string(),

            NodeKind::Signature { parameters } => {
                let params = parameters.iter().map(|p| p.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(signature {})", params)
            }

            NodeKind::MandatoryParameter { variable } => {
                format!("(mandatory_parameter {})", variable.to_sexp())
            }

            NodeKind::OptionalParameter { variable, default_value } => {
                format!("(optional_parameter {} {})", variable.to_sexp(), default_value.to_sexp())
            }

            NodeKind::SlurpyParameter { variable } => {
                format!("(slurpy_parameter {})", variable.to_sexp())
            }

            NodeKind::NamedParameter { variable } => {
                format!("(named_parameter {})", variable.to_sexp())
            }

            NodeKind::Method { name: _, signature, attributes, body } => {
                let block_contents = match &body.kind {
                    NodeKind::Block { statements } => {
                        statements.iter().map(|s| s.to_sexp()).collect::<Vec<_>>().join(" ")
                    }
                    _ => body.to_sexp(),
                };

                let mut parts = vec!["(bareword)".to_string()];

                // Add signature if present
                if let Some(sig) = signature {
                    parts.push(sig.to_sexp());
                }

                // Add attributes if present
                if !attributes.is_empty() {
                    let attrs: Vec<String> = attributes
                        .iter()
                        .map(|_attr| "(attribute (attribute_name))".to_string())
                        .collect();
                    parts.push(format!("(attrlist {})", attrs.join("")));
                }

                parts.push(format!("(block {})", block_contents));
                format!("(method_declaration_statement {})", parts.join(" "))
            }

            NodeKind::Return { value } => {
                if let Some(val) = value {
                    format!("(return {})", val.to_sexp())
                } else {
                    "(return)".to_string()
                }
            }

            NodeKind::LoopControl { op, label } => {
                if let Some(l) = label {
                    format!("({} {})", op, l)
                } else {
                    format!("({})", op)
                }
            }

            NodeKind::MethodCall { object, method, args } => {
                let args_str = args.iter().map(|a| a.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(method_call {} {} ({}))", object.to_sexp(), method, args_str)
            }

            NodeKind::FunctionCall { name, args } => {
                // Special handling for functions that should use call format in tree-sitter tests
                if matches!(
                    name.as_str(),
                    "bless"
                        | "shift"
                        | "unshift"
                        | "open"
                        | "die"
                        | "warn"
                        | "print"
                        | "printf"
                        | "say"
                        | "push"
                        | "pop"
                        | "map"
                        | "sort"
                        | "grep"
                        | "keys"
                        | "values"
                        | "each"
                        | "defined"
                        | "scalar"
                        | "ref"
                ) {
                    let args_str = args.iter().map(|a| a.to_sexp()).collect::<Vec<_>>().join(" ");
                    if args.is_empty() {
                        format!("(call {} ())", name)
                    } else {
                        format!("(call {} ({}))", name, args_str)
                    }
                } else {
                    // Tree-sitter format varies by context
                    let args_str = args.iter().map(|a| a.to_sexp()).collect::<Vec<_>>().join(" ");
                    if args.is_empty() {
                        "(function_call_expression (function))".to_string()
                    } else {
                        format!("(ambiguous_function_call_expression (function) {})", args_str)
                    }
                }
            }

            NodeKind::IndirectCall { method, object, args } => {
                let args_str = args.iter().map(|a| a.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(indirect_call {} {} ({}))", method, object.to_sexp(), args_str)
            }

            NodeKind::Regex { pattern, replacement, modifiers, has_embedded_code } => {
                let risk_marker = if *has_embedded_code { " (risk:code)" } else { "" };
                format!("(regex {:?} {:?} {:?}{})", pattern, replacement, modifiers, risk_marker)
            }

            NodeKind::Match { expr, pattern, modifiers, has_embedded_code } => {
                let risk_marker = if *has_embedded_code { " (risk:code)" } else { "" };
                format!(
                    "(match {} (regex {:?} {:?}{}))",
                    expr.to_sexp(),
                    pattern,
                    modifiers,
                    risk_marker
                )
            }

            NodeKind::Substitution { expr, pattern, replacement, modifiers, has_embedded_code } => {
                let risk_marker = if *has_embedded_code { " (risk:code)" } else { "" };
                format!(
                    "(substitution {} {:?} {:?} {:?}{})",
                    expr.to_sexp(),
                    pattern,
                    replacement,
                    modifiers,
                    risk_marker
                )
            }

            NodeKind::Transliteration { expr, search, replace, modifiers } => {
                format!(
                    "(transliteration {} {:?} {:?} {:?})",
                    expr.to_sexp(),
                    search,
                    replace,
                    modifiers
                )
            }

            NodeKind::Package { name, block, name_span: _ } => {
                if let Some(blk) = block {
                    format!("(package {} {})", name, blk.to_sexp())
                } else {
                    format!("(package {})", name)
                }
            }

            NodeKind::Use { module, args, has_filter_risk } => {
                let risk_marker = if *has_filter_risk { " (risk:filter)" } else { "" };
                if args.is_empty() {
                    format!("(use {}{})", module, risk_marker)
                } else {
                    let args_str = args.join(" ");
                    format!("(use {} ({}){})", module, args_str, risk_marker)
                }
            }

            NodeKind::No { module, args, has_filter_risk } => {
                let risk_marker = if *has_filter_risk { " (risk:filter)" } else { "" };
                if args.is_empty() {
                    format!("(no {}{})", module, risk_marker)
                } else {
                    let args_str = args.join(" ");
                    format!("(no {} ({}){})", module, args_str, risk_marker)
                }
            }

            NodeKind::PhaseBlock { phase, phase_span: _, block } => {
                format!("({} {})", phase, block.to_sexp())
            }

            NodeKind::DataSection { marker, body } => {
                if let Some(body_text) = body {
                    format!("(data_section {} \"{}\")", marker, body_text.escape_default())
                } else {
                    format!("(data_section {})", marker)
                }
            }

            NodeKind::Class { name, body } => {
                format!("(class {} {})", name, body.to_sexp())
            }

            NodeKind::Format { name, body } => {
                format!("(format {} {:?})", name, body)
            }

            NodeKind::Identifier { name } => {
                // Format expected by tests: (identifier name)
                format!("(identifier {})", name)
            }

            NodeKind::Error { message, partial, .. } => {
                if let Some(node) = partial {
                    format!("(ERROR \"{}\" {})", message.escape_default(), node.to_sexp())
                } else {
                    format!("(ERROR \"{}\")", message.escape_default())
                }
            }
            NodeKind::MissingExpression => "(missing_expression)".to_string(),
            NodeKind::MissingStatement => "(missing_statement)".to_string(),
            NodeKind::MissingIdentifier => "(missing_identifier)".to_string(),
            NodeKind::MissingBlock => "(missing_block)".to_string(),
            NodeKind::UnknownRest => "(UNKNOWN_REST)".to_string(),
        }
    }

    /// Convert the AST to S-expression format that unwraps expression statements in programs
    pub fn to_sexp_inner(&self) -> String {
        match &self.kind {
            NodeKind::ExpressionStatement { expression } => {
                // Check if this is an anonymous subroutine - if so, keep it wrapped
                match &expression.kind {
                    NodeKind::Subroutine { name, .. } if name.is_none() => {
                        // Anonymous subroutine should remain wrapped in expression statement
                        self.to_sexp()
                    }
                    _ => {
                        // In the inner format, other expression statements are unwrapped
                        expression.to_sexp()
                    }
                }
            }
            _ => {
                // For all other node types, use regular to_sexp
                self.to_sexp()
            }
        }
    }

    /// Call a function on every direct child node of this node.
    ///
    /// This enables depth-first traversal for operations like heredoc content attachment.
    /// The closure receives a mutable reference to each child node.
    #[inline]
    pub fn for_each_child_mut<F: FnMut(&mut Node)>(&mut self, mut f: F) {
        match &mut self.kind {
            NodeKind::Tie { variable, package, args } => {
                f(variable);
                f(package);
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::Untie { variable } => f(variable),

            // Root program node
            NodeKind::Program { statements } => {
                for stmt in statements {
                    f(stmt);
                }
            }

            // Statement wrappers
            NodeKind::ExpressionStatement { expression } => f(expression),

            // Variable declarations
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                f(variable);
                if let Some(init) = initializer {
                    f(init);
                }
            }
            NodeKind::VariableListDeclaration { variables, initializer, .. } => {
                for var in variables {
                    f(var);
                }
                if let Some(init) = initializer {
                    f(init);
                }
            }
            NodeKind::VariableWithAttributes { variable, .. } => f(variable),

            // Binary operations
            NodeKind::Binary { left, right, .. } => {
                f(left);
                f(right);
            }
            NodeKind::Ternary { condition, then_expr, else_expr } => {
                f(condition);
                f(then_expr);
                f(else_expr);
            }
            NodeKind::Unary { operand, .. } => f(operand),
            NodeKind::Assignment { lhs, rhs, .. } => {
                f(lhs);
                f(rhs);
            }

            // Control flow
            NodeKind::Block { statements } => {
                for stmt in statements {
                    f(stmt);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
                f(condition);
                f(then_branch);
                for (elsif_cond, elsif_body) in elsif_branches {
                    f(elsif_cond);
                    f(elsif_body);
                }
                if let Some(else_body) = else_branch {
                    f(else_body);
                }
            }
            NodeKind::While { condition, body, continue_block, .. } => {
                f(condition);
                f(body);
                if let Some(cont) = continue_block {
                    f(cont);
                }
            }
            NodeKind::For { init, condition, update, body, continue_block, .. } => {
                if let Some(i) = init {
                    f(i);
                }
                if let Some(c) = condition {
                    f(c);
                }
                if let Some(u) = update {
                    f(u);
                }
                f(body);
                if let Some(cont) = continue_block {
                    f(cont);
                }
            }
            NodeKind::Foreach { variable, list, body, continue_block } => {
                f(variable);
                f(list);
                f(body);
                if let Some(cb) = continue_block {
                    f(cb);
                }
            }
            NodeKind::Given { expr, body } => {
                f(expr);
                f(body);
            }
            NodeKind::When { condition, body } => {
                f(condition);
                f(body);
            }
            NodeKind::Default { body } => f(body),
            NodeKind::StatementModifier { statement, condition, .. } => {
                f(statement);
                f(condition);
            }
            NodeKind::LabeledStatement { statement, .. } => f(statement),

            // Eval and Do blocks
            NodeKind::Eval { block } => f(block),
            NodeKind::Do { block } => f(block),
            NodeKind::Try { body, catch_blocks, finally_block } => {
                f(body);
                for (_, catch_body) in catch_blocks {
                    f(catch_body);
                }
                if let Some(finally) = finally_block {
                    f(finally);
                }
            }

            // Function calls
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::MethodCall { object, args, .. } => {
                f(object);
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::IndirectCall { object, args, .. } => {
                f(object);
                for arg in args {
                    f(arg);
                }
            }

            // Functions
            NodeKind::Subroutine { prototype, signature, body, .. } => {
                if let Some(proto) = prototype {
                    f(proto);
                }
                if let Some(sig) = signature {
                    f(sig);
                }
                f(body);
            }
            NodeKind::Method { signature, body, .. } => {
                if let Some(sig) = signature {
                    f(sig);
                }
                f(body);
            }
            NodeKind::Return { value } => {
                if let Some(v) = value {
                    f(v);
                }
            }
            NodeKind::Signature { parameters } => {
                for param in parameters {
                    f(param);
                }
            }
            NodeKind::MandatoryParameter { variable } => f(variable),
            NodeKind::OptionalParameter { variable, default_value } => {
                f(variable);
                f(default_value);
            }
            NodeKind::SlurpyParameter { variable } => f(variable),
            NodeKind::NamedParameter { variable } => f(variable),

            // Pattern matching
            NodeKind::Match { expr, .. } => f(expr),
            NodeKind::Substitution { expr, .. } => f(expr),
            NodeKind::Transliteration { expr, .. } => f(expr),

            // Containers
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    f(elem);
                }
            }
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    f(key);
                    f(value);
                }
            }

            // Package system
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    f(b);
                }
            }
            NodeKind::PhaseBlock { block, .. } => f(block),
            NodeKind::Class { body, .. } => f(body),

            // Error node might have a partial valid tree
            NodeKind::Error { partial, .. } => {
                if let Some(node) = partial {
                    f(node);
                }
            }

            // Leaf nodes (no children to traverse)
            NodeKind::Variable { .. }
            | NodeKind::Identifier { .. }
            | NodeKind::Number { .. }
            | NodeKind::String { .. }
            | NodeKind::Heredoc { .. }
            | NodeKind::Regex { .. }
            | NodeKind::Readline { .. }
            | NodeKind::Glob { .. }
            | NodeKind::Typeglob { .. }
            | NodeKind::Diamond
            | NodeKind::Ellipsis
            | NodeKind::Undef
            | NodeKind::Use { .. }
            | NodeKind::No { .. }
            | NodeKind::Prototype { .. }
            | NodeKind::DataSection { .. }
            | NodeKind::Format { .. }
            | NodeKind::LoopControl { .. }
            | NodeKind::MissingExpression
            | NodeKind::MissingStatement
            | NodeKind::MissingIdentifier
            | NodeKind::MissingBlock
            | NodeKind::UnknownRest => {}
        }
    }

    /// Call a function on every direct child node of this node (immutable version).
    ///
    /// This enables depth-first traversal for read-only operations like AST analysis.
    /// The closure receives an immutable reference to each child node.
    #[inline]
    pub fn for_each_child<'a, F: FnMut(&'a Node)>(&'a self, mut f: F) {
        match &self.kind {
            NodeKind::Tie { variable, package, args } => {
                f(variable);
                f(package);
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::Untie { variable } => f(variable),

            // Root program node
            NodeKind::Program { statements } => {
                for stmt in statements {
                    f(stmt);
                }
            }

            // Statement wrappers
            NodeKind::ExpressionStatement { expression } => f(expression),

            // Variable declarations
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                f(variable);
                if let Some(init) = initializer {
                    f(init);
                }
            }
            NodeKind::VariableListDeclaration { variables, initializer, .. } => {
                for var in variables {
                    f(var);
                }
                if let Some(init) = initializer {
                    f(init);
                }
            }
            NodeKind::VariableWithAttributes { variable, .. } => f(variable),

            // Binary operations
            NodeKind::Binary { left, right, .. } => {
                f(left);
                f(right);
            }
            NodeKind::Ternary { condition, then_expr, else_expr } => {
                f(condition);
                f(then_expr);
                f(else_expr);
            }
            NodeKind::Unary { operand, .. } => f(operand),
            NodeKind::Assignment { lhs, rhs, .. } => {
                f(lhs);
                f(rhs);
            }

            // Control flow
            NodeKind::Block { statements } => {
                for stmt in statements {
                    f(stmt);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
                f(condition);
                f(then_branch);
                for (elsif_cond, elsif_body) in elsif_branches {
                    f(elsif_cond);
                    f(elsif_body);
                }
                if let Some(else_body) = else_branch {
                    f(else_body);
                }
            }
            NodeKind::While { condition, body, continue_block, .. } => {
                f(condition);
                f(body);
                if let Some(cont) = continue_block {
                    f(cont);
                }
            }
            NodeKind::For { init, condition, update, body, continue_block, .. } => {
                if let Some(i) = init {
                    f(i);
                }
                if let Some(c) = condition {
                    f(c);
                }
                if let Some(u) = update {
                    f(u);
                }
                f(body);
                if let Some(cont) = continue_block {
                    f(cont);
                }
            }
            NodeKind::Foreach { variable, list, body, continue_block } => {
                f(variable);
                f(list);
                f(body);
                if let Some(cb) = continue_block {
                    f(cb);
                }
            }
            NodeKind::Given { expr, body } => {
                f(expr);
                f(body);
            }
            NodeKind::When { condition, body } => {
                f(condition);
                f(body);
            }
            NodeKind::Default { body } => f(body),
            NodeKind::StatementModifier { statement, condition, .. } => {
                f(statement);
                f(condition);
            }
            NodeKind::LabeledStatement { statement, .. } => f(statement),

            // Eval and Do blocks
            NodeKind::Eval { block } => f(block),
            NodeKind::Do { block } => f(block),
            NodeKind::Try { body, catch_blocks, finally_block } => {
                f(body);
                for (_, catch_body) in catch_blocks {
                    f(catch_body);
                }
                if let Some(finally) = finally_block {
                    f(finally);
                }
            }

            // Function calls
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::MethodCall { object, args, .. } => {
                f(object);
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::IndirectCall { object, args, .. } => {
                f(object);
                for arg in args {
                    f(arg);
                }
            }

            // Functions
            NodeKind::Subroutine { prototype, signature, body, .. } => {
                if let Some(proto) = prototype {
                    f(proto);
                }
                if let Some(sig) = signature {
                    f(sig);
                }
                f(body);
            }
            NodeKind::Method { signature, body, .. } => {
                if let Some(sig) = signature {
                    f(sig);
                }
                f(body);
            }
            NodeKind::Return { value } => {
                if let Some(v) = value {
                    f(v);
                }
            }
            NodeKind::Signature { parameters } => {
                for param in parameters {
                    f(param);
                }
            }
            NodeKind::MandatoryParameter { variable } => f(variable),
            NodeKind::OptionalParameter { variable, default_value } => {
                f(variable);
                f(default_value);
            }
            NodeKind::SlurpyParameter { variable } => f(variable),
            NodeKind::NamedParameter { variable } => f(variable),

            // Pattern matching
            NodeKind::Match { expr, .. } => f(expr),
            NodeKind::Substitution { expr, .. } => f(expr),
            NodeKind::Transliteration { expr, .. } => f(expr),

            // Containers
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    f(elem);
                }
            }
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    f(key);
                    f(value);
                }
            }

            // Package system
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    f(b);
                }
            }
            NodeKind::PhaseBlock { block, .. } => f(block),
            NodeKind::Class { body, .. } => f(body),

            // Error node might have a partial valid tree
            NodeKind::Error { partial, .. } => {
                if let Some(node) = partial {
                    f(node);
                }
            }

            // Leaf nodes (no children to traverse)
            NodeKind::Variable { .. }
            | NodeKind::Identifier { .. }
            | NodeKind::Number { .. }
            | NodeKind::String { .. }
            | NodeKind::Heredoc { .. }
            | NodeKind::Regex { .. }
            | NodeKind::Readline { .. }
            | NodeKind::Glob { .. }
            | NodeKind::Typeglob { .. }
            | NodeKind::Diamond
            | NodeKind::Ellipsis
            | NodeKind::Undef
            | NodeKind::Use { .. }
            | NodeKind::No { .. }
            | NodeKind::Prototype { .. }
            | NodeKind::DataSection { .. }
            | NodeKind::Format { .. }
            | NodeKind::LoopControl { .. }
            | NodeKind::MissingExpression
            | NodeKind::MissingStatement
            | NodeKind::MissingIdentifier
            | NodeKind::MissingBlock
            | NodeKind::UnknownRest => {}
        }
    }

    /// Count the total number of nodes in this subtree (inclusive).
    pub fn count_nodes(&self) -> usize {
        let mut count = 1;
        self.for_each_child(|child| {
            count += child.count_nodes();
        });
        count
    }

    /// Collect direct child nodes into a vector for convenience APIs.
    #[inline]
    pub fn children(&self) -> Vec<&Node> {
        let mut children = Vec::new();
        self.for_each_child(|child| children.push(child));
        children
    }

    /// Get the first direct child node, if any.
    ///
    /// Optimized to avoid allocating the children vector.
    #[inline]
    pub fn first_child(&self) -> Option<&Node> {
        let mut result = None;
        self.for_each_child(|child| {
            if result.is_none() {
                result = Some(child);
            }
        });
        result
    }
}

/// Comprehensive enumeration of all Perl language constructs supported by the parser.
///
/// This enum represents every possible AST node type that can be parsed from Perl code
/// during the Parse → Index → Navigate → Complete → Analyze workflow. Each variant captures
/// the semantic meaning and structural relationships needed for complete script analysis
/// and transformation.
///
/// # LSP Workflow Integration
///
/// Node kinds are processed differently across workflow stages:
/// - **Parse**: All variants are produced by the parser
/// - **Index**: Symbol-bearing variants feed workspace indexing
/// - **Navigate**: Call and reference variants support navigation features
/// - **Complete**: Expression variants provide completion context
/// - **Analyze**: Semantic variants drive diagnostics and refactoring
///
/// # Performance Considerations
///
/// The enum design optimizes for large codebases:
/// - Box pointers minimize stack usage for recursive structures
/// - Vector storage enables efficient bulk operations on child nodes
/// - Clone operations optimized for concurrent analysis workflows
/// - Pattern matching performance tuned for common Perl constructs
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// Top-level program containing all statements in an Perl script
    ///
    /// This is the root node for any parsed Perl script content, containing all
    /// top-level statements found during the Parse stage of LSP workflow.
    Program {
        /// All top-level statements in the Perl script
        statements: Vec<Node>,
    },

    /// Statement wrapper for expressions that appear at statement level
    ///
    /// Used during Analyze stage to distinguish between expressions used as
    /// statements versus expressions within other contexts during Perl parsing.
    ExpressionStatement {
        /// The expression being used as a statement
        expression: Box<Node>,
    },

    /// Variable declaration with scope declarator in Perl script processing
    ///
    /// Represents declarations like `my $var`, `our $global`, `local $dynamic`, etc.
    /// Critical for Analyze stage symbol table construction during Perl parsing.
    VariableDeclaration {
        /// Scope declarator: "my", "our", "local", "state"
        declarator: String,
        /// The variable being declared
        variable: Box<Node>,
        /// Variable attributes (e.g., ":shared", ":locked")
        attributes: Vec<String>,
        /// Optional initializer expression
        initializer: Option<Box<Node>>,
    },

    /// Multiple variable declaration in a single statement
    ///
    /// Handles constructs like `my ($x, $y) = @values` common in Perl script processing.
    /// Supports efficient bulk variable analysis during Navigate stage operations.
    VariableListDeclaration {
        /// Scope declarator for all variables in the list
        declarator: String,
        /// All variables being declared in the list
        variables: Vec<Node>,
        /// Attributes applied to the variable list
        attributes: Vec<String>,
        /// Optional initializer for the entire variable list
        initializer: Option<Box<Node>>,
    },

    /// Perl variable reference (scalar, array, hash, etc.) in Perl parsing workflow
    Variable {
        /// Variable sigil indicating type: $, @, %, &, *
        sigil: String, // $, @, %, &, *
        /// Variable name without sigil
        name: String,
    },

    /// Variable with additional attributes for enhanced LSP workflow
    VariableWithAttributes {
        /// The base variable node
        variable: Box<Node>,
        /// List of attribute names applied to the variable
        attributes: Vec<String>,
    },

    /// Assignment operation for LSP data processing workflows
    Assignment {
        /// Left-hand side of assignment
        lhs: Box<Node>,
        /// Right-hand side of assignment
        rhs: Box<Node>,
        /// Assignment operator: =, +=, -=, etc.
        op: String, // =, +=, -=, etc.
    },

    // Expressions
    /// Binary operation for Perl parsing workflow calculations
    Binary {
        /// Binary operator
        op: String,
        /// Left operand
        left: Box<Node>,
        /// Right operand
        right: Box<Node>,
    },

    /// Ternary conditional expression for Perl parsing workflow logic
    Ternary {
        /// Condition to evaluate
        condition: Box<Node>,
        /// Expression when condition is true
        then_expr: Box<Node>,
        /// Expression when condition is false
        else_expr: Box<Node>,
    },

    /// Unary operation for Perl parsing workflow
    Unary {
        /// Unary operator
        op: String,
        /// Operand to apply operator to
        operand: Box<Node>,
    },

    // I/O operations
    /// Diamond operator for file input in Perl parsing workflow
    Diamond, // <>

    /// Ellipsis operator for Perl parsing workflow
    Ellipsis, // ...

    /// Undef value for Perl parsing workflow
    Undef, // undef

    /// Readline operation for LSP file processing
    Readline {
        /// Optional filehandle: `<STDIN>`, `<$fh>`, etc.
        filehandle: Option<String>, // <STDIN>, <$fh>, etc.
    },

    /// Glob pattern for LSP workspace file matching
    Glob {
        /// Pattern string for file matching
        pattern: String, // <*.txt>
    },

    /// Typeglob expression: `*foo` or `*main::bar`
    ///
    /// Provides access to all symbol table entries for a given name.
    Typeglob {
        /// Name of the symbol (including package qualification)
        name: String,
    },

    /// Numeric literal in Perl code (integer, float, hex, octal, binary)
    ///
    /// Represents all numeric literal forms: `42`, `3.14`, `0x1A`, `0o755`, `0b1010`.
    Number {
        /// String representation preserving original format
        value: String,
    },

    /// String literal with optional interpolation
    ///
    /// Handles both single-quoted (`'literal'`) and double-quoted (`"$interpolated"`) strings.
    String {
        /// String content (after quote processing)
        value: String,
        /// Whether the string supports variable interpolation
        interpolated: bool,
    },

    /// Heredoc string literal for multi-line content
    ///
    /// Supports all heredoc forms: `<<EOF`, `<<'EOF'`, `<<"EOF"`, `<<~EOF` (indented).
    Heredoc {
        /// Delimiter marking heredoc boundaries
        delimiter: String,
        /// Content between delimiters
        content: String,
        /// Whether content supports variable interpolation
        interpolated: bool,
        /// Whether leading whitespace is stripped (<<~ form)
        indented: bool,
        /// Whether this is a command execution heredoc (<<`EOF`)
        command: bool,
        /// Body span for breakpoint detection (populated by drain_pending_heredocs)
        body_span: Option<SourceLocation>,
    },

    /// Array literal expression: `(1, 2, 3)` or `[1, 2, 3]`
    ArrayLiteral {
        /// Elements in the array
        elements: Vec<Node>,
    },

    /// Hash literal expression: `(key => 'value')` or `{key => 'value'}`
    HashLiteral {
        /// Key-value pairs in the hash
        pairs: Vec<(Node, Node)>,
    },

    /// Block of statements: `{ ... }`
    ///
    /// Used for control structures, subroutine bodies, and bare blocks.
    Block {
        /// Statements within the block
        statements: Vec<Node>,
    },

    /// Eval block for exception handling: `eval { ... }`
    Eval {
        /// Block to evaluate with exception trapping
        block: Box<Node>,
    },

    /// Do block for file inclusion or expression evaluation: `do { ... }` or `do "file"`
    Do {
        /// Block to execute or file expression
        block: Box<Node>,
    },

    /// Try-catch-finally for modern exception handling (Syntax::Keyword::Try style)
    Try {
        /// Try block body
        body: Box<Node>,
        /// Catch blocks: (optional exception variable, handler block)
        catch_blocks: Vec<(Option<String>, Box<Node>)>,
        /// Optional finally block
        finally_block: Option<Box<Node>>,
    },

    /// If-elsif-else conditional statement
    If {
        /// Condition expression
        condition: Box<Node>,
        /// Then branch block
        then_branch: Box<Node>,
        /// Elsif branches: (condition, block) pairs
        elsif_branches: Vec<(Box<Node>, Box<Node>)>,
        /// Optional else branch
        else_branch: Option<Box<Node>>,
    },

    /// Statement with a label for loop control: `LABEL: while (...)`
    LabeledStatement {
        /// Label name (e.g., "OUTER", "LINE")
        label: String,
        /// Labeled statement (typically a loop)
        statement: Box<Node>,
    },

    /// While loop: `while (condition) { ... }`
    While {
        /// Loop condition
        condition: Box<Node>,
        /// Loop body
        body: Box<Node>,
        /// Optional continue block
        continue_block: Option<Box<Node>>,
    },

    /// Tie operation for binding variables to objects: `tie %hash, 'Package', @args`
    Tie {
        /// Variable being tied
        variable: Box<Node>,
        /// Class/package name to tie to
        package: Box<Node>,
        /// Arguments passed to TIE* method
        args: Vec<Node>,
    },

    /// Untie operation for unbinding variables: `untie %hash`
    Untie {
        /// Variable being untied
        variable: Box<Node>,
    },

    /// C-style for loop: `for (init; cond; update) { ... }`
    For {
        /// Initialization expression
        init: Option<Box<Node>>,
        /// Loop condition
        condition: Option<Box<Node>>,
        /// Update expression
        update: Option<Box<Node>>,
        /// Loop body
        body: Box<Node>,
        /// Optional continue block
        continue_block: Option<Box<Node>>,
    },

    /// Foreach loop: `foreach my $item (@list) { ... }`
    Foreach {
        /// Iterator variable
        variable: Box<Node>,
        /// List to iterate
        list: Box<Node>,
        /// Loop body
        body: Box<Node>,
        /// Optional continue block
        continue_block: Option<Box<Node>>,
    },

    /// Given statement for switch-like matching (Perl 5.10+)
    Given {
        /// Expression to match against
        expr: Box<Node>,
        /// Body containing when/default blocks
        body: Box<Node>,
    },

    /// When clause in given/switch: `when ($pattern) { ... }`
    When {
        /// Pattern to match
        condition: Box<Node>,
        /// Handler block
        body: Box<Node>,
    },

    /// Default clause in given/switch: `default { ... }`
    Default {
        /// Handler block for unmatched cases
        body: Box<Node>,
    },

    /// Statement modifier syntax: `print "ok" if $condition`
    StatementModifier {
        /// Statement to conditionally execute
        statement: Box<Node>,
        /// Modifier keyword: if, unless, while, until, for, foreach
        modifier: String,
        /// Modifier condition
        condition: Box<Node>,
    },

    // Functions
    /// Subroutine declaration (function) including name, prototype, signature and body.
    Subroutine {
        /// Name of the subroutine
        ///
        /// # Precise Navigation Support
        /// - Added name_span for exact LSP navigation
        /// - Enables precise go-to-definition and hover behavior
        /// - O(1) span lookup in workspace symbols
        ///
        /// ## Integration Points
        /// - Semantic token providers
        /// - Cross-reference generation
        /// - Symbol renaming
        name: Option<String>,

        /// Source location span of the subroutine name
        ///
        /// ## Usage Notes
        /// - Always corresponds to the name field
        /// - Provides constant-time position information
        /// - Essential for precise editor interactions
        name_span: Option<SourceLocation>,

        /// Optional prototype node (e.g. `($;@)`).
        prototype: Option<Box<Node>>,
        /// Optional signature node (Perl 5.20+ feature).
        signature: Option<Box<Node>>,
        /// Attributes attached to the subroutine (`:lvalue`, etc.).
        attributes: Vec<String>,
        /// The body block of the subroutine.
        body: Box<Node>,
    },

    /// Subroutine prototype specification: `sub foo ($;@) { ... }`
    Prototype {
        /// Prototype string defining argument behavior
        content: String,
    },

    /// Subroutine signature (Perl 5.20+): `sub foo ($x, $y = 0) { ... }`
    Signature {
        /// List of signature parameters
        parameters: Vec<Node>,
    },

    /// Mandatory signature parameter: `$x` in `sub foo ($x) { }`
    MandatoryParameter {
        /// Variable being bound
        variable: Box<Node>,
    },

    /// Optional signature parameter with default: `$y = 0` in `sub foo ($y = 0) { }`
    OptionalParameter {
        /// Variable being bound
        variable: Box<Node>,
        /// Default value expression
        default_value: Box<Node>,
    },

    /// Slurpy parameter collecting remaining args: `@rest` or `%opts` in signature
    SlurpyParameter {
        /// Array or hash variable to receive remaining arguments
        variable: Box<Node>,
    },

    /// Named parameter placeholder in signature (future Perl feature)
    NamedParameter {
        /// Variable for named parameter binding
        variable: Box<Node>,
    },

    /// Method declaration (Perl 5.38+ with `use feature 'class'`)
    Method {
        /// Method name
        name: String,
        /// Optional signature
        signature: Option<Box<Node>>,
        /// Method attributes (e.g., `:lvalue`)
        attributes: Vec<String>,
        /// Method body
        body: Box<Node>,
    },

    /// Return statement: `return;` or `return $value;`
    Return {
        /// Optional return value
        value: Option<Box<Node>>,
    },

    /// Loop control statement: `next`, `last`, or `redo`
    LoopControl {
        /// Control keyword: "next", "last", or "redo"
        op: String,
        /// Optional label: `next LABEL`
        label: Option<String>,
    },

    /// Method call: `$obj->method(@args)` or `$obj->method`
    MethodCall {
        /// Object or class expression
        object: Box<Node>,
        /// Method name being called
        method: String,
        /// Method arguments
        args: Vec<Node>,
    },

    /// Function call: `foo(@args)` or `foo()`
    FunctionCall {
        /// Function name (may be qualified: `Package::func`)
        name: String,
        /// Function arguments
        args: Vec<Node>,
    },

    /// Indirect object call (legacy syntax): `new Class @args`
    IndirectCall {
        /// Method name
        method: String,
        /// Object or class
        object: Box<Node>,
        /// Arguments
        args: Vec<Node>,
    },

    /// Regex literal: `/pattern/modifiers` or `qr/pattern/modifiers`
    Regex {
        /// Regular expression pattern
        pattern: String,
        /// Replacement string (for s/// when parsed as regex)
        replacement: Option<String>,
        /// Regex modifiers (i, m, s, x, g, etc.)
        modifiers: String,
        /// Whether the regex contains embedded code `(?{...})`
        has_embedded_code: bool,
    },

    /// Match operation: `$str =~ /pattern/modifiers`
    Match {
        /// Expression to match against
        expr: Box<Node>,
        /// Pattern to match
        pattern: String,
        /// Match modifiers
        modifiers: String,
        /// Whether the regex contains embedded code `(?{...})`
        has_embedded_code: bool,
    },

    /// Substitution operation: `$str =~ s/pattern/replacement/modifiers`
    Substitution {
        /// Expression to substitute in
        expr: Box<Node>,
        /// Pattern to find
        pattern: String,
        /// Replacement string
        replacement: String,
        /// Substitution modifiers (g, e, r, etc.)
        modifiers: String,
        /// Whether the regex contains embedded code `(?{...})`
        has_embedded_code: bool,
    },

    /// Transliteration operation: `$str =~ tr/search/replace/` or `y///`
    Transliteration {
        /// Expression to transliterate
        expr: Box<Node>,
        /// Characters to search for
        search: String,
        /// Replacement characters
        replace: String,
        /// Transliteration modifiers (c, d, s, r)
        modifiers: String,
    },

    // Package system
    /// Package declaration (e.g. `package Foo;`) and optional inline block form.
    Package {
        /// Name of the package
        ///
        /// # Precise Navigation Support
        /// - Added name_span for exact LSP navigation
        /// - Enables precise go-to-definition and hover behavior
        /// - O(1) span lookup in workspace symbols
        ///
        /// ## Integration Points
        /// - Workspace indexing
        /// - Cross-module symbol resolution
        /// - Code action providers
        name: String,

        /// Source location span of the package name
        ///
        /// ## Usage Notes
        /// - Always corresponds to the name field
        /// - Provides constant-time position information
        /// - Essential for precise editor interactions
        name_span: SourceLocation,

        /// Optional inline block for `package Foo { ... }` declarations.
        block: Option<Box<Node>>,
    },

    /// Use statement for module loading: `use Module qw(imports);`
    Use {
        /// Module name to load
        module: String,
        /// Import arguments (symbols to import)
        args: Vec<String>,
        /// Whether this module is a known source filter (security risk)
        has_filter_risk: bool,
    },

    /// No statement for disabling features: `no strict;`
    No {
        /// Module/pragma name to disable
        module: String,
        /// Arguments for the no statement
        args: Vec<String>,
        /// Whether this module is a known source filter (security risk)
        has_filter_risk: bool,
    },

    /// Phase block for compile/runtime hooks: `BEGIN`, `END`, `CHECK`, `INIT`, `UNITCHECK`
    PhaseBlock {
        /// Phase name: BEGIN, END, CHECK, INIT, UNITCHECK
        phase: String,
        /// Source location span of the phase block name for precise navigation
        phase_span: Option<SourceLocation>,
        /// Block to execute during the specified phase
        block: Box<Node>,
    },

    /// Data section marker: `__DATA__` or `__END__`
    DataSection {
        /// Section marker (__DATA__ or __END__)
        marker: String,
        /// Content following the marker (if any)
        body: Option<String>,
    },

    /// Class declaration (Perl 5.38+ with `use feature 'class'`)
    Class {
        /// Class name
        name: String,
        /// Class body containing methods and attributes
        body: Box<Node>,
    },

    /// Format declaration for legacy report generation
    Format {
        /// Format name (defaults to filehandle name)
        name: String,
        /// Format specification body
        body: String,
    },

    /// Bare identifier (bareword or package-qualified name)
    Identifier {
        /// Identifier string
        name: String,
    },

    /// Parse error placeholder with error message and recovery context
    Error {
        /// Error description
        message: String,
        /// Expected token types (if any)
        expected: Vec<TokenKind>,
        /// The token actually found (if any)
        found: Option<Token>,
        /// Partial AST node parsed before error (if any)
        partial: Option<Box<Node>>,
    },

    /// Missing expression where one was expected
    MissingExpression,
    /// Missing statement where one was expected
    MissingStatement,
    /// Missing identifier where one was expected
    MissingIdentifier,
    /// Missing block where one was expected
    MissingBlock,

    /// Lexer budget exceeded marker preserving partial parse results
    ///
    /// Used when recursion or token limits are hit to preserve already-parsed content.
    UnknownRest,
}

impl NodeKind {
    /// Get the name of this NodeKind as a static string
    pub fn kind_name(&self) -> &'static str {
        match self {
            NodeKind::Program { .. } => "Program",
            NodeKind::ExpressionStatement { .. } => "ExpressionStatement",
            NodeKind::VariableDeclaration { .. } => "VariableDeclaration",
            NodeKind::VariableListDeclaration { .. } => "VariableListDeclaration",
            NodeKind::Variable { .. } => "Variable",
            NodeKind::VariableWithAttributes { .. } => "VariableWithAttributes",
            NodeKind::Assignment { .. } => "Assignment",
            NodeKind::Binary { .. } => "Binary",
            NodeKind::Ternary { .. } => "Ternary",
            NodeKind::Unary { .. } => "Unary",
            NodeKind::Diamond => "Diamond",
            NodeKind::Ellipsis => "Ellipsis",
            NodeKind::Undef => "Undef",
            NodeKind::Readline { .. } => "Readline",
            NodeKind::Glob { .. } => "Glob",
            NodeKind::Typeglob { .. } => "Typeglob",
            NodeKind::Number { .. } => "Number",
            NodeKind::String { .. } => "String",
            NodeKind::Heredoc { .. } => "Heredoc",
            NodeKind::ArrayLiteral { .. } => "ArrayLiteral",
            NodeKind::HashLiteral { .. } => "HashLiteral",
            NodeKind::Block { .. } => "Block",
            NodeKind::Eval { .. } => "Eval",
            NodeKind::Do { .. } => "Do",
            NodeKind::Try { .. } => "Try",
            NodeKind::If { .. } => "If",
            NodeKind::LabeledStatement { .. } => "LabeledStatement",
            NodeKind::While { .. } => "While",
            NodeKind::Tie { .. } => "Tie",
            NodeKind::Untie { .. } => "Untie",
            NodeKind::For { .. } => "For",
            NodeKind::Foreach { .. } => "Foreach",
            NodeKind::Given { .. } => "Given",
            NodeKind::When { .. } => "When",
            NodeKind::Default { .. } => "Default",
            NodeKind::StatementModifier { .. } => "StatementModifier",
            NodeKind::Subroutine { .. } => "Subroutine",
            NodeKind::Prototype { .. } => "Prototype",
            NodeKind::Signature { .. } => "Signature",
            NodeKind::MandatoryParameter { .. } => "MandatoryParameter",
            NodeKind::OptionalParameter { .. } => "OptionalParameter",
            NodeKind::SlurpyParameter { .. } => "SlurpyParameter",
            NodeKind::NamedParameter { .. } => "NamedParameter",
            NodeKind::Method { .. } => "Method",
            NodeKind::Return { .. } => "Return",
            NodeKind::LoopControl { .. } => "LoopControl",
            NodeKind::MethodCall { .. } => "MethodCall",
            NodeKind::FunctionCall { .. } => "FunctionCall",
            NodeKind::IndirectCall { .. } => "IndirectCall",
            NodeKind::Regex { .. } => "Regex",
            NodeKind::Match { .. } => "Match",
            NodeKind::Substitution { .. } => "Substitution",
            NodeKind::Transliteration { .. } => "Transliteration",
            NodeKind::Package { .. } => "Package",
            NodeKind::Use { .. } => "Use",
            NodeKind::No { .. } => "No",
            NodeKind::PhaseBlock { .. } => "PhaseBlock",
            NodeKind::DataSection { .. } => "DataSection",
            NodeKind::Class { .. } => "Class",
            NodeKind::Format { .. } => "Format",
            NodeKind::Identifier { .. } => "Identifier",
            NodeKind::Error { .. } => "Error",
            NodeKind::MissingExpression => "MissingExpression",
            NodeKind::MissingStatement => "MissingStatement",
            NodeKind::MissingIdentifier => "MissingIdentifier",
            NodeKind::MissingBlock => "MissingBlock",
            NodeKind::UnknownRest => "UnknownRest",
        }
    }

    /// Canonical list of **all** `kind_name()` strings, in alphabetical order.
    ///
    /// Every consumer that needs the full set of NodeKind names should reference
    /// this constant instead of maintaining a hand-written copy.
    pub const ALL_KIND_NAMES: &[&'static str] = &[
        "ArrayLiteral",
        "Assignment",
        "Binary",
        "Block",
        "Class",
        "DataSection",
        "Default",
        "Diamond",
        "Do",
        "Ellipsis",
        "Error",
        "Eval",
        "ExpressionStatement",
        "For",
        "Foreach",
        "Format",
        "FunctionCall",
        "Given",
        "Glob",
        "HashLiteral",
        "Heredoc",
        "Identifier",
        "If",
        "IndirectCall",
        "LabeledStatement",
        "LoopControl",
        "MandatoryParameter",
        "Match",
        "Method",
        "MethodCall",
        "MissingBlock",
        "MissingExpression",
        "MissingIdentifier",
        "MissingStatement",
        "NamedParameter",
        "No",
        "Number",
        "OptionalParameter",
        "Package",
        "PhaseBlock",
        "Program",
        "Prototype",
        "Readline",
        "Regex",
        "Return",
        "Signature",
        "SlurpyParameter",
        "StatementModifier",
        "String",
        "Subroutine",
        "Substitution",
        "Ternary",
        "Tie",
        "Transliteration",
        "Try",
        "Typeglob",
        "Unary",
        "Undef",
        "UnknownRest",
        "Untie",
        "Use",
        "Variable",
        "VariableDeclaration",
        "VariableListDeclaration",
        "VariableWithAttributes",
        "When",
        "While",
    ];

    /// Subset of `ALL_KIND_NAMES` that represent synthetic/recovery nodes.
    ///
    /// These kinds are only produced by `parse_with_recovery()` on malformed
    /// input and should not be expected in clean parses.
    pub const RECOVERY_KIND_NAMES: &[&'static str] = &[
        "Error",
        "MissingBlock",
        "MissingExpression",
        "MissingIdentifier",
        "MissingStatement",
        "UnknownRest",
    ];
}

/// Format unary operator for S-expression output
fn format_unary_operator(op: &str) -> String {
    match op {
        // Arithmetic unary operators
        "+" => "unary_+".to_string(),
        "-" => "unary_-".to_string(),

        // Logical unary operators
        "!" => "unary_not".to_string(),
        "not" => "unary_not".to_string(),

        // Bitwise complement
        "~" => "unary_complement".to_string(),

        // Reference operator
        "\\" => "unary_ref".to_string(),

        // Postfix operators
        "++" => "unary_++".to_string(),
        "--" => "unary_--".to_string(),

        // File test operators
        "-f" => "unary_-f".to_string(),
        "-d" => "unary_-d".to_string(),
        "-e" => "unary_-e".to_string(),
        "-r" => "unary_-r".to_string(),
        "-w" => "unary_-w".to_string(),
        "-x" => "unary_-x".to_string(),
        "-o" => "unary_-o".to_string(),
        "-R" => "unary_-R".to_string(),
        "-W" => "unary_-W".to_string(),
        "-X" => "unary_-X".to_string(),
        "-O" => "unary_-O".to_string(),
        "-s" => "unary_-s".to_string(),
        "-p" => "unary_-p".to_string(),
        "-S" => "unary_-S".to_string(),
        "-b" => "unary_-b".to_string(),
        "-c" => "unary_-c".to_string(),
        "-t" => "unary_-t".to_string(),
        "-u" => "unary_-u".to_string(),
        "-g" => "unary_-g".to_string(),
        "-k" => "unary_-k".to_string(),
        "-T" => "unary_-T".to_string(),
        "-B" => "unary_-B".to_string(),
        "-M" => "unary_-M".to_string(),
        "-A" => "unary_-A".to_string(),
        "-C" => "unary_-C".to_string(),
        "-l" => "unary_-l".to_string(),
        "-z" => "unary_-z".to_string(),

        // Postfix dereferencing
        "->@*" => "unary_->@*".to_string(),
        "->%*" => "unary_->%*".to_string(),
        "->$*" => "unary_->$*".to_string(),
        "->&*" => "unary_->&*".to_string(),
        "->**" => "unary_->**".to_string(),

        // Defined operator
        "defined" => "unary_defined".to_string(),

        // Default case for unknown operators
        _ => format!("unary_{}", op.replace(' ', "_")),
    }
}

/// Format binary operator for S-expression output
fn format_binary_operator(op: &str) -> String {
    match op {
        // Arithmetic operators
        "+" => "binary_+".to_string(),
        "-" => "binary_-".to_string(),
        "*" => "binary_*".to_string(),
        "/" => "binary_/".to_string(),
        "%" => "binary_%".to_string(),
        "**" => "binary_**".to_string(),

        // Comparison operators
        "==" => "binary_==".to_string(),
        "!=" => "binary_!=".to_string(),
        "<" => "binary_<".to_string(),
        ">" => "binary_>".to_string(),
        "<=" => "binary_<=".to_string(),
        ">=" => "binary_>=".to_string(),
        "<=>" => "binary_<=>".to_string(),

        // String comparison
        "eq" => "binary_eq".to_string(),
        "ne" => "binary_ne".to_string(),
        "lt" => "binary_lt".to_string(),
        "le" => "binary_le".to_string(),
        "gt" => "binary_gt".to_string(),
        "ge" => "binary_ge".to_string(),
        "cmp" => "binary_cmp".to_string(),

        // Logical operators
        "&&" => "binary_&&".to_string(),
        "||" => "binary_||".to_string(),
        "and" => "binary_and".to_string(),
        "or" => "binary_or".to_string(),
        "xor" => "binary_xor".to_string(),

        // Bitwise operators
        "&" => "binary_&".to_string(),
        "|" => "binary_|".to_string(),
        "^" => "binary_^".to_string(),
        "<<" => "binary_<<".to_string(),
        ">>" => "binary_>>".to_string(),

        // Pattern matching
        "=~" => "binary_=~".to_string(),
        "!~" => "binary_!~".to_string(),

        // Smart match
        "~~" => "binary_~~".to_string(),

        // Concatenation
        "." => "binary_.".to_string(),

        // Range operators
        ".." => "binary_..".to_string(),
        "..." => "binary_...".to_string(),

        // Type checking
        "isa" => "binary_isa".to_string(),

        // Assignment operators
        "=" => "binary_=".to_string(),
        "+=" => "binary_+=".to_string(),
        "-=" => "binary_-=".to_string(),
        "*=" => "binary_*=".to_string(),
        "/=" => "binary_/=".to_string(),
        "%=" => "binary_%=".to_string(),
        "**=" => "binary_**=".to_string(),
        ".=" => "binary_.=".to_string(),
        "&=" => "binary_&=".to_string(),
        "|=" => "binary_|=".to_string(),
        "^=" => "binary_^=".to_string(),
        "<<=" => "binary_<<=".to_string(),
        ">>=" => "binary_>>=".to_string(),
        "&&=" => "binary_&&=".to_string(),
        "||=" => "binary_||=".to_string(),
        "//=" => "binary_//=".to_string(),

        // Defined-or operator
        "//" => "binary_//".to_string(),

        // Method calls and dereferencing
        "->" => "binary_->".to_string(),

        // Hash/array access
        "{}" => "binary_{}".to_string(),
        "[]" => "binary_[]".to_string(),

        // Default case for unknown operators
        _ => format!("binary_{}", op.replace(' ', "_")),
    }
}

// SourceLocation is now provided by perl-position-tracking crate
// See the re-export at the top of this file

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Build a dummy instance for every `NodeKind` variant and return its
    /// `kind_name()`.  This ensures the compiler forces us to update here
    /// whenever a variant is added/removed.
    fn all_kind_names_from_variants() -> BTreeSet<&'static str> {
        let loc = SourceLocation { start: 0, end: 0 };
        let dummy_node = || Node::new(NodeKind::Undef, loc);

        let variants: Vec<NodeKind> = vec![
            NodeKind::Program { statements: vec![] },
            NodeKind::ExpressionStatement { expression: Box::new(dummy_node()) },
            NodeKind::VariableDeclaration {
                declarator: String::new(),
                variable: Box::new(dummy_node()),
                attributes: vec![],
                initializer: None,
            },
            NodeKind::VariableListDeclaration {
                declarator: String::new(),
                variables: vec![],
                attributes: vec![],
                initializer: None,
            },
            NodeKind::Variable { sigil: String::new(), name: String::new() },
            NodeKind::VariableWithAttributes {
                variable: Box::new(dummy_node()),
                attributes: vec![],
            },
            NodeKind::Assignment {
                lhs: Box::new(dummy_node()),
                rhs: Box::new(dummy_node()),
                op: String::new(),
            },
            NodeKind::Binary {
                op: String::new(),
                left: Box::new(dummy_node()),
                right: Box::new(dummy_node()),
            },
            NodeKind::Ternary {
                condition: Box::new(dummy_node()),
                then_expr: Box::new(dummy_node()),
                else_expr: Box::new(dummy_node()),
            },
            NodeKind::Unary { op: String::new(), operand: Box::new(dummy_node()) },
            NodeKind::Diamond,
            NodeKind::Ellipsis,
            NodeKind::Undef,
            NodeKind::Readline { filehandle: None },
            NodeKind::Glob { pattern: String::new() },
            NodeKind::Typeglob { name: String::new() },
            NodeKind::Number { value: String::new() },
            NodeKind::String { value: String::new(), interpolated: false },
            NodeKind::Heredoc {
                delimiter: String::new(),
                content: String::new(),
                interpolated: false,
                indented: false,
                command: false,
                body_span: None,
            },
            NodeKind::ArrayLiteral { elements: vec![] },
            NodeKind::HashLiteral { pairs: vec![] },
            NodeKind::Block { statements: vec![] },
            NodeKind::Eval { block: Box::new(dummy_node()) },
            NodeKind::Do { block: Box::new(dummy_node()) },
            NodeKind::Try {
                body: Box::new(dummy_node()),
                catch_blocks: vec![],
                finally_block: None,
            },
            NodeKind::If {
                condition: Box::new(dummy_node()),
                then_branch: Box::new(dummy_node()),
                elsif_branches: vec![],
                else_branch: None,
            },
            NodeKind::LabeledStatement { label: String::new(), statement: Box::new(dummy_node()) },
            NodeKind::While {
                condition: Box::new(dummy_node()),
                body: Box::new(dummy_node()),
                continue_block: None,
            },
            NodeKind::Tie {
                variable: Box::new(dummy_node()),
                package: Box::new(dummy_node()),
                args: vec![],
            },
            NodeKind::Untie { variable: Box::new(dummy_node()) },
            NodeKind::For {
                init: None,
                condition: None,
                update: None,
                body: Box::new(dummy_node()),
                continue_block: None,
            },
            NodeKind::Foreach {
                variable: Box::new(dummy_node()),
                list: Box::new(dummy_node()),
                body: Box::new(dummy_node()),
                continue_block: None,
            },
            NodeKind::Given { expr: Box::new(dummy_node()), body: Box::new(dummy_node()) },
            NodeKind::When { condition: Box::new(dummy_node()), body: Box::new(dummy_node()) },
            NodeKind::Default { body: Box::new(dummy_node()) },
            NodeKind::StatementModifier {
                statement: Box::new(dummy_node()),
                modifier: String::new(),
                condition: Box::new(dummy_node()),
            },
            NodeKind::Subroutine {
                name: None,
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(dummy_node()),
            },
            NodeKind::Prototype { content: String::new() },
            NodeKind::Signature { parameters: vec![] },
            NodeKind::MandatoryParameter { variable: Box::new(dummy_node()) },
            NodeKind::OptionalParameter {
                variable: Box::new(dummy_node()),
                default_value: Box::new(dummy_node()),
            },
            NodeKind::SlurpyParameter { variable: Box::new(dummy_node()) },
            NodeKind::NamedParameter { variable: Box::new(dummy_node()) },
            NodeKind::Method {
                name: String::new(),
                signature: None,
                attributes: vec![],
                body: Box::new(dummy_node()),
            },
            NodeKind::Return { value: None },
            NodeKind::LoopControl { op: String::new(), label: None },
            NodeKind::MethodCall {
                object: Box::new(dummy_node()),
                method: String::new(),
                args: vec![],
            },
            NodeKind::FunctionCall { name: String::new(), args: vec![] },
            NodeKind::IndirectCall {
                method: String::new(),
                object: Box::new(dummy_node()),
                args: vec![],
            },
            NodeKind::Regex {
                pattern: String::new(),
                replacement: None,
                modifiers: String::new(),
                has_embedded_code: false,
            },
            NodeKind::Match {
                expr: Box::new(dummy_node()),
                pattern: String::new(),
                modifiers: String::new(),
                has_embedded_code: false,
            },
            NodeKind::Substitution {
                expr: Box::new(dummy_node()),
                pattern: String::new(),
                replacement: String::new(),
                modifiers: String::new(),
                has_embedded_code: false,
            },
            NodeKind::Transliteration {
                expr: Box::new(dummy_node()),
                search: String::new(),
                replace: String::new(),
                modifiers: String::new(),
            },
            NodeKind::Package { name: String::new(), name_span: loc, block: None },
            NodeKind::Use { module: String::new(), args: vec![], has_filter_risk: false },
            NodeKind::No { module: String::new(), args: vec![], has_filter_risk: false },
            NodeKind::PhaseBlock {
                phase: String::new(),
                phase_span: None,
                block: Box::new(dummy_node()),
            },
            NodeKind::DataSection { marker: String::new(), body: None },
            NodeKind::Class { name: String::new(), body: Box::new(dummy_node()) },
            NodeKind::Format { name: String::new(), body: String::new() },
            NodeKind::Identifier { name: String::new() },
            NodeKind::Error {
                message: String::new(),
                expected: vec![],
                found: None,
                partial: None,
            },
            NodeKind::MissingExpression,
            NodeKind::MissingStatement,
            NodeKind::MissingIdentifier,
            NodeKind::MissingBlock,
            NodeKind::UnknownRest,
        ];

        variants.iter().map(|v| v.kind_name()).collect()
    }

    #[test]
    fn all_kind_names_is_consistent_with_kind_name() {
        let from_enum = all_kind_names_from_variants();
        let from_const: BTreeSet<&str> = NodeKind::ALL_KIND_NAMES.iter().copied().collect();

        // Check for duplicates in the const array
        assert_eq!(
            NodeKind::ALL_KIND_NAMES.len(),
            from_const.len(),
            "ALL_KIND_NAMES contains duplicates"
        );

        let only_in_enum: Vec<_> = from_enum.difference(&from_const).collect();
        let only_in_const: Vec<_> = from_const.difference(&from_enum).collect();

        assert!(
            only_in_enum.is_empty() && only_in_const.is_empty(),
            "ALL_KIND_NAMES is out of sync with NodeKind variants:\n  \
             in enum but not in ALL_KIND_NAMES: {only_in_enum:?}\n  \
             in ALL_KIND_NAMES but not in enum: {only_in_const:?}"
        );
    }

    #[test]
    fn recovery_kind_names_is_subset_of_all() {
        let all: BTreeSet<&str> = NodeKind::ALL_KIND_NAMES.iter().copied().collect();
        let recovery: BTreeSet<&str> = NodeKind::RECOVERY_KIND_NAMES.iter().copied().collect();

        // No duplicates
        assert_eq!(
            NodeKind::RECOVERY_KIND_NAMES.len(),
            recovery.len(),
            "RECOVERY_KIND_NAMES contains duplicates"
        );

        let not_in_all: Vec<_> = recovery.difference(&all).collect();
        assert!(
            not_in_all.is_empty(),
            "RECOVERY_KIND_NAMES contains entries not in ALL_KIND_NAMES: {not_in_all:?}"
        );
    }
}
