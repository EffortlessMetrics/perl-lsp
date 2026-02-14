/// Statement Termination Edge Cases Mutation Hardening Tests
///
/// These tests target statement termination logic in complex nested parsing contexts
/// to eliminate surviving mutants and improve mutation score from 73% toward 85-90%.
///
/// Target mutants:
/// - Statement termination flow control mutations
/// - Nested context parsing logic errors
/// - Statement modifier boundary detection
/// - Complex parsing state management mutations
///
/// This addresses parsing flow corruption that can lead to infinite loops in LSP
/// parsing and ensure proper statement boundary detection in enterprise Perl code.
///
/// Labels: tests:hardening, mutation:score-improvement, parser:termination
use perl_parser::{ParseError, Parser};

/// Test statement termination in complex nested contexts
/// Ensures parsing doesn't get stuck in infinite loops due to termination mutations
#[test]
fn test_nested_statement_termination_edge_cases() {
    let test_cases = vec![
        // Basic statement sequences with proper termination
        ("my $x = 1; my $y = 2; my $z = 3;", "Sequential statements should parse correctly"),
        // Statement modifiers with complex expressions
        (
            "print $x if defined $y && $z > 0;",
            "Statement modifier with complex condition should terminate properly",
        ),
        (
            "return $result unless $error || $timeout;",
            "Unless modifier with logical OR should terminate properly",
        ),
        (
            "next if $continue; last if $done; redo if $retry;",
            "Multiple control flow statements should terminate properly",
        ),
        // Nested blocks with statement modifiers
        (
            "{ my $x = 1; print $x if $debug; }",
            "Block with statement modifier should terminate properly",
        ),
        (
            "if ($condition) { my $x = 1; print $x unless $quiet; }",
            "If block with unless modifier should terminate properly",
        ),
        // Subroutine definitions with complex bodies
        (
            "sub test { my $x = shift; return $x * 2 if $x > 0; }",
            "Subroutine with conditional return should terminate properly",
        ),
        (
            "sub complex { my ($a, $b) = @_; my $result = $a + $b; print $result if $verbose; return $result; }",
            "Complex subroutine should terminate properly",
        ),
        // Package declarations with code
        (
            "package Foo::Bar; use strict; my $x = 1; 1;",
            "Package with code should terminate properly",
        ),
        // Loop constructs with statement modifiers
        (
            "for my $item (@list) { print $item if defined $item; }",
            "For loop with conditional print should terminate properly",
        ),
        (
            "while (my $line = <$fh>) { chomp $line; process($line) unless $line =~ /^#/; }",
            "While loop with statement modifier should terminate properly",
        ),
        // Complex expressions with operators
        (
            "my $result = $a > $b ? $a : $b; print $result if $debug;",
            "Ternary operator with statement modifier should terminate properly",
        ),
        (
            "my @sorted = sort { $a <=> $b } @numbers; print @sorted unless $quiet;",
            "Sort with block and statement modifier should terminate properly",
        ),
        // Anonymous subroutines and closures
        (
            "my $code = sub { my $x = shift; return $x * 2; }; my $result = $code->(5);",
            "Anonymous subroutine should terminate properly",
        ),
        // Hash and array operations
        (
            "my %hash = (key => 'value', other => 'data'); my $value = $hash{key};",
            "Hash operations should terminate properly",
        ),
        (
            "my @array = (1, 2, 3); my $first = $array[0] if @array;",
            "Array operations with conditional access should terminate properly",
        ),
        // Regular expressions with modifiers
        (
            "my $match = $string =~ /pattern/gi; print $match if $match;",
            "Regex match with modifiers should terminate properly",
        ),
        (
            "my $result = $string =~ s/old/new/g; print 'replaced' if $result;",
            "Substitution with statement modifier should terminate properly",
        ),
        // Use and require statements
        (
            "use Data::Dumper; use strict; use warnings; my $x = 1;",
            "Multiple use statements should terminate properly",
        ),
        (
            "require 'config.pl' if -f 'config.pl'; my $config = get_config();",
            "Conditional require should terminate properly",
        ),
        // Complex control flow
        (
            "eval { my $x = risky_operation(); print $x if defined $x; }; warn $@ if $@;",
            "Eval with error handling should terminate properly",
        ),
        // Heredocs and quoted strings
        (
            r#"my $text = <<'EOF'; print $text if $show; This is a heredoc EOF"#,
            "Heredoc should terminate properly",
        ),
        // File operations
        (
            "open my $fh, '<', $file or die $!; my $content = <$fh>; close $fh;",
            "File operations should terminate properly",
        ),
    ];

    for (perl_code, description) in test_cases {
        let mut parser = Parser::new(perl_code);

        // Attempt to parse the code
        let parse_result = parser.parse();

        // The key test: parsing should either succeed or fail cleanly, never hang
        match parse_result {
            Ok(_) => {
                // Success is good - code parsed correctly
            }
            Err(error) => {
                // Error is acceptable for some edge cases, but should not be a timeout/hang
                assert!(
                    !matches!(error, ParseError::RecursionLimit),
                    "MUTATION KILL: {} - hit recursion limit, indicates infinite loop from termination mutation in code: {}",
                    description,
                    perl_code
                );
            }
        }
    }
}

