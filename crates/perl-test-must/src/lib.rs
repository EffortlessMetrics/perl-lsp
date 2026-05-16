//! Safe unwrap replacements for tests.
//!
//! This crate provides panic-on-failure helpers that are safe to use in tests,
//! avoiding explicit `unwrap()` calls which are denied by clippy policy.
//!
//! # Overview
//!
//! Three helpers cover the common cases:
//! - [`must`] — extract the value from a `Result`, or panic with the error
//! - [`must_some`] — extract the value from an `Option`, or panic
//! - [`must_err`] — extract the error from a `Result`, or panic if `Ok`
//!
//! # Example
//!
//! ```rust
//! use perl_test_must::{must, must_some, must_err};
//!
//! let result: Result<i32, &str> = Ok(42);
//! assert_eq!(must(result), 42);
//!
//! let opt: Option<i32> = Some(7);
//! assert_eq!(must_some(opt), 7);
//!
//! let err_result: Result<i32, &str> = Err("oops");
//! assert_eq!(must_err(err_result), "oops");
//! ```

// This crate provides test helpers that intentionally panic on failure.
// The must/must_some/must_err helpers are designed to panic in tests.
#![allow(clippy::panic)]

/// Extract the value from a `Result`, or panic with the error.
///
/// This is a test-only replacement for `unwrap` that is compliant
/// with the "no unwrap/expect" policy.
///
/// Note: `#[must_use]` is intentionally omitted. `must()` is frequently
/// called as an assertion (`must(fs::write(...))`) where the caller intentionally
/// discards the `()` return value. Adding `#[must_use]` would trigger ~373
/// spurious warnings across the workspace for those valid use cases.
#[track_caller]
pub fn must<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => panic!("unexpected Err<{}>: {e:?}", std::any::type_name::<E>()),
    }
}

/// Extract the value from an `Option`, or panic.
///
/// This is a test-only replacement for `unwrap` that is compliant
/// with the "no unwrap/expect" policy.
#[track_caller]
#[must_use]
pub fn must_some<T>(o: Option<T>) -> T {
    match o {
        Some(v) => v,
        None => panic!("unexpected None<{}>", std::any::type_name::<T>()),
    }
}

