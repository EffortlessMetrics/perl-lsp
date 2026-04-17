//! Property-based tests for label/ternary disambiguation in `is_label_start()`.
//!
//! These tests verify the core invariant:
//! `is_label_start()` returns `false` when the 3rd token cannot start a statement,
//! and returns `true` when it can.
//!
//! Since `is_label_start()` is private, we test indirectly:
//! - Invalid patterns (3rd token can't start statement) should NOT produce `expected_colon` errors
//! - Valid label patterns (3rd token CAN start statement) should parse cleanly

mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Property 1: Invalid third tokens should not produce expected_colon errors
// =============================================================================

/// Invalid third tokens - these cannot start a statement, so `Identifier: <token>`
/// should NOT be treated as a label.
const INVALID_THIRD_TOKENS: &[&str] = &[
    "?",  // Ternary question
    ":",  // Another colon (chained ternary else-part)
    ";",  // Semicolon (immediate statement end)
    ",",  // Comma (expression continuation)
    "=>", // Fat arrow (hash key-value context)
    ")",  // Closing paren
    "]",  // Closing bracket
    "}",  // Closing brace (orphan)
];

/// Property test: All invalid third tokens should not produce `expected_colon` errors.
///
/// For each invalid third token T, parsing `FOO: T` should not produce an `expected_colon`
/// error because the colon in `FOO:` belongs to something else (ternary/hash), not a label.
#[test]
fn property_invalid_third_tokens_no_expected_colon() {
    for invalid_token in INVALID_THIRD_TOKENS {
        let source = format!("FOO: {}", invalid_token);
        let ast = parse(&source);
        let sexp = ast.to_sexp().to_lowercase();

        // Should NOT produce expected_colon error
        assert!(
            !sexp.contains("expected_colon"),
            "Unexpected expected_colon error for pattern 'FOO: {}'\nsexp: {}",
            invalid_token,
            sexp
        );
    }
}

/// Property test: Invalid third tokens at EOF should not produce expected_colon errors.
#[test]
fn property_invalid_third_token_at_eof_no_expected_colon() {
    // Just "FOO:" at end of input - the 3rd token is EOF
    let source = "FOO:";
    let ast = parse(source);
    let sexp = ast.to_sexp().to_lowercase();

    assert!(
        !sexp.contains("expected_colon"),
        "Unexpected expected_colon error for 'FOO:' at EOF\nsexp: {}",
        sexp
    );
}

/// Property test: Various identifier names with invalid third tokens should all work.
#[test]
fn property_various_identifiers_with_invalid_third_tokens() {
    let identifiers = ["X", "FOO", "BAR", "LABEL", "L", "A", "ABC", "XYZ123"];
    let invalid_tokens = ["?", ":", ";", ",", "=>", ")", "]", "}"];

    for id in identifiers {
        for token in invalid_tokens {
            let source = format!("{}: {}", id, token);
            let result = std::panic::catch_unwind(|| {
                let ast = parse(&source);
                let sexp = ast.to_sexp().to_lowercase();
                // Should not panic, and should not produce expected_colon
                assert!(
                    !sexp.contains("expected_colon"),
                    "expected_colon for '{}'\nsexp: {}",
                    source,
                    sexp
                );
            });
            assert!(result.is_ok(), "Panic parsing '{}'", source);
        }
    }
}

// =============================================================================
// Property 2: Valid third tokens should allow clean parsing of labels
// =============================================================================

