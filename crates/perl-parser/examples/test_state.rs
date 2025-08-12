//! Test state variables
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic state variables
        "state $x",
        "state $x = 42",
        "state $counter = 0",
        // Multiple state variables
        "state ($x, $y)",
        "state ($x, $y) = (1, 2)",
        // State with different types
        "state @array",
        "state @array = (1, 2, 3)",
        "state %hash",
        "state %hash = (key => 'value')",
        // State in subroutines
        "sub counter { state $n = 0; return ++$n; }",
        "sub once { state $done; return if $done++; }",
        // Complex state declarations
        "state $obj = My::Class->new()",
        "state $ref = \\$scalar",
    ];

    for test in tests {
        print!("Testing: {:40} ", test);
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
        println!();
    }
}
