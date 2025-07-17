//! Pure Rust Perl parser implementation
//!
//! This module provides a complete Rust-native implementation of the Perl parser
//! using Pest for grammar parsing, without any dependency on tree-sitter's C code.

use pest::{iterators::{Pair, Pairs}, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct PerlParser;

/// AST node types for the pure Rust parser
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    // Program structure
    Program(Vec<AstNode>),
    Statement(Box<AstNode>),
    Block(Vec<AstNode>),
    
    // Declarations
    VariableDeclaration {
        scope: String,
        variables: Vec<AstNode>,
        initializer: Option<Box<AstNode>>,
    },
    SubDeclaration {
        name: String,
        prototype: Option<String>,
        attributes: Vec<String>,
        body: Box<AstNode>,
    },
    PackageDeclaration {
        name: String,
        version: Option<String>,
        block: Option<Box<AstNode>>,
    },
    
    // Control flow
    IfStatement {
        condition: Box<AstNode>,
        then_block: Box<AstNode>,
        elsif_clauses: Vec<(AstNode, AstNode)>,
        else_block: Option<Box<AstNode>>,
    },
    UnlessStatement {
        condition: Box<AstNode>,
        block: Box<AstNode>,
        else_block: Option<Box<AstNode>>,
    },
    WhileStatement {
        label: Option<String>,
        condition: Box<AstNode>,
        block: Box<AstNode>,
    },
    ForStatement {
        label: Option<String>,
        init: Option<Box<AstNode>>,
        condition: Option<Box<AstNode>>,
        update: Option<Box<AstNode>>,
        block: Box<AstNode>,
    },
    ForeachStatement {
        label: Option<String>,
        variable: Option<Box<AstNode>>,
        list: Box<AstNode>,
        block: Box<AstNode>,
    },
    
    // Expressions
    BinaryOp {
        op: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    UnaryOp {
        op: String,
        operand: Box<AstNode>,
    },
    TernaryOp {
        condition: Box<AstNode>,
        true_expr: Box<AstNode>,
        false_expr: Box<AstNode>,
    },
    Assignment {
        target: Box<AstNode>,
        op: String,
        value: Box<AstNode>,
    },
    FunctionCall {
        function: Box<AstNode>,
        args: Vec<AstNode>,
    },
    MethodCall {
        object: Box<AstNode>,
        method: String,
        args: Vec<AstNode>,
    },
    ArrayAccess {
        array: Box<AstNode>,
        index: Box<AstNode>,
    },
    HashAccess {
        hash: Box<AstNode>,
        key: Box<AstNode>,
    },
    
    // Variables
    ScalarVariable(String),
    ArrayVariable(String),
    HashVariable(String),
    ArrayElement {
        array: String,
        index: Box<AstNode>,
    },
    HashElement {
        hash: String,
        key: Box<AstNode>,
    },
    
    // Literals
    Number(String),
    String(String),
    Identifier(String),
    Bareword(String),
    Regex {
        pattern: String,
        flags: String,
    },
    Substitution {
        pattern: String,
        replacement: String,
        flags: String,
    },
    
    // Special statements
    UseStatement {
        module: String,
        imports: Vec<String>,
    },
    RequireStatement {
        module: String,
    },
    ReturnStatement {
        value: Option<Box<AstNode>>,
    },
    LastStatement {
        label: Option<String>,
    },
    NextStatement {
        label: Option<String>,
    },
    
    // Other
    Comment(String),
    Label(String),
    AnonymousSub {
        prototype: Option<String>,
        body: Box<AstNode>,
    },
    List(Vec<AstNode>),
}

/// Pure Rust Perl parser implementation
pub struct PureRustPerlParser;

impl PureRustPerlParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&mut self, source: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        let pairs = PerlParser::parse(Rule::program, source)?;
        self.build_ast(pairs)
    }

    fn build_ast(&mut self, pairs: Pairs<Rule>) -> Result<AstNode, Box<dyn std::error::Error>> {
        let mut nodes = Vec::new();
        for pair in pairs {
            if let Some(node) = self.build_node(pair)? {
                nodes.push(node);
            }
        }
        if nodes.len() == 1 {
            Ok(nodes.into_iter().next().unwrap())
        } else {
            Ok(AstNode::Program(nodes))
        }
    }

    fn build_node(&mut self, pair: Pair<Rule>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
        match pair.as_rule() {
            Rule::program => {
                let mut statements = Vec::new();
                for inner in pair.into_inner() {
                    if let Some(node) = self.build_node(inner)? {
                        statements.push(node);
                    }
                }
                Ok(Some(AstNode::Program(statements)))
            }
            Rule::statements => {
                let mut statements = Vec::new();
                for inner in pair.into_inner() {
                    if let Some(node) = self.build_node(inner)? {
                        statements.push(node);
                    }
                }
                Ok(Some(AstNode::Program(statements)))
            }
            Rule::statement => {
                let inner = pair.into_inner().next().unwrap();
                self.build_node(inner)
            }
            Rule::expression_statement => {
                let inner = pair.into_inner().next().unwrap();
                if let Some(expr) = self.build_node(inner)? {
                    Ok(Some(AstNode::Statement(Box::new(expr))))
                } else {
                    Ok(None)
                }
            }
            Rule::declaration_statement => {
                let inner = pair.into_inner().next().unwrap();
                self.build_node(inner)
            }
            Rule::variable_declaration => {
                let mut inner = pair.into_inner();
                let scope = inner.next().unwrap().as_str().to_string();
                let mut variables = Vec::new();
                let mut initializer = None;
                
                for p in inner {
                    match p.as_rule() {
                        Rule::variable_list => {
                            for var in p.into_inner() {
                                if let Some(v) = self.build_node(var)? {
                                    variables.push(v);
                                }
                            }
                        }
                        Rule::expression => {
                            initializer = self.build_node(p)?.map(Box::new);
                        }
                        _ => {}
                    }
                }
                
                Ok(Some(AstNode::VariableDeclaration {
                    scope,
                    variables,
                    initializer,
                }))
            }
            Rule::sub_declaration => {
                let mut inner = pair.into_inner();
                inner.next(); // skip "sub"
                let name = inner.next().unwrap().as_str().to_string();
                let mut prototype = None;
                let mut attributes = Vec::new();
                let mut body = None;
                
                for p in inner {
                    match p.as_rule() {
                        Rule::prototype => {
                            prototype = Some(p.as_str().to_string());
                        }
                        Rule::attributes => {
                            for attr in p.into_inner() {
                                attributes.push(attr.as_str().to_string());
                            }
                        }
                        Rule::block => {
                            body = self.build_node(p)?.map(Box::new);
                        }
                        _ => {}
                    }
                }
                
                Ok(Some(AstNode::SubDeclaration {
                    name,
                    prototype,
                    attributes,
                    body: body.unwrap_or_else(|| Box::new(AstNode::Block(vec![]))),
                }))
            }
            Rule::if_statement => {
                let mut inner = pair.into_inner();
                inner.next(); // skip "if"
                let condition = Box::new(self.build_node(inner.next().unwrap())?.unwrap());
                let then_block = Box::new(self.build_node(inner.next().unwrap())?.unwrap());
                let mut elsif_clauses = Vec::new();
                let mut else_block = None;
                
                for p in inner {
                    match p.as_rule() {
                        Rule::elsif_clause => {
                            let mut elsif_inner = p.into_inner();
                            elsif_inner.next(); // skip "elsif"
                            let cond = self.build_node(elsif_inner.next().unwrap())?.unwrap();
                            let block = self.build_node(elsif_inner.next().unwrap())?.unwrap();
                            elsif_clauses.push((cond, block));
                        }
                        Rule::else_clause => {
                            let mut else_inner = p.into_inner();
                            else_inner.next(); // skip "else"
                            else_block = self.build_node(else_inner.next().unwrap())?.map(Box::new);
                        }
                        _ => {}
                    }
                }
                
                Ok(Some(AstNode::IfStatement {
                    condition,
                    then_block,
                    elsif_clauses,
                    else_block,
                }))
            }
            Rule::block => {
                let mut statements = Vec::new();
                for inner in pair.into_inner() {
                    if inner.as_rule() == Rule::statements {
                        for stmt in inner.into_inner() {
                            if let Some(node) = self.build_node(stmt)? {
                                statements.push(node);
                            }
                        }
                    }
                }
                Ok(Some(AstNode::Block(statements)))
            }
            Rule::expression => {
                self.build_expression(pair)
            }
            Rule::assignment_expression => {
                let mut inner = pair.into_inner();
                let target = Box::new(self.build_node(inner.next().unwrap())?.unwrap());
                let op = inner.next().unwrap().as_str().to_string();
                let value = Box::new(self.build_node(inner.next().unwrap())?.unwrap());
                Ok(Some(AstNode::Assignment { target, op, value }))
            }
            Rule::scalar_variable => {
                Ok(Some(AstNode::ScalarVariable(pair.as_str().to_string())))
            }
            Rule::array_variable => {
                Ok(Some(AstNode::ArrayVariable(pair.as_str().to_string())))
            }
            Rule::hash_variable => {
                Ok(Some(AstNode::HashVariable(pair.as_str().to_string())))
            }
            Rule::number => {
                Ok(Some(AstNode::Number(pair.as_str().to_string())))
            }
            Rule::identifier => {
                Ok(Some(AstNode::Identifier(pair.as_str().to_string())))
            }
            Rule::string | Rule::single_quoted_string | Rule::double_quoted_string => {
                Ok(Some(AstNode::String(pair.as_str().to_string())))
            }
            Rule::list => {
                let mut elements = Vec::new();
                for inner in pair.into_inner() {
                    if inner.as_rule() == Rule::list_elements {
                        for elem in inner.into_inner() {
                            if let Some(node) = self.build_node(elem)? {
                                elements.push(node);
                            }
                        }
                    }
                }
                Ok(Some(AstNode::List(elements)))
            }
            Rule::comment => {
                Ok(Some(AstNode::Comment(pair.as_str().to_string())))
            }
            Rule::semicolon | Rule::WHITESPACE => Ok(None),
            Rule::standalone_expression => {
                let inner = pair.into_inner().next().unwrap();
                self.build_node(inner)
            }
            Rule::regex => {
                let mut inner = pair.into_inner();
                let pattern = inner.next().map(|p| p.as_str().to_string()).unwrap_or_default();
                let flags = inner.next().map(|p| p.as_str().to_string()).unwrap_or_default();
                Ok(Some(AstNode::Regex { pattern, flags }))
            }
            Rule::for_statement => {
                let mut inner = pair.into_inner();
                let label = None; // TODO: handle label if present
                inner.next(); // skip "for"
                let mut init = None;
                let mut condition = None;
                let mut update = None;
                let mut block = None;
                
                // Parse for loop components
                for p in inner {
                    match p.as_rule() {
                        Rule::for_init => {
                            init = self.build_node(p)?.map(Box::new);
                        }
                        Rule::expression => {
                            if condition.is_none() {
                                condition = self.build_node(p)?.map(Box::new);
                            } else if update.is_none() {
                                update = self.build_node(p)?.map(Box::new);
                            }
                        }
                        Rule::block => {
                            block = self.build_node(p)?.map(Box::new);
                        }
                        _ => {}
                    }
                }
                
                Ok(Some(AstNode::ForStatement {
                    label,
                    init,
                    condition,
                    update,
                    block: block.unwrap_or_else(|| Box::new(AstNode::Block(vec![]))),
                }))
            }
            Rule::package_declaration => {
                let mut inner = pair.into_inner();
                inner.next(); // skip "package"
                // Accept qualified_name as identifier
                let name = if let Some(name_pair) = inner.next() {
                    name_pair.as_str().to_string()
                } else {
                    String::new()
                };
                let mut version = None;
                let mut block = None;
                for p in inner {
                    match p.as_rule() {
                        Rule::version => version = Some(p.as_str().to_string()),
                        Rule::block => block = self.build_node(p)?.map(Box::new),
                        _ => {}
                    }
                }
                Ok(Some(AstNode::PackageDeclaration { name, version, block }))
            }
            Rule::relational_expression => {
                let mut inner = pair.into_inner();
                let left = if let Some(first) = inner.next() {
                    self.build_node(first)?.map(Box::new)
                } else {
                    return Ok(None);
                };
                if let Some(op_pair) = inner.next() {
                    let op = op_pair.as_str().to_string();
                    let right = if let Some(second) = inner.next() {
                        self.build_node(second)?.map(Box::new)
                    } else {
                        return Ok(left.map(|l| *l));
                    };
                    match (left, right) {
                        (Some(left), Some(right)) => Ok(Some(AstNode::BinaryOp { op, left, right })),
                        (Some(left), None) => Ok(Some(*left)),
                        (None, Some(right)) => Ok(Some(*right)),
                        (None, None) => Ok(None),
                    }
                } else {
                    Ok(left.map(|l| *l))
                }
            }
            _ => {
                // For unhandled rules, try to process inner pairs
                let inner: Vec<_> = pair.into_inner().collect();
                if inner.is_empty() {
                    Ok(None)
                } else if inner.len() == 1 {
                    self.build_node(inner.into_iter().next().unwrap())
                } else {
                    let mut nodes = Vec::new();
                    for p in inner {
                        if let Some(node) = self.build_node(p)? {
                            nodes.push(node);
                        }
                    }
                    if nodes.is_empty() {
                        Ok(None)
                    } else if nodes.len() == 1 {
                        Ok(nodes.into_iter().next())
                    } else {
                        Ok(Some(AstNode::List(nodes)))
                    }
                }
            }
        }
    }

    fn build_expression(&mut self, pair: Pair<Rule>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
        // This is a simplified expression builder
        // In a full implementation, this would handle operator precedence
        let inner: Vec<_> = pair.into_inner().collect();
        if inner.is_empty() {
            Ok(None)
        } else if inner.len() == 1 {
            self.build_node(inner.into_iter().next().unwrap())
        } else {
            // For now, just return the first node
            self.build_node(inner.into_iter().next().unwrap())
        }
    }

    pub fn to_sexp(&self, node: &AstNode) -> String {
        match node {
            AstNode::Program(children) => {
                // Remove nested source_file in S-expression output
                let mut flat_children = vec![];
                for c in children {
                    let sexp = self.to_sexp(c);
                    if sexp.starts_with("(source_file ") {
                        let inner = sexp.trim_start_matches("(source_file ").trim_end_matches(")");
                        flat_children.push(inner.to_string());
                    } else {
                        flat_children.push(sexp);
                    }
                }
                if flat_children.is_empty() {
                    "(source_file)".to_string()
                } else {
                    format!("(source_file {})", flat_children.join(" "))
                }
            }
            AstNode::Statement(expr) => {
                self.to_sexp(expr)
            }
            AstNode::Block(statements) => {
                let stmt_sexps: Vec<String> = statements.iter().map(|s| self.to_sexp(s)).collect();
                format!("(block {})", stmt_sexps.join(" "))
            }
            AstNode::VariableDeclaration { scope, variables, initializer } => {
                let var_sexps: Vec<String> = variables.iter().map(|v| self.to_sexp(v)).collect();
                let init_sexp = initializer.as_ref().map(|i| self.to_sexp(i)).unwrap_or_default();
                format!("(variable_declaration ({}) {} {})", scope, var_sexps.join(" "), init_sexp)
            }
            AstNode::SubDeclaration { name, body, .. } => {
                format!("(subroutine (identifier {}) {})", name, self.to_sexp(body))
            }
            AstNode::IfStatement { condition, then_block, .. } => {
                format!("(if_statement {} {})", self.to_sexp(condition), self.to_sexp(then_block))
            }
            AstNode::Assignment { target, op, value } => {
                format!("(assignment {} ({}) {})", self.to_sexp(target), op, self.to_sexp(value))
            }
            AstNode::BinaryOp { op, left, right } => {
                format!("(binary_expression {} ({}) {})", self.to_sexp(left), op, self.to_sexp(right))
            }
            AstNode::ScalarVariable(name) => {
                format!("(scalar_variable {})", name)
            }
            AstNode::ArrayVariable(name) => {
                format!("(array_variable {})", name)
            }
            AstNode::HashVariable(name) => {
                format!("(hash_variable {})", name)
            }
            AstNode::Number(value) => {
                format!("(number {})", value)
            }
            AstNode::String(value) => {
                format!("(string_literal {})", value)
            }
            AstNode::Identifier(name) => {
                format!("(identifier {})", name)
            }
            AstNode::Comment(content) => {
                format!("(comment {})", content)
            }
            AstNode::List(items) => {
                let item_sexps: Vec<String> = items.iter().map(|i| self.to_sexp(i)).collect();
                item_sexps.join(" ")
            }
            AstNode::ForStatement { init, condition, update, block, .. } => {
                let mut parts = vec![];
                if let Some(i) = init { parts.push(format!("(init {})", self.to_sexp(i))); }
                if let Some(c) = condition { parts.push(format!("(condition {})", self.to_sexp(c))); }
                if let Some(u) = update { parts.push(format!("(update {})", self.to_sexp(u))); }
                parts.push(format!("(body {})", self.to_sexp(block)));
                format!("(for_statement {})", parts.join(" "))
            }
            AstNode::PackageDeclaration { name, version, block } => {
                let mut parts = vec![format!("(name {})", name)];
                if let Some(v) = version { parts.push(format!("(version {})", v)); }
                if let Some(b) = block { parts.push(format!("(body {})", self.to_sexp(b))); }
                format!("(package_declaration {})", parts.join(" "))
            }
            AstNode::Regex { pattern, flags } => {
                format!("(regex {} {})", pattern, flags)
            }
            _ => format!("(unhandled_node {:?})", node),
        }
    }
}

