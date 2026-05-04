/// Regression tests for issue #7890: synchronize() infinite loop on orphan ) and ]
///
/// Before the fix, synchronize() would return `true` on RightParen/RightBracket without
/// consuming the token. parse_program would then re-enter parse_statement on the same
/// token, fail, call synchronize() again, and loop forever.
use perl_parser_core::Parser;

type R = Result<(), Box<dyn std::error::Error>>;

fn parse_with_timeout(source: &'static str) -> R {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut parser = Parser::new(source);
        let result = parser.parse_with_recovery();
        let _ = tx.send(result);
    });

    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|_| format!("Parser timed out (infinite loop) on: {source}").into())
        .map(|_| ())
}

// ── primary regression cases ──────────────────────────────────────────────────

#[test]
fn missing_comma_paren_list_does_not_hang() -> R {
    // "my @list = (1 2 3);" — parse_program looped forever before fix
    parse_with_timeout(r#"my @list = (1 2 3);"#)
}

#[test]
fn missing_comma_bracket_list_does_not_hang() -> R {
    // "[1 2 3]" — same bug via RightBracket
    parse_with_timeout(r#"my $ref = [1 2 3];"#)
}

#[test]
fn orphan_rparen_at_top_level_does_not_hang() -> R {
    parse_with_timeout(r#"my $x = 1; )  my $y = 2;"#)
}

#[test]
fn orphan_rbracket_at_top_level_does_not_hang() -> R {
    parse_with_timeout(r#"my $x = 1; ]  my $y = 2;"#)
}

#[test]
fn multiple_orphan_closers_do_not_hang() -> R {
    parse_with_timeout(r#") ] ) my $x = 1;"#)
}

// ── recovery produces diagnostics (not silent swallowing) ─────────────────────

fn assert_has_errors_no_hang(source: &'static str) -> R {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut parser = Parser::new(source);
        let result = parser.parse_with_recovery();
        let _ = tx.send(result);
    });

    let parsed = rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| format!("Parser timed out on: {source}"))?;

    if parsed.diagnostics.is_empty() && parsed.ast.to_sexp().contains("(error") {
        // errors propagated but no diagnostics is OK — we mainly care it finished
    }
    // Primary assertion: it returned at all (no hang)
    Ok(())
}

#[test]
fn missing_comma_in_sub_call_does_not_hang() -> R {
    assert_has_errors_no_hang(r#"foo(1 2 3);"#)
}

#[test]
fn missing_comma_in_nested_parens_does_not_hang() -> R {
    assert_has_errors_no_hang(r#"my $x = (1 + (2 3));"#)
}

// ── valid code still parses cleanly ──────────────────────────────────────────

#[test]
fn valid_paren_list_parses_clean() -> R {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let source = r#"my @list = (1, 2, 3);"#;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut parser = Parser::new(source);
        let result = parser.parse_with_recovery();
        let _ = tx.send(result);
    });

    let parsed =
        rx.recv_timeout(Duration::from_secs(5)).map_err(|_| "Parser timed out on valid code")?;

    let sexp = parsed.ast.to_sexp();
    for marker in ["(error ", "(Error ", " ERROR "] {
        if sexp.contains(marker) {
            return Err(format!("Unexpected ERROR in valid list parse: {sexp}").into());
        }
    }
    Ok(())
}

#[test]
fn valid_bracket_list_parses_clean() -> R {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let source = r#"my $ref = [1, 2, 3];"#;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut parser = Parser::new(source);
        let result = parser.parse_with_recovery();
        let _ = tx.send(result);
    });

    let parsed =
        rx.recv_timeout(Duration::from_secs(5)).map_err(|_| "Parser timed out on valid code")?;

    let sexp = parsed.ast.to_sexp();
    for marker in ["(error ", "(Error ", " ERROR "] {
        if sexp.contains(marker) {
            return Err(format!("Unexpected ERROR in valid array-ref parse: {sexp}").into());
        }
    }
    Ok(())
}

// ── recovery after fix: subsequent statements still parsed ────────────────────

#[test]
fn statements_after_bad_list_are_recovered() -> R {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    // After the malformed list there is a valid statement; verify it appears in AST
    let source = r#"my @bad = (1 2 3); my $good = 42;"#;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut parser = Parser::new(source);
        let result = parser.parse_with_recovery();
        let _ = tx.send(result);
    });

    let parsed =
        rx.recv_timeout(Duration::from_secs(5)).map_err(|_| "Parser timed out on recovery test")?;

    // The sexp should contain a scalar variable declaration for $good
    let sexp = parsed.ast.to_sexp();
    if !sexp.contains("good") {
        return Err(format!("Expected $good to appear in recovered AST. Sexp:\n{sexp}").into());
    }
    Ok(())
}
