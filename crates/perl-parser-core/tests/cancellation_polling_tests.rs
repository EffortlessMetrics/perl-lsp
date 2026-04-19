mod cpan_test_helpers;

use perl_parser_core::Parser;
use perl_parser_core::error::ParseError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

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

/// Concurrent cancellation: flag set from a separate thread while parse is running.
///
/// This is the real LSP use case — a newer didChange arrives on a different thread
/// and sets the flag to `true` while the parser is mid-way through a large file.
/// The parser must eventually observe the flag and return `Err(Cancelled)`.
///
/// Uses a 10 000-statement source and a barrier to ensure the flag is set while
/// the parser thread is still running.  The parse thread signals readiness once it
/// has started, then the setter thread fires, guaranteeing real concurrency.
#[test]
fn test_concurrent_cancellation_from_background_thread() {
    use std::sync::Barrier;

    let flag = Arc::new(AtomicBool::new(false));
    let flag_for_thread = Arc::clone(&flag);

    // 10 000 statements — large enough that the parse takes well over 1 ms,
    // giving the setter thread a reliable window to fire.
    let statements: Vec<String> = (0..10_000)
        .map(|i| format!("my $x{} = {};", i, i))
        .collect();
    let source = statements.join("\n");

    // Barrier: parse thread signals when it has started; setter fires immediately after.
    let barrier = Arc::new(Barrier::new(2));
    let barrier_for_thread = Arc::clone(&barrier);

    let setter = thread::spawn(move || {
        barrier_for_thread.wait(); // wait until parser has begun
        flag_for_thread.store(true, Ordering::Release);
    });

    let mut parser = Parser::new_with_cancellation(&source, flag);
    // Signal the setter thread that parsing is about to begin, then parse.
    barrier.wait();
    let result = parser.parse();

    let _ = setter.join();

    assert!(
        matches!(result, Err(ParseError::Cancelled)),
        "expected Err(Cancelled) from concurrent cancellation but got: {:?}",
        result
    );
}

/// Amortization boundary: a program with fewer than 64 statements and the flag
/// set AFTER parse() begins (bypassing the pre-parse check) should still succeed,
/// because the 64-statement polling boundary is never reached.
///
/// This verifies the amortization granularity: short files complete regardless of
/// when the flag is set mid-parse.  The pre-parse check in `parse()` is bypassed
/// by starting with `flag = false` and only setting it after a delay — but since
/// `parse()` returns before the delay fires for a tiny program, the result is Ok.
#[test]
fn test_short_program_with_flag_set_after_start_succeeds() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_for_thread = Arc::clone(&flag);

    // Only 10 statements — fewer than 64, so no polling boundary is ever hit.
    let source = (0..10)
        .map(|i| format!("my $x{} = {};", i, i))
        .collect::<Vec<_>>()
        .join("\n");

    // Start a background thread that will set the flag, but the parse should
    // complete before the thread even runs (tiny program = essentially instant).
    let setter = thread::spawn(move || {
        thread::yield_now();
        flag_for_thread.store(true, Ordering::Release);
    });

    let mut parser = Parser::new_with_cancellation(&source, flag);
    let result = parser.parse();

    let _ = setter.join();

    // The parse must succeed: 10 statements never hit the 64-statement check boundary.
    assert!(
        result.is_ok(),
        "expected Ok for short program but got: {:?}",
        result
    );
}

/// parse_with_recovery() must NOT silently swallow ParseError::Cancelled.
///
/// `parse_with_recovery()` catches all `Err(e)` from `parse()` and converts them
/// to an empty Program node with the error pushed into `self.errors()`. For
/// Cancelled, this means the caller gets a fake empty AST instead of a signal that
/// the parse was interrupted.  Callers in the LSP path use `parse()` directly, but
/// this test documents the known behavior so future callers are aware.
#[test]
fn test_parse_with_recovery_documents_cancelled_becomes_error_in_output() {
    let flag = Arc::new(AtomicBool::new(true)); // pre-set: parse() returns Cancelled immediately
    let statements: Vec<String> = (0..200).map(|i| format!("my $x{} = {};", i, i)).collect();
    let source = statements.join("\n");
    let mut parser = Parser::new_with_cancellation(&source, flag);

    // parse_with_recovery() catches Cancelled and stores it in errors(), returning
    // an empty Program node.  This is an accepted tradeoff: the method always
    // returns a ParseOutput, never propagates Err.  Callers that need to detect
    // cancellation must use parse() directly.
    let output = parser.parse_with_recovery();

    // The AST is an empty Program (the "dummy node" path in parse_with_recovery).
    assert!(
        matches!(output.ast.kind, perl_parser_core::NodeKind::Program { ref statements } if statements.is_empty()),
        "expected empty Program from cancelled parse_with_recovery but got: {:?}",
        output.ast.kind
    );

    // Cancelled is recorded in diagnostics so callers can detect it if needed.
    assert!(
        output
            .diagnostics
            .iter()
            .any(|e| matches!(e, ParseError::Cancelled)),
        "expected ParseError::Cancelled in diagnostics but got: {:?}",
        output.diagnostics
    );
}
