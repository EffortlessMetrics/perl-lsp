use perl_lexer::PerlLexer;

fn main() {
    let test_cases = vec![
        "sub foo(&@) { }",
        "sub bar(&) { }",
        "sub baz(@) { }",
        "sub mygrep(&@) { }",
    ];

    for code in test_cases {
        println!("\nCode: {}", code);
        let mut lexer = PerlLexer::new(code);
        
        // Skip to after the opening paren
        let mut in_prototype = false;
        println!("Tokens in prototype:");
        
        while let Some(token) = lexer.next_token() {
            if token.text.as_ref() == "(" {
                in_prototype = true;
                continue;
            }
            if token.text.as_ref() == ")" {
                break;
            }
            if in_prototype {
                println!("  {:?}: '{}'", token.token_type, token.text);
            }
        }
    }
}