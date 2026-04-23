use super::*;
use std::time::Instant;

fn errors_contain(errors: &[ParseError], needle: &str) -> bool {
    errors.iter().any(|e| e.to_string().contains(needle))
}

#[test]
fn test_recursive_heredoc_terminator_hang() {
    // Issue #443: Recursive heredoc terminators hang risk
    // The parser should not hang when encountering identical terminators in content
    let code = r#"
        my $outer = <<'END';
        Content before inner
        my $inner = <<'END';
        Inner content
        END
        Content after inner
        END
    "#;

    let start = Instant::now();
    let mut parser = Parser::new(code);
    let result = parser.parse();
    let duration = start.elapsed();

    // Should complete quickly (not hang)
    assert!(duration.as_secs() < 2, "Parser should not hang on recursive terminators");

    match result {
        Ok(_) => {}
        Err(err) => unreachable!("parse should complete without fatal error: {}", err),
    }

    let errors = parser.errors();
    assert!(
        !errors_contain(errors, "Heredoc parsing timed out"),
        "unexpected heredoc timeout: {errors:?}"
    );
}

#[test]
fn test_excessive_pending_heredocs() {
    // Test that we limit the number of pending heredocs (recursion depth limit)
    // 110 heredocs on one line (limit is 100)
    let mut code = String::from("print ");
    let heredocs: Vec<&str> = (0..110).map(|_| "<<EOF").collect();
    code.push_str(&heredocs.join(", "));
    code.push_str(";\n");
    // We don't even need to provide bodies if the declaration parsing fails early
    // But let's provide them to be valid if it were allowed
    for _ in 0..110 {
        code.push_str("content\nEOF\n");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    if let Err(err) = result {
        assert!(
            err.to_string().contains("Heredoc depth limit exceeded"),
            "Unexpected error: {}",
            err
        );
        return;
    }

    let errors = parser.errors();
    assert!(
        errors_contain(errors, "Heredoc depth limit exceeded"),
        "Expected heredoc depth limit error, got: {errors:?}"
    );
}

#[test]
fn test_heredoc_parsing_timeout() {
    // To test timeout, we need a very long heredoc declaration process or infinite loop
    // Simulating time passage is hard in unit tests without mocking Instant.
    let code = "my $x = <<EOF;\ncontent\nEOF";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "expected parse to succeed for simple heredoc");

    let errors = parser.errors();
    assert!(
        !errors_contain(errors, "Heredoc parsing timed out"),
        "unexpected heredoc timeout: {errors:?}"
    );
}

#[test]
fn test_multiple_heredocs_same_line() {
    // Test multiple heredocs on the same line complete without hanging
    let code = r#"my ($a, $b, $c) = (<<EOF1, <<EOF2, <<EOF3);
Content for first heredoc
EOF1
Content for second heredoc
EOF2
Content for third heredoc
EOF3
"#;

    let start = Instant::now();
    let mut parser = Parser::new(code);
    let result = parser.parse();
    let duration = start.elapsed();

    // Should complete quickly
    assert!(duration.as_secs() < 2, "Parser should handle multiple heredocs efficiently");

    // Should succeed or have non-timeout errors
    match result {
        Ok(_) => {}
        Err(err) => {
            assert!(!err.to_string().contains("timeout"), "Should not timeout: {}", err);
        }
    }

    let errors = parser.errors();
    assert!(
        !errors_contain(errors, "Heredoc parsing timed out"),
        "unexpected heredoc timeout: {errors:?}"
    );
}

#[test]
fn test_nested_heredocs_within_limit() {
    // Test that nesting within the limit (100) works without errors
    let mut code = String::from("my @h = (");
    for i in 0..50 {
        code.push_str(&format!("<<EOF{}, ", i));
    }
    code.push_str(");\n");

    // Add bodies for all heredocs
    for i in 0..50 {
        code.push_str(&format!("Content for heredoc {}\n", i));
        code.push_str(&format!("EOF{}\n", i));
    }

    let start = Instant::now();
    let mut parser = Parser::new(&code);
    let _result = parser.parse();
    let duration = start.elapsed();

    // Should complete quickly
    assert!(duration.as_secs() < 2, "Parser should handle 50 heredocs efficiently");

    // Should not have depth limit errors for 50 heredocs (limit is 100)
    let errors = parser.errors();
    assert!(
        !errors_contain(errors, "Heredoc depth limit exceeded"),
        "Should not have depth errors within limit: {errors:?}"
    );
}

#[test]
fn test_deeply_nested_heredocs_hit_limit() {
    // Test that exceeding 100 heredocs triggers the depth limit
    let mut code = String::from("my @h = (");
    for i in 0..150 {
        code.push_str(&format!("<<EOF{}, ", i));
    }
    code.push_str(");\n");

    let mut parser = Parser::new(&code);
    let _result = parser.parse();

    let errors = parser.errors();
    assert!(
        errors_contain(errors, "Heredoc depth limit exceeded"),
        "Expected depth limit error for 150 heredocs: {errors:?}"
    );
}
