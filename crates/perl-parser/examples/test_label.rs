use perl_parser::Parser;
use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let code = "LABEL: for (@list) { }";
    
    // First test the is_label_start detection
    let mut parser = Parser::new(code);
    println!("Testing is_label_start detection...");
    // We need to access this through public API instead
    
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Parsed successfully!");
            println!("AST S-expression: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("Parse error: {}", e);
            
            // Let's debug the tokens
            println!("\nTokens from lexer:");
            let mut lexer = PerlLexer::new(code);
            while let Some(token) = lexer.next_token() {
                println!("  {:?} '{}' at {}..{}", token.token_type, &code[token.start..token.end], token.start, token.end);
                if matches!(token.token_type, TokenType::EOF) {
                    break;
                }
            }
        }
    }
}