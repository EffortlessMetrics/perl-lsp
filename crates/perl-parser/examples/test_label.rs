use perl_lexer::{PerlLexer, TokenType};
use perl_parser::Parser;

fn main() {
    let code = std::env::args()
        .nth(1)
        .map(|path| std::fs::read_to_string(path).unwrap())
        .unwrap_or_else(|| "LABEL: for (@list) { }".to_string());

    let mut parser = Parser::new(&code);
    match parser.parse() {
        Ok(ast) => {
            println!("Parsed successfully!");
            println!("AST S-expression: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("Parse error: {}", e);

            // Let's debug the tokens
            println!("\nTokens from lexer:");
            let mut lexer = PerlLexer::new(&code);
            while let Some(token) = lexer.next_token() {
                println!(
                    "  {:?} '{}' at {}..{}",
                    token.token_type,
                    &code[token.start..token.end],
                    token.start,
                    token.end
                );
                if matches!(token.token_type, TokenType::EOF) {
                    break;
                }
            }
        }
    }
}