/// Valid third tokens - these CAN start a statement, so `Identifier: <token>`
/// IS a valid label pattern.
const VALID_THIRD_TOKENS: &[&str] = &[
    "{",       // Block start
    "(",       // Parenthesized expression
    "my",      // Variable declaration keyword
    "our",     // Package variable declaration
    "state",   // State variable declaration
    "local",   // Local variable declaration
    "print",   // Print statement
    "printf",  // Printf statement
    "say",     // Say statement
    "return",  // Return statement
    "exit",    // Exit statement
    "die",     // Die statement
    "warn",    // Warn statement
    "if",      // If statement
    "unless",  // Unless statement
    "while",   // While loop
    "until",   // Until loop
    "for",     // For loop
    "foreach", // Foreach loop
    "do",      // Do block
    "sub",     // Subroutine declaration
    "eval",    // Eval block
    "begin",   // BEGIN block
    "END",     // END block
    "sort",    // Sort function
    "map",     // Map function
    "grep",    // Grep function
    "+",       // Unary plus
    "-",       // Unary minus
    "!",       // Logical not
    "~",       // Bitwise not
    "--",      // Pre-decrement
    "++",      // Pre-increment
    "\\",      // Reference
    "scalar",  // Scalar function
    "undef",   // Undef
];

/// Property test: All valid third tokens should parse as valid labels without errors.
#[test]
fn property_valid_third_tokens_parse_cleanly() {
    for valid_token in VALID_THIRD_TOKENS {
        let source = format!("FOO: {} 1;", valid_token);
        let result = std::panic::catch_unwind(|| {
            // Should parse without panic
            let ast = parse(&source);
            // The sexp should not be empty
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for valid label: '{}'", source);
        });
        assert!(result.is_ok(), "Panic parsing valid label pattern 'FOO: {} 1;'", valid_token);
    }
}

/// Property test: Valid third tokens should not produce expected_colon errors.
#[test]
fn property_valid_third_tokens_no_expected_colon() {
    for valid_token in VALID_THIRD_TOKENS {
        let source = format!("FOO: {} 1;", valid_token);
        let ast = parse(&source);
        let sexp = ast.to_sexp().to_lowercase();

        assert!(
            !sexp.contains("expected_colon"),
            "Unexpected expected_colon for valid label pattern 'FOO: {} 1;'\nsexp: {}",
            valid_token,
            sexp
        );
    }
}

// =============================================================================
// Property 3: Label statements followed by statements should parse cleanly
// =============================================================================

/// Property test: Labels followed by various statement types should parse cleanly.
#[test]
fn property_labels_with_various_statements() {
    let label_statements = [
        ("LABEL: my $x = 1;", "variable declaration"),
        ("LABEL: print 'hello';", "print statement"),
        ("LABEL: { 1; }", "bare block"),
        ("LABEL: if (1) { 1; }", "if statement"),
        ("LABEL: while (1) { last; }", "while loop"),
        ("LABEL: for my $i (1..10) { 1; }", "for loop"),
        ("LABEL: foreach my $x (@arr) { 1; }", "foreach loop"),
        ("LABEL: return 42;", "return statement"),
        ("LABEL: do { 1; };", "do block"),
        ("LABEL: sub foo { 1; }", "subroutine declaration"),
        ("LABEL: 42;", "expression statement"),
        ("LABEL: 'string';", "string literal"),
        ("LABEL: $scalar;", "scalar variable"),
        ("LABEL: @array;", "array variable"),
        ("LABEL: %hash;", "hash variable"),
    ];

    for (statement, description) in label_statements {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(statement);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for label with {}", description);
        });
        assert!(result.is_ok(), "Panic parsing label with {}: {}", description, statement);
    }
}

/// Property test: Labels followed by statements with modifiers should parse cleanly.
#[test]
fn property_labels_with_modified_statements() {
    let modified_statements = [
        "LABEL: print 'hi' if $debug;",
        "LABEL: print 'hi' unless $silent;",
        "LABEL: print 'hi' while $count-- > 0;",
        "LABEL: print 'hi' until $done;",
        "LABEL: print 'hi' for @items;",
        "LABEL: 1 if $x;",
        "LABEL: 1 unless $y;",
    ];

    for statement in modified_statements {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(statement);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for: {}", statement);
        });
        assert!(result.is_ok(), "Panic parsing: {}", statement);
    }
}

// =============================================================================
// Property 4: Ternary expressions should not produce expected_colon errors
// =============================================================================

