//! Test list context and function calls without parentheses
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Function calls with parentheses
        "print()",
        "print('hello')",
        "print('hello', 'world')",
        // Function calls without parentheses
        "print 'hello'",
        "print 'hello', 'world'",
        "print $x, $y, $z",
        // List assignment
        "my ($x, $y) = (1, 2)",
        "my @list = (1, 2, 3)",
        // Multiple expressions on one line
        "$x = 1, $y = 2",
        // Function as statement
        "die 'error message'",
        "warn 'warning'",
        "return $value",
        "return",
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
