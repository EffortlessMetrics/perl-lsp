//! Test function calls without parentheses
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic calls
        "print 'hello'",
        "print 'hello', 'world'",
        "die 'error'",
        "warn 'warning'",
        // With variables
        "print $x",
        "print $x, $y",
        "print @array",
        // Multiple arguments
        "push @array, 1, 2, 3",
        "join ', ', @list",
        // Chained calls
        "print sort @array",
        "print reverse sort @array",
        // With operators
        "print $x + $y",
        "print $x . $y",
        // Statement vs expression
        "my $x = shift",
        "my $len = length $str",
        // Mixed
        "open FILE, '<', 'data.txt'",
        "close FILE",
    ];

    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
