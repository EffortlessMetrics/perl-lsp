use perl_lexer::{PerlLexer, TokenType};

fn main() {
    // Test various dollar-brace patterns
    let inputs = vec!["${\tAA", "${AA", "${ AA", "${", "$ {"];

    for input in inputs {
        println!("\nInput: {:?}", input);
        let mut lx = PerlLexer::new(input);
        let mut count = 0;
        let mut non_ws_count = 0;
        while let Some(t) = lx.next_token() {
            count += 1;
            if count > 10 {
                break;
            }
            if !matches!(t.token_type, TokenType::EOF | TokenType::Whitespace | TokenType::Newline)
            {
                println!(
                    "  Token #{}: {:?}: {:?} [{},{}]",
                    non_ws_count,
                    t.token_type,
                    t.text.as_ref(),
                    t.start,
                    t.end
                );
                non_ws_count += 1;
            }
        }
        println!("  Total non-whitespace tokens: {}", non_ws_count);
    }
}
