#[cfg(test)]
mod tests {
    use crate::parser::Parser;

    #[test]
    fn test_division_vs_regex() {
        // Division: $a / $b
        let code_div = "my $res = $a / $b;";
        let mut parser = Parser::new(code_div);
        let result = parser.parse();
        assert!(result.is_ok(), "Failed to parse division");
        let ast = result.unwrap();
        let sexp = ast.to_sexp();
        // Should contain binary division operator
        assert!(sexp.contains("binary_/"), "Should be parsed as division: {}", sexp);
        assert!(!sexp.contains("regex"), "Should not be parsed as regex: {}", sexp);

        // Regex match: $a =~ /pattern/
        let code_regex = "my $match = $a =~ /pattern/;";
        let mut parser2 = Parser::new(code_regex);
        let result2 = parser2.parse();
        assert!(result2.is_ok(), "Failed to parse regex match");
        let ast2 = result2.unwrap();
        let sexp2 = ast2.to_sexp();
        assert!(sexp2.contains("regex"), "Should be parsed as regex: {}", sexp2);
        
        // Regex without =~ (e.g. as function argument or in void context)
        // /pattern/ matches $_
        let code_bare_regex = "/pattern/;";
        let mut parser3 = Parser::new(code_bare_regex);
        let result3 = parser3.parse();
        assert!(result3.is_ok());
        let ast3 = result3.unwrap();
        let sexp3 = ast3.to_sexp();
        assert!(sexp3.contains("regex"), "Should be parsed as regex in void context: {}", sexp3);
    }

    #[test]
    fn test_complex_slash_chain() {
        // $x / $y / $z -> ($x / $y) / $z
        let code = "$x / $y / $z;";
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        let sexp = ast.to_sexp();
        
        // Should see two division operations
        // Format might be (binary_/ (binary_/ ... ) ...)
        assert!(sexp.matches("binary_/").count() >= 2, "Should be two divisions: {}", sexp);
        assert!(!sexp.contains("regex"), "Should not be parsed as regex: {}", sexp);
    }

    #[test]
    fn test_regex_with_variables() {
        // /$x/$y/$z/ is a regex pattern with interpolation, followed by invalid syntax or modifiers?
        // Actually /$x/ is a regex. Then $y/$z/ would be... what?
        // Perl parses /.../ as regex.
        // So /$x/ is a regex matching $_ against $x.
        // Then $y ... division?
        // /$x/ $y / $z / -> Regex, then variable, then division?
        // This is syntactically weird but let's see how we handle it.
        // Actually, typical case is s/$x/$y/ or just /$x/
        
        let code = "/$x/";
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex: {}", sexp);
    }
    
    #[test]
    fn test_division_after_function() {
        // time / 60
        // time is a nullary function. / should be division.
        let code = "time / 60;";
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division after nullary function: {}", sexp);
    }
}
