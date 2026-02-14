//! Basic usage example for perl-parser
//!
//! This example demonstrates how to parse Perl code and work with the resulting AST.

use perl_parser::Parser;

fn main() {
    // Read from file if provided, otherwise use default code
    let code = if std::env::args().len() > 1 {
        let filename = std::env::args().nth(1).unwrap();
        match std::fs::read_to_string(&filename) {
            Ok(content) => {
                println!("🔍 Parsing file: {}", filename);
                println!("📊 File size: {} bytes", content.len());
                content
            }
            Err(e) => {
                eprintln!("Error reading file '{}': {}", filename, e);
                std::process::exit(1);
            }
        }
    } else {
        r#"
# A simple Perl script
my $name = "World";
my $count = 42;

sub greet {
    my ($who) = @_;
    print "Hello, $who!\n";
}

greet($name);

for (my $i = 0; $i < $count; $i++) {
    print "$i ";
}
print "\n;
"#.to_string()
    };

    // Create a parser and parse the code
    let mut parser = Parser::new(code);

    match parser.parse() {
        Ok(ast) => {
            println!("✅ Parse successful!\n");

            // Output the S-expression representation
            println!("S-Expression:");
            println!("{}\n", ast.to_sexp());

            // Count different node types
            let stats = analyze_ast(&ast);
            println!("AST Statistics:");
            println!("  Total nodes: {}", stats.total_nodes);
            println!("  Variables: {}", stats.variables);
            println!("  Function calls: {}", stats.function_calls);
            println!("  Subroutines: {}", stats.subroutines);
            println!("  Loops: {}", stats.loops);

            // Walk the AST and print variable declarations
            println!("\nVariable Declarations:");
            walk_variables(&ast, 0);
        }
        Err(e) => {
            eprintln!("❌ Parse error: {}", e);
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
        NodeKind::For { body, init, condition, update, continue_block, .. } => {
            stats.loops += 1;
            if let Some(i) = init {
                count_nodes(i, stats);
            }
            if let Some(c) = condition {
                count_nodes(c, stats);
            }
            if let Some(u) = update {
                count_nodes(u, stats);
            }
            count_nodes(body, stats);
            if let Some(cont) = continue_block {
                count_nodes(cont, stats);
            }
        }
        NodeKind::While { body, .. } => {
            stats.loops += 1;
            count_nodes(body, stats);
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

fn walk_variables(node: &perl_parser::Node, depth: usize) {
    use perl_parser::NodeKind;

    let indent = "  ".repeat(depth);

    match &node.kind {
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            print!("{}{}  ", indent, declarator);
            if let NodeKind::Variable { sigil, name } = &variable.kind {
                print!("{}{}", sigil, name);
                if let Some(init) = initializer {
                    print!(" = ");
                    print_value(init);
                }
                println!();
            }
        }

        // Recurse into compound nodes
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for stmt in statements {
                walk_variables(stmt, depth);
            }
        }
        NodeKind::Subroutine { body, .. } => {
            walk_variables(body, depth + 1);
        }
        NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
            walk_variables(then_branch, depth);
            for (_, branch) in elsif_branches {
                walk_variables(branch, depth);
            }
            if let Some(else_b) = else_branch {
                walk_variables(else_b, depth);
            }
        }
        NodeKind::For { body, .. } | NodeKind::While { body, .. } => {
            walk_variables(body, depth);
        }
        _ => {} // Skip other nodes
    }
}

fn print_value(node: &perl_parser::Node) {
    use perl_parser::NodeKind;

    match &node.kind {
        NodeKind::String { value, interpolated: _ } => print!("{:?}", value),
        NodeKind::Number { value } => print!("{}", value),
        NodeKind::Variable { sigil, name } => print!("{}{}", sigil, name),
        _ => print!("<expression>"),
    }
}
