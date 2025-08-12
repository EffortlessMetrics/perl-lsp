//! Test logical word operators
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic word operators
        "$x and $y",
        "$x or $y",
        "not $x",
        "$x xor $y",
        // Precedence tests
        "$x || $y or $z",
        "$x && $y and $z",
        // Statement modifiers
        "print 'yes' if $x and $y",
        "die 'error' unless $x or $y",
        // Complex expressions
        "$a > 0 and $b < 10",
        "defined($x) or die 'undefined'",
        "open(FILE, 'data.txt') or die $!",
        // Assignment with or
        "$x ||= 5",
        "$x //= 'default'",
        "$config{debug} ||= 1",
        // List context
        "@result = grep { $_ > 0 and $_ < 10 } @numbers",
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
