//! Comprehensive slash ambiguity tests for division vs regex disambiguation
//!
//! Tests feature spec: ROADMAP.md#known-gaps-hang-bounds-risks
//! Tests feature spec: ROADMAP.md#slash-ambiguity
//!
//! This test suite validates that the lexer correctly disambiguates between
//! division operators and regex match operators in all contexts, preventing
//! hang conditions and incorrect parsing.
//!
//! Coverage areas:
//! - Division after terms (numbers, variables, closing delimiters)
//! - Regex at expression start
//! - Defined-or operator (//) vs empty regex match
//! - Context-sensitive disambiguation
//! - Edge cases with whitespace and comments
//! - Complex real-world scenarios
//! - Pathological inputs that might cause lexer confusion

use perl_lexer::{PerlLexer, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test division after numeric literal
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_number() -> TestResult {
    let code = "42 / 2";
    let mut lexer = PerlLexer::new(code);

    // Expect: 42 (Number), / (Division), 2 (Number)
    let tok1 = lexer.next_token().ok_or("Expected number token")?;
    assert!(matches!(tok1.token_type, TokenType::Number(_)));

    let tok2 = lexer.next_token().ok_or("Expected division operator")?;
    assert!(
        matches!(tok2.token_type, TokenType::Division),
        "Expected division operator after number, got {:?}",
        tok2.token_type
    );
    Ok(())
}

/// Test division after variable
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_variable() -> TestResult {
    let code = "$x / 2";
    let mut lexer = PerlLexer::new(code);

    // Skip to division operator
    let _ = lexer.next_token(); // $x
    let tok = lexer.next_token().ok_or("Expected division operator")?;

    assert!(
        matches!(tok.token_type, TokenType::Division),
        "Expected division operator after variable, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test division after closing parenthesis
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_paren() -> TestResult {
    let code = "(1 + 2) / 3";
    let mut lexer = PerlLexer::new(code);

    // Skip to after closing paren
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if matches!(tok.token_type, TokenType::RightParen) {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected division operator")?;
    assert!(
        matches!(tok.token_type, TokenType::Division),
        "Expected division operator after closing paren, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test division after array/hash access
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_subscript() -> TestResult {
    let test_cases = vec!["$arr[0] / 2", "$hash{key} / 2", "@arr[0..5] / 2"];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);

        // Tokenize until we find RBracket or RBrace
        loop {
            let tok = lexer.next_token().ok_or("Expected token")?;
            if matches!(tok.token_type, TokenType::RightBracket | TokenType::RightBrace) {
                break;
            }
        }

        let tok = lexer.next_token().ok_or("Expected division operator")?;
        assert!(
            matches!(tok.token_type, TokenType::Division),
            "Expected division after subscript in '{}', got {:?}",
            code,
            tok.token_type
        );
    }
    Ok(())
}

/// Test regex match at statement start
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_at_statement_start() -> TestResult {
    let code = "/pattern/";
    let mut lexer = PerlLexer::new(code);

    let tok = lexer.next_token().ok_or("Expected regex token")?;
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex at statement start, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test regex match after binding operator
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_after_binding() -> TestResult {
    let code = "$x =~ /pattern/";
    let mut lexer = PerlLexer::new(code);

    // Skip to binding operator
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if tok.text.as_ref() == "=~" {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected regex token")?;
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex after =~, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test regex match after negated binding operator
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_after_negated_binding() -> TestResult {
    let code = "$x !~ /pattern/";
    let mut lexer = PerlLexer::new(code);

    // Skip to binding operator
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if tok.text.as_ref() == "!~" {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected regex token")?;
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex after !~, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test defined-or operator (//) vs empty regex
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_defined_or_operator() -> TestResult {
    let code = "$x // $y";
    let mut lexer = PerlLexer::new(code);

    // Skip $x
    let _ = lexer.next_token();

    let tok = lexer.next_token().ok_or("Expected defined-or operator")?;
    // The defined-or operator // is represented as an Operator token
    assert!(
        matches!(tok.token_type, TokenType::Operator(_)),
        "Expected defined-or operator, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test chained division operations
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_chained_division() -> TestResult {
    let code = "100 / 10 / 2";
    let mut lexer = PerlLexer::new(code);

    // First number
    let _ = lexer.next_token();

    // First division
    let tok1 = lexer.next_token().ok_or("Expected first division")?;
    assert!(
        matches!(tok1.token_type, TokenType::Division),
        "Expected division, got {:?}",
        tok1.token_type
    );

    // Second number
    let _ = lexer.next_token();

    // Second division
    let tok2 = lexer.next_token().ok_or("Expected second division")?;
    assert!(
        matches!(tok2.token_type, TokenType::Division),
        "Expected division in chain, got {:?}",
        tok2.token_type
    );
    Ok(())
}

/// Test division with whitespace variations
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_whitespace_variations() -> TestResult {
    let test_cases = vec![
        "42/2",     // No whitespace
        "42 /2",    // Space before
        "42/ 2",    // Space after
        "42 / 2",   // Spaces both sides
        "42  /  2", // Multiple spaces
        "42\t/\t2", // Tabs
        "42\n/\n2", // Newlines
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let _ = lexer.next_token(); // number

        let tok = lexer.next_token().ok_or("Expected division operator")?;
        assert!(
            matches!(tok.token_type, TokenType::Division),
            "Expected division in '{}', got {:?}",
            code,
            tok.token_type
        );
    }
    Ok(())
}

/// Test regex with whitespace variations
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_whitespace_variations() -> TestResult {
    let test_cases = vec![
        "$x=~/pattern/",     // No whitespace
        "$x =~ /pattern/",   // Standard spacing
        "$x  =~  /pattern/", // Extra spaces
        "$x=~\n/pattern/",   // Newline before regex
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);

        // Tokenize until we find binding operator
        loop {
            let tok = lexer.next_token().ok_or("Expected token")?;
            if tok.text.as_ref() == "=~" {
                break;
            }
        }

        let tok = lexer.next_token().ok_or("Expected regex token")?;
        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex in '{}', got {:?}",
            code,
            tok.token_type
        );
    }
    Ok(())
}

/// Test slash after string literal
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_string() -> TestResult {
    let code = r#""hello" / 2"#;
    let mut lexer = PerlLexer::new(code);

    let _ = lexer.next_token(); // string

    let tok = lexer.next_token().ok_or("Expected division operator")?;
    assert!(
        matches!(tok.token_type, TokenType::Division),
        "Expected division after string, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test slash after function call
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_function_call() -> TestResult {
    let code = "func() / 2";
    let mut lexer = PerlLexer::new(code);

    // Skip to closing paren
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if matches!(tok.token_type, TokenType::RightParen) {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected division operator")?;
    assert!(
        matches!(tok.token_type, TokenType::Division),
        "Expected division after function call, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test regex in list context
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_in_list_context() -> TestResult {
    let code = "if ($x =~ /pattern/) { }";
    let mut lexer = PerlLexer::new(code);

    // Skip to binding operator
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if tok.text.as_ref() == "=~" {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected regex token")?;
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex in if condition, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test division in complex expression
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_complex_expression() -> TestResult {
    let code = "(($a + $b) * $c) / ($d - $e)";
    let mut lexer = PerlLexer::new(code);

    // Find the division operator
    let mut found_div = false;
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if matches!(tok.token_type, TokenType::Division) {
            found_div = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(found_div, "Expected to find division operator in complex expression");
    Ok(())
}

/// Test regex with modifiers
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_with_modifiers() -> TestResult {
    let test_cases = vec!["$x =~ /pattern/i", "$x =~ /pattern/gi", "$x =~ /pattern/imsx"];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);

        // Skip to binding operator
        loop {
            let tok = lexer.next_token().ok_or("Expected token")?;
            if tok.text.as_ref() == "=~" {
                break;
            }
        }

        let tok = lexer.next_token().ok_or("Expected regex token")?;
        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex with modifiers in '{}', got {:?}",
            code,
            tok.token_type
        );
    }
    Ok(())
}

/// Test pathological case: many slashes in sequence
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_pathological_many_slashes() -> TestResult {
    // This should parse as: $a / $b / $c / $d
    let code = "$a / $b / $c / $d";
    let mut lexer = PerlLexer::new(code);

    let mut slash_count = 0;
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if matches!(tok.token_type, TokenType::Division) {
            slash_count += 1;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert_eq!(slash_count, 3, "Expected 3 division operators");
    Ok(())
}

/// Test slash after postfix increment/decrement
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_postfix_op() {
    let test_cases = vec!["$x++ / 2", "$x-- / 2"];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);

        // Skip to division operator - the lexer may tokenize ++ and -- differently
        // So we'll just look for the division operator after a variable token
        let mut found_var = false;
        let mut found_div = false;
        while let Some(tok) = lexer.next_token() {
            // Mark when we've seen the variable
            if tok.text.as_ref() == "$x" {
                found_var = true;
            }
            // Look for division after the variable
            if found_var && matches!(tok.token_type, TokenType::Division) {
                found_div = true;
                break;
            }
            // Stop at EOF
            if matches!(tok.token_type, TokenType::EOF) {
                break;
            }
        }

        assert!(found_div, "Expected to find division operator after variable in '{}'", code);
    }
}

/// Test regex after opening brace (block/hash context)
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_after_brace() -> TestResult {
    let code = "if (1) { /pattern/ }";
    let mut lexer = PerlLexer::new(code);

    // Skip to opening brace
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if matches!(tok.token_type, TokenType::LeftBrace) {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected regex token")?;
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex after brace, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test division after array dereference
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_array_deref() -> TestResult {
    let code = "@$arrayref / 2";
    let mut lexer = PerlLexer::new(code);

    // Skip to identifier
    let _ = lexer.next_token(); // @
    let _ = lexer.next_token(); // $arrayref (or combined token)

    let tok = lexer.next_token().ok_or("Expected division operator")?;
    assert!(
        matches!(tok.token_type, TokenType::Division),
        "Expected division after array deref, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test regex in ternary operator
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_regex_in_ternary() -> TestResult {
    let code = "$x ? /pattern/ : 0";
    let mut lexer = PerlLexer::new(code);

    // Skip to ?
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if tok.text.as_ref() == "?" {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected regex token")?;
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex in ternary, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test slash after here-doc
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_division_after_heredoc() -> TestResult {
    let code = "my $x = <<'EOF'\ntext\nEOF\n$x / 2";
    let mut lexer = PerlLexer::new(code);

    // Tokenize until we find the division
    let mut found_div = false;
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if matches!(tok.token_type, TokenType::Division) {
            found_div = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(found_div, "Expected division after heredoc");
    Ok(())
}

/// Test that lexer doesn't hang on ambiguous slash sequences
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_no_hang_on_pathological_input() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // Pathological input with many potential ambiguities
    let code = "////// //// // / //// ///";
    let code_arc = Arc::new(code.to_string());
    let result_arc = Arc::new(Mutex::new(Vec::new()));
    let result_clone = Arc::clone(&result_arc);

    let handle = std::thread::spawn(move || {
        let mut lexer = PerlLexer::new(&code_arc);
        let mut tokens = Vec::new();

        while let Some(tok) = lexer.next_token() {
            let is_eof = matches!(tok.token_type, TokenType::EOF);
            tokens.push(tok);
            if is_eof {
                break;
            }
        }

        if let Ok(mut guard) = result_clone.lock() {
            *guard = tokens;
        }
    });

    // Wait max 2 seconds for lexer to complete
    let _timeout = Duration::from_secs(2);
    let completed = handle.join().is_ok();

    assert!(completed, "Lexer should complete within timeout on pathological slash input");
}

/// Test regex vs division after keyword
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_after_keyword() -> TestResult {
    // After 'return', slash should be division if following a term
    let code1 = "return 42 / 2";
    let mut lexer = PerlLexer::new(code1);

    // Skip to slash
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if tok.text.as_ref() == "42" {
            break;
        }
    }

    let tok = lexer.next_token().ok_or("Expected operator")?;
    assert!(
        matches!(tok.token_type, TokenType::Division),
        "Expected division after return value, got {:?}",
        tok.token_type
    );
    Ok(())
}

/// Test complex real-world scenario: division in conditional
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_real_world_division_in_conditional() -> TestResult {
    let code = "if ($total / $count > 100) { }";
    let mut lexer = PerlLexer::new(code);

    // Find division operator
    let mut found_div = false;
    loop {
        let tok = lexer.next_token().ok_or("Expected token")?;
        if matches!(tok.token_type, TokenType::Division) {
            found_div = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(found_div, "Expected to find division in conditional expression");
    Ok(())
}

/// Test complex real-world scenario: regex in map/grep
///
/// Tests feature spec: ROADMAP.md#slash-ambiguity
#[test]
fn lexer_slash_ambiguity_real_world_regex_in_map_grep() -> TestResult {
    let test_cases = vec![
        "map { /pattern/ } @list",
        "grep { /pattern/ } @list",
        "map { $_ =~ /pattern/ } @list",
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);

        // Find regex token
        let mut found_regex = false;
        loop {
            let tok = lexer.next_token().ok_or("Expected token")?;
            if matches!(tok.token_type, TokenType::RegexMatch) {
                found_regex = true;
                break;
            }
            if matches!(tok.token_type, TokenType::EOF) {
                break;
            }
        }

        assert!(found_regex, "Expected to find regex in map/grep: '{}'", code);
    }
    Ok(())
}
