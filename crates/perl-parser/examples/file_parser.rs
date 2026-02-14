//! File parser example for perl-parser
//!
//! This example demonstrates how to parse Perl files and analyze them.

use perl_parser::Parser;
use std::env;
use std::fs;
use std::process;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <perl-file>", args[0]);
        process::exit(1);
    }
    
    let filename = &args[1];
    
    // Read the file
    let source = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };
    
    println!("🔍 Parsing file: {}", filename);
    println!("📊 File size: {} bytes", source.len());
    println!("📝 Lines: {}", source.lines().count());
    
    // Parse the source with timing
    let start = Instant::now();
    let mut parser = Parser::new(&source);
    
    match parser.parse() {
        Ok(ast) => {
            let duration = start.elapsed();
            println!("✅ Parse successful in {:?}", duration);
            
            // Analyze the AST
            let stats = analyze_ast(&ast);
            println!("\n📈 AST Statistics:");
            println!("  Total nodes: {}", stats.total_nodes);
            println!("  Variables: {}", stats.variables);
            println!("  Function calls: {}", stats.function_calls);
            println!("  Subroutines: {}", stats.subroutines);
            println!("  Loops: {}", stats.loops);
            println!("  Packages: {}", stats.packages);
            println!("  Uses/imports: {}", stats.uses);
            
            // Check for complex features
            println!("\n🔧 Features detected:");
            if stats.has_unicode {
                println!("  ✅ Unicode identifiers");
            }
            if stats.has_heredocs {
                println!("  ✅ Heredocs");
            }
            if stats.has_regex {
                println!("  ✅ Regular expressions");
            }
            if stats.has_references {
                println!("  ✅ References");
            }
            if stats.has_complex_data {
                println!("  ✅ Complex data structures");
            }
            
            // Print first 500 chars of S-expression
            let sexp = ast.to_sexp();
            println!("\n🌳 S-Expression (first 500 chars):");
            if sexp.len() > 500 {
                println!("{}...", &sexp[..500]);
            } else {
                println!("{}", sexp);
            }
        }
        Err(e) => {
            let duration = start.elapsed();
            println!("❌ Parse error after {:?}: {}", duration, e);
        }
    }
}

#[derive(Default)]
struct AstStats {
    total_nodes: usize,
    variables: usize,
    function_calls: usize,
    subroutines: usize,
    loops: usize,
    packages: usize,
    uses: usize,
    has_unicode: bool,
    has_heredocs: bool,
    has_regex: bool,
    has_references: bool,
    has_complex_data: bool,
}

fn analyze_ast(node: &perl_parser::Node) -> AstStats {
    let mut stats = AstStats::default();
    count_nodes(node, &mut stats);
    stats
}

fn count_nodes(node: &perl_parser::Node, stats: &mut AstStats) {
    use perl_parser::NodeKind;
    
    stats.total_nodes += 1;
    
    match &node.kind {
        NodeKind::Variable { .. } => stats.variables += 1,
        NodeKind::FunctionCall { args, .. } => {
            stats.function_calls += 1;
            for arg in args {
                count_nodes(arg, stats);
            }
        }
        NodeKind::Subroutine { body, .. } => {
            stats.subroutines += 1;
            count_nodes(body, stats);
        }
        NodeKind::Package { block, .. } => {
            stats.packages += 1;
            if let Some(b) = block {
                count_nodes(b, stats);
            }
        }
        NodeKind::Use { .. } => stats.uses += 1,
        NodeKind::For { body, init, condition, update, continue_block, .. } => {
            stats.loops += 1;
            if let Some(i) = init { count_nodes(i, stats); }
            if let Some(c) = condition { count_nodes(c, stats); }
            if let Some(u) = update { count_nodes(u, stats); }
            count_nodes(body, stats);
            if let Some(cont) = continue_block { count_nodes(cont, stats); }
        }
        NodeKind::While { body, .. } => {
            stats.loops += 1;
            count_nodes(body, stats);
        }
        NodeKind::Heredoc { .. } => stats.has_heredocs = true,
        NodeKind::Regex { .. } => stats.has_regex = true,
        NodeKind::Unary { op, .. } if op == "\\" => stats.has_references = true,
        NodeKind::HashLiteral { .. } | NodeKind::ArrayLiteral { .. } => stats.has_complex_data = true,
        
        // Check for Unicode in variable names
        NodeKind::VariableDeclaration { variable, .. } => {
            count_nodes(variable, stats);
        }
        
        // Recurse into child nodes
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for stmt in statements {
                count_nodes(stmt, stats);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            count_nodes(left, stats);
            count_nodes(right, stats);
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            count_nodes(condition, stats);
            count_nodes(then_branch, stats);
            for (cond, branch) in elsif_branches {
                count_nodes(cond, stats);
                count_nodes(branch, stats);
            }
            if let Some(else_b) = else_branch {
                count_nodes(else_b, stats);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            count_nodes(lhs, stats);
            count_nodes(rhs, stats);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            count_nodes(variable, stats);
            if let Some(init) = initializer {
                count_nodes(init, stats);
            }
        }
        _ => {} // Leaf nodes
    }
}