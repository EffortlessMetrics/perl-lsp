//! Panic diagnostic contract tests for `perl-test-must` helpers.
//!
//! These helpers intentionally panic when a test precondition fails. The panic
//! text is part of their usefulness: it should identify the helper path, the
//! involved type, and enough debug detail to diagnose the failing value.

use std::any::Any;
use std::panic::catch_unwind;

use perl_test_must::{must, must_err, must_some};

#[test]
fn must_err_panic_names_error_type_and_value() -> Result<(), String> {
    let payload = must_err(catch_unwind(|| must::<i32, &str>(Err("boom"))));
    let message = must_some(panic_message(payload.as_ref()));

    assert!(message.contains("unexpected Err<&str>"), "message was: {message}");
    assert!(message.contains("\"boom\""), "message was: {message}");
    Ok(())
}

#[test]
fn must_some_panic_names_option_payload_type() -> Result<(), String> {
    let payload = must_err(catch_unwind(|| must_some::<String>(None)));
    let message = must_some(panic_message(payload.as_ref()));

    assert!(message.contains("unexpected None<alloc::string::String>"), "message was: {message}");
    Ok(())
}

#[test]
fn must_err_on_ok_panic_names_error_and_ok_types_and_value() -> Result<(), String> {
    let payload = must_err(catch_unwind(|| must_err::<i32, &str>(Ok(7))));
    let message = must_some(panic_message(payload.as_ref()));

    assert!(message.contains("expected Err, got Ok"), "message was: {message}");
    assert!(message.contains("expected Err<&str>, got Ok<i32>(7)"), "message was: {message}");
    Ok(())
}

fn panic_message(payload: &(dyn Any + Send)) -> Option<&str> {
    if let Some(message) = payload.downcast_ref::<String>() {
        return Some(message.as_str());
    }

    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return Some(*message);
    }

    None
}
