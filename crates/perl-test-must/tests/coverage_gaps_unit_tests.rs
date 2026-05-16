//! Coverage-gap unit tests for the three test helpers in `perl-test-must`.
//!
//! The internal `#[cfg(test)]` module covers only the basic happy/panic paths
//! for a single type each.  These integration-style tests in `tests/` cover:
//!
//! * `must`       — multiple value types, panic message content
//! * `must_some`  — multiple value types, panic message content
//! * `must_err`   — multiple error types, panic message content
//!
//! Panic-path tests use `std::panic::catch_unwind`.  All helpers are
//! `#[track_caller]`, so the panic message is produced by the helper
//! itself and we can assert on the human-readable format.

use perl_test_must::{must, must_err, must_some};

// ---------------------------------------------------------------------------
// Helper: downcast a catch_unwind payload to &str
// ---------------------------------------------------------------------------

fn panic_msg(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        String::from("<non-string panic payload>")
    }
}

// ---------------------------------------------------------------------------
// must — happy paths
// ---------------------------------------------------------------------------

#[test]
fn must_returns_i32() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<i32, &str> = Ok(42);
    assert_eq!(must(r), 42);
    Ok(())
}

#[test]
fn must_returns_string() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<String, &str> = Ok(String::from("hello"));
    assert_eq!(must(r), "hello");
    Ok(())
}

#[test]
fn must_returns_vec_u8() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<Vec<u8>, &str> = Ok(vec![1, 2, 3]);
    assert_eq!(must(r), vec![1u8, 2, 3]);
    Ok(())
}

#[test]
fn must_returns_unit() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<(), &str> = Ok(());
    must(r);
    Ok(())
}

#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn must_returns_custom_struct() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<Point, &str> = Ok(Point { x: 3, y: 4 });
    assert_eq!(must(r), Point { x: 3, y: 4 });
    Ok(())
}

// ---------------------------------------------------------------------------
// must — panic paths
// ---------------------------------------------------------------------------

