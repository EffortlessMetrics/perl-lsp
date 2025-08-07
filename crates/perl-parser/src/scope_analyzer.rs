use crate::Parser;
use crate::ast::{Node, NodeKind};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IssueKind {
    VariableShadowing,
    UnusedVariable,
    UndeclaredVariable,
    VariableRedeclaration,
}

#[derive(Debug, Clone)]
pub struct ScopeIssue {
    pub kind: IssueKind,
    pub variable_name: String,
    pub line: usize,
    pub description: String,
}

#[derive(Debug, Clone)]
struct Scope {
    variables: HashMap<String, (usize, bool)>, // (line, is_used)
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Scope) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    fn declare_variable(&mut self, name: &str, line: usize) -> Option<IssueKind> {
        if self.variables.contains_key(name) {
            return Some(IssueKind::VariableRedeclaration);
        }
        
        // Check if it shadows a parent scope variable
        if self.is_variable_in_parent(name) {
            self.variables.insert(name.to_string(), (line, false));
            return Some(IssueKind::VariableShadowing);
        }
        
        self.variables.insert(name.to_string(), (line, false));
        None
    }

    fn use_variable(&mut self, name: &str) -> bool {
        if let Some((line, _)) = self.variables.get(name).cloned() {
            self.variables.insert(name.to_string(), (line, true));
            return true;
        }
        
        if let Some(ref mut parent) = self.parent {
            return parent.use_variable(name);
        }
        
        false
    }

    fn is_variable_in_parent(&self, name: &str) -> bool {
        if let Some(ref parent) = self.parent {
            parent.variables.contains_key(name) || parent.is_variable_in_parent(name)
        } else {
            false
        }
    }

    fn is_variable_declared(&self, name: &str) -> bool {
        self.variables.contains_key(name) || 
            self.parent.as_ref().map_or(false, |p| p.is_variable_declared(name))
    }

    fn get_unused_variables(&self) -> Vec<(String, usize)> {
        self.variables.iter()
            .filter_map(|(name, (line, used))| {
                if !used {
                    Some((name.clone(), *line))
                } else {
                    None
                }
            })
            .collect()
    }
}

pub struct ScopeAnalyzer {
    strict_mode: bool,
}

impl ScopeAnalyzer {
    pub fn new() -> Self {
        Self {
            strict_mode: false,
        }
    }

    pub fn analyze(&self, code: &str) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        let mut parser = Parser::new(code);
        
        match parser.parse() {
            Ok(ast) => {
                let mut scope = Scope::new();
                let mut strict_mode = self.strict_mode;
                
                // Check for 'use strict' pragma
                if code.contains("use strict") {
                    strict_mode = true;
                }
                
                self.analyze_node(&ast, &mut scope, &mut issues, code, strict_mode);
                
                // Report unused variables
                for (var_name, line) in scope.get_unused_variables() {
                    issues.push(ScopeIssue {
                        kind: IssueKind::UnusedVariable,
                        variable_name: var_name.clone(),
                        line: self.get_line_number(code, line),
                        description: format!("Variable '{}' is declared but never used", var_name),
                    });
                }
            }
            Err(_) => {
                // Parse error, can't analyze scope
            }
        }
        
