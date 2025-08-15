//! Test modern Perl features
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Version declaration
        ("use v5.36;", "version declaration"),
        ("use v5.32.1;", "version with patch level"),
        ("use 5.036;", "numeric version"),
        // Try/catch
        ("try { die 'oops' } catch ($e) { warn $e }", "try/catch"),
        ("try { risky() } catch ($e) { handle($e) }", "try/catch with calls"),
        // Defer
        ("defer { cleanup() }", "defer block"),
        // Class/method (Corinna)
        ("class Point { }", "empty class"),
        ("class Point { field $x; }", "class with field"),
        ("class Point { method new() { } }", "class with method"),
        ("method new($x, $y) { }", "method with signature"),
        // Field declarations
        ("field $x;", "field declaration"),
        ("field $x = 42;", "field with default"),
    ];

    for (test, desc) in tests {
        println!("\n=== Testing: {} ===", desc);
        println!("Input: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success! S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
