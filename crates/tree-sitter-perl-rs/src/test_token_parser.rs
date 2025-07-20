//! Test the token parser components

#[cfg(test)]
mod tests {
    use crate::simple_token::{Token, PerlLexer};
    use crate::context_lexer_simple::ContextLexer;
    
    #[test]
    fn test_basic_token_lexing() {
        let input = "my $x = 42;";
        let mut lexer = PerlLexer::new(input);
        
        assert_eq!(lexer.next_token(), Token::My);
        assert_eq!(lexer.next_token(), Token::ScalarVar);
        assert_eq!(lexer.next_token(), Token::Assign);
        assert_eq!(lexer.next_token(), Token::IntegerLiteral);
        assert_eq!(lexer.next_token(), Token::Semicolon);
        assert_eq!(lexer.next_token(), Token::Eof);
    }
    
    #[test]
    fn test_context_aware_slash_division() {
        let input = "$x / 2";
        let mut lexer = ContextLexer::new(input);
        
        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::Divide)); // Should be division
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
    }
    
    #[test]
    fn test_context_aware_slash_regex() {
        let input = "if (/test/)";
        let mut lexer = ContextLexer::new(input);
        
        assert_eq!(lexer.next(), Some(Token::If));
        assert_eq!(lexer.next(), Some(Token::LParen));
        assert_eq!(lexer.next(), Some(Token::Regex)); // Should be regex
        assert_eq!(lexer.next(), Some(Token::RParen));
    }
    
    #[test]
    fn test_complex_slash_disambiguation() {
        let input = "$x = 10 / 2 + /test/";
        let mut lexer = ContextLexer::new(input);
        
        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::Assign));
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
        assert_eq!(lexer.next(), Some(Token::Divide)); // Division
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
        assert_eq!(lexer.next(), Some(Token::Plus));
        assert_eq!(lexer.next(), Some(Token::Regex)); // Regex
    }
    
    #[test]
    fn test_regex_with_match_operator() {
        let input = "$str =~ /pattern/i";
        let mut lexer = ContextLexer::new(input);
        
        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::BinMatch));
        assert_eq!(lexer.next(), Some(Token::Regex)); // Should parse as regex with modifiers
    }
    
    #[test]
    fn test_division_after_parenthesis() {
        let input = "($a + $b) / 2";
        let mut lexer = ContextLexer::new(input);
        
        assert_eq!(lexer.next(), Some(Token::LParen));
        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::Plus));
        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::RParen));
        assert_eq!(lexer.next(), Some(Token::Divide)); // Should be division
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
    }
}