impl Default for PureRustPerlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let mut parser = PureRustPerlParser::new();
        let source = "$var";
        let result = parser.parse(source);
        assert!(result.is_ok());
        let ast = result.unwrap();
        let sexp = parser.to_sexp(&ast);
        println!("AST: {:?}", ast);
        println!("S-expression: {}", sexp);
    }

    #[test]
    fn test_variable_parsing() {
        let mut parser = PureRustPerlParser::new();
        let source = "$scalar @array %hash";
        let result = parser.parse(source);
        assert!(result.is_ok());
        let ast = result.unwrap();
        let sexp = parser.to_sexp(&ast);
        println!("S-expression: {}", sexp);
    }

    #[test]
    fn test_assignment_parsing() {
        let mut parser = PureRustPerlParser::new();
        let source = "my $var = 42;";
        let result = parser.parse(source);
        match result {
            Ok(ast) => {
                let sexp = parser.to_sexp(&ast);
                println!("Success! AST: {:?}", ast);
                println!("S-expression: {}", sexp);
            }
            Err(e) => {
                println!("Parse error: {}", e);
                assert!(false, "Parse should succeed");
            }
        }
    }
    
    #[test]
    fn test_function_declaration() {
        let mut parser = PureRustPerlParser::new();
        let source = "sub hello { print 'Hello'; }";
        let result = parser.parse(source);
        match result {
            Ok(ast) => {
                let sexp = parser.to_sexp(&ast);
                println!("S-expression: {}", sexp);
            }
            Err(e) => {
                println!("Parse error: {}", e);
                assert!(false, "Parse should succeed");
            }
        }
    }
    
    #[test]
    fn test_if_statement() {
        let mut parser = PureRustPerlParser::new();
        let source = "if ($x > 0) { print 'positive'; }";
        let result = parser.parse(source);
        match result {
            Ok(ast) => {
                let sexp = parser.to_sexp(&ast);
                println!("S-expression: {}", sexp);
            }
            Err(e) => {
                println!("Parse error: {}", e);
                assert!(false, "Parse should succeed");
            }
        }
    }
    
    #[test]
    fn test_array_assignment() {
        let mut parser = PureRustPerlParser::new();
        let source = "@array = (1, 2, 3);";
        let result = parser.parse(source);
        match result {
            Ok(ast) => {
                let sexp = parser.to_sexp(&ast);
                println!("Array assignment AST: {:?}", ast);
                println!("S-expression: {}", sexp);
            }
            Err(e) => {
                println!("Parse error: {}", e);
                assert!(false, "Parse should succeed");
            }
        }
    }
    
    #[test]
    fn test_hash_assignment() {
        let mut parser = PureRustPerlParser::new();
        let source = "%hash = (a => 1, b => 2);";
        let result = parser.parse(source);
        match result {
            Ok(ast) => {
                let sexp = parser.to_sexp(&ast);
                println!("Hash assignment AST: {:?}", ast);
                println!("S-expression: {}", sexp);
            }
            Err(e) => {
                println!("Parse error: {}", e);
                assert!(false, "Parse should succeed");
            }
        }
    }
}