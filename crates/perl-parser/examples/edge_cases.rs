//! Debug edge cases in perl-parser

use perl_parser::Parser;
use perl_lexer::PerlLexer;

fn test_case(name: &str, code: &str) {
    println!("\n=== {} ===", name);
    println!("Code: {}", code);
    
    // Show lexer output
    println!("\nLexer tokens:");
    let mut lexer = PerlLexer::new(code);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }
    
    // Try parsing
    println!("\nParser result:");
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("  ✅ Success!");
            println!("  S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("  ❌ Error: {}", e);
        }
    }
}

fn main() {
    // Test the failing cases
    test_case("While loop", "while ($i < 10) { $i++; }");
    test_case("Postfix increment", "$i++;");
    test_case("Method call", "$obj->method($arg);");
    test_case("Complex expression", "$result = ($a + $b) * $c;");
    test_case("For loop", "for (my $i = 0; $i < 10; $i++) { print $i; }");
    test_case("String interpolation", "print \"Hello, $name!\";");
    
    // Test nested structures
    test_case("Nested if/while", r#"
if ($x) {
    while ($y) {
        print $z;
    }
}"#);
}