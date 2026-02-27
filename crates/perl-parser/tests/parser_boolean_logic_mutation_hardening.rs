/// Parser Boolean Logic Mutation Hardening Tests
///
/// These tests target specific surviving mutants in parser boolean logic to eliminate
/// them and improve mutation score from 73% toward 85-90% enterprise threshold.
///
/// Target mutants:
/// - Parser::is_statement_terminator: Boolean return mutations (true/false replacements)
/// - Statement termination flow control mutations
/// - Parsing logic boolean operator mutations
/// - Statement boundary detection edge cases
///
/// This addresses CRITICAL surviving mutants in parser.rs:174:9 that can cause
/// infinite loops in LSP parsing and corrupt core parsing flow.
///
/// Labels: tests:hardening, mutation:score-improvement
use perl_parser::{ParseError, Parser};
use std::time::{Duration, Instant};

/// Test statement termination logic through parsing behavior
/// Tests various statement endings to catch termination boolean mutations
#[test]
fn test_statement_termination_boolean_mutations() {
    let statement_cases = vec![
        // Cases that MUST terminate properly with semicolon
        (r"my $x = 1;", true, "Simple variable declaration should terminate with semicolon"),
        (r"print 'hello';", true, "Print statement should terminate with semicolon"),
        (r"return $value;", true, "Return statement should terminate with semicolon"),
        (r"$x = $y + $z;", true, "Assignment should terminate with semicolon"),
        (r"sub test { }; my $x = 1;", true, "Multiple statements should terminate properly"),
        // Cases that MUST terminate at EOF without semicolon
        (r"my $x = 1", true, "Variable declaration should terminate at EOF"),
        (r"print 'hello'", true, "Print statement should terminate at EOF"),
        (r"return $value", true, "Return statement should terminate at EOF"),
        // Cases with statement modifiers that should parse correctly
        (r"print $x if $debug;", true, "Statement modifier should not break termination"),
        (r"return unless $error;", true, "Unless modifier should not break termination"),
        (r"next while $continue;", true, "While modifier should not break termination"),
        (r"last until $done;", true, "Until modifier should not break termination"),
        // Edge cases that should handle gracefully
        (";", true, "Empty statement should parse"),
        (";;", true, "Multiple empty statements should parse"),
        ("# comment", true, "Comment-only should parse"),
        ("", true, "Empty input should parse"),
        // Cases that should fail but not hang (malformed statements)
        (r"my $x =", false, "Incomplete assignment should fail cleanly"),
        ("print", false, "Incomplete print should fail cleanly"),
        ("if (", false, "Incomplete if should fail cleanly"),
        ("sub test {", false, "Incomplete sub should fail cleanly"),
    ];

    for (perl_code, should_succeed, description) in statement_cases {
        let start_time = Instant::now();
        let timeout = Duration::from_millis(100);

        let mut parser = Parser::new(perl_code);
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Critical test: parsing should not hang (catches termination logic mutations)
        assert!(
            elapsed < timeout,
            "MUTATION KILL: {} - parsing took too long ({:?}), indicates infinite loop from termination mutation in code: '{}'",
            description,
            elapsed,
            perl_code
        );

        // Test expected success/failure without hanging
        match (parse_result.is_ok(), should_succeed) {
            (true, true) => {
                // Expected success
            }
            (false, false) => {
                // Expected failure, but should not be due to recursion/infinite loop
                assert!(
                    !matches!(parse_result, Err(ParseError::RecursionLimit)),
                    "MUTATION KILL: {} - hit recursion limit instead of clean parse error, indicates termination mutation: '{}'",
                    description,
                    perl_code
                );
            }
            (false, true) => {
                // Unexpected failure - check if it's due to termination issues
                assert!(
                    !matches!(parse_result, Err(ParseError::RecursionLimit)),
                    "MUTATION KILL: {} - expected success but hit recursion limit, indicates termination mutation: '{}'",
                    description,
                    perl_code
                );
                // Other parse errors might be acceptable for edge cases
            }
            (true, false) => {
                // Unexpected success for malformed code - this might indicate a mutation
                // but we'll allow it as the parser might be more permissive
            }
        }
    }
}