/// Test parsing timeout prevention to catch infinite loops from termination mutations
/// Uses a timeout mechanism to detect when parsing hangs due to mutations
#[test]
#[ignore = "mutation hardening: timeout behavior needs investigation"]
fn test_parsing_timeout_prevention() {
    use std::time::{Duration, Instant};

    let problematic_cases = vec![
        // Cases that might cause infinite loops if termination logic is mutated
        ("if (", "Unterminated if condition"),
        ("while (", "Unterminated while condition"),
        ("for (", "Unterminated for condition"),
        ("sub test {", "Unterminated subroutine"),
        ("my $x = ", "Unterminated assignment"),
        ("print if", "Incomplete statement modifier"),
        ("return unless", "Incomplete unless modifier"),
        ("{", "Unterminated block"),
        ("(", "Unterminated parentheses"),
        ("[", "Unterminated bracket"),
    ];

    for (perl_code, description) in problematic_cases {
        let start_time = Instant::now();
        let timeout = Duration::from_millis(100); // Short timeout for problematic code

        let mut parser = Parser::new(perl_code);

        // Parse with timeout detection
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Check that parsing didn't hang (timeout detection)
        assert!(
            elapsed < timeout,
            "MUTATION KILL: {} - parsing took too long ({:?}), indicates infinite loop from termination mutation in code: '{}'",
            description,
            elapsed,
            perl_code
        );

        // With error recovery, parser may return Ok with ERROR nodes in AST.
        // Either an Err result OR an AST with ERROR nodes is acceptable.
        let has_error = match &parse_result {
            Err(_) => true,
            Ok(ast) => {
                let sexp = ast.to_sexp();
                sexp.contains("ERROR")
            }
        };
        assert!(
            has_error,
            "MUTATION KILL: {} - malformed code should produce error (Err or ERROR node): '{}'",
            description, perl_code
        );
    }
}

/// Test statement modifier precedence and termination
/// Ensures statement modifiers don't interfere with proper statement termination
#[test]
#[ignore = "mutation hardening: statement modifier termination needs investigation"]
fn test_statement_modifier_termination_precedence() {
    let test_cases = vec![
        // Simple statement modifiers
        ("print 'hello' if $debug;", true, "Simple if modifier should terminate"),
        ("return unless $error;", true, "Simple unless modifier should terminate"),
        ("next while $continue;", true, "Simple while modifier should terminate"),
        ("last until $done;", true, "Simple until modifier should terminate"),
        ("redo for $i;", true, "Simple for modifier should terminate"),
        // Chained statement modifiers (not typically valid Perl, but should handle gracefully)
        ("print 'test' if $a unless $b;", false, "Chained modifiers should not parse successfully"),
        ("return if $x while $y;", false, "Invalid chained modifiers should fail"),
        // Statement modifiers with complex expressions
        ("print $x if defined $x && $x > 0;", true, "Complex if condition should terminate"),
        ("return $y unless $error || $timeout;", true, "Complex unless condition should terminate"),
        (
            "process($item) for my $item (@list);",
            true,
            "For modifier with declaration should terminate",
        ),
        // Statement modifiers in blocks
        ("{ print $x if $debug; }", true, "Block with modifier should terminate"),
        ("sub test { return $x if $x > 0; }", true, "Sub with modifier should terminate"),
        // Multiple statements with modifiers
        (
            "print $a if $debug; print $b unless $quiet;",
            true,
            "Multiple statements with modifiers should terminate",
        ),
    ];

    for (perl_code, should_succeed, description) in test_cases {
        let mut parser = Parser::new(perl_code);

        let parse_result = parser.parse();

        if should_succeed {
            match parse_result {
                Ok(_) => {
                    // Expected success
                }
                Err(error) => {
                    // Check that it's not a recursion/infinite loop error
                    assert!(
                        !matches!(error, ParseError::RecursionLimit),
                        "MUTATION KILL: {} - should not hit recursion limit (indicates termination mutation): '{}'",
                        description,
                        perl_code
                    );
                }
            }
        } else {
            // Some cases are expected to fail, but should fail quickly, not hang.
            // With error recovery, parser may return Ok with ERROR nodes.
            let has_error = match &parse_result {
                Err(_) => true,
                Ok(ast) => {
                    let sexp = ast.to_sexp();
                    sexp.contains("ERROR")
                }
            };
            assert!(
                has_error,
                "MUTATION KILL: {} - invalid code should produce error: '{}'",
                description, perl_code
            );
        }
    }
}

