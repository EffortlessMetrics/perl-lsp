//! Integration tests for `perl-dap-value`.
//!
//! Covers every `PerlValue` variant across: constructors, `Default`,
//! `is_expandable`, `type_name`, `child_count`, `Clone`/`PartialEq`,
//! serde round-trip, edge cases (empty, extreme, deeply-nested), and `Debug`.

use perl_dap::value::PerlValue;

// ════════════════════════════════════════════════════════════════════
// Constructors
// ════════════════════════════════════════════════════════════════════

#[test]
fn scalar_constructor_from_str() {
    let v = PerlValue::scalar("hello");
    assert!(matches!(v, PerlValue::Scalar(ref s) if s == "hello"));
}

#[test]
fn scalar_constructor_from_string() {
    let v = PerlValue::scalar(String::from("world"));
    assert!(matches!(v, PerlValue::Scalar(ref s) if s == "world"));
}

#[test]
fn array_constructor() {
    let v = PerlValue::array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);
    assert!(matches!(v, PerlValue::Array(ref a) if a.len() == 2));
}

#[test]
fn hash_constructor() {
    let v = PerlValue::hash(vec![("key".to_string(), PerlValue::scalar("value"))]);
    assert!(matches!(v, PerlValue::Hash(ref h) if h.len() == 1));
}

#[test]
fn reference_constructor() {
    let v = PerlValue::reference(PerlValue::Integer(42));
    assert!(matches!(v, PerlValue::Reference(_)));
}

#[test]
fn object_constructor_from_str() {
    let v = PerlValue::object("My::Class", PerlValue::Hash(vec![]));
    assert!(matches!(v, PerlValue::Object { ref class, .. } if class == "My::Class"));
}

#[test]
fn object_constructor_from_string() {
    let v = PerlValue::object(String::from("Foo::Bar"), PerlValue::Undef);
    assert!(matches!(v, PerlValue::Object { ref class, .. } if class == "Foo::Bar"));
}

// ════════════════════════════════════════════════════════════════════
// Default
// ════════════════════════════════════════════════════════════════════

#[test]
fn default_is_undef() {
    assert_eq!(PerlValue::default(), PerlValue::Undef);
}

// ════════════════════════════════════════════════════════════════════
// is_expandable — all variants
// ════════════════════════════════════════════════════════════════════

#[test]
fn undef_not_expandable() {
    assert!(!PerlValue::Undef.is_expandable());
}

#[test]
fn scalar_not_expandable() {
    assert!(!PerlValue::Scalar("test".into()).is_expandable());
}

#[test]
fn number_not_expandable() {
    assert!(!PerlValue::Number(1.5).is_expandable());
}

#[test]
fn integer_not_expandable() {
    assert!(!PerlValue::Integer(42).is_expandable());
}

#[test]
fn code_not_expandable() {
    assert!(!PerlValue::Code { name: None }.is_expandable());
    assert!(
        !PerlValue::Code {
            name: Some("foo".into())
        }
        .is_expandable()
    );
}

#[test]
fn glob_not_expandable() {
    assert!(!PerlValue::Glob("*main::STDOUT".into()).is_expandable());
}

#[test]
fn regex_not_expandable() {
    assert!(!PerlValue::Regex("^foo$".into()).is_expandable());
}

#[test]
fn truncated_not_expandable() {
    assert!(
        !PerlValue::Truncated {
            summary: "...".into(),
            total_count: Some(100)
        }
        .is_expandable()
    );
}

#[test]
fn error_not_expandable() {
    assert!(!PerlValue::Error("oops".into()).is_expandable());
}

#[test]
fn array_is_expandable() {
    assert!(PerlValue::Array(vec![]).is_expandable());
    assert!(PerlValue::Array(vec![PerlValue::Undef]).is_expandable());
}

#[test]
fn hash_is_expandable() {
    assert!(PerlValue::Hash(vec![]).is_expandable());
}

#[test]
fn reference_is_expandable() {
    assert!(PerlValue::Reference(Box::new(PerlValue::Undef)).is_expandable());
}