#[test]
#[allow(clippy::unwrap_used)]
fn must_panics_on_err_with_type_name_in_message() {
    // Use a named unit struct so the type name appears in the panic message.
    // The struct name acts as both type-name evidence and debug representation.
    #[derive(Debug)]
    struct MyDistinctError;

    let r: Result<i32, MyDistinctError> = Err(MyDistinctError);
    let result = std::panic::catch_unwind(|| must(r));
    let msg = panic_msg(result.unwrap_err());
    // Panic format: "unexpected Err<{type_name}>: {e:?}"
    assert!(msg.contains("MyDistinctError"), "panic message should contain type name, got: {msg}");
    assert!(
        msg.contains("unexpected Err"),
        "panic message should start with 'unexpected Err', got: {msg}"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn must_panics_on_err_str() {
    let r: Result<i32, &str> = Err("something went wrong");
    let result = std::panic::catch_unwind(|| must(r));
    let msg = panic_msg(result.unwrap_err());
    assert!(
        msg.contains("something went wrong"),
        "panic message should contain Debug of the &str error, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// must_some — happy paths
// ---------------------------------------------------------------------------

#[test]
fn must_some_returns_i32() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(must_some(Some(7_i32)), 7);
    Ok(())
}

#[test]
fn must_some_returns_string() -> Result<(), Box<dyn std::error::Error>> {
    let s = String::from("world");
    assert_eq!(must_some(Some(s)), "world");
    Ok(())
}

#[test]
fn must_some_returns_vec() -> Result<(), Box<dyn std::error::Error>> {
    let v: Option<Vec<u8>> = Some(vec![10, 20]);
    assert_eq!(must_some(v), vec![10u8, 20]);
    Ok(())
}

#[test]
fn must_some_returns_custom_struct() -> Result<(), Box<dyn std::error::Error>> {
    let p: Option<Point> = Some(Point { x: 1, y: 2 });
    assert_eq!(must_some(p), Point { x: 1, y: 2 });
    Ok(())
}

// ---------------------------------------------------------------------------
// must_some — panic paths
// ---------------------------------------------------------------------------

#[test]
#[allow(clippy::unwrap_used)]
fn must_some_panics_on_none_with_type_name() {
    let result = std::panic::catch_unwind(|| must_some(Option::<String>::None));
    let msg = panic_msg(result.unwrap_err());
    // Panic format: "unexpected None<{type_name}>"
    assert!(
        msg.contains("unexpected None"),
        "panic message should say 'unexpected None', got: {msg}"
    );
    assert!(
        msg.contains("String") || msg.contains("alloc::string::String"),
        "panic message should contain the type name, got: {msg}"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn must_some_panics_on_none_u32() {
    let result = std::panic::catch_unwind(|| must_some(Option::<u32>::None));
    let msg = panic_msg(result.unwrap_err());
    assert!(msg.contains("u32"), "panic message should contain 'u32', got: {msg}");
}

// ---------------------------------------------------------------------------
// must_err — happy paths
// ---------------------------------------------------------------------------

#[test]
fn must_err_returns_str() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<i32, &str> = Err("expected error");
    assert_eq!(must_err(r), "expected error");
    Ok(())
}

#[test]
fn must_err_returns_string() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<i32, String> = Err(String::from("error string"));
    assert_eq!(must_err(r), "error string");
    Ok(())
}

#[test]
fn must_err_returns_i32_error() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<bool, i32> = Err(-1);
    assert_eq!(must_err(r), -1_i32);
    Ok(())
}

#[derive(Debug, PartialEq)]
struct AppError {
    code: u32,
}

#[test]
fn must_err_returns_custom_error_struct() -> Result<(), Box<dyn std::error::Error>> {
    let r: Result<i32, AppError> = Err(AppError { code: 404 });
    assert_eq!(must_err(r).code, 404);
    Ok(())
}

// ---------------------------------------------------------------------------
// must_err — panic paths
// ---------------------------------------------------------------------------

#[test]
#[allow(clippy::unwrap_used)]
fn must_err_panics_on_ok_with_type_names() {
    let r: Result<i32, &str> = Ok(99);
    let result = std::panic::catch_unwind(|| must_err(r));
    let msg = panic_msg(result.unwrap_err());
    // Panic format: "expected Err<{E}>, got Ok<{T}>({v:?})"
    assert!(msg.contains("expected Err"), "panic message should say 'expected Err', got: {msg}");
    assert!(msg.contains("i32"), "panic message should contain Ok type name 'i32', got: {msg}");
    assert!(msg.contains("99"), "panic message should contain the Ok value '99', got: {msg}");
}

#[test]
#[allow(clippy::unwrap_used)]
fn must_err_panics_on_ok_string_value() {
    let r: Result<String, i32> = Ok(String::from("oops"));
    let result = std::panic::catch_unwind(|| must_err(r));
    let msg = panic_msg(result.unwrap_err());
    assert!(msg.contains("oops"), "panic message should contain the Ok value, got: {msg}");
}

// ---------------------------------------------------------------------------
// Generic Send + 'static invariant smoke tests
// ---------------------------------------------------------------------------

/// Verify helpers work across a thread boundary, confirming Send + 'static.
#[test]
fn must_is_usable_from_thread() -> Result<(), Box<dyn std::error::Error>> {
    let handle = std::thread::spawn(|| {
        let r: Result<i32, &str> = Ok(5);
        must(r)
    });
    let val = handle.join().map_err(|_| "thread panicked")?;
    assert_eq!(val, 5);
    Ok(())
}

#[test]
fn must_some_is_usable_from_thread() -> Result<(), Box<dyn std::error::Error>> {
    let handle = std::thread::spawn(|| must_some(Some(true)));
    let val = handle.join().map_err(|_| "thread panicked")?;
    assert!(val);
    Ok(())
}

#[test]
fn must_err_is_usable_from_thread() -> Result<(), Box<dyn std::error::Error>> {
    let handle = std::thread::spawn(|| {
        let r: Result<i32, &str> = Err("threaded error");
        must_err(r)
    });
    let val = handle.join().map_err(|_| "thread panicked")?;
    assert_eq!(val, "threaded error");
    Ok(())
}
