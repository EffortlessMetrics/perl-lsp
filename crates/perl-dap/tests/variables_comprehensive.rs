//! Comprehensive integration tests for the perl-dap-variables crate.
//!
//! Tests cover PerlValue, VariableParser, RenderedVariable, and PerlVariableRenderer.

use perl_dap::variables::{
    PerlValue, PerlVariableRenderer, RenderedVariable, VariableParseError, VariableParser,
    VariableRenderer,
};
use perl_tdd_support::{must, must_err};

// ───────────────────────────── PerlValue ─────────────────────────────

#[test]
fn perl_value_default_is_undef() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::default();
    assert_eq!(val, PerlValue::Undef);
    Ok(())
}

#[test]
fn perl_value_scalar_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::scalar("hello");
    assert_eq!(val, PerlValue::Scalar("hello".to_string()));
    Ok(())
}

#[test]
fn perl_value_scalar_constructor_from_string() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::scalar(String::from("world"));
    assert_eq!(val, PerlValue::Scalar("world".to_string()));
    Ok(())
}

#[test]
fn perl_value_array_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);
    assert_eq!(val.child_count(), Some(2));
    Ok(())
}

#[test]
fn perl_value_hash_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = vec![
        ("a".to_string(), PerlValue::Integer(1)),
        ("b".to_string(), PerlValue::Integer(2)),
    ];
    let val = PerlValue::hash(pairs);
    assert_eq!(val.child_count(), Some(2));
    Ok(())
}

#[test]
fn perl_value_reference_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::reference(PerlValue::Integer(42));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "REF");
    Ok(())
}

#[test]
fn perl_value_object_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::object("Foo::Bar", PerlValue::Hash(vec![]));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "OBJECT");
    Ok(())
}

// ── is_expandable coverage ──

#[test]
fn is_expandable_for_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    // Not expandable
    assert!(!PerlValue::Undef.is_expandable());
    assert!(!PerlValue::Scalar("x".to_string()).is_expandable());
    assert!(!PerlValue::Number(1.0).is_expandable());
    assert!(!PerlValue::Integer(1).is_expandable());
    assert!(!PerlValue::Code { name: None }.is_expandable());
    assert!(!PerlValue::Glob("main::foo".to_string()).is_expandable());
    assert!(!PerlValue::Regex("abc".to_string()).is_expandable());
    assert!(
        !PerlValue::Truncated {
            summary: "...".to_string(),
            total_count: Some(100)
        }
        .is_expandable()
    );
    assert!(!PerlValue::Error("oops".to_string()).is_expandable());

    // Expandable
    assert!(PerlValue::Array(vec![]).is_expandable());
    assert!(PerlValue::Hash(vec![]).is_expandable());
    assert!(PerlValue::Reference(Box::new(PerlValue::Undef)).is_expandable());
    assert!(
        PerlValue::Object {
            class: "X".to_string(),
            value: Box::new(PerlValue::Undef)
        }
        .is_expandable()
    );
    assert!(
        PerlValue::Tied {
            class: "Y".to_string(),
            value: None
        }
        .is_expandable()
    );
    Ok(())
}

// ── type_name coverage ──

#[test]
fn type_name_for_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlValue::Undef.type_name(), "undef");
    assert_eq!(PerlValue::Scalar("".to_string()).type_name(), "SCALAR");
    assert_eq!(PerlValue::Number(0.0).type_name(), "SCALAR");
    assert_eq!(PerlValue::Integer(0).type_name(), "SCALAR");
    assert_eq!(PerlValue::Array(vec![]).type_name(), "ARRAY");
    assert_eq!(PerlValue::Hash(vec![]).type_name(), "HASH");
    assert_eq!(
        PerlValue::Reference(Box::new(PerlValue::Undef)).type_name(),
        "REF"
    );
    assert_eq!(
        PerlValue::Object {
            class: "C".to_string(),
            value: Box::new(PerlValue::Undef)
        }
        .type_name(),
        "OBJECT"
    );
    assert_eq!(PerlValue::Code { name: None }.type_name(), "CODE");
    assert_eq!(PerlValue::Glob("g".to_string()).type_name(), "GLOB");
    assert_eq!(PerlValue::Regex("r".to_string()).type_name(), "Regexp");
    assert_eq!(
        PerlValue::Tied {
            class: "T".to_string(),
            value: None
        }
        .type_name(),
        "TIED"
    );
    assert_eq!(
        PerlValue::Truncated {
            summary: "s".to_string(),
            total_count: None
        }
        .type_name(),
        "..."
    );
    assert_eq!(PerlValue::Error("e".to_string()).type_name(), "ERROR");
    Ok(())
}

// ── child_count coverage ──

#[test]
fn child_count_for_non_collection_types() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlValue::Undef.child_count(), None);
    assert_eq!(PerlValue::Scalar("x".to_string()).child_count(), None);
    assert_eq!(PerlValue::Number(1.0).child_count(), None);
    assert_eq!(PerlValue::Integer(1).child_count(), None);
    assert_eq!(
        PerlValue::Reference(Box::new(PerlValue::Undef)).child_count(),
        None
    );
    assert_eq!(PerlValue::Code { name: None }.child_count(), None);
    assert_eq!(PerlValue::Glob("g".to_string()).child_count(), None);
    assert_eq!(PerlValue::Regex("r".to_string()).child_count(), None);
    assert_eq!(PerlValue::Error("e".to_string()).child_count(), None);
    Ok(())
}

