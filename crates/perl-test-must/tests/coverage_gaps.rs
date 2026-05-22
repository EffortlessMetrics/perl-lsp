//! Coverage tests for the public `perl-test-must` helpers.
//!
//! These tests lock generic type coverage, ownership transfer, and boundary
//! values without changing the helper panic contract.

use perl_test_must::{must, must_err, must_some};

#[test]
fn must_accepts_unit_ok_for_side_effect_calls() -> Result<(), String> {
    let result: Result<(), &str> = Ok(());
    must(result);
    Ok(())
}

#[test]
fn must_transfers_string_ownership() -> Result<(), String> {
    let result: Result<String, &str> = Ok(String::from("hello"));
    assert_eq!(must(result), "hello");
    Ok(())
}

#[test]
fn must_transfers_vec_ownership() -> Result<(), String> {
    let result: Result<Vec<u32>, &str> = Ok(vec![10, 20, 30]);
    assert_eq!(must(result), [10, 20, 30]);
    Ok(())
}

#[test]
fn must_accepts_bool() -> Result<(), String> {
    let result: Result<bool, &str> = Ok(true);
    assert!(must(result));
    Ok(())
}

#[test]
fn must_accepts_zero_and_negative_boundary_values() -> Result<(), String> {
    let zero: Result<i32, &str> = Ok(0);
    let negative: Result<i32, &str> = Ok(-1);

    assert_eq!(must(zero), 0);
    assert_eq!(must(negative), -1);
    Ok(())
}

#[test]
fn must_accepts_tuple_values() -> Result<(), String> {
    let result: Result<(i32, &str), &str> = Ok((3, "hello"));
    assert_eq!(must(result), (3, "hello"));
    Ok(())
}

#[test]
fn must_accepts_enum_values() -> Result<(), String> {
    #[derive(Debug, PartialEq)]
    #[allow(dead_code)]
    enum Color {
        Red,
        Green,
        Blue,
    }

    let result: Result<Color, &str> = Ok(Color::Green);
    assert_eq!(must(result), Color::Green);
    Ok(())
}

#[test]
fn must_returns_nested_result_as_value() -> Result<(), String> {
    let inner: Result<i32, &str> = Ok(1);
    let outer: Result<Result<i32, &str>, &str> = Ok(inner);

    assert_eq!(must(must(outer)), 1);
    Ok(())
}

#[test]
fn must_can_chain_nested_result_to_must_err() -> Result<(), String> {
    let inner: Result<i32, &str> = Err("inner fail");
    let outer: Result<Result<i32, &str>, &str> = Ok(inner);

    assert_eq!(must_err(must(outer)), "inner fail");
    Ok(())
}

#[test]
fn must_some_transfers_string_ownership() -> Result<(), String> {
    let opt = Some(String::from("world"));
    assert_eq!(must_some(opt), "world");
    Ok(())
}

#[test]
fn must_some_transfers_vec_ownership() -> Result<(), String> {
    let opt = Some(vec![1_u8, 2, 3]);
    assert_eq!(must_some(opt), [1, 2, 3]);
    Ok(())
}

#[test]
fn must_some_accepts_scalar_values() -> Result<(), String> {
    assert!(must_some(Some(true)));
    assert_eq!(must_some(Some('z')), 'z');
    assert_eq!(must_some(Some(0_i32)), 0);
    Ok(())
}

#[test]
fn must_some_accepts_tuple_values() -> Result<(), String> {
    let opt = Some((1_u8, 2_u8));
    assert_eq!(must_some(opt), (1, 2));
    Ok(())
}

#[test]
fn must_some_returns_nested_option_as_value() -> Result<(), String> {
    let outer = Some(Some(42));
    assert_eq!(must_some(must_some(outer)), 42);
    Ok(())
}

#[test]
fn must_some_preserves_value_identity() -> Result<(), String> {
    let val = String::from("identity");
    let opt = Some(val.clone());

    assert_eq!(must_some(opt), val);
    Ok(())
}

#[test]
fn must_err_transfers_string_error_ownership() -> Result<(), String> {
    let result: Result<i32, String> = Err(String::from("bad input"));
    assert_eq!(must_err(result), "bad input");
    Ok(())
}

#[test]
fn must_err_accepts_numeric_error_values() -> Result<(), String> {
    let not_found: Result<&str, u32> = Err(404);
    let zero: Result<&str, u32> = Err(0);

    assert_eq!(must_err(not_found), 404);
    assert_eq!(must_err(zero), 0);
    Ok(())
}

#[test]
fn must_err_accepts_bool_error_values() -> Result<(), String> {
    let result: Result<i32, bool> = Err(false);
    assert!(!must_err(result));
    Ok(())
}

#[test]
fn must_err_accepts_tuple_error_values() -> Result<(), String> {
    let result: Result<i32, (u32, &str)> = Err((42, "oops"));
    assert_eq!(must_err(result), (42, "oops"));
    Ok(())
}

#[test]
fn must_err_accepts_enum_error_values() -> Result<(), String> {
    #[derive(Debug, PartialEq)]
    #[allow(dead_code)]
    enum AppError {
        NotFound,
        Timeout,
    }

    let result: Result<i32, AppError> = Err(AppError::NotFound);
    assert_eq!(must_err(result), AppError::NotFound);
    Ok(())
}

#[test]
fn must_err_accepts_custom_error_structs() -> Result<(), String> {
    #[derive(Debug, PartialEq)]
    struct ParseError {
        line: u32,
    }

    let result: Result<i32, ParseError> = Err(ParseError { line: 10 });
    assert_eq!(must_err(result), ParseError { line: 10 });
    Ok(())
}

#[test]
#[should_panic(expected = "unexpected Err")]
fn must_panics_on_numeric_error() {
    let result: Result<i32, u8> = Err(7);
    let _ = must(result);
}

#[test]
#[should_panic(expected = "expected Err")]
fn must_err_panics_on_ok_unit() {
    let result: Result<(), &str> = Ok(());
    let _ = must_err(result);
}

#[test]
#[should_panic(expected = "expected Err")]
fn must_err_panics_on_ok_vec() {
    let result: Result<Vec<i32>, &str> = Ok(vec![1, 2, 3]);
    let _ = must_err(result);
}