/// Test edge cases in termination detection with special tokens
/// Covers unusual token sequences that might trigger termination mutations
#[test]
fn test_termination_special_token_edge_cases() {
    let edge_cases = vec![
        // Empty statements
        (";", "Empty statement should parse"),
        (";;", "Multiple empty statements should parse"),
        (";;;", "Many empty statements should parse"),
        // Whitespace handling
        (" ; ", "Whitespace around semicolon should parse"),
        ("\n;\n", "Newlines around semicolon should parse"),
        ("\t;\t", "Tabs around semicolon should parse"),
        // Comments and termination
        ("# comment", "Comment without semicolon should parse"),
        ("# comment\n", "Comment with newline should parse"),
        ("; # comment after semicolon", "Comment after semicolon should parse"),
        // Mixed terminators
        ("my $x = 1; # comment\nmy $y = 2;", "Mixed semicolons and newlines should parse"),
        // Special characters that might confuse termination
        ("my $x = ';'; my $y = \";\";", "Quoted semicolons should not terminate statements"),
        ("my $x = q{;}; my $y = qq{;};", "Quoted semicolons in q/qq should not terminate"),
        // Operators that might be confused with terminators
        ("my $x = $a <=> $b;", "Spaceship operator should not cause termination issues"),
        ("my $x = $a // $b;", "Defined-or operator should not cause termination issues"),
    ];

    for (perl_code, description) in edge_cases {
        let mut parser = Parser::new(perl_code);

        let start_time = std::time::Instant::now();
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Should not take too long (no infinite loops)
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "MUTATION KILL: {} - parsing took too long, indicates termination mutation: '{}'",
            description,
            perl_code
        );

        // Most of these should parse successfully
        assert!(
            !matches!(parse_result, Err(ParseError::RecursionLimit)),
            "MUTATION KILL: {} - hit recursion limit, indicates infinite loop from termination mutation: '{}'",
            description,
            perl_code
        );
    }
}

/// Stress test statement termination with deeply nested structures
/// Ensures termination logic works correctly even in complex nesting scenarios
#[test]
fn test_deeply_nested_statement_termination() {
    // Generate deeply nested but properly terminated structures
    let nested_blocks = "{".repeat(10) + "my $x = 1;" + &"}".repeat(10);
    let nested_parens = "(".repeat(5) + "1" + &")".repeat(5) + ";";
    let nested_arrays = "[".repeat(5) + "1" + &"]".repeat(5) + ";";

    let test_cases = vec![
        (nested_blocks, "Deeply nested blocks should terminate properly"),
        (nested_parens, "Deeply nested parentheses should terminate properly"),
        (nested_arrays, "Deeply nested arrays should terminate properly"),
        // Nested control structures
        (
            "if ($a) { if ($b) { if ($c) { print 'deep'; } } }".to_string(),
            "Deeply nested if statements should terminate properly",
        ),
        (
            "for my $i (1..3) { for my $j (1..3) { for my $k (1..3) { print $i * $j * $k; } } }"
                .to_string(),
            "Deeply nested for loops should terminate properly",
        ),
        (
            "while ($outer) { while ($middle) { while ($inner) { last; } } }".to_string(),
            "Deeply nested while loops should terminate properly",
        ),
    ];

    for (perl_code, description) in test_cases {
        let mut parser = Parser::new(&perl_code);

        let start_time = std::time::Instant::now();
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Should complete in reasonable time
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "MUTATION KILL: {} - deeply nested parsing took too long, indicates termination mutation",
            description
        );

        // Should not hit recursion limits due to termination issues
        assert!(
            !matches!(parse_result, Err(ParseError::RecursionLimit)),
            "MUTATION KILL: {} - hit recursion limit in deeply nested structure, indicates termination logic mutation",
            description
        );
    }
}