#[test]
fn child_count_for_arrays() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlValue::Array(vec![]).child_count(), Some(0));
    assert_eq!(
        PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]).child_count(),
        Some(2)
    );
    Ok(())
}

#[test]
fn child_count_for_hashes() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlValue::Hash(vec![]).child_count(), Some(0));
    let pairs = vec![("k".to_string(), PerlValue::Undef)];
    assert_eq!(PerlValue::Hash(pairs).child_count(), Some(1));
    Ok(())
}

#[test]
fn child_count_for_truncated() -> Result<(), Box<dyn std::error::Error>> {
    let with_count = PerlValue::Truncated {
        summary: "...".to_string(),
        total_count: Some(50),
    };
    assert_eq!(with_count.child_count(), Some(50));

    let without_count = PerlValue::Truncated {
        summary: "...".to_string(),
        total_count: None,
    };
    assert_eq!(without_count.child_count(), None);
    Ok(())
}

// ── Serialization round-trip ──

#[test]
fn perl_value_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let values = vec![
        PerlValue::Undef,
        PerlValue::Scalar("hello".to_string()),
        PerlValue::Number(9.81),
        PerlValue::Integer(42),
        PerlValue::Array(vec![PerlValue::Integer(1)]),
        PerlValue::Hash(vec![("k".to_string(), PerlValue::Scalar("v".to_string()))]),
        PerlValue::Reference(Box::new(PerlValue::Integer(1))),
        PerlValue::object("Foo", PerlValue::Hash(vec![])),
        PerlValue::Code {
            name: Some("my_sub".to_string()),
        },
        PerlValue::Code { name: None },
        PerlValue::Glob("main::STDOUT".to_string()),
        PerlValue::Regex("^\\d+$".to_string()),
        PerlValue::Tied {
            class: "Tie::Hash".to_string(),
            value: None,
        },
        PerlValue::Tied {
            class: "Tie::Array".to_string(),
            value: Some(Box::new(PerlValue::Array(vec![]))),
        },
        PerlValue::Truncated {
            summary: "large".to_string(),
            total_count: Some(1000),
        },
        PerlValue::Error("something broke".to_string()),
    ];

    for val in &values {
        let json = serde_json::to_string(val)?;
        let deserialized: PerlValue = serde_json::from_str(&json)?;
        assert_eq!(val, &deserialized, "Round-trip failed for {:?}", val);
    }
    Ok(())
}

// ── Clone and Debug ──

#[test]
fn perl_value_clone_equality() -> Result<(), Box<dyn std::error::Error>> {
    let original = PerlValue::object(
        "My::Class",
        PerlValue::Hash(vec![(
            "attr".to_string(),
            PerlValue::Scalar("val".to_string()),
        )]),
    );
    let cloned = original.clone();
    assert_eq!(original, cloned);
    Ok(())
}

#[test]
fn perl_value_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Integer(42);
    let debug = format!("{:?}", val);
    assert!(debug.contains("42"));
    Ok(())
}

// ───────────────────────── VariableParser ─────────────────────────

#[test]
fn parser_new_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    // Should be able to parse without error at default depth
    let val = must(parser.parse_value("42", 0));
    assert_eq!(val, PerlValue::Integer(42));
    Ok(())
}

#[test]
fn parser_with_max_depth() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new().with_max_depth(1);
    // Depth 0 → 1 is fine, but 1 → 2 should fail
    let err = must_err(parser.parse_value("((1))", 0));
    assert!(matches!(err, VariableParseError::MaxDepthExceeded(_)));
    Ok(())
}

#[test]
fn parser_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::default();
    let val = must(parser.parse_value("undef", 0));
    assert_eq!(val, PerlValue::Undef);
    Ok(())
}

// ── parse_value: undef ──

#[test]
fn parse_value_undef() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("undef", 0));
    assert_eq!(val, PerlValue::Undef);
    Ok(())
}

#[test]
fn parse_value_undef_with_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("  undef  ", 0));
    assert_eq!(val, PerlValue::Undef);
    Ok(())
}

// ── parse_value: integers ──

#[test]
fn parse_value_positive_integer() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("42", 0));
    assert_eq!(val, PerlValue::Integer(42));
    Ok(())
}

#[test]
fn parse_value_negative_integer() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("-17", 0));
    assert_eq!(val, PerlValue::Integer(-17));
    Ok(())
}

#[test]
fn parse_value_zero() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("0", 0));
    assert_eq!(val, PerlValue::Integer(0));
    Ok(())
}

// ── parse_value: floating point ──

#[test]
fn parse_value_float() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("3.25", 0));
    if let PerlValue::Number(n) = val {
        assert!((n - 3.25).abs() < 0.001);
    } else {
        return Err("Expected Number".into());
    }
    Ok(())
}

#[test]
fn parse_value_negative_float() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("-2.5", 0));
    if let PerlValue::Number(n) = val {
        assert!((n - (-2.5)).abs() < 0.001);
    } else {
        return Err("Expected Number".into());
    }
    Ok(())
}

#[test]
fn parse_value_scientific_notation() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("1.5e10", 0));
    assert!(matches!(val, PerlValue::Number(_)));

    let val = must(parser.parse_value("2.0E-3", 0));
    assert!(matches!(val, PerlValue::Number(_)));
    Ok(())
}

#[test]
fn parse_value_leading_dot_number() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value(".5", 0));
    if let PerlValue::Number(n) = val {
        assert!((n - 0.5).abs() < 0.001);
    } else {
        return Err("Expected Number".into());
    }
    Ok(())
}

