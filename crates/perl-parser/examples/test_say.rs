//! Test say builtin
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic say
        "say 'Hello, World!'",
        "say \"Hello, World!\"",
        "say $message",
        // Say with multiple arguments
        "say 'Hello', ' ', 'World'",
        "say $x, $y, $z",
        // Say with no arguments
        "say",
        "say()",
        // Say with filehandle
        "say STDERR 'Error message'",
        "say $fh 'Output'",
        // Say with statement modifiers
        "say 'Debug' if $debug",
        "say $_ for @items",
        // Say in subroutines
        "sub greet { say 'Hello' }",
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
