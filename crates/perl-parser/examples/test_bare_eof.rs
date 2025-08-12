//! Test bare statements at EOF
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Bare builtins at EOF
        "print",
        "say",
        "return",
        "die",
        "warn",
        "exit",
        // With semicolon
        "print;",
        "say;",
        "return;",
        // In blocks
        "{ print }",
        "{ say }",
        "{ return }",
        // As last statement in program
        "my $x = 1;\nprint",
        "$x = 42;\nsay",
    ];

    for test in tests {
        print!("Testing: {:30} ", test.replace('\n', "\\n"));
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅");
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
