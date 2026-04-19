//! Serde round-trip tests for all PerlValue variants.
//!
//! Ensures every variant serializes and deserializes correctly,
//! including nested structures and edge cases.

use perl_dap::value::PerlValue;

// ── Simple variant round-trips ─────────────────────────────────────

#[test]
fn undef_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Undef;
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, PerlValue::Undef);
    Ok(())
}

#[test]
fn scalar_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Scalar("hello world".to_string());
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn scalar_empty_string_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Scalar(String::new());
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn number_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Number(7.125);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn number_negative_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Number(-42.5);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn integer_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Integer(9_999_999);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn integer_negative_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Integer(-1);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn integer_zero_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Integer(0);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Collection variant round-trips ─────────────────────────────────

#[test]
fn array_empty_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Array(vec![]);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn array_with_mixed_elements_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Array(vec![
        PerlValue::Integer(1),
        PerlValue::Scalar("two".to_string()),
        PerlValue::Undef,
        PerlValue::Number(4.0),
    ]);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn hash_empty_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Hash(vec![]);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn hash_with_entries_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Hash(vec![
        ("name".to_string(), PerlValue::Scalar("Alice".to_string())),
        ("age".to_string(), PerlValue::Integer(30)),
        ("active".to_string(), PerlValue::Integer(1)),
    ]);
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Reference variant round-trips ──────────────────────────────────

#[test]
fn reference_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Reference(Box::new(PerlValue::Integer(42)));
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn reference_to_array_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Reference(Box::new(PerlValue::Array(vec![
        PerlValue::Integer(1),
        PerlValue::Integer(2),
    ])));
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Object variant round-trips ─────────────────────────────────────

#[test]
fn object_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Object {
        class: "My::Class".to_string(),
        value: Box::new(PerlValue::Hash(vec![(
            "field".to_string(),
            PerlValue::Scalar("value".to_string()),
        )])),
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn object_with_array_base_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Object {
        class: "Array::Based".to_string(),
        value: Box::new(PerlValue::Array(vec![PerlValue::Integer(1)])),
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Code variant round-trips ───────────────────────────────────────

#[test]
fn code_named_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Code {
        name: Some("main::handler".to_string()),
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn code_anonymous_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Code { name: None };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Glob variant round-trip ────────────────────────────────────────

#[test]
fn glob_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Glob("*main::STDOUT".to_string());
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Regex variant round-trip ───────────────────────────────────────

#[test]
fn regex_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Regex("^foo\\d+bar$".to_string());
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Tied variant round-trips ───────────────────────────────────────

#[test]
fn tied_with_value_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Tied {
        class: "Tie::StdHash".to_string(),
        value: Some(Box::new(PerlValue::Hash(vec![(
            "key".to_string(),
            PerlValue::Scalar("val".to_string()),
        )]))),
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn tied_without_value_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Tied {
        class: "Tie::File".to_string(),
        value: None,
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Truncated variant round-trips ──────────────────────────────────

#[test]
fn truncated_with_count_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Truncated {
        summary: "Array too large".to_string(),
        total_count: Some(50000),
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

#[test]
fn truncated_without_count_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Truncated {
        summary: "Deep nesting".to_string(),
        total_count: None,
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Error variant round-trip ───────────────────────────────────────

#[test]
fn error_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Error("Cannot inspect: variable optimized away".to_string());
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Deeply nested round-trip ───────────────────────────────────────

#[test]
fn deeply_nested_structure_round_trip() -> Result<(), serde_json::Error> {
    let val = PerlValue::Object {
        class: "App::Config".to_string(),
        value: Box::new(PerlValue::Hash(vec![
            (
                "db".to_string(),
                PerlValue::Object {
                    class: "DBI::db".to_string(),
                    value: Box::new(PerlValue::Hash(vec![
                        (
                            "host".to_string(),
                            PerlValue::Scalar("localhost".to_string()),
                        ),
                        ("port".to_string(), PerlValue::Integer(5432)),
                    ])),
                },
            ),
            (
                "items".to_string(),
                PerlValue::Reference(Box::new(PerlValue::Array(vec![
                    PerlValue::Scalar("first".to_string()),
                    PerlValue::Undef,
                    PerlValue::Number(9.81),
                ]))),
            ),
        ])),
    };
    let json = serde_json::to_string(&val)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(back, val);
    Ok(())
}

// ── Behavioral edge cases ──────────────────────────────────────────

#[test]
fn tied_is_expandable() {
    let val = PerlValue::Tied {
        class: "Tie::Hash".to_string(),
        value: Some(Box::new(PerlValue::Hash(vec![]))),
    };
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "TIED");
    assert_eq!(val.child_count(), None);
}

#[test]
fn regex_is_not_expandable() {
    let val = PerlValue::Regex("pattern".to_string());
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "Regexp");
    assert_eq!(val.child_count(), None);
}

#[test]
fn glob_is_not_expandable() {
    let val = PerlValue::Glob("*main::STDERR".to_string());
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "GLOB");
    assert_eq!(val.child_count(), None);
}

#[test]
fn error_is_not_expandable() {
    let val = PerlValue::Error("inspection failed".to_string());
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "ERROR");
    assert_eq!(val.child_count(), None);
}

#[test]
fn truncated_child_count_returns_total() {
    let val = PerlValue::Truncated {
        summary: "big array".to_string(),
        total_count: Some(10000),
    };
    assert_eq!(val.child_count(), Some(10000));

    let val_none = PerlValue::Truncated {
        summary: "unknown".to_string(),
        total_count: None,
    };
    assert_eq!(val_none.child_count(), None);
}

#[test]
fn number_and_integer_share_scalar_type_name() {
    assert_eq!(PerlValue::Number(1.0).type_name(), "SCALAR");
    assert_eq!(PerlValue::Integer(1).type_name(), "SCALAR");
    assert_eq!(PerlValue::Scalar("1".to_string()).type_name(), "SCALAR");
}

#[test]
fn code_type_name_is_code() {
    assert_eq!(PerlValue::Code { name: None }.type_name(), "CODE");
    assert_eq!(
        PerlValue::Code {
            name: Some("f".to_string())
        }
        .type_name(),
        "CODE"
    );
}

#[test]
fn object_child_count_is_none() {
    // Objects delegate to their inner value in the renderer, but
    // PerlValue::child_count() only returns Some for Array/Hash/Truncated
    let val = PerlValue::Object {
        class: "My::Obj".to_string(),
        value: Box::new(PerlValue::Hash(vec![("k".to_string(), PerlValue::Undef)])),
    };
    assert_eq!(val.child_count(), None);
}

#[test]
fn reference_child_count_is_none() {
    let val = PerlValue::Reference(Box::new(PerlValue::Array(vec![PerlValue::Undef])));
    assert_eq!(val.child_count(), None);
}
