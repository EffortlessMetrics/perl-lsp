#[cfg(test)]
mod test_slash {
    use crate::perl_lexer::{PerlLexer, TokenType};

    #[test]
    fn test_basic_disambiguation() {
        // Test 1: Division after identifier
        let mut lexer = PerlLexer::new("x / 2");
        
        let token1 = lexer.next_token().unwrap();
        assert!(matches!(token1.token_type, TokenType::Identifier(_)));
        
        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, TokenType::Division);
        
        let token3 = lexer.next_token().unwrap();
        assert!(matches!(token3.token_type, TokenType::Number(_)));
        
        // Test 2: Regex after operator
        let mut lexer = PerlLexer::new("=~ /foo/");
        
        let token1 = lexer.next_token().unwrap();
        assert!(matches!(token1.token_type, TokenType::Operator(ref op) if op.as_ref() == "=~"));
        
        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, TokenType::RegexMatch);
        assert!(token2.text.contains("foo"));
    }
    
    #[test]
    fn test_complex_cases() {
        // Test: 1/ /abc/
        let mut lexer = PerlLexer::new("1/ /abc/");
        
        let token1 = lexer.next_token().unwrap();
        assert!(matches!(token1.token_type, TokenType::Number(_)));
        
        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, TokenType::Division);
        
        let token3 = lexer.next_token().unwrap();
        assert_eq!(token3.token_type, TokenType::RegexMatch);
        assert!(token3.text.contains("abc"));
    }
    
    #[test]
    fn test_substitution() {
        let mut lexer = PerlLexer::new("s/foo/bar/g");
        
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
        assert_eq!(token.text.as_ref(), "s/foo/bar/g");
        
        // Test with braces
        let mut lexer = PerlLexer::new("s{foo}{bar}g");
        
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
    }
    
    #[test]
    fn test_token_positions() {
        let input = "my $x = 42 + 3.14;";
        let mut lexer = PerlLexer::new(input);
        
        // "my"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Keyword(ref k) if k.as_ref() == "my"));
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 2);
        assert_eq!(&input[token.start..token.end], "my");
        
        // "$x"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "$x"));
        assert_eq!(token.start, 3);
        assert_eq!(token.end, 5);
        assert_eq!(&input[token.start..token.end], "$x");
        
        // "="
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "="));
        assert_eq!(token.start, 6);
        assert_eq!(token.end, 7);
        assert_eq!(&input[token.start..token.end], "=");
        
        // "42"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Number(ref n) if n.as_ref() == "42"));
        assert_eq!(token.start, 8);
        assert_eq!(token.end, 10);
        assert_eq!(&input[token.start..token.end], "42");
        
        // "+"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "+"));
        assert_eq!(token.start, 11);
        assert_eq!(token.end, 12);
        assert_eq!(&input[token.start..token.end], "+");
        
        // "3.14"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Number(ref n) if n.as_ref() == "3.14"));
        assert_eq!(token.start, 13);
        assert_eq!(token.end, 17);
        assert_eq!(&input[token.start..token.end], "3.14");
        
        // ";"
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Semicolon);
        assert_eq!(token.start, 17);
        assert_eq!(token.end, 18);
        assert_eq!(&input[token.start..token.end], ";");
    }
    
    #[test]
    fn test_variable_types() {
        // Test scalar
        let mut lexer = PerlLexer::new("$foo");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "$foo"));
        
        // Test array
        let mut lexer = PerlLexer::new("@bar");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "@bar"));
        
        // Test hash
        let mut lexer = PerlLexer::new("%baz");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "%baz"));
        
        // Test glob
        let mut lexer = PerlLexer::new("*STDOUT");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "*STDOUT"));
    }
    
    #[test]
    fn test_operators() {
        let input = "=~ !~ == != <= >= <=> .. ...";
        let mut lexer = PerlLexer::new(input);
        
        let expected = vec!["=~", "!~", "==", "!=", "<=", ">=", "<=>", "..", "..."];
        
        for exp in expected {
            let token = lexer.next_token().unwrap();
            assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == exp), 
                   "Expected operator {}, got {:?}", exp, token);
        }
    }
    
    #[test]
    fn test_edge_cases() {
        // Empty variable (just sigil)
        let mut lexer = PerlLexer::new("$ ");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "$"));
        
        // Modulo operator
        let mut lexer = PerlLexer::new("10 % 3");
        let _num = lexer.next_token().unwrap();
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "%"));
        
        // Multiplication
        let mut lexer = PerlLexer::new("5 * 3");
        let _num = lexer.next_token().unwrap();
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "*"));
    }
    
    #[test]
    fn test_regex_operators() {
        // Match operator
        let mut lexer = PerlLexer::new("m/pattern/i");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::RegexMatch);
        assert!(token.text.contains("pattern"));
        
        // Transliteration
        let mut lexer = PerlLexer::new("tr/abc/def/");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Transliteration);
        
        // Quote regex
        let mut lexer = PerlLexer::new("qr{pattern}i");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::QuoteRegex);
    }
}