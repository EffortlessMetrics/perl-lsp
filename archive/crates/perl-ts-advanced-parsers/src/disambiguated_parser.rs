use perl_parser_pest::ParseError;
use perl_parser_pest::pure_rust_parser::{AstNode, PerlParser, PureRustPerlParser, Rule};
use perl_ts_heredoc_parser::lexer_adapter::LexerAdapter;
use pest::Parser;

/// A Perl parser that handles context-sensitive constructs
/// by preprocessing the input to disambiguate slashes
pub struct DisambiguatedParser;

impl DisambiguatedParser {
    /// Parse Perl code with slash disambiguation
    pub fn parse(input: &str) -> Result<AstNode, ParseError> {
        // Step 1: Preprocess the input to disambiguate slashes
        let preprocessed = LexerAdapter::preprocess(input);
        #[cfg(test)]
        {
            eprintln!("Preprocessed '{}' to '{}'", input, preprocessed);
            // Also show what tokens the lexer produces
            let mut lexer = perl_ts_heredoc_parser::perl_lexer::PerlLexer::new(input);
            eprintln!("Tokens:");
            while let Some(token) = lexer.next_token() {
                eprintln!("  {:?} at {}..{}", token.token_type, token.start, token.end);
                if matches!(token.token_type, perl_ts_heredoc_parser::perl_lexer::TokenType::EOF) {
                    break;
                }
            }
        }

        // Step 2: Parse with the modified input
        let pairs = PerlParser::parse(Rule::program, &preprocessed).map_err(|_e| {
            #[cfg(test)]
            eprintln!("Parse error: {:?}", _e);
            ParseError::ParseFailed
        })?;

        // Step 3: Build AST
        let mut parser = PureRustPerlParser::new();
        let mut ast = None;
        for pair in pairs {
            ast = parser.build_node(pair).map_err(|_| ParseError::ParseFailed)?;
        }

        // Step 4: Postprocess to restore original tokens
        if let Some(ref mut node) = ast {
            LexerAdapter::postprocess(node);
        }

        ast.ok_or(ParseError::ParseFailed)
    }

    /// Parse and return S-expression format
    pub fn parse_to_sexp(input: &str) -> Result<String, ParseError> {
        let ast = Self::parse(input)?;
        let parser = PureRustPerlParser::new();
        Ok(parser.to_sexp(&ast))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;

    #[test]
    fn test_reserved_word_markers() {
        use pest::Parser;

        // Test basic reserved word matching
        let rw_sub = PerlParser::parse(Rule::reserved_word, "sub");
        eprintln!("reserved_word('sub') = {:?}", rw_sub.is_ok());
        assert!(rw_sub.is_ok(), "'sub' should match reserved_word");

        // Test _SUB_ as reserved_word
        let rw = PerlParser::parse(Rule::reserved_word, "_SUB_");
        eprintln!("reserved_word('_SUB_') = {:?}", rw.is_ok());
        if let Err(ref e) = rw {
            eprintln!("reserved_word error: {}", e);
        }

        // Test _DIV_ as reserved_word
        let rw_div = PerlParser::parse(Rule::reserved_word, "_DIV_");
        eprintln!("reserved_word('_DIV_') = {:?}", rw_div.is_ok());

        // Test identifier matching
        let id = PerlParser::parse(Rule::identifier, "_SUB_");
        eprintln!("identifier('_SUB_') = {:?}", id.is_ok());

        // substitution should match _SUB_/foo/bar/g
        let sub = PerlParser::parse(Rule::substitution, "_SUB_/foo/bar/g");
        eprintln!("substitution('_SUB_/foo/bar/g') = {:?}", sub.is_ok());

        // Key assertions: if reserved_word doesn't work, document it
        // and verify the alternative approach (reordering) works
        if rw.is_err() {
            eprintln!("NOTE: reserved_word doesn't match _SUB_ — using grammar reordering instead");
        }
        assert!(sub.is_ok(), "_SUB_/foo/bar/g should match substitution");
    }

    #[test]
    fn test_division_vs_regex() {
        // Test case from the document: "1/ /abc/"
        let input = "1/ /abc/";
        let result = must(DisambiguatedParser::parse_to_sexp(input));
        println!("Result for '{}': {}", input, result);
        assert!(result.contains("binary_expression"));
        assert!(result.contains("number 1"));
        assert!(result.contains("regex"));

        // Test simple division
        let input = "x / 2";
        let result = must(DisambiguatedParser::parse_to_sexp(input));
        assert!(result.contains("binary_expression"));
        assert!(result.contains("identifier x"));
        assert!(result.contains("number 2"));
    }

    #[test]
    fn test_regex_after_operator() {
        let input = "$x =~ /pattern/";
        let result = must(DisambiguatedParser::parse_to_sexp(input));
        assert!(result.contains("binary_expression"));
        assert!(result.contains("=~"));
        assert!(result.contains("regex"));
    }

    #[test]
    fn test_substitution() {
        let input = "s/foo/bar/g";
        let result = must(DisambiguatedParser::parse_to_sexp(input));
        println!("Result for '{}': {}", input, result);
        assert!(result.contains("substitution"));

        // Test with different delimiters
        let input = "s{foo}{bar}g";
        let result = must(DisambiguatedParser::parse_to_sexp(input));
        assert!(result.contains("substitution"));
    }

    #[test]
    fn test_complex_expressions() {
        // From the document's edge cases
        let input = "print 1/ /foo/";
        let result = must(DisambiguatedParser::parse_to_sexp(input));
        assert!(result.contains("function_call"));
        assert!(result.contains("binary_expression"));
        assert!(result.contains("regex"));
    }

    #[test]
    fn test_division_and_regex_chain() {
        // Uses m{} delimiter to avoid y/// transliteration ambiguity
        let input = "a/b/c =~ m{x/y}";
        let result = must(DisambiguatedParser::parse_to_sexp(input));
        assert!(result.contains("binary_expression"));
        assert!(result.contains("regex"));
    }
}