// ── parse_value: strings ──

#[test]
fn parse_value_double_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("\"hello world\"", 0));
    assert_eq!(val, PerlValue::Scalar("hello world".to_string()));
    Ok(())
}

#[test]
fn parse_value_single_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("'hello world'", 0));
    assert_eq!(val, PerlValue::Scalar("hello world".to_string()));
    Ok(())
}

#[test]
fn parse_value_string_with_escape_sequences() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("\"line1\\nline2\\ttab\"", 0));
    if let PerlValue::Scalar(s) = val {
        assert!(s.contains('\n'));
        assert!(s.contains('\t'));
    } else {
        return Err("Expected Scalar".into());
    }
    Ok(())
}

#[test]
fn parse_value_string_with_escaped_quote() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value(r#""say \"hi\"""#, 0));
    if let PerlValue::Scalar(s) = val {
        assert!(s.contains('"'));
    } else {
        return Err("Expected Scalar".into());
    }
    Ok(())
}

#[test]
fn parse_value_string_with_escaped_backslash() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("\"path\\\\to\\\\file\"", 0));
    if let PerlValue::Scalar(s) = val {
        assert!(s.contains('\\'));
    } else {
        return Err("Expected Scalar".into());
    }
    Ok(())
}

#[test]
fn parse_value_empty_double_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("\"\"", 0));
    assert_eq!(val, PerlValue::Scalar(String::new()));
    Ok(())
}

#[test]
fn parse_value_empty_single_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("''", 0));
    assert_eq!(val, PerlValue::Scalar(String::new()));
    Ok(())
}

#[test]
fn parse_value_bareword_as_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("bareword", 0));
    assert_eq!(val, PerlValue::Scalar("bareword".to_string()));
    Ok(())
}

// ── parse_value: references ──

#[test]
fn parse_value_array_ref() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("ARRAY(0x1a2b3c4d)", 0));
    assert!(matches!(val, PerlValue::Array(ref a) if a.is_empty()));
    Ok(())
}

#[test]
fn parse_value_hash_ref() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("HASH(0xdeadbeef)", 0));
    assert!(matches!(val, PerlValue::Hash(ref h) if h.is_empty()));
    Ok(())
}

#[test]
fn parse_value_code_ref() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("CODE(0xfeedface)", 0));
    assert_eq!(val, PerlValue::Code { name: None });
    Ok(())
}

// ── parse_value: objects ──

#[test]
fn parse_value_object_hash() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("My::Class=HASH(0x1234)", 0));
    if let PerlValue::Object { class, value } = val {
        assert_eq!(class, "My::Class");
        assert!(matches!(*value, PerlValue::Hash(_)));
    } else {
        return Err("Expected Object".into());
    }
    Ok(())
}

#[test]
fn parse_value_object_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("IO::File=ARRAY(0xabcd)", 0));
    if let PerlValue::Object { class, value } = val {
        assert_eq!(class, "IO::File");
        assert!(matches!(*value, PerlValue::Array(_)));
    } else {
        return Err("Expected Object".into());
    }
    Ok(())
}

#[test]
fn parse_value_object_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("URI=SCALAR(0x5678)", 0));
    if let PerlValue::Object { class, value } = val {
        assert_eq!(class, "URI");
        assert!(matches!(*value, PerlValue::Scalar(_)));
    } else {
        return Err("Expected Object".into());
    }
    Ok(())
}

#[test]
fn parse_value_object_glob() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("IO::Handle=GLOB(0x9abc)", 0));
    if let PerlValue::Object { class, value } = val {
        assert_eq!(class, "IO::Handle");
        assert!(matches!(*value, PerlValue::Scalar(_)));
    } else {
        return Err("Expected Object".into());
    }
    Ok(())
}

// ── parse_value: globs ──

#[test]
fn parse_value_glob_simple() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("*main::foo", 0));
    assert_eq!(val, PerlValue::Glob("main::foo".to_string()));
    Ok(())
}

#[test]
fn parse_value_glob_bare() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("*STDOUT", 0));
    assert_eq!(val, PerlValue::Glob("STDOUT".to_string()));
    Ok(())
}

// ── parse_value: array literals ──

#[test]
fn parse_value_empty_paren_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("()", 0));
    assert_eq!(val, PerlValue::Array(vec![]));
    Ok(())
}

#[test]
fn parse_value_paren_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("(1, 2, 3)", 0));
    if let PerlValue::Array(elems) = val {
        assert_eq!(elems.len(), 3);
        assert_eq!(elems[0], PerlValue::Integer(1));
        assert_eq!(elems[1], PerlValue::Integer(2));
        assert_eq!(elems[2], PerlValue::Integer(3));
    } else {
        return Err("Expected Array".into());
    }
    Ok(())
}

#[test]
fn parse_value_bracket_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("[10, 20]", 0));
    if let PerlValue::Array(elems) = val {
        assert_eq!(elems.len(), 2);
        assert_eq!(elems[0], PerlValue::Integer(10));
        assert_eq!(elems[1], PerlValue::Integer(20));
    } else {
        return Err("Expected Array".into());
    }
    Ok(())
}

#[test]
fn parse_value_empty_bracket_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("[]", 0));
    assert_eq!(val, PerlValue::Array(vec![]));
    Ok(())
}

