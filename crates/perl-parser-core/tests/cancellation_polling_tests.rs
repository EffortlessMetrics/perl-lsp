mod cpan_test_helpers;

use perl_parser_core::Parser;
use perl_parser_core::error::ParseError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// After the fix: parser polls the cancellation flag and returns `Err(Cancelled)`
/// when the flag is already set before parsing begins.
#[test]
fn test_parse_with_flag_pre_set_returns_cancelled() {
    let flag = Arc::new(AtomicBool::new(true));
    let statements: Vec<String> = (0..200).map(|i| format!("my $x{} = {};", i, i)).collect();
    let source = statements.join("\n");
    let mut parser = Parser::new_with_cancellation(&source, Arc::clone(&flag));
    let result = parser.parse();
    assert!(
        matches!(result, Err(ParseError::Cancelled)),
        "expected Err(Cancelled) but got: {:?}",
        result
    );
}

/// After the fix: parser polls the cancellation flag and returns `Err(Cancelled)`
/// when the flag is set before parsing the block body.
#[test]
fn test_cancellation_flag_in_nested_blocks_returns_cancelled() {
    let flag = Arc::new(AtomicBool::new(true));
    let mut source = String::from("{\n");
    for i in 0..200 {
        source.push_str(&format!("  my $x{} = {};\n", i, i));
    }
    source.push('}');
    let mut parser = Parser::new_with_cancellation(&source, Arc::clone(&flag));
    let result = parser.parse();
    assert!(
        matches!(result, Err(ParseError::Cancelled)),
        "expected Err(Cancelled) but got: {:?}",
        result
    );
}

/// After the fix: parser polls the cancellation flag set before parsing starts
/// and returns `Err(Cancelled)` rather than a successful parse.
#[test]
fn test_parse_with_delayed_cancellation_flag_returns_cancelled() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&flag);
    let statements: Vec<String> = (0..200).map(|i| format!("my $x{} = {};", i, i)).collect();
    let source = statements.join("\n");
    // Set the flag before calling parse()
    flag_clone.store(true, Ordering::Release);
    let mut parser = Parser::new_with_cancellation(&source, flag);
    let result = parser.parse();
    assert!(
        matches!(result, Err(ParseError::Cancelled)),
        "expected Err(Cancelled) but got: {:?}",
        result
    );
}

/// Sanity check: parser with cancellation available but flag not set still succeeds.
#[test]
fn test_parse_with_cancellation_available_but_not_cancelled_succeeds() {
    let flag = Arc::new(AtomicBool::new(false));
    let mut parser = Parser::new_with_cancellation("my $x = 1; my $y = 2;", flag);
    let result = parser.parse();
    assert!(result.is_ok(), "expected Ok(...) but got: {:?}", result);
}
