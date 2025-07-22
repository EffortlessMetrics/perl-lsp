//! Test qw() operator
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic qw usage
        "qw(foo bar baz)",
        "qw(hello world)",
        "qw(one)",
        "qw()",
        
        // Different delimiters
        "qw/foo bar baz/",
        "qw{foo bar baz}",
        "qw[foo bar baz]",
        "qw<foo bar baz>",
        "qw!foo bar baz!",
        
        // In context
        "my @words = qw(foo bar baz)",
        "for my $word (qw(foo bar baz)) { }",
        "use Module qw(import1 import2)",
        
        // With special characters
        "qw(foo-bar baz_qux)",
        "qw(::foo Bar::Baz)",
        
        // Multiline
        "qw(
            foo
            bar
            baz
        )",
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