#[test]
fn parse_value_mixed_type_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("(1, \"two\", 3.0, undef)", 0));
    if let PerlValue::Array(elems) = val {
        assert_eq!(elems.len(), 4);
        assert_eq!(elems[0], PerlValue::Integer(1));
        assert_eq!(elems[1], PerlValue::Scalar("two".to_string()));
        assert!(matches!(elems[2], PerlValue::Number(_)));
        assert_eq!(elems[3], PerlValue::Undef);
    } else {
        return Err("Expected Array".into());
    }
    Ok(())
}

// ── parse_value: hash literals ──

#[test]
fn parse_value_empty_hash() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("{}", 0));
    assert_eq!(val, PerlValue::Hash(vec![]));
    Ok(())
}

#[test]
fn parse_value_simple_hash() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("{foo => 1, bar => 2}", 0));
    if let PerlValue::Hash(pairs) = val {
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "foo");
        assert_eq!(pairs[0].1, PerlValue::Integer(1));
        assert_eq!(pairs[1].0, "bar");
        assert_eq!(pairs[1].1, PerlValue::Integer(2));
    } else {
        return Err("Expected Hash".into());
    }
    Ok(())
}

#[test]
fn parse_value_hash_with_quoted_keys() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("{\"key\" => 1}", 0));
    if let PerlValue::Hash(pairs) = val {
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
    } else {
        return Err("Expected Hash".into());
    }
    Ok(())
}

#[test]
fn parse_value_hash_key_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("{orphan_key}", 0));
    if let PerlValue::Hash(pairs) = val {
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "orphan_key");
        assert_eq!(pairs[0].1, PerlValue::Undef);
    } else {
        return Err("Expected Hash".into());
    }
    Ok(())
}

// ── parse_value: nested structures ──

#[test]
fn parse_value_nested_array_in_hash() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("{arr => [1, 2], hash => {a => 1}}", 0));
    if let PerlValue::Hash(pairs) = val {
        assert_eq!(pairs.len(), 2);
        assert!(matches!(pairs[0].1, PerlValue::Array(_)));
        assert!(matches!(pairs[1].1, PerlValue::Hash(_)));
    } else {
        return Err("Expected Hash".into());
    }
    Ok(())
}

#[test]
fn parse_value_nested_array_in_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let val = must(parser.parse_value("([1, 2], [3, 4])", 0));
    if let PerlValue::Array(elems) = val {
        assert_eq!(elems.len(), 2);
        assert!(matches!(&elems[0], PerlValue::Array(a) if a.len() == 2));
        assert!(matches!(&elems[1], PerlValue::Array(a) if a.len() == 2));
    } else {
        return Err("Expected Array".into());
    }
    Ok(())
}

// ── parse_assignment ──

#[test]
fn parse_assignment_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let (name, val) = must(parser.parse_assignment("$x = 42"));
    assert_eq!(name, "$x");
    assert_eq!(val, PerlValue::Integer(42));
    Ok(())
}

#[test]
fn parse_assignment_array() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let (name, val) = must(parser.parse_assignment("@arr = (1, 2, 3)"));
    assert_eq!(name, "@arr");
    assert!(matches!(val, PerlValue::Array(a) if a.len() == 3));
    Ok(())
}

#[test]
fn parse_assignment_hash() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let (name, val) = must(parser.parse_assignment("%hash = {a => 1}"));
    assert_eq!(name, "%hash");
    assert!(matches!(val, PerlValue::Hash(_)));
    Ok(())
}

#[test]
fn parse_assignment_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let (name, val) = must(parser.parse_assignment("$Foo::Bar::baz = \"hello\""));
    assert_eq!(name, "$Foo::Bar::baz");
    assert_eq!(val, PerlValue::Scalar("hello".to_string()));
    Ok(())
}

#[test]
fn parse_assignment_undef_value() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let (name, val) = must(parser.parse_assignment("$x = undef"));
    assert_eq!(name, "$x");
    assert_eq!(val, PerlValue::Undef);
    Ok(())
}

#[test]
fn parse_assignment_invalid_format() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let err = must_err(parser.parse_assignment("not a variable assignment"));
    assert!(matches!(err, VariableParseError::UnrecognizedFormat(_)));
    Ok(())
}

#[test]
fn parse_assignment_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let err = must_err(parser.parse_assignment(""));
    assert!(matches!(err, VariableParseError::UnrecognizedFormat(_)));
    Ok(())
}

// ── parse_variables (multi-line) ──

#[test]
fn parse_variables_multi_line() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let output = "$x = 1\n$y = 2\n$z = \"hello\"";
    let vars = parser.parse_variables(output);
    assert_eq!(vars.len(), 3);
    assert_eq!(vars[0].0, "$x");
    assert_eq!(vars[0].1, PerlValue::Integer(1));
    assert_eq!(vars[1].0, "$y");
    assert_eq!(vars[1].1, PerlValue::Integer(2));
    assert_eq!(vars[2].0, "$z");
    assert_eq!(vars[2].1, PerlValue::Scalar("hello".to_string()));
    Ok(())
}

#[test]
fn parse_variables_skips_invalid_lines() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let output = "$x = 1\ngarbage line\n$y = 2";
    let vars = parser.parse_variables(output);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].0, "$x");
    assert_eq!(vars[1].0, "$y");
    Ok(())
}

#[test]
fn parse_variables_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let vars = parser.parse_variables("");
    assert!(vars.is_empty());
    Ok(())
}

// ── max depth ──

