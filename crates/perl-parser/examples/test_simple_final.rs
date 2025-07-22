//! Simple final test
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Test each major feature
        "my $x = 42",
        "$x += 10",
        "$x and $y",
        "$x ? $y : $z",
        "-e $file",
        "print 'hello'",
        "qw(a b c)",
        "BEGIN { }",
    ];
    
    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ {}", e);
            }
        }
    }
}