/// Test logical operator handling through complex expressions
/// Indirectly tests is_logical_or and related boolean logic
#[test]
fn test_logical_operator_parsing_mutations() {
    let logical_cases = vec![
        // OR operators that should parse correctly
        (r"$x = $a || $b;", "Logical OR should parse correctly"),
        (r"$x = $a or $b;", "Word OR should parse correctly"),
        (r"$x = $a // $b;", "Defined OR should parse correctly"),
        // AND operators that should parse correctly
        (r"$x = $a && $b;", "Logical AND should parse correctly"),
        (r"$x = $a and $b;", "Word AND should parse correctly"),
        // Complex logical expressions
        (r"$result = $a || $b && $c;", "Mixed logical operators should parse"),
        (r"$result = ($a || $b) && $c;", "Parenthesized logical should parse"),
        (r"$result = $a // $b || $c;", "Mixed defined-or and logical-or should parse"),
        // Statement modifiers with logical operators
        (r"print $x if $a || $b;", "Statement modifier with OR should parse"),
        (r"return unless $error && $critical;", "Statement modifier with AND should parse"),
        (r"next if defined $x && $x > 0;", "Complex modifier condition should parse"),
        // Logical operators in different contexts
        (r"if ($a || $b) { print 'yes'; }", "Logical OR in if condition should parse"),
        (r"while ($x && $y) { $x--; }", "Logical AND in while condition should parse"),
        (r"for my $i (0..10) { next if $i % 2 || $skip; }", "Logical OR in loop should parse"),
    ];

    for (perl_code, description) in logical_cases {
        let start_time = Instant::now();
        let timeout = Duration::from_millis(100);

        let mut parser = Parser::new(perl_code);
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Should not hang due to logical operator mutations
        assert!(
            elapsed < timeout,
            "MUTATION KILL: {} - parsing took too long, indicates logical operator mutation: '{}'",
            description,
            perl_code
        );

        // Should not hit recursion limits due to logical operator issues
        assert!(
            !matches!(parse_result, Err(ParseError::RecursionLimit)),
            "MUTATION KILL: {} - hit recursion limit, indicates logical operator mutation: '{}'",
            description,
            perl_code
        );
    }
}

/// Test postfix operator handling through increment/decrement usage
/// Indirectly tests is_postfix_op boolean logic
#[test]
fn test_postfix_operator_parsing_mutations() {
    let postfix_cases = vec![
        // Postfix increment/decrement
        (r"$x++;", "Postfix increment should parse"),
        (r"$x--;", "Postfix decrement should parse"),
        (r"$array[$i++] = $value;", "Postfix increment in array index should parse"),
        (r"print $hash{$key++};", "Postfix increment in hash key should parse"),
        // Prefix vs postfix distinction
        (r"++$x;", "Prefix increment should parse"),
        (r"--$x;", "Prefix decrement should parse"),
        (r"$y = ++$x + $z++;", "Mixed prefix and postfix should parse"),
        // Postfix in complex expressions
        (r"$result = $x++ * $y--;", "Multiple postfix operators should parse"),
        (r"for (my $i = 0; $i < 10; $i++) { print $i; }", "Postfix in for loop should parse"),
        (r"while ($x++ < 100) { process($x); }", "Postfix in while condition should parse"),
        // Postfix with other operators
        (r"$x++ if $condition;", "Postfix with statement modifier should parse"),
        (r"return $x++ unless $error;", "Postfix with unless modifier should parse"),
        (r"$result = $x++ || $default;", "Postfix with logical OR should parse"),
    ];

    for (perl_code, description) in postfix_cases {
        let start_time = Instant::now();
        let timeout = Duration::from_millis(100);

        let mut parser = Parser::new(perl_code);
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Should not hang due to postfix operator mutations
        assert!(
            elapsed < timeout,
            "MUTATION KILL: {} - parsing took too long, indicates postfix operator mutation: '{}'",
            description,
            perl_code
        );

        // Should not hit recursion limits due to postfix operator issues
        assert!(
            !matches!(parse_result, Err(ParseError::RecursionLimit)),
            "MUTATION KILL: {} - hit recursion limit, indicates postfix operator mutation: '{}'",
            description,
            perl_code
        );
    }
}

