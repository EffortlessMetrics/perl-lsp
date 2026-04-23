use perl_parser::Parser;
use perl_lexer::PerlLexer;

fn main() {
    let code = r#"format STDOUT =
@<<<< @|||| @>>>>
$a,   $b,   $c
.
"#;

    println!("Testing format declaration parsing:");
    println!("Code:\n{}", code);
    
    // Show lexer tokens
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
            println!("  AST: {:?}", ast);
            println!("  S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("  ❌ Error: {}", e);
        }
    }
}