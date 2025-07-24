//! Test anonymous subroutines
use perl_parser::Parser;
use perl_lexer::PerlLexer;

fn main() {
    // Test the specific failing case
    let input = r#"my $anon = sub { return "anonymous"; };"#;
    println!("=== Testing: {} ===", input);
    
    // First check lexer output
    println!("\nLexer output:");
    let mut lexer = PerlLexer::new(input);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }
    
    // Then try parser
    println!("\nParser output:");
    let mut parser = Parser::new(input);
    match parser.parse() {
        Ok(ast) => {
            println!("  Success! AST: {:?}", ast);
            println!("  S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
    
    let tests = vec![
        // Basic anonymous subs
        "sub { }",
        "sub { return 42 }",
        "sub { my $x = shift; return $x + 1 }",
        
        // Assigned to variables
        "my $f = sub { }",
        "my $add = sub { $_[0] + $_[1] }",
        "our $handler = sub { die 'Not implemented' }",
        
        // As arguments
        "map { $_ * 2 } @list",
        "grep { $_ > 10 } @numbers",
        "sort { $a <=> $b } @values",
        
        // With signatures (modern Perl)
        "sub ($x) { $x * 2 }",
        "sub ($x, $y) { $x + $y }",
        
        // In expressions
        "(sub { 42 })->()",
        "my $result = (sub { $_[0] ** 2 })->(5)",
    ];

    for test in tests {
        print!("Testing: {:50} ", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}