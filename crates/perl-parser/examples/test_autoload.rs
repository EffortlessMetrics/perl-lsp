//! Test AUTOLOAD support
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic AUTOLOAD
        "sub AUTOLOAD { }",
        "sub AUTOLOAD { print $AUTOLOAD }",
        
        // AUTOLOAD with our
        "our $AUTOLOAD",
        
        // AUTOLOAD in packages
        "package MyClass; sub AUTOLOAD { my $method = $AUTOLOAD; }",
        
        // Using AUTOLOAD variable
        "$AUTOLOAD",
        "$AUTOLOAD =~ s/.*:://",
        "my $method = $AUTOLOAD",
        
        // AUTOLOAD with DESTROY
        "sub AUTOLOAD { return if $AUTOLOAD =~ /::DESTROY$/; }",
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