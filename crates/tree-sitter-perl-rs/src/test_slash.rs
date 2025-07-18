#[cfg(test)]
mod test_slash {
    use crate::perl_lexer::{PerlLexer, TokenType};
    use std::sync::Arc;

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
}