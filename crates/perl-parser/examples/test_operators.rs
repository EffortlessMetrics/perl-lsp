//! Test various operators
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Spaceship operator
        "$a <=> $b",
        "1 <=> 2",
        // Power operator
        "$x ** 2",
        "2 ** 10",
        // Other comparison operators
        "$a < $b",
        "$a <= $b",
        "$a > $b",
        "$a >= $b",
        // In context
        "sort { $a <=> $b } @list",
        "map { $_ ** 2 } @nums",
    ];

    for test in tests {
        print!("Testing: {:30} ", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
