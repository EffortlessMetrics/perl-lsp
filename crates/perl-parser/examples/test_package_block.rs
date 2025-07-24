//! Test package block syntax
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Traditional package declaration
        ("package Foo;", "simple package"),
        ("package Foo::Bar;", "package with namespace"),
        
        // Package blocks (Perl 5.14+)
        ("package Foo { }", "empty package block"),
        ("package Bar { sub method { } }", "package block with method"),
        
        // The test case
        (r#"package Bar {
    use parent 'Foo';
    
    sub method {
        my $self = shift;
        $self->SUPER::method();
    }
}"#, "package block with use and method"),
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