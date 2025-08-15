//! AST Visitor Pattern Example
//!
//! This example shows how to implement a visitor pattern for the Perl AST.

use perl_parser::{Node, NodeKind, Parser};
use std::collections::HashMap;

/// A trait for visiting AST nodes
trait AstVisitor {
    fn visit_node(&mut self, node: &Node) {
        self.visit(&node.kind);
        self.visit_children(node);
    }

    fn visit(&mut self, kind: &NodeKind);

    fn visit_children(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left);
                self.visit_node(right);
            }
            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.visit_node(condition);
                self.visit_node(then_branch);
                for (cond, branch) in elsif_branches {
                    self.visit_node(cond);
                    self.visit_node(branch);
                }
                if let Some(else_b) = else_branch {
                    self.visit_node(else_b);
                }
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                self.visit_node(lhs);
                self.visit_node(rhs);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.visit_node(arg);
                }
            }
            NodeKind::MethodCall { object, args, .. } => {
                self.visit_node(object);
                for arg in args {
                    self.visit_node(arg);
                }
            }
            _ => {} // Leaf nodes
        }
    }
}

/// Example visitor that collects variable usage statistics
struct VariableUsageCollector {
    declarations: HashMap<String, usize>,
    usages: HashMap<String, usize>,
    current_scope: Vec<String>,
}

impl VariableUsageCollector {
    fn new() -> Self {
        Self {
            declarations: HashMap::new(),
            usages: HashMap::new(),
            current_scope: vec!["global".to_string()],
        }
    }

    fn get_variable_name(node: &Node) -> Option<String> {
        if let NodeKind::Variable { sigil, name } = &node.kind {
            Some(format!("{}{}", sigil, name))
        } else {
            None
        }
    }
}

impl AstVisitor for VariableUsageCollector {
    fn visit(&mut self, kind: &NodeKind) {
        match kind {
            NodeKind::VariableDeclaration { variable, .. } => {
                if let Some(var_name) = Self::get_variable_name(variable) {
                    *self.declarations.entry(var_name).or_insert(0) += 1;
                }
            }
            NodeKind::Variable { sigil, name } => {
                let var_name = format!("{}{}", sigil, name);
                *self.usages.entry(var_name).or_insert(0) += 1;
            }
            NodeKind::Subroutine { name: Some(n), .. } => {
                self.current_scope.push(n.clone());
            }
            NodeKind::Subroutine { name: None, .. } => {}
            NodeKind::Block { .. } => {
                self.current_scope.push("block".to_string());
            }
            _ => {}
        }
    }

    fn visit_children(&mut self, node: &Node) {
        let pushed_scope =
            matches!(&node.kind, NodeKind::Subroutine { .. } | NodeKind::Block { .. });

        // Call default implementation
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left);
                self.visit_node(right);
            }
            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.visit_node(condition);
                self.visit_node(then_branch);
                for (cond, branch) in elsif_branches {
                    self.visit_node(cond);
                    self.visit_node(branch);
                }
                if let Some(else_b) = else_branch {
                    self.visit_node(else_b);
                }
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                self.visit_node(lhs);
                self.visit_node(rhs);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.visit_node(arg);
                }
            }
            NodeKind::MethodCall { object, args, .. } => {
                self.visit_node(object);
                for arg in args {
                    self.visit_node(arg);
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                self.visit_node(variable);
                if let Some(init) = initializer {
                    self.visit_node(init);
                }
            }
            NodeKind::Subroutine { body, .. } => {
                self.visit_node(body);
            }
            _ => {}
        }

        if pushed_scope {
            self.current_scope.pop();
        }
    }
}

fn main() {
    let code = r#"
my $global = 10;
my @data = (1, 2, 3);

sub process_data {
    my ($limit) = @_;
    my $sum = 0;
    
    foreach my $item (@data) {
        if ($item < $limit) {
            $sum += $item;
        }
    }
    
    return $sum;
}

my $result = process_data($global);
print "Result: $result\n";

# Unused variable
my $unused = 42;
"#;

    let mut parser = Parser::new(code);

    match parser.parse() {
        Ok(ast) => {
            println!("🔍 Variable Usage Analysis\n");

            let mut collector = VariableUsageCollector::new();
            collector.visit_node(&ast);

            println!("📊 Variable Declarations:");
            for (var, count) in &collector.declarations {
                println!("  {} - declared {} time(s)", var, count);
            }

            println!("\n📈 Variable Usages:");
            for (var, count) in &collector.usages {
                println!("  {} - used {} time(s)", var, count);
            }

            println!("\n⚠️  Potential Issues:");

            // Find unused variables
            for var in collector.declarations.keys() {
                if !collector.usages.contains_key(var) {
                    println!("  Unused variable: {}", var);
                }
            }

            // Find undefined variables
            for var in collector.usages.keys() {
                if !collector.declarations.contains_key(var) {
                    // Skip built-ins like @_
                    if var != "@_" {
                        println!("  Possibly undefined variable: {}", var);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
        }
    }
}
