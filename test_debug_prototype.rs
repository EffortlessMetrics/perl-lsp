use perl_parser::Parser;

fn main() {
    let code = r#"sub mygrep(&@) { }"#;
    println!("Code: {}", code);
    
    // Create lexer to see tokens
    let mut lexer = perl_lexer::PerlLexer::new(code);
    println!("\nTokens:");
    while let Some(token) = lexer.next_token() {
        println!("  {:?}: '{}'", token.token_type, token.text);
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }
    
    // Now parse
    println!("\nParsing:");
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ SUCCESS!");
            println!("S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
        }
    }
}