/// Property test: Ternary expressions should parse without expected_colon errors.
#[test]
fn property_ternary_no_expected_colon() {
    let ternary_patterns = [
        "my $x = $cond ? $then : $else;",
        "my $x = $a ? $b ? $c : $d : $e;",      // Nested ternary
        "my $x = $a ? 1 : $b ? 2 : 3;",         // Chained ternary
        "my $x = COND: ? $then : $else;",       // Looks like label but isn't
        "my $x = $obj->method ? $a : $b;",      // Method call condition
        "my $x = $hash{key} ? $a : $b;",        // Subscript condition
        "my $x = $arr[0] ? $a : $b;",           // Array subscript condition
        "my $x = $cond ? {a => 1} : {b => 2};", // Hash ref in branches
        "my $x = $cond ? [1, 2] : [3, 4];",     // Array ref in branches
        "my $x = $cond ? do { 1 } : 0;",        // Do block in branch
    ];

    for pattern in ternary_patterns {
        let ast = parse(pattern);
        let sexp = ast.to_sexp().to_lowercase();

        assert!(
            !sexp.contains("expected_colon"),
            "expected_colon in ternary pattern: {}\nsexp: {}",
            pattern,
            sexp
        );
    }
}

// =============================================================================
// Property 5: Hash constructors with fat arrows should not produce expected_colon errors
// =============================================================================

/// Property test: Hash constructors with fat arrows should parse cleanly.
#[test]
fn property_hash_constructor_no_expected_colon() {
    let hash_patterns = [
        "my %h = (KEY1 => 'value1', KEY2 => 'value2');",
        "my %h = (FOO => 1, BAR => 2, BAZ => 3);",
        "my %h = (if => 1, for => 2, return => 3);", // Keywords as keys
        "my %h = (a => 1, b => 2, c => 3, d => 4, e => 5);",
        "my %h = (outer => {inner => 'value'}, other => 'x');",
        "my $x = $cond ? {a => 1} : {b => 2};", // Ternary with hash refs
        "my %h = (key => $x ? $a : $b);",       // Ternary as hash value
    ];

    for pattern in hash_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for hash pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing hash pattern: {}", pattern);
    }
}

// =============================================================================
// Property 6: Qualified identifiers (::) should not be confused with labels
// =============================================================================

/// Property test: Qualified identifiers with :: should not be treated as labels.
#[test]
fn property_qualified_identifiers_not_labels() {
    let qualified_patterns = [
        "my $x = $Foo::Bar;",
        "my $x = $Class::method;",
        "my $x = $h{Foo::Bar};",
        "my $x = $obj->$method;",
        "Foo::Bar::baz();",
    ];

    for pattern in qualified_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing qualified identifier: {}", pattern);
    }
}

// =============================================================================
// Property 7: Multiple labels in sequence should parse cleanly
// =============================================================================

/// Property test: Multiple consecutive labels should parse cleanly.
#[test]
fn property_multiple_consecutive_labels() {
    let multi_label_patterns =
        ["OUTER: INNER: my $x = 1;", "A: B: C: 1;", "L1: L2: L3: L4: L5: 42;"];

    for pattern in multi_label_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing multiple labels: {}", pattern);
    }
}

/// Property test: Labels inside nested blocks should parse cleanly.
#[test]
fn property_labels_in_nested_contexts() {
    let nested_patterns = [
        r#"my $x = do { FOO: my $y = 1; $y };"#,
        r#"{ FOO: my $y = 1; }"#,
        r#"
        OUTER: for my $i (1..3) {
            INNER: for my $j (1..3) {
                next OUTER if $i == 2;
            }
        }
        "#,
    ];

    for pattern in nested_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for nested label pattern");
        });
        assert!(result.is_ok(), "Panic parsing nested labels");
    }
}

// =============================================================================
// Property 8: Unicode and long identifiers as labels should work
// =============================================================================

/// Property test: Unicode identifiers as labels should parse cleanly.
#[test]
fn property_unicode_labels() {
    let unicode_patterns = ["日本語: my $x = 1;", "中文: print 1;", "Ελληνικά: 42;"];

    for pattern in unicode_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for unicode label: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing unicode label");
    }
}

