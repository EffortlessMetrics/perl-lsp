//! Demo of the Perl parser with lexer integration

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

#[cfg(not(feature = "pure-rust"))]
compile_error!("This example requires the 'pure-rust' feature");

fn main() {
    println!("=== Perl Parser Demo ===\n");

    // Test cases
    let test_cases = vec![
        // Basic variable declaration
        "my $x = 42;",
        // String assignment
        r#"my $name = "World";"#,
        // Print statement
        r#"print "Hello, world!\n";"#,
        // Multiple statements
        r#"
my $x = 10;
my $y = 20;
my $sum = $x + $y;
print "Sum: $sum\n";
"#,
        // Function call
        "length($string);",
        // Array usage
        "my @items = (1, 2, 3);",
    ];

    for (i, source) in test_cases.iter().enumerate() {
        println!("Test case {}:", i + 1);
        println!("Source: {}", source.trim());
        println!("---");

        #[cfg(feature = "pure-rust")]
        {
            let ast = PureRustPerlParser::parse(source).unwrap();
            println!("S-expression:\n{}\n", ast.to_sexp());
        }
    }

    // Larger example
    println!("=== Complex Example ===");
    let complex = r#"
# Perl subroutine example
sub factorial {
    my $n = shift;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

my $result = factorial(5);
print "5! = $result\n";
"#;

    println!("Source:\n{}", complex);
    #[cfg(feature = "pure-rust")]
    {
        let ast = PureRustPerlParser::parse(complex).unwrap();
        println!("\nS-expression:\n{}", ast.to_sexp());

        // Show AST structure
        println!("\n=== AST Debug Output ===");
        println!("{:#?}", ast);
    }
}