#[test]
fn parse_value_max_depth_exceeded() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new().with_max_depth(2);
    let err = must_err(parser.parse_value("(((1)))", 0));
    assert!(matches!(err, VariableParseError::MaxDepthExceeded(_)));
    Ok(())
}

#[test]
fn parse_value_at_exact_max_depth() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new().with_max_depth(2);
    // depth 0 → parse "(..)" increments to 1 → parse "(1)" increments to 2 → parse "1" at depth 2
    // which is fine since the check is > max_depth
    let val = must(parser.parse_value("((1))", 0));
    if let PerlValue::Array(outer) = val {
        assert_eq!(outer.len(), 1);
    } else {
        return Err("Expected Array".into());
    }
    Ok(())
}

// ── VariableParseError display ──

#[test]
fn parse_error_display_messages() -> Result<(), Box<dyn std::error::Error>> {
    let err = VariableParseError::UnrecognizedFormat("bad".to_string());
    assert!(err.to_string().contains("bad"));

    let err = VariableParseError::MaxDepthExceeded(10);
    assert!(err.to_string().contains("10"));

    let err = VariableParseError::UnterminatedString;
    assert!(err.to_string().contains("unterminated"));

    let err = VariableParseError::UnterminatedCollection;
    assert!(err.to_string().contains("unterminated"));
    Ok(())
}

// ───────────────────── RenderedVariable ─────────────────────────

#[test]
fn rendered_variable_new() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("$x", "42");
    assert_eq!(rv.name, "$x");
    assert_eq!(rv.value, "42");
    assert_eq!(rv.type_name, None);
    assert_eq!(rv.variables_reference, 0);
    assert_eq!(rv.named_variables, None);
    assert_eq!(rv.indexed_variables, None);
    assert_eq!(rv.presentation_hint, None);
    assert_eq!(rv.memory_reference, None);
    assert!(!rv.is_expandable());
    Ok(())
}

#[test]
fn rendered_variable_builder_methods() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("@arr", "[1, 2, 3]")
        .with_type("ARRAY")
        .with_reference(42)
        .with_indexed_variables(3)
        .with_named_variables(0);

    assert_eq!(rv.type_name, Some("ARRAY".to_string()));
    assert_eq!(rv.variables_reference, 42);
    assert_eq!(rv.indexed_variables, Some(3));
    assert_eq!(rv.named_variables, Some(0));
    assert!(rv.is_expandable());
    Ok(())
}

#[test]
fn rendered_variable_is_expandable() -> Result<(), Box<dyn std::error::Error>> {
    let not_expandable = RenderedVariable::new("$x", "42");
    assert!(!not_expandable.is_expandable());

    let expandable = RenderedVariable::new("@arr", "[]").with_reference(1);
    assert!(expandable.is_expandable());

    let negative_ref = RenderedVariable::new("$r", "ref").with_reference(-1);
    assert!(negative_ref.is_expandable());
    Ok(())
}

#[test]
fn rendered_variable_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("$x", "42")
        .with_type("SCALAR")
        .with_reference(0);
    let json = serde_json::to_string(&rv)?;
    let deserialized: RenderedVariable = serde_json::from_str(&json)?;
    assert_eq!(rv, deserialized);
    Ok(())
}

#[test]
fn rendered_variable_serde_skips_none_fields() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("$x", "42");
    let json = serde_json::to_string(&rv)?;
    // Optional None fields should not appear in JSON
    assert!(!json.contains("namedVariables"));
    assert!(!json.contains("indexedVariables"));
    assert!(!json.contains("presentationHint"));
    assert!(!json.contains("memoryReference"));
    Ok(())
}

#[test]
fn rendered_variable_serde_includes_some_fields() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("@a", "[1]")
        .with_type("ARRAY")
        .with_indexed_variables(1)
        .with_named_variables(0);
    let json = serde_json::to_string(&rv)?;
    assert!(json.contains("indexedVariables"));
    assert!(json.contains("namedVariables"));
    Ok(())
}

#[test]
fn rendered_variable_type_renames_to_type_in_json() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("$x", "42").with_type("SCALAR");
    let json = serde_json::to_string(&rv)?;
    // Should serialize as "type" not "type_name"
    assert!(json.contains("\"type\""));
    Ok(())
}

// ───────────────────── PerlVariableRenderer ─────────────────────

#[test]
fn renderer_new_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let rendered = renderer.render("$x", &PerlValue::Scalar("test".to_string()));
    assert_eq!(rendered.name, "$x");
    assert_eq!(rendered.value, "\"test\"");
    Ok(())
}

#[test]
fn renderer_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::default();
    let rendered = renderer.render("$x", &PerlValue::Integer(1));
    assert_eq!(rendered.value, "1");
    Ok(())
}

// ── render: each PerlValue variant ──

#[test]
fn render_undef() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let rendered = renderer.render("$x", &PerlValue::Undef);
    assert_eq!(rendered.value, "undef");
    assert_eq!(rendered.type_name, Some("undef".to_string()));
    assert_eq!(rendered.variables_reference, 0);
    Ok(())
}

#[test]
fn render_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let rendered = renderer.render("$greeting", &PerlValue::Scalar("hello".to_string()));
    assert_eq!(rendered.name, "$greeting");
    assert_eq!(rendered.value, "\"hello\"");
    assert_eq!(rendered.type_name, Some("SCALAR".to_string()));
    Ok(())
}

