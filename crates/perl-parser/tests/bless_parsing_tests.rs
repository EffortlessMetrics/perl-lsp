#[cfg(test)]
mod bless_parsing_tests {
    use perl_parser::Parser;

    fn parse_and_check(input: &str, expected_sexp: &str) {
        let mut parser = Parser::new(input);
        let result = parser.parse().expect("Failed to parse");
        let sexp = result.to_sexp();
        assert_eq!(sexp.trim(), expected_sexp.trim(), "Input: {}", input);
    }

    #[test]
    fn test_bless_empty_hash() {
        parse_and_check("bless {}", "(source_file (call bless ((hash ))))");
    }

    #[test]
    fn test_bless_empty_hash_with_class() {
        parse_and_check(
            "bless {}, $class",
            "(source_file (call bless ((hash ) (variable $ class))))",
        );
    }

    #[test]
    fn test_bless_with_string_literal() {
        parse_and_check(
            "bless {}, 'Foo'",
            "(source_file (call bless ((hash ) (string \"'Foo'\"))))",
        );
    }

    #[test]
    fn test_return_bless_empty_hash() {
        parse_and_check("return bless {}", "(source_file (return (call bless ((hash )))))");
    }

    #[test]
    fn test_return_bless_with_class() {
        parse_and_check(
            "return bless {}, $class",
            "(source_file (return (call bless ((hash ) (variable $ class)))))",
        );
    }

    #[test]
    fn test_bless_in_subroutine() {
        parse_and_check(
            "sub new { return bless {}, shift; }",
            "(source_file (sub new ()(block (return (call bless ((hash ) (call shift ())))))))",
        );
    }

    #[test]
    fn test_bless_with_hashref_data() {
        parse_and_check(
            "bless { foo => 1, bar => 2 }, $class",
            "(source_file (call bless ((hash ((identifier foo) (number 1)) ((identifier bar) (number 2))) (variable $ class))))",
        );
    }

    #[test]
    fn test_nested_bless_calls() {
        parse_and_check(
            "bless { inner => bless {}, 'Inner' }, 'Outer'",
            "(source_file (call bless ((hash ((identifier inner) (call bless ((hash ) (string \"'Inner'\"))))) (string \"'Outer'\"))))",
        );
    }

    #[test]
    fn test_bless_with_variable_hashref() {
        parse_and_check(
            "bless $data, $class",
            "(source_file (call bless ((variable $ data) (variable $ class))))",
        );
    }

    #[test]
    fn test_my_variable_assignment_with_bless() {
        parse_and_check(
            "my $obj = bless {}, $class",
            "(source_file (my_declaration (variable $ obj)(call bless ((hash ) (variable $ class)))))",
        );
    }
}
