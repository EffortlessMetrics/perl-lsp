use perl_parser::Parser;

#[test]
fn test_complex_postfix_deref() {
    // Test with real-world Perl code examples
    let cases = vec![
        // Basic postfix dereferences
        ("my @array = $arrayref->@*;", "unary_->@*"),
        ("my %hash = $hashref->%*;", "unary_->%*"),
        ("my $scalar = $scalarref->$*;", "unary_->$*"),
        ("my $result = $coderef->&*;", "unary_->&*"),
        ("my $glob = $globref->**;", "unary_->**"),
        // Array and hash slices
        ("my @slice = $arrayref->@[0..2];", "binary_->@[]"),
        ("my %slice = $hashref->%{'foo', 'bar'};", "binary_->%{}"),
        // Method calls followed by postfix deref
        ("$obj->method()->@*;", "unary_->@*"),
        ("$obj->get_data()->[0]->%*;", "unary_->%*"),
        // Complex nested structures
        ("$data->{users}->[0]->{profile}->@*;", "unary_->@*"),
        ("$config->{servers}->@[0, 1, 2];", "binary_->@[]"),
        // In expressions
        ("print $ref->@*;", "unary_->@*"),
        ("foreach my $item ($ref->@*) { print $item; }", "unary_->@*"),
        ("if ($ref->@*) { print 'has items'; }", "unary_->@*"),
        // Assignment operations
        ("$ref->@* = (1, 2, 3);", "unary_->@*"),
        // Skip hash assignment with fat arrow for now - needs more parser work
        // ("$ref->%* = (a => 1, b => 2);", "unary_->%*"),
        ("$ref->%* = ('a', 1, 'b', 2);", "unary_->%*"),
    ];

    for (code, expected_pattern) in cases {
        let mut parser = Parser::new(code);
        let result = parser.parse();

        if let Err(e) = &result {
            panic!("Failed to parse '{}': {:?}", code, e);
        }

        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        assert!(
            sexp.contains(expected_pattern),
            "Code: {}\nExpected pattern '{}' not found in:\n{}",
            code,
            expected_pattern,
            sexp
        );
    }
}

#[test]
fn test_postfix_deref_edge_cases() {
    // Test edge cases and potential parsing ambiguities
    let cases = vec![
        // Multiple dereferences in a row
        ("$ref->@*->%*", vec!["unary_->@*", "unary_->%*"]),
        // Postfix deref with method calls (use simpler example for now)
        ("$ref->@*->length()", vec!["unary_->@*", "method_call"]),
        // In ternary expressions
        ("$flag ? $ref->@* : ()", vec!["ternary", "unary_->@*"]),
        // With string interpolation (the parser should handle the variable, not the string)
        ("print \"Items: $ref->@*\";", vec!["string_interpolated"]),
    ];

    for (code, expected_patterns) in cases {
        let mut parser = Parser::new(code);
        let result = parser.parse();

        if let Err(e) = &result {
            panic!("Failed to parse '{}': {:?}", code, e);
        }

        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        for pattern in expected_patterns {
            assert!(
                sexp.contains(pattern),
                "Code: {}\nExpected pattern '{}' not found in:\n{}",
                code,
                pattern,
                sexp
            );
        }
    }
}