#[test]
fn render_number() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let rendered = renderer.render("$g", &PerlValue::Number(9.81));
    assert_eq!(rendered.name, "$g");
    assert_eq!(rendered.value, "9.81");
    assert_eq!(rendered.type_name, Some("SCALAR".to_string()));
    Ok(())
}

#[test]
fn render_integer() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let rendered = renderer.render("$n", &PerlValue::Integer(42));
    assert_eq!(rendered.name, "$n");
    assert_eq!(rendered.value, "42");
    assert_eq!(rendered.type_name, Some("SCALAR".to_string()));
    Ok(())
}

#[test]
fn render_empty_array() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let rendered = renderer.render("@arr", &PerlValue::Array(vec![]));
    assert_eq!(rendered.value, "[]");
    assert_eq!(rendered.type_name, Some("ARRAY".to_string()));
    assert_eq!(rendered.indexed_variables, Some(0));
    Ok(())
}

#[test]
fn render_array_within_preview_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);
    let rendered = renderer.render("@arr", &val);
    assert!(rendered.value.starts_with('['));
    assert!(rendered.value.ends_with(']'));
    assert!(rendered.value.contains('1'));
    assert!(rendered.value.contains('2'));
    assert_eq!(rendered.indexed_variables, Some(2));
    Ok(())
}

#[test]
fn render_array_exceeds_preview_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(2);
    let val = PerlValue::Array(vec![
        PerlValue::Integer(1),
        PerlValue::Integer(2),
        PerlValue::Integer(3),
        PerlValue::Integer(4),
    ]);
    let rendered = renderer.render("@arr", &val);
    assert!(rendered.value.contains("..."));
    assert!(rendered.value.contains("4 total"));
    assert_eq!(rendered.indexed_variables, Some(4));
    Ok(())
}

#[test]
fn render_empty_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let rendered = renderer.render("%h", &PerlValue::Hash(vec![]));
    assert_eq!(rendered.value, "{}");
    assert_eq!(rendered.type_name, Some("HASH".to_string()));
    assert_eq!(rendered.named_variables, Some(0));
    Ok(())
}

#[test]
fn render_hash_within_preview_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![
        ("foo".to_string(), PerlValue::Integer(1)),
        ("bar".to_string(), PerlValue::Integer(2)),
    ]);
    let rendered = renderer.render("%h", &val);
    assert!(rendered.value.starts_with('{'));
    assert!(rendered.value.ends_with('}'));
    assert!(rendered.value.contains("foo"));
    assert!(rendered.value.contains("bar"));
    assert_eq!(rendered.named_variables, Some(2));
    Ok(())
}

#[test]
fn render_hash_exceeds_preview_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_hash_preview(1);
    let val = PerlValue::Hash(vec![
        ("a".to_string(), PerlValue::Integer(1)),
        ("b".to_string(), PerlValue::Integer(2)),
        ("c".to_string(), PerlValue::Integer(3)),
    ]);
    let rendered = renderer.render("%h", &val);
    assert!(rendered.value.contains("..."));
    assert!(rendered.value.contains("3 keys"));
    Ok(())
}

#[test]
fn render_reference() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Reference(Box::new(PerlValue::Integer(42)));
    let rendered = renderer.render("$ref", &val);
    assert!(rendered.value.contains("42"));
    assert_eq!(rendered.type_name, Some("REF".to_string()));
    Ok(())
}

#[test]
fn render_object_with_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Object {
        class: "My::Class".to_string(),
        value: Box::new(PerlValue::Hash(vec![(
            "attr".to_string(),
            PerlValue::Scalar("val".to_string()),
        )])),
    };
    let rendered = renderer.render("$obj", &val);
    assert!(rendered.value.contains("My::Class"));
    assert_eq!(rendered.type_name, Some("My::Class".to_string()));
    assert_eq!(rendered.named_variables, Some(1));
    Ok(())
}

#[test]
fn render_object_without_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Object {
        class: "My::Scalar".to_string(),
        value: Box::new(PerlValue::Scalar("inner".to_string())),
    };
    let rendered = renderer.render("$obj", &val);
    assert!(rendered.value.contains("My::Scalar"));
    assert_eq!(rendered.named_variables, None);
    Ok(())
}

#[test]
fn render_code_named() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Code {
        name: Some("my_sub".to_string()),
    };
    let rendered = renderer.render("$code", &val);
    assert!(rendered.value.contains("my_sub"));
    assert_eq!(rendered.type_name, Some("CODE".to_string()));
    Ok(())
}

#[test]
fn render_code_anonymous() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Code { name: None };
    let rendered = renderer.render("$code", &val);
    assert!(rendered.value.contains("sub"));
    assert_eq!(rendered.type_name, Some("CODE".to_string()));
    Ok(())
}

#[test]
fn render_glob() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Glob("main::STDOUT".to_string());
    let rendered = renderer.render("$glob", &val);
    assert!(rendered.value.contains("main::STDOUT"));
    assert_eq!(rendered.type_name, Some("GLOB".to_string()));
    Ok(())
}

#[test]
fn render_regex() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Regex("^\\d+$".to_string());
    let rendered = renderer.render("$re", &val);
    assert!(rendered.value.contains("qr/"));
    assert_eq!(rendered.type_name, Some("Regexp".to_string()));
    Ok(())
}

