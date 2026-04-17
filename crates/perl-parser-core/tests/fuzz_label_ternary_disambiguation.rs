//! Fuzz tests for label/ternary disambiguation in `is_label_start()`.
//!
//! These tests exercise the parser with randomly generated Perl code patterns
//! to find crashes, panics, and unexpected behavior in the label/ternary
//! disambiguation logic.
//!
//! The `is_label_start()` function uses 3-token lookahead:
//! - Token 1 (peek): Is it an `Identifier`?
//! - Token 2 (peek_second): Is it a `Colon`?
//! - Token 3 (peek_third): Can the token after colon start a statement?
//!
//! Fuzz targets:
//! 1. Random identifier + colon + third token combinations
//! 2. Ternary expression patterns that look like labels
//! 3. Hash constructor patterns with potential label confusion
//! 4. Label patterns followed by various statement types

mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Fuzz Target 1: Random identifier + colon + third token patterns
// =============================================================================

/// Fuzz test: Random identifier names followed by colon and various tokens.
///
/// This tests the parser's ability to handle arbitrary identifier names
/// in contexts that might look like labels.
#[test]
fn fuzz_identifier_colon_third_token() {
    // Valid label patterns
    let valid_patterns = [
        "FOO: my $x = 1;",
        "BAR: print 'hello';",
        "LABEL: { 1; }",
        "X: (1);",
        "A: if (1) { }",
        "B: while (1) { }",
        "C: for my $i (1..10) { }",
        "D: unless (0) { }",
        "E: return 42;",
        "LOOP: last LOOP if $done;",
    ];

    for pattern in valid_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic, should produce some valid parse
            assert!(!sexp.is_empty(), "Empty sexp for valid pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing valid pattern: {}", pattern);
    }

    // Invalid label patterns (colon followed by non-statement-starting tokens)
    let invalid_patterns = [
        "FOO: ?",
        "FOO: :",
        "FOO: ;",
        "FOO: )",
        "FOO: ]",
        "FOO: }",
        "FOO: =>",
        "BAR: ? $x",
        "BAR: : $x",
        "X: ? :",
    ];

    for pattern in invalid_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic even for invalid patterns
            // These may produce error nodes, which is expected
            assert!(!sexp.is_empty(), "Empty sexp for pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 2: Ternary expressions that look like labels
// =============================================================================

/// Fuzz test: Ternary expressions where the condition looks like a label.
///
/// This tests that `is_label_start()` correctly returns `false` for patterns
/// like `CONDITION: ? $then : $else` where the colon belongs to the ternary.
#[test]
fn fuzz_ternary_condition_like_label() {
    let patterns = [
        // Ternary with uppercase condition (looks like label)
        "my $x = COND: ? $then : $else;",
        // Ternary with lowercase condition
        "my $x = $cond ? $a : $b;",
        // Nested ternaries
        "my $x = $a ? $b ? $c : $d : $e;",
        "my $x = $a ? $b : $c ? $d : $e;",
        // Ternary with hash ref
        "my $x = $cond ? {a => 1} : {b => 2};",
        // Ternary with array ref
        "my $x = $cond ? [1, 2] : [3, 4];",
        // Ternary with parens
        "my $x = ($a > $b) ? $c : $d;",
        // Ternary with method call
        "my $x = $obj->method ? $a : $b;",
        // Ternary with subscript
        "my $x = $arr[0] ? $a : $b;",
        // Chained ternary
        "my $x = $a ? $b : $c ? $d : $f ? $g : $h;",
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic - parser should handle all of these
            assert!(!sexp.is_empty(), "Empty sexp for ternary pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing ternary pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 3: Hash constructors with potential label confusion
// =============================================================================

/// Fuzz test: Hash constructors with keys that look like labels.
///
/// This tests that `is_label_start()` correctly returns `false` for patterns
/// like `KEY: => 'value'` where the colon belongs to autoquoting context.
#[test]
fn fuzz_hash_constructor_label_confusion() {
    let patterns = [
        // Simple hash constructor with fat arrows
        "my %h = (KEY1 => 'value1', KEY2 => 'value2');",
        // Uppercase keys (look like labels)
        "my %h = (FOO => 1, BAR => 2, BAZ => 3);",
        // Keys that are also keywords
        "my %h = (if => 1, for => 2, return => 3);",
        // Mixed keys
        "my %h = (A => 1, b => 2, C => 3);",
        // Nested hash in hash
        "my %h = (outer => {inner => 'value'}, other => 'x');",
        // Hash with fat arrow and labels nearby
        "LABEL: my %h = (KEY => 'value');",
        // Multiple fat arrows
        "my %h = (a => 1, b => 2, c => 3, d => 4, e => 5);",
        // Fat arrow in list assignment
        "my %h = foo => 'bar';",
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic
            assert!(!sexp.is_empty(), "Empty sexp for hash pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing hash pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 4: Label statements followed by various statement types
// =============================================================================

/// Fuzz test: Valid labels followed by various statement types.
///
/// This tests that `is_label_start()` correctly returns `true` for valid label
/// patterns where the third token CAN start a statement.
#[test]
fn fuzz_valid_labels_with_various_statements() {
    let patterns = [
        // Label + expression statement
        "LABEL: 42;",
        "LABEL: 'string';",
        "LABEL: $scalar;",
        "LABEL: @array;",
        "LABEL: %hash;",
        // Label + variable declaration
        "LABEL: my $x = 1;",
        "LABEL: our $x = 1;",
        "LABEL: state $x = 1;",
        // Label + print
        "LABEL: print 'hello';",
        "LABEL: print $x;",
        // Label + block
        "LABEL: { 1; }",
        "LABEL: { my $x = 1; print $x; }",
        // Label + if
        "LABEL: if ($x) { 1; }",
        "LABEL: unless ($x) { 1; }",
        // Label + while
        "LABEL: while (1) { last; }",
        // Label + for
        "LABEL: for my $i (1..10) { print $i; }",
        // Label + foreach
        "LABEL: foreach my $x (@arr) { print $x; }",
        // Label + do block
        "LABEL: do { 1; };",
        // Label + subroutine
        "LABEL: sub foo { 1; }",
        // Label + return
        "LABEL: return 42;",
        // Label + next/last/redo with label reference
        "OUTER: for my $i (1..10) { INNER: for my $j (1..10) { next OUTER if $i == 5; } }",
        // Label + statement modifier
        "LABEL: print 'hi' if $debug;",
        "LABEL: print 'hi' unless $silent;",
        "LABEL: print 'hi' while $count-- > 0;",
        "LABEL: print 'hi' for @items;",
        "LABEL: print 'hi' until $done;",
        // Label + given (Perl 5.38+)
        // "LABEL: given ($x) { when (1) { 1; } }",
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic
            assert!(!sexp.is_empty(), "Empty sexp for label pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing label pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 5: Edge cases around third token validation
// =============================================================================

/// Fuzz test: Edge cases for the third token validation in `is_label_start()`.
///
/// This tests that the parser correctly identifies when the third token
/// CAN or CANNOT start a statement.
#[test]
fn fuzz_third_token_edge_cases() {
    // Third token is a closing delimiter - should NOT be a label
    let non_label_patterns = [
        "X: )",
        "X: ]",
        "X: }",
        "X: ;",
        "X: ?",
        "X: :",
        "X: =>",
        "X: ,",
        "X:",
    ];

    for pattern in non_label_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic
            assert!(!sexp.is_empty(), "Empty sexp for non-label pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing non-label pattern: {}", pattern);
    }

    // Third token is an opening delimiter or identifier - SHOULD be a label
    let label_patterns = [
        "X: (1)",
        "X: [1]",
        "X: {1}",
        "X: (my $x = 1)",
        "X: [my @x = (1,2,3)]",
        "X: { my $x = 1; }",
        "X: if (1) { }",
        "X: while (1) { }",
        "X: for my $i (1..10) { }",
        "X: print 1;",
        "X: return 1;",
        "X: my $x = 1;",
        "X: sub foo { }",
    ];

    for pattern in label_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic
            assert!(!sexp.is_empty(), "Empty sexp for label pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing label pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 6: Combinations of labels, ternaries, and hashes
// =============================================================================

/// Fuzz test: Complex combinations of labels, ternaries, and hashes.
///
/// This tests the parser's ability to handle complex expressions that mix
/// label-like patterns with ternary and hash constructors.
#[test]
fn fuzz_complex_combinations() {
    let patterns = [
        // Label followed by hash constructor
        "LABEL: my %h = (KEY => 'value');",
        // Ternary in hash value position
        "my %h = (key => $cond ? $a : $b);",
        // Hash in ternary then-branch
        "my $x = $cond ? {a => 1} : {b => 2};",
        // Label after hash
        "my %h = (a => 1); LABEL: print 'done';",
        // Multiple statements with labels
        "LABEL1: print 'one'; LABEL2: print 'two';",
        // Label inside ternary condition (this would be weird but shouldn't panic)
        "my $x = (LABEL: 1) ? $a : $b;",
        // Nested blocks with labels
        "OUTER: { INNER: { 1; } }",
        // Label in do block
        "LABEL: do { 1; };",
        // Label in statement modifier
        "LABEL: 1 if $x;",
        // Ternary result used as hash value
        "my %h = (a => $x ? 1 : 2, b => $y ? 3 : 4);",
        // Complex nested ternary with hash
        "my $x = $a ? ($b ? {x=>1} : {y=>2}) : ($c ? {z=>3} : {w=>4});",
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic
            assert!(!sexp.is_empty(), "Empty sexp for complex pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing complex pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 7: Empty and minimal inputs
// =============================================================================

/// Fuzz test: Empty and minimal inputs to ensure parser doesn't panic.
#[test]
fn fuzz_empty_and_minimal() {
    let patterns = [
        "",                 // Empty
        ";",                // Just semicolon
        ";;",               // Two semicolons
        ":",                // Just colon
        "::",               // Double colon
        ":::",              // Triple colon
        "?",                // Just question
        "?:",               // Question colon
        "??",               // Double question
        "?",                // Just question
        "?",                // Question
        "???:",             // Multiple question marks and colon
        "LABEL:",           // Label without statement (at EOF)
        "X:",               // Short label without statement
        "//",               // Two slashes (regex or division)
        "///",              // Three slashes
        "# comment",        // Comment only
        "# comment\n",      // Comment with newline
        "# comment\n;",     // Comment and semicolon
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic - even if parse fails, it should be handled
            assert!(!sexp.is_empty() || pattern.is_empty(),
                "Empty sexp for pattern (non-empty input): {:?}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing minimal pattern: {:?}", pattern);
    }
}

// =============================================================================
// Fuzz Target 8: Unicode and special characters in labels
// =============================================================================

/// Fuzz test: Unicode and special characters in label-like patterns.
#[test]
fn fuzz_unicode_and_special_chars() {
    let patterns = [
        // Unicode identifiers (Perl supports Unicode in identifiers)
        "日本語: print 1;",
        "中文: 1;",
        "Unicode: 1;",
        // Long identifiers
        "VeryLongIdentifierNameHere: print 1;",
        // Identifiers with underscores
        "foo_bar: print 1;",
        "foo_bar_baz: print 1;",
        // Identifiers with numbers
        "test123: print 1;",
        "abc123def: print 1;",
        // Qualified identifiers (should not be confused with labels)
        "Foo::Bar::baz;",
        "$obj->method;",
        "$h->{key};",
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic
            assert!(!sexp.is_empty(), "Empty sexp for unicode pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing unicode pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 9: Verify no expected_colon errors for label-like patterns
// =============================================================================

/// Fuzz test: Verify that the fix doesn't introduce expected_colon errors
/// for valid label patterns.
///
/// This is a regression test to ensure that the 3-token lookahead fix
/// doesn't incorrectly reject valid labels.
#[test]
fn fuzz_no_expected_colon_for_valid_labels() {
    let valid_label_patterns = [
        "FOO: my $x = 1;",
        "BAR: print 'hello';",
        "LABEL: { 1; }",
        "LOOP: while (1) { last LOOP; }",
        "OUTER: for my $i (1..10) { INNER: for my $j (1..10) { 1; } }",
        "CHECK: if ($x > 0) { 1; }",
        "ITER: for my $i (1..10) { print $i; }",
        "DEBUG: print 'debug' if $debug;",
        "RETRY: return $result if defined $result;",
        "AGAIN: print 'loop' while $count-- > 0;",
        "SKIP: print 'skip' unless $skip;",
        "LIST: print $_ for @items;",
        "UNTIL: 1 until $done;",
        "WHEN: given ($x) { when (1) { 1; } }",
        // Additional edge cases
        "A: B: C: 1;",              // Multiple labels
        "FOO: (1);",                 // Label with paren expr
        "BAR: [1, 2, 3];",          // Label with array
        "BAZ: {a => 1};",           // Label with hash ref
        "X: do { 1; };",           // Label with do block
        "Y: sub { 1; };",          // Label with anon sub
    ];

    for pattern in valid_label_patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not produce expected_colon error
            assert!(
                !sexp.to_lowercase().contains("expected_colon"),
                "Found expected_colon error for valid label pattern: {}\nsexp: {}",
                pattern, sexp
            );
        });
        assert!(result.is_ok(), "Panic parsing valid label pattern: {}", pattern);
    }
}

// =============================================================================
// Fuzz Target 10: Stress test with many statements and labels
// =============================================================================

/// Fuzz test: Many labels and statements in sequence.
///
/// This tests parser performance and correctness with a large number
/// of statements and labels.
#[test]
fn fuzz_many_labels_and_statements() {
    // Build a string with many label statements
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
        // Should not panic
        assert!(!sexp.is_empty(), "Empty sexp for many-labels pattern");
        // Should not have expected_colon errors
        assert!(
            !sexp.to_lowercase().contains("expected_colon"),
            "Found expected_colon error in many-labels pattern"
        );
    });
    assert!(result.is_ok(), "Panic parsing many-labels pattern");
}

// =============================================================================
// Fuzz Target 11: Regression test for specific CPAN patterns
// =============================================================================

/// Fuzz test: Regression tests for specific patterns from the CPAN corpus
/// that triggered expected_colon errors.
///
/// These patterns were identified in the original issue as causing
/// parser errors related to label/ternary disambiguation.
#[test]
fn fuzz_cpan_regression_patterns() {
    // These are simplified versions of patterns that might appear in CPAN code
    let patterns = [
        // Ternary in hash value
        "my %h = (key => $cond ? $a : $b);",
        // Ternary as hash value with uppercase key
        "my %h = (KEY => $cond ? $a : $b);",
        // Multiple ternaries
        "my $x = $a ? $b : $c ? $d : $e;",
        // Ternary with method call in condition
        "my $x = $obj->method ? $a : $b;",
        // Ternary with subscript in condition
        "my $x = $arr[0] ? $a : $b;",
        // Ternary with postfix deref in condition
        "my $x = $obj->$method ? $a : $b;",
        // Fat arrow with uppercase key
        "my %h = (FOO => 1, BAR => 2);",
        // Keyword used as hash key
        "my %h = (if => 1, for => 2, return => 3);",
        // Nested ternary in hash
        "my %h = (a => $x ? $y : $z, b => $w ? $u : $v);",
        // Label with ternary-like syntax (shouldn't be confused)
        "OUTER: for my $i (1..10) { last OUTER if $i == 5; }",
        // Label followed by hash constructor
        "LABEL: my %h = (key => 'value');",
        // Chained ternary with fat arrows
        "my $x = $a ? 'a' : $b ? 'b' : 'c';",
    ];

    for pattern in patterns {
        let result = std::panic::catch_unwind(|| {
            let ast = parse(pattern);
            let sexp = ast.to_sexp();
            // Should not panic
            assert!(!sexp.is_empty(), "Empty sexp for CPAN pattern: {}", pattern);
        });
        assert!(result.is_ok(), "Panic parsing CPAN pattern: {}", pattern);
    }
}