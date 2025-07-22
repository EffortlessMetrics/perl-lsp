//! Test bless and object-oriented features
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic bless
        "bless $ref",
        "bless $ref, 'MyClass'",
        "bless {}, 'MyClass'",
        "bless [], 'MyClass'",
        
        // Bless in context
        "my $obj = bless {}, 'MyClass'",
        "return bless $self, $class",
        
        // Constructor pattern
        "sub new { my $class = shift; bless {}, $class }",
        
        // Method calls on blessed references
        "$obj->method()",
        "$obj->method($arg1, $arg2)",
        "MyClass->new()",
        "MyClass->new($arg1, $arg2)",
        
        // ref operator
        "ref $obj",
        "if (ref $obj eq 'MyClass') { }",
        
        // isa operator
        "$obj->isa('MyClass')",
        "if ($obj->isa('MyClass')) { }",
        
        // UNIVERSAL methods
        "$obj->can('method')",
        "$obj->VERSION",
        
        // DESTROY method
        "sub DESTROY { my $self = shift; }",
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