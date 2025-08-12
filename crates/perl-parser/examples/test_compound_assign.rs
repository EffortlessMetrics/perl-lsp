//! Test compound assignment operators
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Arithmetic assignments
        "$x += 5",
        "$x -= 3",
        "$x *= 2",
        "$x /= 4",
        "$x %= 3",
        "$x **= 2",
        // String assignment
        "$str .= 'suffix'",
        // Bitwise assignments
        "$x &= 0xFF",
        "$x |= 0x01",
        "$x ^= 0xAA",
        "$x <<= 2",
        "$x >>= 1",
        // Logical assignments
        "$x &&= $y",
        "$x ||= $default",
        "$x //= 'default'",
        // In context
        "my $sum = 0; $sum += $_ for @numbers",
        "$hash{$key} ||= []",
        "$config{debug} //= 0",
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
