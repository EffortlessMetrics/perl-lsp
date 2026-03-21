mod cpan_test_helpers;

use perl_parser_core::Parser;
use perl_parser_core::error::ParseError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_parse_without_cancellation_succeeds() {
    let flag = Arc::new(AtomicBool::new(false));
    let mut parser = Parser::new_with_cancellation("my $x = 1; my $y = 2;", flag);
    let result = parser.parse();
    assert!(result.is_ok());
}

#[test]
fn test_parse_with_cancellation_flag_set_returns_cancelled() {
    // Parser now polls the cancellation flag — pre-set flag returns Cancelled.
    let flag = Arc::new(AtomicBool::new(true));
    let statements: Vec<String> = (0..200).map(|i| format!("my $x{} = {};", i, i)).collect();
    let source = statements.join("\n");
    let mut parser = Parser::new_with_cancellation(&source, flag);
    let result = parser.parse();
    assert!(matches!(result, Err(ParseError::Cancelled)));
}

#[test]
fn test_parse_with_delayed_cancellation_flag_returns_cancelled() {
    // Parser polls the flag — flag set before parse() returns Cancelled.
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&flag);
    let statements: Vec<String> = (0..200).map(|i| format!("my $x{} = {};", i, i)).collect();
    let source = statements.join("\n");
    flag_clone.store(true, Ordering::Release);
    let mut parser = Parser::new_with_cancellation(&source, flag);
    let result = parser.parse();
    assert!(matches!(result, Err(ParseError::Cancelled)));
}

#[test]
fn test_parse_without_cancellation_flag_works() {
    let mut parser = Parser::new("my $x = 1; my $y = 2;");
    let result = parser.parse();
    assert!(result.is_ok());
}

#[test]
fn test_cancelled_error_display() {
    let err = ParseError::Cancelled;
    assert_eq!(err.to_string(), "Parsing cancelled");
}

#[test]
fn test_cancellation_flag_in_nested_blocks_returns_cancelled() {
    // Parser polls the flag inside block loops — pre-set flag returns Cancelled.
    let flag = Arc::new(AtomicBool::new(true));
    let mut source = String::from("{\n");
    for i in 0..200 {
        source.push_str(&format!("  my $x{} = {};\n", i, i));
    }
    source.push('}');
    let mut parser = Parser::new_with_cancellation(&source, flag);
    let result = parser.parse();
    assert!(matches!(result, Err(ParseError::Cancelled)));
}
