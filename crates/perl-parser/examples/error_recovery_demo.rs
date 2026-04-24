//! Demonstration of error recovery in the Perl parser
//!
//! This example shows how the parser can continue parsing after encountering
//! syntax errors, producing partial ASTs with error nodes.

use perl_parser::{NodeKind, Parser};

fn main() {
    println!("=== Error Recovery Parser Demo ===\n");

    // Test cases with various syntax errors
    let test_cases = vec![
        ("Valid code", "my $x = 42; my $y = 99;"),
        ("Missing value in assignment", "my $x = ; my $y = 99;"),
        ("Missing semicolons", "my $x = 42 my $y = 99"),
        ("Unclosed block", "if ($condition) { my $x = 42"),
        ("Multiple errors", "my $x = \nif { \nmy $y ="),
        ("Incomplete variable declaration", "my = 42; our $valid = 99;"),
    ];

    for (description, code) in test_cases {
        println!("Test: {}", description);
        println!("Code: {}", code);
        println!("---");

        let mut parser = Parser::new(code);
        let output = parser.parse_with_recovery();

        // Print AST
        println!("AST: {}", output.ast.to_sexp());

        // Print statements count
        if let NodeKind::Program { statements } = &output.ast.kind {
            println!("Parsed {} statements", statements.len());

            // Show error nodes
            for (i, stmt) in statements.iter().enumerate() {
                match &stmt.kind {
                    NodeKind::Error { message, expected, .. } => {
                        println!("  Statement {}: ERROR - {}", i + 1, message);
                        if !expected.is_empty() {
                            let names: Vec<String> =
                                expected.iter().map(|t| format!("{:?}", t)).collect();
                            println!("    Expected: {}", names.join(", "));
                        }
                    }
                    NodeKind::MissingExpression => {
                        println!("  Statement {}: MISSING EXPRESSION", i + 1);
                    }
                    _ => {
                        println!("  Statement {}: {}", i + 1, stmt.to_sexp());
                    }
                }
            }
        }

        // Print errors
        if !output.diagnostics.is_empty() {
            println!("\nErrors found:");
            for (i, error) in output.diagnostics.iter().enumerate() {
                println!("  {}. {}", i + 1, error);
            }
        } else {
            println!("\nNo errors found!");
        }

        println!("\n---\n");
    }

    // Demonstrate recovery statistics
    println!("=== Recovery Statistics Demo ===\n");

    let complex_code = r#"
# Valid function
sub valid_func {
    my $x = 42;
}

# Missing closing brace
sub broken_func {
    my $y =

# Another valid function (parser should recover)
sub recovered_func {
    return 99;
}

# Missing semicolon and value
my $global =
my $another = "recovered"

# Final valid statement
print "Done";
"#;

    let mut parser = Parser::new(complex_code);
    let output = parser.parse_with_recovery();

    // Count different types of nodes
    let mut valid_count = 0;
    let mut error_count = 0;
    let mut missing_count = 0;

    fn count_nodes(
        node: &perl_parser::Node,
        valid: &mut usize,
        error: &mut usize,
        missing: &mut usize,
    ) {
        match &node.kind {
            NodeKind::Error { .. } => *error += 1,
            NodeKind::MissingExpression | NodeKind::MissingStatement => *missing += 1,
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    count_nodes(stmt, valid, error, missing);
                }
            }
            _ => {
                *valid += 1;
                // Count children recursively
                match &node.kind {
                    NodeKind::VariableDeclaration { variable, initializer, .. } => {
                        count_nodes(variable, valid, error, missing);
                        if let Some(init) = initializer {
                            count_nodes(init, valid, error, missing);
                        }
                    }
                    NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                        count_nodes(condition, valid, error, missing);
                        count_nodes(then_branch, valid, error, missing);
                        for (cond, branch) in elsif_branches {
                            count_nodes(cond, valid, error, missing);
                            count_nodes(branch, valid, error, missing);
                        }
                        if let Some(branch) = else_branch {
                            count_nodes(branch, valid, error, missing);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    count_nodes(&output.ast, &mut valid_count, &mut error_count, &mut missing_count);

    println!("Complex code parsing results:");
    println!("  Valid nodes: {}", valid_count);
    println!("  Error nodes: {}", error_count);
    println!("  Missing nodes: {}", missing_count);
    println!("  Total errors reported: {}", output.diagnostics.len());
    println!("  Terminated early: {}", output.terminated_early);
    println!(
        "  Recovery rate: {:.1}%",
        (valid_count as f64 / (valid_count + error_count + missing_count).max(1) as f64) * 100.0
    );
}
