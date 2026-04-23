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
#[track_caller]
#[must_use]
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

    #[test]
    fn must_unwraps_ok() {
        let result: Result<i32, &str> = Ok(42);
        assert_eq!(must(result), 42);
    }

    #[test]
    #[should_panic(expected = "unexpected Err")]
    fn must_panics_on_err() {
        let result: Result<i32, &str> = Err("oops");
        let _ = must(result);
    }

    #[test]
    fn must_some_unwraps_some() {
        assert_eq!(must_some(Some(99)), 99);
    }

    #[test]
    #[should_panic(expected = "unexpected None")]
    fn must_some_panics_on_none() {
        let _ = must_some(Option::<i32>::None);
    }

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
}
