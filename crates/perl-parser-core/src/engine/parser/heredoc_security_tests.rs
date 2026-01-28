use super::*;

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

    let mut parser = Parser::new(code);
    let result = parser.parse();

    match result {
        Ok(_) => {}
        Err(err) => panic!("parse should complete without fatal error: {}", err),
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