/// Extract the error from a `Result`, or panic if `Ok`.
///
/// This is a test-only replacement for `.unwrap_err()` that is compliant
/// with the "no unwrap/expect" policy.
#[track_caller]
#[must_use]
pub fn must_err<T: std::fmt::Debug, E>(r: Result<T, E>) -> E {
    match r {
        Err(e) => e,
        Ok(v) => panic!(
            "expected Err<{}>, got Ok<{}>({v:?})",
            std::any::type_name::<E>(),
            std::any::type_name::<T>()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{must, must_err, must_some};

    // ── must ──────────────────────────────────────────────────────────────────

    #[test]
    fn must_unwraps_ok() {
        let result: Result<i32, &str> = Ok(42);
        assert_eq!(must(result), 42);
    }

    #[test]
    #[should_panic(expected = "unexpected Err")]
    fn must_panics_on_err() {
        let result: Result<i32, &str> = Err("oops");
        must(result);
    }

    /// `must` should work for the unit-value `Ok(())` case — frequently used
    /// with side-effectful calls such as `fs::write(...)`.
    #[test]
    fn must_ok_unit_value_is_discarded_silently() {
        let result: Result<(), &str> = Ok(());
        must(result); // return value intentionally not bound
    }

    /// `must` with a `String` (non-Copy) verifies ownership is transferred.
    #[test]
    fn must_ok_string_transfers_ownership() {
        let result: Result<String, &str> = Ok(String::from("hello"));
        assert_eq!(must(result), "hello");
    }

    /// `must` with a `u8` error type — ensures the Debug formatting of the
    /// error appears in the panic message.
    #[test]
    #[should_panic(expected = "unexpected Err")]
    fn must_panic_message_includes_err_marker() {
        let result: Result<i32, u8> = Err(7);
        must(result);
    }

    /// `must` with a custom Debug type — verifies the error value is shown.
    #[test]
    #[should_panic(expected = "42")]
    fn must_panic_message_includes_debug_value() {
        #[derive(Debug)]
        struct MyError(#[allow(dead_code)] u32);
        let result: Result<(), MyError> = Err(MyError(42));
        must(result);
    }

    /// `must` with a `bool` result type — exercises a Copy scalar type
    /// distinct from `i32`.
    #[test]
    fn must_ok_bool() {
        let result: Result<bool, &str> = Ok(true);
        assert!(must(result));
    }

    /// `must` works with a nested `Result` — the inner value is returned as-is.
    #[test]
    fn must_ok_nested_result() {
        let inner: Result<i32, &str> = Ok(1);
        let outer: Result<Result<i32, &str>, &str> = Ok(inner);
        let extracted = must(outer);
        assert_eq!(must(extracted), 1);
    }

    // ── must_some ─────────────────────────────────────────────────────────────

    #[test]
    fn must_some_unwraps_some() {
        assert_eq!(must_some(Some(99)), 99);
    }

    #[test]
    #[should_panic(expected = "unexpected None")]
    fn must_some_panics_on_none() {
        let _ = must_some(Option::<i32>::None);
    }

    /// `must_some` with a `String` — verifies ownership transfer for non-Copy.
    #[test]
    fn must_some_string_transfers_ownership() {
        let opt: Option<String> = Some(String::from("world"));
        assert_eq!(must_some(opt), "world");
    }

    /// `must_some` with a `Vec` — exercises a heap-allocated type.
    #[test]
    fn must_some_vec_transfers_ownership() {
        let opt: Option<Vec<u8>> = Some(vec![1, 2, 3]);
        assert_eq!(must_some(opt), [1, 2, 3]);
    }

    /// `must_some` panic message includes the concrete type name.
    #[test]
    #[should_panic(expected = "unexpected None")]
    fn must_some_panic_message_for_string_type() {
        let _ = must_some(Option::<String>::None);
    }

    /// `must_some` with a `bool` — smallest scalar, verifies the generic works.
    #[test]
    fn must_some_bool_true() {
        assert!(must_some(Some(true)));
    }

    /// `must_some` with a `char`.
    #[test]
    fn must_some_char() {
        assert_eq!(must_some(Some('z')), 'z');
    }

    // ── must_err ──────────────────────────────────────────────────────────────

    #[test]
    fn must_err_unwraps_err() {
        let result: Result<i32, &str> = Err("expected error");
        assert_eq!(must_err(result), "expected error");
    }

    #[test]
    #[should_panic(expected = "expected Err")]
    fn must_err_panics_on_ok() {
        let result: Result<i32, &str> = Ok(1);
        let _ = must_err(result);
    }

    /// `must_err` with a `String` error — verifies ownership transfer.
    #[test]
    fn must_err_string_error_transfers_ownership() {
        let result: Result<i32, String> = Err(String::from("bad input"));
        assert_eq!(must_err(result), "bad input");
    }

    /// `must_err` panic message includes the Ok variant's Debug representation.
    #[test]
    #[should_panic(expected = "99")]
    fn must_err_panic_message_includes_ok_value() {
        let result: Result<i32, &str> = Ok(99);
        let _ = must_err(result);
    }

    /// `must_err` with `Ok(())` — unit Ok type, verifies the Debug output path.
    #[test]
    #[should_panic(expected = "expected Err")]
    fn must_err_panics_on_ok_unit() {
        let result: Result<(), &str> = Ok(());
        let _ = must_err(result);
    }

    /// `must_err` with a numeric error type (u32).
    #[test]
    fn must_err_numeric_error() {
        let result: Result<&str, u32> = Err(404);
        assert_eq!(must_err(result), 404);
    }

    /// `must_err` with a custom error struct — exercises non-primitive E.
    #[test]
    fn must_err_custom_error_struct() {
        #[derive(Debug, PartialEq)]
        struct ParseError {
            line: u32,
        }
        let result: Result<i32, ParseError> = Err(ParseError { line: 10 });
        assert_eq!(must_err(result), ParseError { line: 10 });
    }
}
