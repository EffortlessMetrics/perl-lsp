#[cfg(test)]
mod simple_tests {
    use crate::test_harness::parse_perl_code;

    #[test]
    fn test_basic_parsing() {
        let code = "print 'Hello';";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse basic Perl code");
    }

    #[test]
    fn test_variable_declaration() {
        let code = "my $var = 42;";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse variable declaration");
    }

    #[test]
    fn test_function_call() {
        let code = "sub test { return 1; }";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse function definition");
    }
} 