#[test]
fn render_tied_with_value() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Tied {
        class: "Tie::Hash".to_string(),
        value: Some(Box::new(PerlValue::Hash(vec![]))),
    };
    let rendered = renderer.render("$tied", &val);
    assert!(rendered.value.contains("TIED"));
    assert!(rendered.value.contains("Tie::Hash"));
    assert_eq!(rendered.type_name, Some("TIED".to_string()));
    Ok(())
}

#[test]
fn render_tied_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Tied {
        class: "Tie::File".to_string(),
        value: None,
    };
    let rendered = renderer.render("$tied", &val);
    assert!(rendered.value.contains("TIED"));
    assert!(rendered.value.contains("Tie::File"));
    Ok(())
}

#[test]
fn render_truncated_with_count() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Truncated {
        summary: "large array".to_string(),
        total_count: Some(1000),
    };
    let rendered = renderer.render("@big", &val);
    assert!(rendered.value.contains("large array"));
    assert!(rendered.value.contains("1000"));
    assert_eq!(rendered.type_name, Some("...".to_string()));
    Ok(())
}

#[test]
fn render_truncated_without_count() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Truncated {
        summary: "big structure".to_string(),
        total_count: None,
    };
    let rendered = renderer.render("$big", &val);
    assert_eq!(rendered.value, "big structure");
    Ok(())
}

#[test]
fn render_error() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Error("something went wrong".to_string());
    let rendered = renderer.render("$err", &val);
    assert!(rendered.value.contains("something went wrong"));
    assert_eq!(rendered.type_name, Some("ERROR".to_string()));
    Ok(())
}

// ── render string formatting ──

#[test]
fn render_string_with_special_chars() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Scalar("line1\nline2\ttab\r\n".to_string());
    let rendered = renderer.render("$s", &val);
    assert!(rendered.value.contains("\\n"));
    assert!(rendered.value.contains("\\t"));
    assert!(rendered.value.contains("\\r"));
    Ok(())
}

#[test]
fn render_string_with_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Scalar("say \"hi\"".to_string());
    let rendered = renderer.render("$s", &val);
    assert!(rendered.value.contains("\\\""));
    Ok(())
}

#[test]
fn render_string_with_backslashes() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Scalar("C:\\path\\to".to_string());
    let rendered = renderer.render("$s", &val);
    assert!(rendered.value.contains("\\\\"));
    Ok(())
}

#[test]
fn render_string_truncation() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(5);
    let val = PerlValue::Scalar("abcdefghij".to_string());
    let rendered = renderer.render("$s", &val);
    assert!(rendered.value.contains("..."));
    Ok(())
}

#[test]
fn render_string_no_truncation_at_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(5);
    let val = PerlValue::Scalar("abcde".to_string());
    let rendered = renderer.render("$s", &val);
    assert!(!rendered.value.contains("..."));
    assert_eq!(rendered.value, "\"abcde\"");
    Ok(())
}

// ── render_with_reference ──

#[test]
fn render_with_reference_expandable() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![PerlValue::Integer(1)]);
    let rendered = renderer.render_with_reference("@arr", &val, 100);
    assert_eq!(rendered.variables_reference, 100);
    assert!(rendered.is_expandable());
    Ok(())
}

#[test]
fn render_with_reference_not_expandable() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Integer(42);
    let rendered = renderer.render_with_reference("$n", &val, 100);
    // Non-expandable values should keep reference at 0
    assert_eq!(rendered.variables_reference, 0);
    assert!(!rendered.is_expandable());
    Ok(())
}

#[test]
fn render_with_reference_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![("k".to_string(), PerlValue::Integer(1))]);
    let rendered = renderer.render_with_reference("%h", &val, 50);
    assert_eq!(rendered.variables_reference, 50);
    Ok(())
}

#[test]
fn render_with_reference_object() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object("Pkg", PerlValue::Hash(vec![]));
    let rendered = renderer.render_with_reference("$obj", &val, 77);
    assert_eq!(rendered.variables_reference, 77);
    Ok(())
}

#[test]
fn render_with_reference_tied() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Tied {
        class: "T".to_string(),
        value: None,
    };
    let rendered = renderer.render_with_reference("$t", &val, 33);
    assert_eq!(rendered.variables_reference, 33);
    Ok(())
}

// ── render_children ──

#[test]
fn render_children_array() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![
        PerlValue::Integer(10),
        PerlValue::Integer(20),
        PerlValue::Integer(30),
    ]);
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].name, "[0]");
    assert_eq!(children[0].value, "10");
    assert_eq!(children[1].name, "[1]");
    assert_eq!(children[1].value, "20");
    assert_eq!(children[2].name, "[2]");
    assert_eq!(children[2].value, "30");
    Ok(())
}

#[test]
fn render_children_array_pagination() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![
        PerlValue::Integer(10),
        PerlValue::Integer(20),
        PerlValue::Integer(30),
        PerlValue::Integer(40),
    ]);

    // Skip first 1, take 2
    let children = renderer.render_children(&val, 1, 2);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "[1]");
    assert_eq!(children[0].value, "20");
    assert_eq!(children[1].name, "[2]");
    assert_eq!(children[1].value, "30");
    Ok(())
}

#[test]
fn render_children_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![
        ("alpha".to_string(), PerlValue::Integer(1)),
        ("beta".to_string(), PerlValue::Scalar("two".to_string())),
    ]);
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "alpha");
    assert_eq!(children[0].value, "1");
    assert_eq!(children[1].name, "beta");
    assert_eq!(children[1].value, "\"two\"");
    Ok(())
}