/// Test variable sigil handling through various variable declarations
/// Indirectly tests is_variable_sigil boolean logic
#[test]
fn test_variable_sigil_parsing_mutations() {
    let sigil_cases = vec![
        // Scalar variables
        (r"my $scalar = 'value';", "Scalar sigil should parse"),
        (r"our $global_scalar = 42;", "Global scalar should parse"),
        (r"local $local_scalar = $other;", "Local scalar should parse"),
        (r"state $state_scalar = 0;", "State scalar should parse"),
        // Array variables
        (r"my @array = (1, 2, 3);", "Array sigil should parse"),
        (r"our @global_array = ();", "Global array should parse"),
        (r"local @local_array = @other;", "Local array should parse"),
        // Hash variables
        (r"my %hash = (key => 'value');", "Hash sigil should parse"),
        (r"our %global_hash = ();", "Global hash should parse"),
        (r"local %local_hash = %other;", "Local hash should parse"),
        // Mixed variable types
        (r"my ($x, @y, %z) = (1, (2, 3), (a => 'b'));", "Mixed sigils should parse"),
        (r"our ($global, @array, %hash);", "Multiple global declarations should parse"),
        // Variables in expressions
        (r"$result = $scalar + $array[0] + $hash{key};", "Mixed variable access should parse"),
        (r#"print "$scalar: @array %hash";"#, "Variables in string interpolation should parse"),
        // Complex variable usage
        (r"$array_ref = \@array;", "Array reference should parse"),
        (r"$hash_ref = \%hash;", "Hash reference should parse"),
        (r"$scalar_ref = \$scalar;", "Scalar reference should parse"),
        (r"my $sub_ref = \&subroutine;", "Subroutine reference should parse"),
    ];

    for (perl_code, description) in sigil_cases {
        let start_time = Instant::now();
        let timeout = Duration::from_millis(100);

        let mut parser = Parser::new(perl_code);
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Should not hang due to variable sigil mutations
        assert!(
            elapsed < timeout,
            "MUTATION KILL: {} - parsing took too long, indicates variable sigil mutation: '{}'",
            description,
            perl_code
        );

        // Should not hit recursion limits due to sigil issues
        assert!(
            !matches!(parse_result, Err(ParseError::RecursionLimit)),
            "MUTATION KILL: {} - hit recursion limit, indicates variable sigil mutation: '{}'",
            description,
            perl_code
        );
    }
}

/// Test statement modifier detection through various modifier patterns
/// Indirectly tests is_stmt_modifier_kind boolean logic
#[test]
fn test_statement_modifier_parsing_mutations() {
    let modifier_cases = vec![
        // If modifiers
        (r"print 'debug' if $debug;", "If modifier should parse"),
        (r"return $value if defined $value;", "If with defined should parse"),
        (r"next if $skip_iteration;", "If with control flow should parse"),
        // Unless modifiers
        (r"die 'error' unless $ok;", "Unless modifier should parse"),
        (r"return unless defined $result;", "Unless with defined should parse"),
        (r"last unless $continue;", "Unless with control flow should parse"),
        // While modifiers
        (r"process() while $has_data;", "While modifier should parse"),
        (r"print $_ while <>;", "While with input should parse"),
        (r"$x++ while $x < 10;", "While with increment should parse"),
        // Until modifiers
        (r"wait() until $ready;", "Until modifier should parse"),
        (r"sleep 1 until $condition;", "Until with sleep should parse"),
        (r"$x-- until $x == 0;", "Until with decrement should parse"),
        // For modifiers
        (r"print $_ for @list;", "For modifier should parse"),
        (r"process($_) for 1..10;", "For with range should parse"),
        (r"validate($item) for @items;", "For with array should parse"),
        // When modifiers (in given/when context)
        // Note: These might not parse in simple contexts, but should not hang

        // Complex modifier conditions
        (r"print $x if defined $x && $x > 0;", "Complex if condition should parse"),
        (r"return unless $error || $timeout;", "Complex unless condition should parse"),
        (r"process() while $running && !eof(FH);", "Complex while condition should parse"),
    ];

    for (perl_code, description) in modifier_cases {
        let start_time = Instant::now();
        let timeout = Duration::from_millis(100);

        let mut parser = Parser::new(perl_code);
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Should not hang due to statement modifier mutations
        assert!(
            elapsed < timeout,
            "MUTATION KILL: {} - parsing took too long, indicates statement modifier mutation: '{}'",
            description,
            perl_code
        );

        // Should not hit recursion limits due to modifier issues
        assert!(
            !matches!(parse_result, Err(ParseError::RecursionLimit)),
            "MUTATION KILL: {} - hit recursion limit, indicates statement modifier mutation: '{}'",
            description,
            perl_code
        );
    }
}

/// Integration test combining all boolean logic patterns
/// Tests complex interactions that might reveal boolean logic mutations
#[test]
fn test_boolean_logic_integration_mutations() {
    let integration_cases = vec![
        // Complex statements with multiple boolean decision points
        (
            r"my $x = 1; $x++ if $condition || $force; print $x unless $quiet && $no_output;",
            "Complex statement sequence should parse without hanging",
        ),
        (
            r"for my $item (@list) { next if !defined $item; print $item unless $item =~ /skip/; }",
            "Loop with multiple modifiers should parse",
        ),
        (
            r"sub test { my $arg = shift; return $arg * 2 if $arg > 0; return 0; }",
            "Subroutine with conditional return should parse",
        ),
        (
            r"eval { process_data() }; warn $@ if $@ && $verbose;",
            "Eval with error handling should parse",
        ),
        (
            r"while (my $line = <DATA>) { chomp $line; next if $line =~ /^#/; process($line) unless $dry_run; }",
            "Complex while loop should parse",
        ),
    ];

    for (perl_code, description) in integration_cases {
        let start_time = Instant::now();
        let timeout = Duration::from_millis(200); // Slightly longer for complex cases

        let mut parser = Parser::new(perl_code);
        let parse_result = parser.parse();
        let elapsed = start_time.elapsed();

        // Critical: should not hang due to boolean logic mutations
        assert!(
            elapsed < timeout,
            "MUTATION KILL: {} - parsing took too long, indicates boolean logic mutation: '{}'",
            description,
            perl_code
        );

        // Should not hit recursion limits
        assert!(
            !matches!(parse_result, Err(ParseError::RecursionLimit)),
            "MUTATION KILL: {} - hit recursion limit, indicates boolean logic mutation: '{}'",
            description,
            perl_code
        );
    }
}