#[test]
fn object_is_expandable() {
    assert!(
        PerlValue::Object {
            class: "Foo".into(),
            value: Box::new(PerlValue::Hash(vec![]))
        }
        .is_expandable()
    );
}

#[test]
fn tied_is_expandable() {
    assert!(
        PerlValue::Tied {
            class: "Tie::Hash".into(),
            value: None
        }
        .is_expandable()
    );
    assert!(
        PerlValue::Tied {
            class: "Tie::Scalar".into(),
            value: Some(Box::new(PerlValue::Integer(1))),
        }
        .is_expandable()
    );
}

// ════════════════════════════════════════════════════════════════════
// type_name — all variants
// ════════════════════════════════════════════════════════════════════

#[test]
fn type_name_undef() {
    assert_eq!(PerlValue::Undef.type_name(), "undef");
}

#[test]
fn type_name_scalar() {
    assert_eq!(PerlValue::Scalar("s".into()).type_name(), "SCALAR");
}

#[test]
fn type_name_number() {
    assert_eq!(PerlValue::Number(1.0).type_name(), "SCALAR");
}

#[test]
fn type_name_integer() {
    assert_eq!(PerlValue::Integer(1).type_name(), "SCALAR");
}

#[test]
fn type_name_array() {
    assert_eq!(PerlValue::Array(vec![]).type_name(), "ARRAY");
}

#[test]
fn type_name_hash() {
    assert_eq!(PerlValue::Hash(vec![]).type_name(), "HASH");
}

#[test]
fn type_name_reference() {
    assert_eq!(
        PerlValue::Reference(Box::new(PerlValue::Undef)).type_name(),
        "REF"
    );
}

#[test]
fn type_name_object() {
    assert_eq!(
        PerlValue::Object {
            class: "Foo".into(),
            value: Box::new(PerlValue::Undef)
        }
        .type_name(),
        "OBJECT"
    );
}

#[test]
fn type_name_code() {
    assert_eq!(PerlValue::Code { name: None }.type_name(), "CODE");
    assert_eq!(
        PerlValue::Code {
            name: Some("sub".into())
        }
        .type_name(),
        "CODE"
    );
}

#[test]
fn type_name_glob() {
    assert_eq!(PerlValue::Glob("g".into()).type_name(), "GLOB");
}

#[test]
fn type_name_regex() {
    assert_eq!(PerlValue::Regex("r".into()).type_name(), "Regexp");
}

#[test]
fn type_name_tied() {
    assert_eq!(
        PerlValue::Tied {
            class: "T".into(),
            value: None
        }
        .type_name(),
        "TIED"
    );
}

#[test]
fn type_name_truncated() {
    assert_eq!(
        PerlValue::Truncated {
            summary: "s".into(),
            total_count: None
        }
        .type_name(),
        "..."
    );
}

#[test]
fn type_name_error() {
    assert_eq!(PerlValue::Error("e".into()).type_name(), "ERROR");
}

// ════════════════════════════════════════════════════════════════════
// child_count — all variants
// ════════════════════════════════════════════════════════════════════

#[test]
fn child_count_none_for_leaf_variants() {
    assert_eq!(PerlValue::Undef.child_count(), None);
    assert_eq!(PerlValue::Scalar("s".into()).child_count(), None);
    assert_eq!(PerlValue::Number(1.0).child_count(), None);
    assert_eq!(PerlValue::Integer(1).child_count(), None);
    assert_eq!(
        PerlValue::Reference(Box::new(PerlValue::Undef)).child_count(),
        None
    );
    assert_eq!(PerlValue::Code { name: None }.child_count(), None);
    assert_eq!(PerlValue::Glob("g".into()).child_count(), None);
    assert_eq!(PerlValue::Regex("r".into()).child_count(), None);
    assert_eq!(PerlValue::Error("e".into()).child_count(), None);
}

#[test]
fn child_count_for_empty_array() {
    assert_eq!(PerlValue::Array(vec![]).child_count(), Some(0));
}

