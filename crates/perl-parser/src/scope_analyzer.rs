use crate::Parser;
use crate::ast::{Node, NodeKind};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;

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

#[derive(Debug)]
struct Variable {
    name: String,
    line: usize,
    is_used: RefCell<bool>,
    is_our: bool,
}

#[derive(Debug)]
struct Scope {
    variables: RefCell<HashMap<String, Rc<Variable>>>,
    parent: Option<Rc<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: RefCell::new(HashMap::new()),
            parent: None,
        }
    }

    fn with_parent(parent: Rc<Scope>) -> Self {
        Self {
            variables: RefCell::new(HashMap::new()),
            parent: Some(parent),
        }
    }

    fn declare_variable(&self, name: &str, line: usize, is_our: bool) -> Option<IssueKind> {
        // First check if already declared in this scope
        {
            let vars = self.variables.borrow();
            if vars.contains_key(name) {
                return Some(IssueKind::VariableRedeclaration);
            }
        }
        
        // Check if it shadows a parent scope variable
        let shadows = if let Some(ref parent) = self.parent {
            parent.lookup_variable(name).is_some()
        } else {
            false
        };
        
        // Now insert the variable
        let mut vars = self.variables.borrow_mut();
        vars.insert(name.to_string(), Rc::new(Variable {
            name: name.to_string(),
            line,
            is_used: RefCell::new(is_our), // 'our' variables are considered used
            is_our,
        }));
        
        if shadows {
            Some(IssueKind::VariableShadowing)
        } else {
            None
        }
    }

    fn lookup_variable(&self, name: &str) -> Option<Rc<Variable>> {
        self.variables.borrow().get(name).cloned()
            .or_else(|| self.parent.as_ref()?.lookup_variable(name))
    }

    fn use_variable(&self, name: &str) -> bool {
        if let Some(var) = self.lookup_variable(name) {
            *var.is_used.borrow_mut() = true;
            true
        } else {
            false
        }
    }

    fn get_unused_variables(&self) -> Vec<(String, usize)> {
        let mut unused = Vec::new();
        
        for var in self.variables.borrow().values() {
            if !*var.is_used.borrow() && !var.is_our {
                unused.push((var.name.clone(), var.line));
            }
        }
        
        // Recursively collect from parent scopes if needed
        // (Not needed for our current use case)
        
        unused
    }
}

pub struct ScopeAnalyzer;

impl ScopeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, code: &str) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        let mut parser = Parser::new(code);
        
        match parser.parse() {
            Ok(ast) => {
                let root_scope = Rc::new(Scope::new());
                let strict_mode = code.contains("use strict");
                
                self.analyze_node(&ast, &root_scope, &mut issues, code, strict_mode);
                
                // Collect all unused variables from all scopes
                self.collect_unused_variables(&root_scope, &mut issues, code);
            }
            Err(_) => {
                // Parse error, can't analyze scope
            }
        }
        
        issues
    }

    fn analyze_node(&self, node: &Node, scope: &Rc<Scope>, issues: &mut Vec<ScopeIssue>, code: &str, strict_mode: bool) {
        match &node.kind {
            NodeKind::VariableDeclaration { declarator, variable, .. } => {
                let var_name = self.extract_variable_name(variable);
                let line = self.get_line_from_node(variable, code);
                let is_our = declarator == "our";
                
                if let Some(issue_kind) = scope.declare_variable(&var_name, variable.location.start, is_our) {
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
                
                // Skip package-qualified variables
                if full_name.contains("::") {
                    return;
                }
                
                // Try to use the variable
                if !scope.use_variable(&full_name) {
                    // Variable not found - check if we should report it
                    if strict_mode {
                        issues.push(ScopeIssue {
                            kind: IssueKind::UndeclaredVariable,
                            variable_name: full_name.clone(),
                            line: self.get_line_from_node(node, code),
                            description: format!("Variable '{}' is used but not declared", full_name),
                        });
                    }
                }
            }
            
            NodeKind::Block { statements } => {
                let block_scope = Rc::new(Scope::with_parent(scope.clone()));
                for stmt in statements {
                    self.analyze_node(stmt, &block_scope, issues, code, strict_mode);
                }
                self.collect_unused_variables(&block_scope, issues, code);
            }
            
            NodeKind::For { init, condition, update, body, .. } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));
                
                if let Some(init_node) = init {
                    self.analyze_node(init_node, &loop_scope, issues, code, strict_mode);
                }
                if let Some(cond) = condition {
                    self.analyze_node(cond, &loop_scope, issues, code, strict_mode);
                }
                if let Some(upd) = update {
                    self.analyze_node(upd, &loop_scope, issues, code, strict_mode);
                }
                self.analyze_node(body, &loop_scope, issues, code, strict_mode);
                
                self.collect_unused_variables(&loop_scope, issues, code);
            }
            
            NodeKind::Foreach { variable, list, body } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));
                
                // Declare the loop variable
                self.analyze_node(variable, &loop_scope, issues, code, strict_mode);
                self.analyze_node(list, &loop_scope, issues, code, strict_mode);
                self.analyze_node(body, &loop_scope, issues, code, strict_mode);
                
                self.collect_unused_variables(&loop_scope, issues, code);
            }
            
            NodeKind::Subroutine { params, body, .. } => {
                let sub_scope = Rc::new(Scope::with_parent(scope.clone()));
                
                // Declare parameters
                for param in params {
                    if let NodeKind::Variable { sigil, name } = &param.kind {
                        let full_name = format!("{}{}", sigil, name);
                        sub_scope.declare_variable(&full_name, param.location.start, false);
                        // Mark parameters as used
                        sub_scope.use_variable(&full_name);
                    }
                }
                
                self.analyze_node(body, &sub_scope, issues, code, strict_mode);
                self.collect_unused_variables(&sub_scope, issues, code);
            }
            
            _ => {
                // Recursively analyze children
                for child in node.children() {
                    self.analyze_node(child, scope, issues, code, strict_mode);
                }
            }
        }
    }

    fn collect_unused_variables(&self, scope: &Rc<Scope>, issues: &mut Vec<ScopeIssue>, code: &str) {
        for (var_name, offset) in scope.get_unused_variables() {
            issues.push(ScopeIssue {
                kind: IssueKind::UnusedVariable,
                variable_name: var_name.clone(),
                line: self.get_line_number(code, offset),
                description: format!("Variable '{}' is declared but never used", var_name),
            });
        }
    }

    fn extract_variable_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Variable { sigil, name } => format!("{}{}", sigil, name),
            _ => {
                if let Some(child) = node.children().first() {
                    self.extract_variable_name(child)
                } else {
                    String::new()
                }
            }
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
                    format!("Consider rename '{}' to avoid shadowing", issue.variable_name)
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