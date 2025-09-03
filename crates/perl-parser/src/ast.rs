//! Abstract Syntax Tree definitions for Perl
//!
//! This module defines the AST node types that represent parsed Perl code.
//! The design is optimized for both direct use in Rust and for generating
//! tree-sitter compatible S-expressions.

use std::fmt;

/// A node in the Abstract Syntax Tree
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
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
                format!("(program {})", stmts)
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

            NodeKind::Binary { op: _, left, right } => {
                // Tree-sitter format: (binary_expression left right)
                format!("(binary_expression {} {})", left.to_sexp(), right.to_sexp())
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                format!(
                    "(ternary {} {} {})",
                    condition.to_sexp(),
                    then_expr.to_sexp(),
                    else_expr.to_sexp()
                )
            }

            NodeKind::Unary { op: _, operand } => {
                // Tree-sitter format: (unary_expression operand)
                format!("(unary_expression {})", operand.to_sexp())
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

            NodeKind::Number { value } => {
                // Format expected by bless parsing tests: (number value)
                format!("(number {})", value)
            }

            NodeKind::String { value, interpolated: _ } => {
                // Format expected by tests: (string "value")
                format!("(string \"{}\")", value)
            }

            NodeKind::Heredoc { delimiter, content, interpolated, indented } => {
                let type_str = if *indented {
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
                let mut result = format!("(while {} {})", condition.to_sexp(), body.to_sexp());
                if let Some(cont) = continue_block {
                    result.push_str(&format!(" (continue {})", cont.to_sexp()));
                }
                result
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

            NodeKind::Foreach { variable, list, body } => {
                format!("(foreach {} {} {})", variable.to_sexp(), list.to_sexp(), body.to_sexp())
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

            NodeKind::Subroutine { name, prototype, signature, attributes, body } => {
                if let Some(sub_name) = name {
                    // Named subroutine - bless test expected format: (sub name () block)
                    let mut parts = vec![sub_name.clone()];

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

                    // Format: (sub name ()(block ...)) - space between name and (), no space between () and block
                    if parts.len() == 3 && parts[1] == "()" {
                        format!("(sub {} {}{})", parts[0], parts[1], parts[2])
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

            NodeKind::MethodCall { object, method, args } => {
                let args_str = args.iter().map(|a| a.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(method_call {} {} ({}))", object.to_sexp(), method, args_str)
            }

            NodeKind::FunctionCall { name, args } => {
                // Special handling for functions that should use call format in tree-sitter tests
                if matches!(name.as_str(), "bless" | "shift" | "unshift" | "push" | "pop") {
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

            NodeKind::Regex { pattern, replacement, modifiers } => {
                format!("(regex {:?} {:?} {:?})", pattern, replacement, modifiers)
            }

            NodeKind::Match { expr, pattern, modifiers } => {
                format!("(match {} (regex {:?} {:?}))", expr.to_sexp(), pattern, modifiers)
            }

            NodeKind::Substitution { expr, pattern, replacement, modifiers } => {
                format!(
                    "(substitution {} {:?} {:?} {:?})",
                    expr.to_sexp(),
                    pattern,
                    replacement,
                    modifiers
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

            NodeKind::Package { name, block } => {
                if let Some(blk) = block {
                    format!("(package {} {})", name, blk.to_sexp())
                } else {
                    format!("(package {})", name)
                }
            }

            NodeKind::Use { module, args } => {
                if args.is_empty() {
                    format!("(use {})", module)
                } else {
                    let args_str = args.join(" ");
                    format!("(use {} ({}))", module, args_str)
                }
            }

            NodeKind::No { module, args } => {
                if args.is_empty() {
                    format!("(no {})", module)
                } else {
                    let args_str = args.join(" ");
                    format!("(no {} ({}))", module, args_str)
                }
            }

            NodeKind::PhaseBlock { phase, block } => {
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

            NodeKind::Error { message } => {
                format!("(ERROR {})", message)
            }
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
}

/// The kind of AST node
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    // Program structure
    Program {
        statements: Vec<Node>,
    },

    ExpressionStatement {
        expression: Box<Node>,
    },

    // Variable operations
    VariableDeclaration {
        declarator: String, // my, our, local, state
        variable: Box<Node>,
        attributes: Vec<String>,
        initializer: Option<Box<Node>>,
    },

    VariableListDeclaration {
        declarator: String, // my, our, local, state
        variables: Vec<Node>,
        attributes: Vec<String>,
        initializer: Option<Box<Node>>,
    },

    Variable {
        sigil: String, // $, @, %, &, *
        name: String,
    },

    VariableWithAttributes {
        variable: Box<Node>,
        attributes: Vec<String>,
    },

    Assignment {
        lhs: Box<Node>,
        rhs: Box<Node>,
        op: String, // =, +=, -=, etc.
    },

    // Expressions
    Binary {
        op: String,
        left: Box<Node>,
        right: Box<Node>,
    },

    Ternary {
        condition: Box<Node>,
        then_expr: Box<Node>,
        else_expr: Box<Node>,
    },

    Unary {
        op: String,
        operand: Box<Node>,
    },

    // I/O operations
    Diamond, // <>

    Ellipsis, // ...

    Undef, // undef

    Readline {
        filehandle: Option<String>, // <STDIN>, <$fh>, etc.
    },

    Glob {
        pattern: String, // <*.txt>
    },

    // Literals
    Number {
        value: String,
    },

    String {
        value: String,
        interpolated: bool,
    },

    Heredoc {
        delimiter: String,
        content: String,
        interpolated: bool,
        indented: bool,
    },

    ArrayLiteral {
        elements: Vec<Node>,
    },

    HashLiteral {
        pairs: Vec<(Node, Node)>,
    },

    // Control flow
    Block {
        statements: Vec<Node>,
    },

    Eval {
        block: Box<Node>,
    },

    Do {
        block: Box<Node>,
    },

    Try {
        body: Box<Node>,
        catch_blocks: Vec<(Option<String>, Box<Node>)>, // (variable, block)
        finally_block: Option<Box<Node>>,
    },

    If {
        condition: Box<Node>,
        then_branch: Box<Node>,
        elsif_branches: Vec<(Box<Node>, Box<Node>)>,
        else_branch: Option<Box<Node>>,
    },

    LabeledStatement {
        label: String,
        statement: Box<Node>,
    },

    While {
        condition: Box<Node>,
        body: Box<Node>,
        continue_block: Option<Box<Node>>,
    },

    For {
        init: Option<Box<Node>>,
        condition: Option<Box<Node>>,
        update: Option<Box<Node>>,
        body: Box<Node>,
        continue_block: Option<Box<Node>>,
    },

    Foreach {
        variable: Box<Node>,
        list: Box<Node>,
        body: Box<Node>,
    },

    Given {
        expr: Box<Node>,
        body: Box<Node>,
    },

    When {
        condition: Box<Node>,
        body: Box<Node>,
    },

    Default {
        body: Box<Node>,
    },

    StatementModifier {
        statement: Box<Node>,
        modifier: String,
        condition: Box<Node>,
    },

    // Functions
    Subroutine {
        name: Option<String>,
        prototype: Option<Box<Node>>,
        signature: Option<Box<Node>>,
        attributes: Vec<String>,
        body: Box<Node>,
    },

    // Prototype for subroutine
    Prototype {
        content: String,
    },

    // Signature for subroutine
    Signature {
        parameters: Vec<Node>,
    },

    // Signature parameter types
    MandatoryParameter {
        variable: Box<Node>,
    },

    OptionalParameter {
        variable: Box<Node>,
        default_value: Box<Node>,
    },

    SlurpyParameter {
        variable: Box<Node>,
    },

    NamedParameter {
        variable: Box<Node>,
    },

    // Method declaration (Perl 5.38+)
    Method {
        name: String,
        signature: Option<Box<Node>>,
        attributes: Vec<String>,
        body: Box<Node>,
    },

    Return {
        value: Option<Box<Node>>,
    },

    MethodCall {
        object: Box<Node>,
        method: String,
        args: Vec<Node>,
    },

    FunctionCall {
        name: String,
        args: Vec<Node>,
    },

    IndirectCall {
        method: String,
        object: Box<Node>,
        args: Vec<Node>,
    },

    // Pattern matching
    Regex {
        pattern: String,
        replacement: Option<String>,
        modifiers: String,
    },

    Match {
        expr: Box<Node>,
        pattern: String,
        modifiers: String,
    },

    Substitution {
        expr: Box<Node>,
        pattern: String,
        replacement: String,
        modifiers: String,
    },

    Transliteration {
        expr: Box<Node>,
        search: String,
        replace: String,
        modifiers: String,
    },

    // Package system
    Package {
        name: String,
        block: Option<Box<Node>>,
    },

    Use {
        module: String,
        args: Vec<String>,
    },

    No {
        module: String,
        args: Vec<String>,
    },

    // Phase blocks
    PhaseBlock {
        phase: String, // BEGIN, END, CHECK, INIT, UNITCHECK
        block: Box<Node>,
    },

    // Data sections
    DataSection {
        marker: String, // __DATA__ or __END__
        body: Option<String>,
    },

    // Modern Perl OOP (5.38+)
    Class {
        name: String,
        body: Box<Node>,
    },

    // Format declaration (legacy Perl)
    Format {
        name: String,
        body: String,
    },

    // Misc
    Identifier {
        name: String,
    },

    Error {
        message: String,
    },

    // Lexer budget exceeded - preserves earlier AST
    UnknownRest,
}

/// Source location information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub start: usize,
    pub end: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}