#[test]
fn child_count_for_populated_array() {
    assert_eq!(
        PerlValue::Array(vec![
            PerlValue::Integer(1),
            PerlValue::Integer(2),
            PerlValue::Integer(3)
        ])
        .child_count(),
        Some(3)
    );
}

#[test]
fn child_count_for_empty_hash() {
    assert_eq!(PerlValue::Hash(vec![]).child_count(), Some(0));
}

#[test]
fn child_count_for_populated_hash() {
    assert_eq!(
        PerlValue::Hash(vec![
            ("a".into(), PerlValue::Undef),
            ("b".into(), PerlValue::Undef),
        ])
        .child_count(),
        Some(2)
    );
}

#[test]
fn child_count_truncated_with_total() {
    assert_eq!(
        PerlValue::Truncated {
            summary: "big".into(),
            total_count: Some(500)
        }
        .child_count(),
        Some(500)
    );
}

#[test]
fn child_count_truncated_without_total() {
    assert_eq!(
        PerlValue::Truncated {
            summary: "big".into(),
            total_count: None
        }
        .child_count(),
        None
    );
}

#[test]
fn child_count_object_returns_none() {
    // Objects don't report child_count directly (need to look inside the value).
    let obj = PerlValue::object(
        "Foo",
        PerlValue::Hash(vec![("x".into(), PerlValue::Integer(1))]),
    );
    assert_eq!(obj.child_count(), None);
}

#[test]
fn child_count_tied_returns_none() {
    assert_eq!(
        PerlValue::Tied {
            class: "T".into(),
            value: None
        }
        .child_count(),
        None
    );
}

// ════════════════════════════════════════════════════════════════════
// Nesting depth
// ════════════════════════════════════════════════════════════════════

#[test]
fn deeply_nested_references() {
    let mut value = PerlValue::scalar("leaf");
    for _ in 0..100 {
        value = PerlValue::reference(value);
    }
    // All layers should be REF except the innermost.
    assert_eq!(value.type_name(), "REF");
    assert!(value.is_expandable());
}

#[test]
fn deeply_nested_arrays() {
    let mut value = PerlValue::scalar("leaf");
    for _ in 0..50 {
        value = PerlValue::array(vec![value]);
    }
    assert_eq!(value.type_name(), "ARRAY");
    assert_eq!(value.child_count(), Some(1));
}

#[test]
fn deeply_nested_objects() {
    let mut value = PerlValue::scalar("leaf");
    for i in 0..20 {
        value = PerlValue::object(format!("Layer{i}"), value);
    }
    assert_eq!(value.type_name(), "OBJECT");
    assert!(value.is_expandable());
}

// ════════════════════════════════════════════════════════════════════
// Large structures
// ════════════════════════════════════════════════════════════════════

#[test]
fn large_array() {
    let elements: Vec<PerlValue> = (0..1_000).map(PerlValue::Integer).collect();
    let arr = PerlValue::array(elements);
    assert_eq!(arr.child_count(), Some(1_000));
    assert!(arr.is_expandable());
}

#[test]
fn large_hash() {
    let pairs: Vec<(String, PerlValue)> = (0..1_000)
        .map(|i| (format!("key{i}"), PerlValue::Integer(i)))
        .collect();
    let hash = PerlValue::hash(pairs);
    assert_eq!(hash.child_count(), Some(1_000));
    assert!(hash.is_expandable());
}

// ════════════════════════════════════════════════════════════════════
// Clone and PartialEq
// ════════════════════════════════════════════════════════════════════