/// Property test: Long identifiers as labels should parse cleanly.
#[test]
fn property_long_label_identifiers() {
    let long_name = "A".repeat(1000);
    let source = format!("{}: my $x = 1;", long_name);

    let result = std::panic::catch_unwind(|| {
        let ast = parse(&source);
        let sexp = ast.to_sexp();
        assert!(!sexp.is_empty(), "Empty sexp for long label");
    });
    assert!(result.is_ok(), "Panic parsing long label identifier");
}

// =============================================================================
// Property 9: Parser should never panic on any input
// =============================================================================

/// Property test: Parser should never panic on any valid Perl-like input.
#[test]
fn property_parser_never_panics() {
    let all_patterns = [
        // Invalid third tokens
        "X: ?",
        "X: :",
        "X: ;",
        "X: ,",
        "X: =>",
        "X: )",
        "X: ]",
        "X: }",
        "X:",
        // Valid third tokens
        "X: { 1; }",
        "X: (1)",
        "X: my $x = 1;",
        "X: if (1) { }",
        "X: while (1) { }",
        "X: for my $i (1..10) { }",
        // Ternary patterns
        "my $x = $a ? $b : $c;",
        "my $x = $a ? 1 : $b ? 2 : 3;",
        // Hash patterns
        "my %h = (KEY => 'value');",
        // Mixed
        "FOO: my %h = (KEY => 'value');",
        "LABEL: print 'hi' if $debug;",
    ];

    for pattern in all_patterns {
        let result = std::panic::catch_unwind(|| {
            let _ast = parse(pattern);
        });
        assert!(result.is_ok(), "Parser panicked on: {}", pattern);
    }
}

// =============================================================================
// Property 10: Stress test - many iterations of label parsing
// =============================================================================

/// Property test: Parse many label patterns in sequence without error.
#[test]
fn property_many_labels_stress() {
    let pattern = r#"
        L1: my $x1 = 1;
        L2: my $x2 = 2;
        L3: my $x3 = 3;
        L4: my $x4 = 4;
        L5: my $x5 = 5;
        L6: print $x1;
        L7: print $x2;
        L8: print $x3;
        L9: print $x4;
        L10: print $x5;
        L11: if ($x1 > 0) { L12: print 'positive'; }
        L13: while ($x2 > 0) { L14: { last; } }
        L15: for my $i (1..3) { L16: print $i; }
        L17: { L18: 1; L19: 2; L20: 3; }
        L21: do { L22: 1; };
        L23: return $x1 + $x2 + $x3 + $x4 + $x5;
    "#;

    let result = std::panic::catch_unwind(|| {
        let ast = parse(pattern);
        let sexp = ast.to_sexp();
        assert!(!sexp.is_empty(), "Empty sexp for many labels");
        assert!(
            !sexp.to_lowercase().contains("expected_colon"),
            "expected_colon in many labels pattern"
        );
    });
    assert!(result.is_ok(), "Panic parsing many labels");
}

// =============================================================================
// Property 11: Systematic enumeration of all third token categories
// =============================================================================

/// Property test: All closing delimiters should be invalid third tokens.
#[test]
fn property_closing_delimiters_invalid() {
    let closing_delimiters = [")", "]", "}"];

    for delimiter in closing_delimiters {
        let source = format!("X: {}", delimiter);
        let ast = parse(&source);
        let sexp = ast.to_sexp().to_lowercase();

        assert!(
            !sexp.contains("expected_colon"),
            "expected_colon for closing delimiter 'X: {}'\nsexp: {}",
            delimiter,
            sexp
        );
    }
}

/// Property test: All opening delimiters should be valid third tokens.
#[test]
fn property_opening_delimiters_valid() {
    let opening_delimiters = ["(", "[", "{"];

    for delimiter in opening_delimiters {
        let source = if delimiter == "(" {
            format!("X: {} 1);", delimiter)
        } else if delimiter == "[" {
            format!("X: {} 1];", delimiter)
        } else {
            format!("X: {} 1; }}", delimiter)
        };
        let result = std::panic::catch_unwind(|| {
            let ast = parse(&source);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for opening delimiter 'X: {}'", delimiter);
        });
        assert!(result.is_ok(), "Panic for opening delimiter 'X: {}'", delimiter);
    }
}

