use super::*;
use perl_tdd_support::{must, must_some};

#[test]
fn test_simple_variable() {
    let mut parser = Parser::new("my $x = 42;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = must(result);
    println!("AST: {}", ast.to_sexp());
}

#[test]
fn test_if_statement() {
    let mut parser = Parser::new("if ($x > 10) { print $x; }");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = must(result);
    println!("AST: {}", ast.to_sexp());
}

#[test]
fn test_function_definition() {
    let mut parser = Parser::new("sub greet { print \"Hello\"; }");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = must(result);
    println!("AST: {}", ast.to_sexp());
}

#[test]
fn test_method_attributes_parse() {
    let mut parser = Parser::new("method size :lvalue :prototype($self) ($self) { $self; }");
    let ast = must(parser.parse());

    let NodeKind::Program { statements } = &ast.kind else {
        panic!("Expected program node, got: {:?}", ast.kind);
    };
    let first = must_some(statements.first());
    match &first.kind {
        NodeKind::Method {
            signature,
            attributes,
            ..
        } => {
            assert!(signature.is_some(), "Expected method signature");
            assert_eq!(
                attributes,
                &vec!["lvalue".to_string(), "prototype($self)".to_string()]
            );
        }
        other => panic!("Expected method declaration, got: {:?}", other),
    }
}

#[test]
fn test_method_attributes_without_signature_parse() {
    let mut parser = Parser::new("method reset :lvalue { return; }");
    let ast = must(parser.parse());

    let NodeKind::Program { statements } = &ast.kind else {
        panic!("Expected program node, got: {:?}", ast.kind);
    };
    let first = must_some(statements.first());
    match &first.kind {
        NodeKind::Method {
            signature,
            attributes,
            ..
        } => {
            assert!(signature.is_none(), "Did not expect method signature");
            assert_eq!(attributes, &vec!["lvalue".to_string()]);
        }
        other => panic!("Expected method declaration, got: {:?}", other),
    }
}

#[test]
fn test_declaration_attributes_require_name() {
    let mut parser = Parser::new("sub broken : { 1; }");
    let _ast = must(parser.parse());
    assert!(
        parser.errors().iter().any(|err| err
            .to_string()
            .contains("Expected attribute name after ':'")),
        "expected parser recovery error for missing attribute name, got: {:?}",
        parser.errors()
    );
}

#[test]
fn test_declaration_attributes_require_closing_paren() {
    let mut parser = Parser::new("method broken :prototype($self { 1; }");
    let _ast = must(parser.parse());
    assert!(
        parser.errors().iter().any(|err| err
            .to_string()
            .contains("Unterminated attribute argument list")),
        "expected parser recovery error for unterminated attribute argument list, got: {:?}",
        parser.errors()
    );
}

#[test]
fn test_list_declarations() {
    // Test simple list declaration
    let mut parser = Parser::new("my ($x, $y);");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    println!("List declaration AST: {}", ast.to_sexp());

    // Test list declaration with initialization
    let mut parser = Parser::new("state ($a, $b) = (1, 2);");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    println!("List declaration with init AST: {}", ast.to_sexp());

    // Test mixed sigils
    let mut parser = Parser::new("our ($scalar, @array, %hash);");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    println!("Mixed sigils AST: {}", ast.to_sexp());

    // Test empty list
    let mut parser = Parser::new("my ();");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    println!("Empty list AST: {}", ast.to_sexp());
}

#[test]
fn test_qw_delimiters() {
    // Test qw with parentheses
    let mut parser = Parser::new("qw(one two three)");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    assert_eq!(
        ast.to_sexp(),
        r#"(source_file (array (string "one") (string "two") (string "three")))"#
    );

    // Test qw with brackets
    let mut parser = Parser::new("qw[foo bar]");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    assert_eq!(
        ast.to_sexp(),
        r#"(source_file (array (string "foo") (string "bar")))"#
    );

    // Test qw with non-paired delimiters
    let mut parser = Parser::new("qw/alpha beta/");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    assert_eq!(
        ast.to_sexp(),
        r#"(source_file (array (string "alpha") (string "beta")))"#
    );

    // Test qw with exclamation marks
    let mut parser = Parser::new("qw!hello world!");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    assert_eq!(
        ast.to_sexp(),
        r#"(source_file (array (string "hello") (string "world")))"#
    );
}

#[test]
fn test_block_vs_hash_context() {
    // Statement context: block containing hash
    let mut parser = Parser::new("{ key => 'value' }");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    // Statement context: block with hash inside
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(block (expression_statement (hash"),
        "Statement context should have block containing hash, got: {}",
        sexp
    );

    // Expression context: direct hash literal in assignment
    let mut parser = Parser::new("my $x = { key => 'value' }");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    // In expression context, should have hash
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(hash"),
        "Expression context should have hash, got: {}",
        sexp
    );
    assert!(
        sexp.contains("my"),
        "Should have my declaration, got: {}",
        sexp
    );

    // Hash reference with parentheses
    let mut parser = Parser::new("$ref = ( a => 1, b => 2 )");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    // Parentheses with fat arrow should create hash
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(hash") || sexp.contains("(array"),
        "Should have hash or array, got: {}",
        sexp
    );
}

#[test]
fn test_qualified_function_call() {
    let mut parser = Parser::new("return Data::Dumper::Dumper($param);");
    let result = parser.parse();
    match result {
        Ok(ast) => {
            println!(
                "✅ Successfully parsed qualified function call: {}",
                ast.to_sexp()
            );
        }
        Err(e) => {
            println!("❌ Failed to parse qualified function call: {}", e);
            unreachable!("Parsing failed: {}", e);
        }
    }
}

#[test]
fn test_issue_461_variable_length_lookbehind() {
    // Variable-length lookbehind
    let code = r#"my $pattern = qr/(?<=\d{1,1000})\w+/;"#;
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse variable-length lookbehind");

    // Deeply nested lookbehind
    let code_nested = r#"my $nested = qr/(?<=(?<=(?<=\d)\w+)\s+)\w+/;"#;
    let mut parser_nested = Parser::new(code_nested);
    let result_nested = parser_nested.parse();
    assert!(result_nested.is_ok(), "Failed to parse nested lookbehind");

    // Check if the AST contains the regex pattern
    let ast = must(result_nested);
    println!("Nested Lookbehind AST: {}", ast.to_sexp());
}

#[test]
fn test_regex_complexity_failure() {
    // 11 levels of nesting (max is 10)
    let code = r#"qr/(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=\d)))))))))))\w+/"#;
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Parser might recover, so check either result is Err or errors list has the error
    if let Err(e) = result {
        assert!(
            e.to_string().contains("Regex lookbehind nesting too deep"),
            "Error was: {}",
            e
        );
    } else {
        let errors = parser.errors();
        assert!(
            !errors.is_empty(),
            "Should have recorded errors for excessive nesting"
        );
        let found = errors
            .iter()
            .any(|e| e.to_string().contains("Regex lookbehind nesting too deep"));
        assert!(found, "Should have found specific error in: {:?}", errors);
    }
}

#[test]
fn test_unicode_property_valid() {
    // 50 properties (limit is 50, so this should pass)
    let mut pattern = String::from("qr/");
    for i in 0..50 {
        pattern.push_str(&format!("\\p{{Prop{}}}", i));
    }
    pattern.push('/');

    let mut parser = Parser::new(&pattern);
    let result = parser.parse();
    assert!(result.is_ok(), "Should accept 50 Unicode properties");
}

#[test]
fn test_unicode_property_complexity() {
    // 51 properties (max is 50)
    let mut pattern = String::from("qr/");
    for i in 0..51 {
        pattern.push_str(&format!("\\p{{Prop{}}}", i));
    }
    pattern.push('/');

    let mut parser = Parser::new(&pattern);
    let result = parser.parse();

    // Parser might recover, so check either result is Err or errors list has the error
    if let Err(e) = result {
        assert!(
            e.to_string().contains("Too many Unicode properties"),
            "Error was: {}",
            e
        );
    } else {
        let errors = parser.errors();
        assert!(
            !errors.is_empty(),
            "Should have recorded errors for excessive Unicode properties"
        );
        let found = errors
            .iter()
            .any(|e| e.to_string().contains("Too many Unicode properties"));
        assert!(found, "Should have found specific error in: {:?}", errors);
    }
}

#[test]
fn test_deep_nesting_stack_overflow() {
    // Issue #423: Deep nesting stack overflow
    // Nested if statements
    let mut code = String::new();
    for _ in 0..100 {
        code.push_str("if ($a) { ");
    }
    code.push_str("print 'hi';");
    for _ in 0..100 {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    // It might fail with nesting limit, or pass if the limit is high enough (64 is default)
    // 100 levels should trigger the limit
    if let Err(e) = result {
        assert!(
            e.to_string().contains("Nesting depth limit exceeded"),
            "Error was: {}",
            e
        );
    } else {
        let errors = parser.errors();
        // If it didn't fail immediately, it might have recovered, but we expect an error
        assert!(
            !errors.is_empty(),
            "Should have recorded errors for excessive nesting"
        );
        let found = errors
            .iter()
            .any(|e| e.to_string().contains("Nesting depth limit exceeded"));
        // Note: RecursionLimit might be converted to NestingTooDeep
        assert!(found, "Should have found specific error in: {:?}", errors);
    }
}

#[test]
fn test_source_filter_detection() {
    // Known filter
    let code = "use Filter::Util::Call;";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(risk:filter)"),
        "Should detect filter usage in: {}",
        sexp
    );

    // Safe module
    let code_safe = "use strict;";
    let mut parser_safe = Parser::new(code_safe);
    let result_safe = parser_safe.parse();
    assert!(result_safe.is_ok());
    let ast_safe = must(result_safe);
    let sexp_safe = ast_safe.to_sexp();
    assert!(
        !sexp_safe.contains("(risk:filter)"),
        "Should not flag strict as filter in: {}",
        sexp_safe
    );
}

#[test]
fn test_regex_code_execution_detection() {
    // Regex with code execution
    let code = r#"my $re = qr/(?{ print "hi" })/;"#;
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = must(result);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(risk:code)"),
        "Should detect regex code execution in: {}",
        sexp
    );

    // Safe regex
    let code_safe = r#"my $re = qr/hello/;"#;
    let mut parser_safe = Parser::new(code_safe);
    let result_safe = parser_safe.parse();
    assert!(result_safe.is_ok());
    let ast_safe = must(result_safe);
    let sexp_safe = ast_safe.to_sexp();
    assert!(
        !sexp_safe.contains("(risk:code)"),
        "Should not flag safe regex in: {}",
        sexp_safe
    );
}

#[test]
fn test_heredoc_deep_nesting() {
    // Create a deeply nested expression ending with a heredoc
    // $a[0][0]...[0] . <<EOF
    // 5000 nesting levels is far above the pending heredoc safety limit and should
    // fail gracefully instead of recursing or hanging.
    let mut code = String::from("$a");
    for _ in 0..5000 {
        code.push_str("[0]");
    }
    code.push_str(" . <<EOF;\ncontent\nEOF");

    let mut parser = Parser::new(&code);
    let result = parser.parse();
    let failed_gracefully = result.as_ref().err().is_some_and(|error| {
        matches!(error, ParseError::NestingTooDeep { .. })
            || error.to_string().contains("Heredoc depth limit exceeded")
    }) || parser
        .errors()
        .iter()
        .any(|error| error.to_string().contains("Heredoc depth limit exceeded"));

    assert!(
        failed_gracefully,
        "Deep heredoc nesting should fail gracefully via nesting or heredoc limits. result={result:?}, parser_errors={:?}",
        parser.errors()
    );
}

#[test]
fn test_multiple_heredocs_same_line() {
    // Issue #440: Multiple heredocs on a single line
    let code = "
    my $a = <<'EOF1'; my $b = <<'EOF2';
Content 1
EOF1
Content 2
EOF2
    ";

    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Failed to parse multiple heredocs on same line"
    );

    let ast = must(result);
    let sexp = ast.to_sexp();

    // Check that both contents were captured correctly
    assert!(sexp.contains("Content 1"), "Missing content 1");
    assert!(sexp.contains("Content 2"), "Missing content 2");
}

#[test]
fn test_deeply_nested_quotes() {
    // The lexer handles nested quote delimiters using a simple depth counter (not recursion),
    // so deeply nested quotes are processed efficiently without stack overflow risk.
    // This test verifies that deep nesting works correctly.
    let mut code = String::from("q{");
    for _ in 0..100 {
        code.push('{');
    }
    for _ in 0..100 {
        code.push('}');
    }
    code.push('}');

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    // The lexer handles this safely with O(n) complexity using a counter
    assert!(
        result.is_ok(),
        "Deeply nested quotes should parse successfully: {:?}",
        result.err()
    );
}

#[test]
fn test_branch_reset_complexity() {
    // 51 branches (max is 50)
    let mut pattern = String::from("qr/(?|");
    for i in 0..51 {
        pattern.push_str(&format!("(a{})|", i));
    }
    // Remove last pipe
    pattern.pop();
    pattern.push_str(")/");

    let mut parser = Parser::new(&pattern);
    let result = parser.parse();

    // Parser might recover, so check either result is Err or errors list has the error
    if let Err(e) = result {
        assert!(
            e.to_string().contains("Too many branches"),
            "Error was: {}",
            e
        );
    } else {
        let errors = parser.errors();
        assert!(
            !errors.is_empty(),
            "Should have recorded errors for excessive branches"
        );
        let found = errors
            .iter()
            .any(|e| e.to_string().contains("Too many branches"));
        assert!(found, "Should have found specific error in: {:?}", errors);
    }
}

#[test]
fn test_builtin_block_list_in_assignment_context() {
    // Verifies that grep/map/sort with block form correctly capture trailing
    // list arguments even when used on the RHS of an assignment.
    let test_cases = vec![
        ("grep block array", "my @result = grep { $_ > 5 } @array;"),
        (
            "sort block hash access",
            r#"my @sorted = sort { $a->{name} cmp $b->{name} } @records;"#,
        ),
        (
            "map block subscript",
            r#"my @mapped = map { $_->[0] + $_->[1] } @pairs;"#,
        ),
        (
            "sort block function call",
            r#"my @x = sort { length($a) <=> length($b) } keys %hash;"#,
        ),
    ];

    for (name, code) in test_cases {
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let ast = must(result);
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "{} should not contain ERROR: {}",
            name,
            sexp
        );
    }
}

#[test]
fn test_catastrophic_backtracking_detection() {
    // Nested quantifiers (a+)+ -- now recorded as a non-fatal diagnostic
    let code = r#"qr/(a+)+/;"#;
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Parse should succeed (nested quantifiers are no longer a hard error)
    assert!(
        result.is_ok(),
        "Parse should succeed for nested quantifiers: {:?}",
        result.err()
    );
    let errors = parser.errors();
    let found = errors
        .iter()
        .any(|e| e.to_string().contains("Nested quantifiers"));
    assert!(
        found,
        "Should have found backtracking diagnostic in: {:?}",
        errors
    );

    // Another case: (a*)*
    let code2 = r#"qr/(a*)*b/;"#;
    let mut parser2 = Parser::new(code2);
    let result2 = parser2.parse();

    assert!(
        result2.is_ok(),
        "Parse should succeed for nested quantifiers: {:?}",
        result2.err()
    );
    let errors2 = parser2.errors();
    let found2 = errors2
        .iter()
        .any(|e| e.to_string().contains("Nested quantifiers"));
    assert!(
        found2,
        "Should have found backtracking diagnostic in: {:?}",
        errors2
    );
}

#[test]
fn test_valid_regex_patterns_no_false_positive() {
    // These valid Perl patterns should parse cleanly with no errors at all
    let valid_patterns = vec![
        (
            r#"$x =~ /(?:pattern)+/;"#,
            "non-capturing group with literal",
        ),
        (
            r#"$x =~ /(?:ab)+/;"#,
            "non-capturing group with two-char literal",
        ),
    ];

    for (code, desc) in valid_patterns {
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(
            result.is_ok(),
            "Pattern '{}' ({}) should parse without error: {:?}",
            code,
            desc,
            result.err()
        );
        let backtracking_errors: Vec<_> = parser
            .errors()
            .iter()
            .filter(|e| {
                let msg = e.to_string();
                msg.contains("backtracking") || msg.contains("Nested quantifiers")
            })
            .collect();
        assert!(
            backtracking_errors.is_empty(),
            "Pattern '{}' ({}) should not produce backtracking diagnostics: {:?}",
            code,
            desc,
            backtracking_errors
        );
    }
}

#[test]
fn test_debug_paren_regex_binding() {
    // Quick diagnostic test
    let test_cases = vec![
        ("simple =~", "$text =~ s/foo/bar/;"),
        ("paren assign =~", "(my $x = $y) =~ s/foo/bar/;"),
        ("paren assign tr", "($str = $input) =~ tr/a-z/A-Z/;"),
        ("paren assign chomp", "(my $line = <STDIN>) =~ s/\\n$//;"),
        (
            "paren with if modifier",
            "(my $x = $input) =~ s/foo/bar/ if $cond;",
        ),
        ("paren assign !~", "(my $x = $y) !~ /pattern/;"),
        ("paren assign regex match", "(my $x = $y) =~ /pattern/;"),
        ("paren assign regex global", "(my $x = $y) =~ s/foo/bar/g;"),
        ("nested paren", "((my $x = $y)) =~ s/foo/bar/;"),
        (
            "paren list assign split if",
            "(my @parts = split /,/, $str) if $cond;",
        ),
    ];

    for (name, code) in test_cases {
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let ast = must(result);
        let sexp = ast.to_sexp();
        println!("{}: {}", name, sexp);
        assert!(
            !sexp.contains("ERROR"),
            "{} should not contain ERROR: {}",
            name,
            sexp
        );
    }
}