#[test]
fn clone_preserves_equality_for_all_variants() {
    let variants: Vec<PerlValue> = vec![
        PerlValue::Undef,
        PerlValue::Scalar("hello".into()),
        PerlValue::Number(1.5),
        PerlValue::Integer(42),
        PerlValue::Array(vec![PerlValue::Integer(1)]),
        PerlValue::Hash(vec![("k".into(), PerlValue::Undef)]),
        PerlValue::Reference(Box::new(PerlValue::Integer(1))),
        PerlValue::Object {
            class: "Foo".into(),
            value: Box::new(PerlValue::Undef),
        },
        PerlValue::Code { name: None },
        PerlValue::Code {
            name: Some("bar".into()),
        },
        PerlValue::Glob("*main::STDOUT".into()),
        PerlValue::Regex("^foo$".into()),
        PerlValue::Tied {
            class: "T".into(),
            value: None,
        },
        PerlValue::Tied {
            class: "T".into(),
            value: Some(Box::new(PerlValue::Undef)),
        },
        PerlValue::Truncated {
            summary: "...".into(),
            total_count: Some(100),
        },
        PerlValue::Truncated {
            summary: "...".into(),
            total_count: None,
        },
        PerlValue::Error("oops".into()),
    ];

    for v in &variants {
        let cloned = v.clone();
        assert_eq!(v, &cloned, "clone equality failed for {:?}", v);
    }
}

#[test]
fn different_variants_are_not_equal() {
    assert_ne!(PerlValue::Undef, PerlValue::Scalar("".into()));
    assert_ne!(PerlValue::Integer(0), PerlValue::Number(0.0));
    assert_ne!(PerlValue::Array(vec![]), PerlValue::Hash(vec![]));
}

// ════════════════════════════════════════════════════════════════════
// Serde round-trip
// ════════════════════════════════════════════════════════════════════

#[test]
fn serde_round_trip_undef() -> Result<(), Box<dyn std::error::Error>> {
    let v = PerlValue::Undef;
    let json = serde_json::to_string(&v)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(v, back);
    Ok(())
}

#[test]
fn serde_round_trip_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let v = PerlValue::scalar("hello world");
    let json = serde_json::to_string(&v)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(v, back);
    Ok(())
}

#[test]
fn serde_round_trip_number() -> Result<(), Box<dyn std::error::Error>> {
    let v = PerlValue::Number(1.5);
    let json = serde_json::to_string(&v)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(v, back);
    Ok(())
}

#[test]
fn serde_round_trip_integer() -> Result<(), Box<dyn std::error::Error>> {
    let v = PerlValue::Integer(i64::MAX);
    let json = serde_json::to_string(&v)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(v, back);
    Ok(())
}

#[test]
fn serde_round_trip_complex_nested() -> Result<(), Box<dyn std::error::Error>> {
    let v = PerlValue::object(
        "HTTP::Response",
        PerlValue::hash(vec![
            ("status".into(), PerlValue::Integer(200)),
            (
                "headers".into(),
                PerlValue::hash(vec![(
                    "Content-Type".into(),
                    PerlValue::scalar("text/html"),
                )]),
            ),
            ("body".into(), PerlValue::scalar("<html></html>")),
            (
                "cookies".into(),
                PerlValue::array(vec![PerlValue::scalar("session=abc123")]),
            ),
            (
                "handler".into(),
                PerlValue::Code {
                    name: Some("on_response".into()),
                },
            ),
            ("pattern".into(), PerlValue::Regex("\\d+".into())),
        ]),
    );
    let json = serde_json::to_string(&v)?;
    let back: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(v, back);
    Ok(())
}

#[test]
fn serde_round_trip_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    let variants: Vec<PerlValue> = vec![
        PerlValue::Undef,
        PerlValue::Scalar("s".into()),
        PerlValue::Number(9.81),
        PerlValue::Integer(-1),
        PerlValue::Array(vec![PerlValue::Undef]),
        PerlValue::Hash(vec![("k".into(), PerlValue::Integer(1))]),
        PerlValue::Reference(Box::new(PerlValue::Scalar("ref".into()))),
        PerlValue::Object {
            class: "Foo".into(),
            value: Box::new(PerlValue::Hash(vec![])),
        },
        PerlValue::Code { name: None },
        PerlValue::Code {
            name: Some("sub_name".into()),
        },
        PerlValue::Glob("*STDOUT".into()),
        PerlValue::Regex("^pat$".into()),
        PerlValue::Tied {
            class: "Tie::File".into(),
            value: None,
        },
        PerlValue::Tied {
            class: "Tie::Hash".into(),
            value: Some(Box::new(PerlValue::Hash(vec![]))),
        },
        PerlValue::Truncated {
            summary: "large array".into(),
            total_count: Some(10_000),
        },
        PerlValue::Truncated {
            summary: "unknown".into(),
            total_count: None,
        },
        PerlValue::Error("cannot inspect".into()),
    ];

    for v in &variants {
        let json = serde_json::to_string(v)?;
        let back: PerlValue = serde_json::from_str(&json)?;
        assert_eq!(v, &back, "round-trip failed for {:?}", v);
    }

    Ok(())
}