        issues
    }

    fn analyze_node(&self, node: &Node, scope: &mut Scope, issues: &mut Vec<ScopeIssue>, code: &str, strict_mode: bool) {
        match &node.kind {
            NodeKind::VariableDeclaration { variable, .. } => {
                let var_name = self.extract_variable_name(variable);
                let line = self.get_line_from_node(node, code);
                
                if let Some(issue_kind) = scope.declare_variable(&var_name, node.location.start) {
                    issues.push(ScopeIssue {
                        kind: issue_kind,
                        variable_name: var_name.clone(),
                        line,
                        description: match issue_kind {
                            IssueKind::VariableShadowing => 
                                format!("Variable '{}' shadows a variable in outer scope", var_name),
                            IssueKind::VariableRedeclaration =>
                                format!("Variable '{}' is already declared in this scope", var_name),
                            _ => String::new(),
                        },
                    });
                }
            }
            NodeKind::Variable { sigil, name } => {
                let full_name = format!("{}{}", sigil, name);
                if !scope.use_variable(&full_name) && strict_mode {
                    if !scope.is_variable_declared(&full_name) {
                        issues.push(ScopeIssue {
                            kind: IssueKind::UndeclaredVariable,
                            variable_name: full_name.clone(),
                            line: self.get_line_from_node(node, code),
                            description: format!("Variable '{}' is used but not declared", name),
                        });
                    }
                }
            }
            NodeKind::Block { statements } => {
                let mut new_scope = Scope::with_parent(scope.clone());
                for stmt in statements {
                    self.analyze_node(stmt, &mut new_scope, issues, code, strict_mode);
                }
                
                // Merge back used variable information
                for (var_name, (_, used)) in &new_scope.variables {
                    if *used {
                        scope.use_variable(var_name);
                    }
                }
                
                // Report unused variables in this block
                for (var_name, line) in new_scope.get_unused_variables() {
                    issues.push(ScopeIssue {
                        kind: IssueKind::UnusedVariable,
                        variable_name: var_name.clone(),
                        line: self.get_line_number(code, line),
                        description: format!("Variable '{}' is declared but never used", var_name),
                    });
                }
            }
            NodeKind::For { init, condition, update, body, .. } => {
                let mut loop_scope = Scope::with_parent(scope.clone());
                
                // Handle loop initialization
                if let Some(init_node) = init {
                    self.analyze_node(init_node, &mut loop_scope, issues, code, strict_mode);
                }
                
                // Handle condition
                if let Some(cond) = condition {
                    self.analyze_node(cond, &mut loop_scope, issues, code, strict_mode);
                }
                
                // Handle update
                if let Some(upd) = update {
                    self.analyze_node(upd, &mut loop_scope, issues, code, strict_mode);
                }
                
                // Analyze loop body
                self.analyze_node(body, &mut loop_scope, issues, code, strict_mode);
            }
            NodeKind::Foreach { variable, list, body } => {
                let mut loop_scope = Scope::with_parent(scope.clone());
                
                // Handle loop variable  
                self.analyze_node(variable, &mut loop_scope, issues, code, strict_mode);
                
                // Handle list
                self.analyze_node(list, &mut loop_scope, issues, code, strict_mode);
                
                // Analyze loop body
                self.analyze_node(body, &mut loop_scope, issues, code, strict_mode);
            }
            NodeKind::Subroutine { params, body, .. } => {
                let mut sub_scope = Scope::with_parent(scope.clone());
                
                // Declare parameters
                for param in params {
                    self.declare_params(param, &mut sub_scope, node.location.start);
                }
                
                // Analyze subroutine body
                self.analyze_node(body, &mut sub_scope, issues, code, strict_mode);
            }
            _ => {
                // Recursively analyze children
                for child in node.children() {
                    self.analyze_node(child, scope, issues, code, strict_mode);
                }
            }
        }
    }

    fn extract_variable_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Variable { sigil, name } => format!("{}{}", sigil, name),
            _ => {
                // Try to extract from source
                if let Some(child) = node.children().first() {
                    self.extract_variable_name(child)
                } else {
                    String::new()
                }
            }
        }
    }

    fn declare_params(&self, param: &Node, scope: &mut Scope, start: usize) {
        match &param.kind {
            NodeKind::Variable { sigil, name } => {
                let full_name = format!("{}{}", sigil, name);
                scope.declare_variable(&full_name, start);
                scope.use_variable(&full_name); // Parameters are considered used
            }
            _ => {}
        }
    }

    fn get_line_from_node(&self, node: &Node, code: &str) -> usize {
        self.get_line_number(code, node.location.start)
    }

    fn get_line_number(&self, code: &str, offset: usize) -> usize {
        code[..offset.min(code.len())]
            .chars()
            .filter(|&c| c == '\n')
            .count() + 1
    }

    pub fn get_suggestions(&self, issues: &[ScopeIssue]) -> Vec<String> {
        issues.iter().map(|issue| {
            match issue.kind {
                IssueKind::VariableShadowing => {
                    format!("Consider renaming '{}' to avoid shadowing", issue.variable_name)
                }
                IssueKind::UnusedVariable => {
                    format!("Remove unused variable '{}' or prefix with underscore", issue.variable_name)
                }
                IssueKind::UndeclaredVariable => {
                    format!("Declare '{}' with 'my', 'our', or 'local'", issue.variable_name)
                }
                IssueKind::VariableRedeclaration => {
                    format!("Remove duplicate declaration of '{}'", issue.variable_name)
                }
            }
        }).collect()
    }
}

impl Node {
    fn children(&self) -> Vec<&Node> {
        match &self.kind {
            NodeKind::Program { statements } => statements.iter().collect(),
            NodeKind::Block { statements } => statements.iter().collect(),
            NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
            NodeKind::Unary { operand, .. } => vec![operand.as_ref()],
            NodeKind::If { condition, then_branch, else_branch, .. } => {
                let mut children = vec![condition.as_ref(), then_branch.as_ref()];
                if let Some(else_b) = else_branch {
                    children.push(else_b.as_ref());
                }
                children
            }
            NodeKind::FunctionCall { args, .. } => {
                args.iter().collect()
            }
            NodeKind::MethodCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter());
                children
            }
            NodeKind::Assignment { lhs, rhs, .. } => vec![lhs.as_ref(), rhs.as_ref()],
            _ => vec![],
        }
    }
}