#[test]
fn render_children_hash_pagination() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![
        ("a".to_string(), PerlValue::Integer(1)),
        ("b".to_string(), PerlValue::Integer(2)),
        ("c".to_string(), PerlValue::Integer(3)),
    ]);
    let children = renderer.render_children(&val, 1, 1);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "b");
    Ok(())
}

#[test]
fn render_children_reference() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Reference(Box::new(PerlValue::Integer(42)));
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "$_");
    assert_eq!(children[0].value, "42");
    Ok(())
}

#[test]
fn render_children_object_delegates_to_inner() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Object {
        class: "My::Obj".to_string(),
        value: Box::new(PerlValue::Hash(vec![
            ("name".to_string(), PerlValue::Scalar("test".to_string())),
            ("id".to_string(), PerlValue::Integer(1)),
        ])),
    };
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "name");
    assert_eq!(children[1].name, "id");
    Ok(())
}

#[test]
fn render_children_tied_with_value() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Tied {
        class: "Tie::Hash".to_string(),
        value: Some(Box::new(PerlValue::Hash(vec![(
            "k".to_string(),
            PerlValue::Integer(1),
        )]))),
    };
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "k");
    Ok(())
}

#[test]
fn render_children_tied_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Tied {
        class: "Tie::File".to_string(),
        value: None,
    };
    let children = renderer.render_children(&val, 0, 10);
    assert!(children.is_empty());
    Ok(())
}

#[test]
fn render_children_non_expandable_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();

    assert!(
        renderer
            .render_children(&PerlValue::Undef, 0, 10)
            .is_empty()
    );
    assert!(
        renderer
            .render_children(&PerlValue::Integer(1), 0, 10)
            .is_empty()
    );
    assert!(
        renderer
            .render_children(&PerlValue::Scalar("x".to_string()), 0, 10)
            .is_empty()
    );
    assert!(
        renderer
            .render_children(&PerlValue::Number(1.0), 0, 10)
            .is_empty()
    );
    assert!(
        renderer
            .render_children(&PerlValue::Code { name: None }, 0, 10)
            .is_empty()
    );
    assert!(
        renderer
            .render_children(&PerlValue::Glob("g".to_string()), 0, 10)
            .is_empty()
    );
    assert!(
        renderer
            .render_children(&PerlValue::Regex("r".to_string()), 0, 10)
            .is_empty()
    );
    assert!(
        renderer
            .render_children(&PerlValue::Error("e".to_string()), 0, 10)
            .is_empty()
    );
    Ok(())
}

// ── renderer configuration ──

#[test]
fn renderer_with_max_string_length() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(3);
    let val = PerlValue::Scalar("abcdef".to_string());
    let rendered = renderer.render("$s", &val);
    assert!(rendered.value.contains("..."));
    Ok(())
}

#[test]
fn renderer_with_max_array_preview() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(1);
    let val = PerlValue::Array(vec![
        PerlValue::Integer(1),
        PerlValue::Integer(2),
        PerlValue::Integer(3),
    ]);
    let rendered = renderer.render("@a", &val);
    assert!(rendered.value.contains("3 total"));
    Ok(())
}

#[test]
fn renderer_with_max_hash_preview() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_hash_preview(1);
    let val = PerlValue::Hash(vec![
        ("a".to_string(), PerlValue::Integer(1)),
        ("b".to_string(), PerlValue::Integer(2)),
    ]);
    let rendered = renderer.render("%h", &val);
    assert!(rendered.value.contains("2 keys"));
    Ok(())
}

// ── format_value_brief coverage (via array/hash previews) ──

#[test]
fn brief_format_covers_all_variants_in_preview() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(20);

    let elements = vec![
        PerlValue::Undef,
        PerlValue::Scalar("hi".to_string()),
        PerlValue::Number(1.5),
        PerlValue::Integer(7),
        PerlValue::Array(vec![PerlValue::Integer(1)]),
        PerlValue::Hash(vec![("k".to_string(), PerlValue::Undef)]),
        PerlValue::Reference(Box::new(PerlValue::Integer(1))),
        PerlValue::object("Cls", PerlValue::Hash(vec![])),
        PerlValue::Code {
            name: Some("fn".to_string()),
        },
        PerlValue::Code { name: None },
        PerlValue::Glob("main::FOO".to_string()),
        PerlValue::Regex("abc".to_string()),
        PerlValue::Tied {
            class: "T".to_string(),
            value: None,
        },
        PerlValue::Truncated {
            summary: "trunc".to_string(),
            total_count: None,
        },
        PerlValue::Error("err".to_string()),
    ];

    let val = PerlValue::Array(elements);
    let rendered = renderer.render("@all", &val);

    // Just check the preview contains recognizable strings for each type
    assert!(rendered.value.contains("undef"));
    assert!(rendered.value.contains("\"hi\""));
    assert!(rendered.value.contains("1.5"));
    assert!(rendered.value.contains("ARRAY(1)"));
    assert!(rendered.value.contains("HASH(1)"));
    assert!(rendered.value.contains("\\1"));
    assert!(rendered.value.contains("Cls = HASH(...)"));
    assert!(rendered.value.contains("\\&fn"));
    assert!(rendered.value.contains("CODE(...)"));
    assert!(rendered.value.contains("*main::FOO"));
    assert!(rendered.value.contains("qr/abc/"));
    assert!(rendered.value.contains("TIED(T)"));
    assert!(rendered.value.contains("trunc"));
    assert!(rendered.value.contains("<error: err>"));
    Ok(())
}