// ════════════════════════════════════════════════════════════════════
// Edge cases
// ════════════════════════════════════════════════════════════════════

#[test]
fn empty_scalar() {
    let v = PerlValue::scalar("");
    assert_eq!(v.type_name(), "SCALAR");
    assert!(!v.is_expandable());
}

#[test]
fn negative_integer() {
    let v = PerlValue::Integer(i64::MIN);
    assert_eq!(v.type_name(), "SCALAR");
}

#[test]
fn special_float_values() {
    // NaN
    let nan = PerlValue::Number(f64::NAN);
    assert_eq!(nan.type_name(), "SCALAR");

    // Infinity
    let inf = PerlValue::Number(f64::INFINITY);
    assert_eq!(inf.type_name(), "SCALAR");

    // Negative infinity
    let neg_inf = PerlValue::Number(f64::NEG_INFINITY);
    assert_eq!(neg_inf.type_name(), "SCALAR");
}

#[test]
fn error_with_empty_message() {
    let v = PerlValue::Error(String::new());
    assert_eq!(v.type_name(), "ERROR");
    assert!(!v.is_expandable());
}

#[test]
fn glob_with_empty_name() {
    let v = PerlValue::Glob(String::new());
    assert_eq!(v.type_name(), "GLOB");
    assert!(!v.is_expandable());
}

#[test]
fn regex_with_empty_pattern() {
    let v = PerlValue::Regex(String::new());
    assert_eq!(v.type_name(), "Regexp");
    assert!(!v.is_expandable());
}

#[test]
fn truncated_with_zero_total() {
    let v = PerlValue::Truncated {
        summary: "empty".into(),
        total_count: Some(0),
    };
    assert_eq!(v.child_count(), Some(0));
}

// ════════════════════════════════════════════════════════════════════
// Debug impl (derive)
// ════════════════════════════════════════════════════════════════════

#[test]
fn debug_format_includes_variant_name_for_all_types() {
    let cases: Vec<(PerlValue, &str)> = vec![
        (PerlValue::Undef, "Undef"),
        (PerlValue::Scalar("test".into()), "Scalar"),
        (PerlValue::Number(1.0), "Number"),
        (PerlValue::Integer(1), "Integer"),
        (PerlValue::Array(vec![]), "Array"),
        (PerlValue::Hash(vec![]), "Hash"),
        (
            PerlValue::Reference(Box::new(PerlValue::Undef)),
            "Reference",
        ),
        (
            PerlValue::Object {
                class: "C".into(),
                value: Box::new(PerlValue::Undef),
            },
            "Object",
        ),
        (PerlValue::Code { name: None }, "Code"),
        (PerlValue::Glob("g".into()), "Glob"),
        (PerlValue::Regex("r".into()), "Regex"),
        (
            PerlValue::Tied {
                class: "T".into(),
                value: None,
            },
            "Tied",
        ),
        (
            PerlValue::Truncated {
                summary: "s".into(),
                total_count: None,
            },
            "Truncated",
        ),
        (PerlValue::Error("e".into()), "Error"),
    ];

    for (value, expected_variant) in &cases {
        let debug_str = format!("{:?}", value);
        assert!(
            !debug_str.is_empty(),
            "Debug output should be non-empty for {expected_variant}"
        );
        assert!(
            debug_str.contains(expected_variant),
            "Debug output for {expected_variant} should contain variant name, got: {debug_str}"
        );
    }
}
