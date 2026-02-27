//! Demo of the working Perl parser integration

#![allow(unused_variables, unused_imports)]

#[cfg(not(feature = "pure-rust-standalone"))]
use tree_sitter_perl::working_parser::WorkingParser;

#[cfg(not(feature = "pure-rust-standalone"))]
fn main() {
    println!("=== Working Perl Parser Demo ===\n");

    // Test cases
    let test_cases = [
        // Basic variable declaration
        "my $x = 42;",
        // String assignment
        "my $name = \"Perl\";",
        // Array operations
        "my @array = (1, 2, 3);",
        // Hash declaration
        "my %hash = (key => \"value\");",
        // If statement
        "if ($x > 10) { print \"Large\"; }",
        // Function definition
        "sub greet { print \"Hello, World!\"; }",
        // Complex expression
        "$result = $a + $b * $c;",
        // Nested if
        "if ($x) { if ($y) { print $z; } }",
        // Multiple statements
        "my $a = 1; my $b = 2; my $c = $a + $b;",
        // Function with variables
        "sub calculate { my $x = shift; return $x * 2; }",
    ];

    for (i, code) in test_cases.iter().enumerate() {
        println!("Test {}: {}", i + 1, code);

        let mut parser = WorkingParser::new(code);
        let ast = parser.parse();

        println!("S-expression: {}", ast.to_sexp());

        // Check if parsing succeeded
        if matches!(ast.kind, tree_sitter_perl::ast::NodeKind::Error { .. }) {
            println!("❌ Failed to parse");
        } else {
            println!("✅ Parsed successfully");
        }

        println!();
    }

    // Show a more complex example
    println!("=== Complex Example ===");
    let complex_code = r#"
sub fibonacci {
    my $n = shift;
    if ($n <= 1) {
        return $n;
    }
    my $a = 0;
    my $b = 1;
    my $i = 2;
    while ($i <= $n) {
        my $temp = $a + $b;
        $a = $b;
        $b = $temp;
        $i = $i + 1;
    }
    return $b;
}

my $result = fibonacci(10);
print "Fibonacci(10) = $result\n";
"#;

    println!("Code:\n{}", complex_code);

    let mut parser = WorkingParser::new(complex_code);
    let ast = parser.parse();

    println!("\nS-expression:");
    println!("{}", ast.to_sexp());

    if !matches!(ast.kind, tree_sitter_perl::ast::NodeKind::Error { .. }) {
        println!("\n✅ Complex example parsed successfully!");
    } else {
        println!("\n❌ Complex example failed to parse");
    }
}

#[cfg(feature = "pure-rust-standalone")]
fn main() {
    eprintln!("'demo_working_parser' example is disabled with the 'pure-rust-standalone' feature.");
}