/// Property test: All fat arrow family tokens should be invalid third tokens.
#[test]
fn property_fat_arrow_family_invalid() {
    // Note: Just "=>" by itself is not valid Perl, but we test it doesn't cause expected_colon
    let patterns = ["X: =>", "X: => 1", "X: => 'value'"];

    for source in patterns {
        let ast = parse(source);
        let sexp = ast.to_sexp().to_lowercase();

        assert!(
            !sexp.contains("expected_colon"),
            "expected_colon for pattern '{}'\nsexp: {}",
            source,
            sexp
        );
    }
}

/// Property test: Statement keywords should be valid third tokens.
#[test]
fn property_statement_keywords_valid() {
    let keywords = [
        "if",
        "unless",
        "while",
        "until",
        "for",
        "foreach",
        "do",
        "eval",
        "BEGIN",
        "END",
        "CHECK",
        "INIT",
        "UNITCHECK",
    ];

    for keyword in keywords {
        let source = format!("LABEL: {} 1;", keyword);
        let result = std::panic::catch_unwind(|| {
            let ast = parse(&source);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for keyword '{}'", keyword);
        });
        assert!(result.is_ok(), "Panic for keyword 'LABEL: {} 1;'", keyword);
    }
}

// =============================================================================
// Property 12: Identifier name variations
// =============================================================================

/// Property test: All common identifier patterns should work as label names.
#[test]
fn property_common_identifier_patterns() {
    let identifiers = [
        // Short names
        "A",
        "X",
        "F",
        // Standard cases
        "FOO",
        "BAR",
        "BAZ",
        "LABEL",
        "LOOP",
        "OUTER",
        "INNER",
        // Lowercase
        "foo",
        "bar",
        "label",
        "loop",
        // Mixed case
        "Foo",
        "Bar",
        "Label",
        // With underscores
        "foo_bar",
        "foo_bar_baz",
        "a_b_c",
        // With numbers
        "foo1",
        "foo2",
        "label123",
        // Perl special
        "_",
        "__",
        "_foo",
        // All caps (idiomatic labels)
        "BEGIN",
        "END",
        "CHECK",
        "INIT",
    ];

    for id in identifiers {
        let source = format!("{}: my $x = 1;", id);
        let result = std::panic::catch_unwind(|| {
            let ast = parse(&source);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for identifier '{}'", id);
        });
        assert!(result.is_ok(), "Panic for identifier '{}'", id);
    }
}

// =============================================================================
// Property 13: All unary operators should be valid statement starters
// =============================================================================

/// Property test: All unary operators should be valid after a label.
#[test]
fn property_unary_operators_valid() {
    let unary_ops =
        [("+", "5"), ("-", "5"), ("!", "5"), ("~", "5"), ("++", "$x"), ("--", "$x"), ("\\", "$x")];

    for (op, expr) in unary_ops {
        let source = format!("LABEL: {} {};", op, expr);
        let result = std::panic::catch_unwind(|| {
            let ast = parse(&source);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for unary op 'LABEL: {} {}'", op, expr);
        });
        assert!(result.is_ok(), "Panic for unary op 'LABEL: {} {}'", op, expr);
    }
}

// =============================================================================
// Property 14: Control flow with labels
// =============================================================================

/// Property test: next/last/redo with label references should work.
#[test]
fn property_label_control_flow() {
    let patterns = [
        "OUTER: for my $i (1..10) { last OUTER if $i == 5; }",
        "LOOP: while (1) { last LOOP if $done; }",
        "COUNT: { redo COUNT if ++$n < 3; }",
        "OUTER: for my $i (1..3) { INNER: for my $j (1..3) { next OUTER if $i == 2; } }",
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            assert!(!sexp.is_empty(), "Empty sexp for: {}", pattern);
        });
        assert!(result.is_ok(), "Panic for: {}", pattern);
    }
}
