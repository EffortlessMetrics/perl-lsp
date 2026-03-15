//! Tests for improved DAP variable inspection.
//!
//! Covers the five requested scenarios:
//! 1. Hash variable display with nested values
//! 2. Array variable display with indices
//! 3. Reference variable display (deref chain)
//! 4. Blessed object display with class name
//! 5. Large data structure truncation

use perl_dap_variables::{PerlValue, PerlVariableRenderer, VariableRenderer};

// ═══════════════════════════════════════════════════════════════════════
// 1. Hash variable display with nested values
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn hash_preview_with_scalar_values() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![
        ("name".into(), PerlValue::scalar("Alice")),
        ("age".into(), PerlValue::Integer(30)),
        ("active".into(), PerlValue::Integer(1)),
    ]);
    let rendered = renderer.render("%user", &val);

    assert_eq!(rendered.name, "%user");
    assert_eq!(rendered.type_name.as_deref(), Some("HASH"));
    assert_eq!(rendered.named_variables, Some(3));
    // Preview should show key => value pairs
    assert!(rendered.value.contains("name => "));
    assert!(rendered.value.contains("age => "));
    assert!(rendered.value.contains("active => "));
    Ok(())
}

#[test]
fn hash_preview_with_nested_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let inner_hash = PerlValue::Hash(vec![
        ("street".into(), PerlValue::scalar("123 Main St")),
        ("city".into(), PerlValue::scalar("Springfield")),
    ]);
    let val = PerlValue::Hash(vec![
        ("name".into(), PerlValue::scalar("Bob")),
        ("address".into(), inner_hash),
    ]);
    let rendered = renderer.render("%person", &val);

    // The nested hash should appear as HASH(2) in the brief preview
    assert!(rendered.value.contains("HASH(2)"));
    assert_eq!(rendered.named_variables, Some(2));
    Ok(())
}

#[test]
fn hash_preview_with_nested_array() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let scores = PerlValue::Array(vec![
        PerlValue::Integer(95),
        PerlValue::Integer(87),
        PerlValue::Integer(92),
    ]);
    let val = PerlValue::Hash(vec![
        ("student".into(), PerlValue::scalar("Charlie")),
        ("scores".into(), scores),
    ]);
    let rendered = renderer.render("%record", &val);

    // The nested array should appear as ARRAY(3) in brief preview
    assert!(rendered.value.contains("ARRAY(3)"));
    Ok(())
}

#[test]
fn hash_children_show_nested_values() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let inner_hash = PerlValue::Hash(vec![
        ("x".into(), PerlValue::Integer(10)),
        ("y".into(), PerlValue::Integer(20)),
    ]);
    let val = PerlValue::Hash(vec![
        ("origin".into(), inner_hash),
        ("label".into(), PerlValue::scalar("point")),
    ]);

    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);

    // First child is the nested hash "origin"
    assert_eq!(children[0].name, "origin");
    assert_eq!(children[0].type_name.as_deref(), Some("HASH"));
    assert_eq!(children[0].named_variables, Some(2));

    // Second child is the scalar "label"
    assert_eq!(children[1].name, "label");
    assert_eq!(children[1].value, "\"point\"");
    Ok(())
}

#[test]
fn hash_with_undef_values() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![
        ("defined".into(), PerlValue::Integer(42)),
        ("missing".into(), PerlValue::Undef),
    ]);
    let rendered = renderer.render("%h", &val);

    assert!(rendered.value.contains("undef"));
    assert!(rendered.value.contains("42"));
    Ok(())
}

#[test]
fn hash_deeply_nested_three_levels() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let level3 = PerlValue::Hash(vec![("deep".into(), PerlValue::scalar("value"))]);
    let level2 = PerlValue::Hash(vec![("mid".into(), level3)]);
    let level1 = PerlValue::Hash(vec![("top".into(), level2)]);

    // Render and expand each level
    let top_children = renderer.render_children(&level1, 0, 10);
    assert_eq!(top_children.len(), 1);
    assert_eq!(top_children[0].name, "top");
    assert_eq!(top_children[0].type_name.as_deref(), Some("HASH"));

    // The child value should indicate it has nested content
    assert_eq!(top_children[0].named_variables, Some(1));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Array variable display with indices
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn array_children_have_bracket_indices() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![
        PerlValue::scalar("first"),
        PerlValue::scalar("second"),
        PerlValue::scalar("third"),
    ]);
    let children = renderer.render_children(&val, 0, 10);

    assert_eq!(children.len(), 3);
    assert_eq!(children[0].name, "[0]");
    assert_eq!(children[0].value, "\"first\"");
    assert_eq!(children[1].name, "[1]");
    assert_eq!(children[1].value, "\"second\"");
    assert_eq!(children[2].name, "[2]");
    assert_eq!(children[2].value, "\"third\"");
    Ok(())
}

#[test]
fn array_children_pagination_start_offset() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![
        PerlValue::Integer(10),
        PerlValue::Integer(20),
        PerlValue::Integer(30),
        PerlValue::Integer(40),
        PerlValue::Integer(50),
    ]);

    // Skip first 2, take 2
    let children = renderer.render_children(&val, 2, 2);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "[2]");
    assert_eq!(children[0].value, "30");
    assert_eq!(children[1].name, "[3]");
    assert_eq!(children[1].value, "40");
    Ok(())
}

#[test]
fn array_children_pagination_past_end() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);

    // Start at 1, ask for 10 but only 1 remaining
    let children = renderer.render_children(&val, 1, 10);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "[1]");
    Ok(())
}

#[test]
fn array_children_with_mixed_types() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![
        PerlValue::Integer(42),
        PerlValue::scalar("hello"),
        PerlValue::Undef,
        PerlValue::Array(vec![PerlValue::Integer(1)]),
        PerlValue::Hash(vec![("k".into(), PerlValue::Integer(2))]),
    ]);

    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 5);

    assert_eq!(children[0].name, "[0]");
    assert_eq!(children[0].value, "42");
    assert_eq!(children[0].type_name.as_deref(), Some("SCALAR"));

    assert_eq!(children[1].name, "[1]");
    assert_eq!(children[1].value, "\"hello\"");

    assert_eq!(children[2].name, "[2]");
    assert_eq!(children[2].value, "undef");
    assert_eq!(children[2].type_name.as_deref(), Some("undef"));

    assert_eq!(children[3].name, "[3]");
    assert_eq!(children[3].type_name.as_deref(), Some("ARRAY"));
    assert_eq!(children[3].indexed_variables, Some(1));

    assert_eq!(children[4].name, "[4]");
    assert_eq!(children[4].type_name.as_deref(), Some("HASH"));
    assert_eq!(children[4].named_variables, Some(1));
    Ok(())
}

#[test]
fn array_preview_within_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(3);
    let val =
        PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2), PerlValue::Integer(3)]);
    let rendered = renderer.render("@arr", &val);

    // All 3 fit within preview limit
    assert_eq!(rendered.value, "[1, 2, 3]");
    assert_eq!(rendered.indexed_variables, Some(3));
    Ok(())
}

#[test]
fn array_preview_exceeds_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(2);
    let val = PerlValue::Array(vec![
        PerlValue::Integer(1),
        PerlValue::Integer(2),
        PerlValue::Integer(3),
        PerlValue::Integer(4),
    ]);
    let rendered = renderer.render("@arr", &val);

    // Shows first 2 elements then truncation marker
    assert!(rendered.value.contains("1, 2"));
    assert!(rendered.value.contains("4 total"));
    assert_eq!(rendered.indexed_variables, Some(4));
    Ok(())
}

#[test]
fn empty_array_display() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![]);
    let rendered = renderer.render("@empty", &val);

    assert_eq!(rendered.value, "[]");
    assert_eq!(rendered.indexed_variables, Some(0));

    let children = renderer.render_children(&val, 0, 10);
    assert!(children.is_empty());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Reference variable display (deref chain)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn single_reference_to_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Reference(Box::new(PerlValue::Integer(42)));
    let rendered = renderer.render("$ref", &val);

    assert_eq!(rendered.type_name.as_deref(), Some("REF"));
    assert!(rendered.value.starts_with('\\'));
    assert!(rendered.value.contains("42"));
    Ok(())
}

#[test]
fn single_reference_to_string() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::scalar("hello"));
    let rendered = renderer.render("$sref", &val);

    assert!(rendered.value.starts_with('\\'));
    assert!(rendered.value.contains("hello"));
    Ok(())
}

#[test]
fn double_reference_deref_chain() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    // \\42 -- reference to reference to integer
    let val = PerlValue::reference(PerlValue::reference(PerlValue::Integer(42)));
    let rendered = renderer.render("$rref", &val);

    // Should show two backslashes and the final value
    assert!(rendered.value.starts_with("\\\\"));
    assert!(rendered.value.contains("42"));
    Ok(())
}

#[test]
fn triple_reference_deref_chain() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val =
        PerlValue::reference(PerlValue::reference(PerlValue::reference(PerlValue::scalar("deep"))));
    let rendered = renderer.render("$rrref", &val);

    // Should show three backslashes and the final string value
    assert!(rendered.value.starts_with("\\\\\\"));
    assert!(rendered.value.contains("deep"));
    Ok(())
}

#[test]
fn reference_to_array_shows_content() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::Array(vec![
        PerlValue::Integer(1),
        PerlValue::Integer(2),
        PerlValue::Integer(3),
    ]));
    let rendered = renderer.render("$aref", &val);

    // Full format shows actual array content
    assert!(rendered.value.starts_with('\\'));
    assert!(rendered.value.contains("[1, 2, 3]"));
    Ok(())
}

#[test]
fn reference_to_hash_shows_content() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::Hash(vec![("foo".into(), PerlValue::Integer(1))]));
    let rendered = renderer.render("$href", &val);

    assert!(rendered.value.starts_with('\\'));
    assert!(rendered.value.contains("foo => 1"));
    Ok(())
}

#[test]
fn reference_children_show_dereferenced_value() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::Integer(42));
    let children = renderer.render_children(&val, 0, 10);

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "$_");
    assert_eq!(children[0].value, "42");
    Ok(())
}

#[test]
fn nested_reference_children_peel_one_layer() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::reference(PerlValue::Integer(42)));
    let children = renderer.render_children(&val, 0, 10);

    // render_children peels one layer of reference
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "$_");
    // The child is itself a reference, so it should show as expandable
    assert_eq!(children[0].type_name.as_deref(), Some("REF"));
    Ok(())
}

#[test]
fn reference_to_object_shows_class() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let obj = PerlValue::object(
        "HTTP::Response",
        PerlValue::Hash(vec![("status".into(), PerlValue::Integer(200))]),
    );
    let val = PerlValue::reference(obj);
    let rendered = renderer.render("$resp", &val);

    assert!(rendered.value.contains("HTTP::Response"));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Blessed object display with class name
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn object_type_name_is_class() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object(
        "My::Class",
        PerlValue::Hash(vec![("attr".into(), PerlValue::scalar("value"))]),
    );
    let rendered = renderer.render("$obj", &val);

    // type_name should be the class name, not generic "OBJECT"
    assert_eq!(rendered.type_name.as_deref(), Some("My::Class"));
    assert!(rendered.value.contains("My::Class"));
    Ok(())
}

#[test]
fn object_with_hash_backing_has_named_variables() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object(
        "Person",
        PerlValue::Hash(vec![
            ("name".into(), PerlValue::scalar("Alice")),
            ("age".into(), PerlValue::Integer(30)),
            ("email".into(), PerlValue::scalar("alice@example.com")),
        ]),
    );
    let rendered = renderer.render("$person", &val);

    assert_eq!(rendered.type_name.as_deref(), Some("Person"));
    assert_eq!(rendered.named_variables, Some(3));
    assert_eq!(rendered.indexed_variables, None);
    Ok(())
}

#[test]
fn object_with_array_backing_has_indexed_variables() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object(
        "My::Vector",
        PerlValue::Array(vec![
            PerlValue::Number(1.0),
            PerlValue::Number(2.5),
            PerlValue::Number(3.7),
        ]),
    );
    let rendered = renderer.render("$vec", &val);

    assert_eq!(rendered.type_name.as_deref(), Some("My::Vector"));
    assert_eq!(rendered.indexed_variables, Some(3));
    assert_eq!(rendered.named_variables, None);
    Ok(())
}

#[test]
fn object_with_scalar_backing() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object("URI", PerlValue::scalar("https://example.com"));
    let rendered = renderer.render("$uri", &val);

    assert_eq!(rendered.type_name.as_deref(), Some("URI"));
    assert!(rendered.value.contains("URI"));
    // Scalar-backed objects have no named or indexed variables
    assert_eq!(rendered.named_variables, None);
    assert_eq!(rendered.indexed_variables, None);
    Ok(())
}

#[test]
fn object_children_delegate_to_inner_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object(
        "Dog",
        PerlValue::Hash(vec![
            ("breed".into(), PerlValue::scalar("Labrador")),
            ("age".into(), PerlValue::Integer(5)),
        ]),
    );

    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "breed");
    assert_eq!(children[0].value, "\"Labrador\"");
    assert_eq!(children[1].name, "age");
    assert_eq!(children[1].value, "5");
    Ok(())
}

#[test]
fn object_children_delegate_to_inner_array() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object(
        "Tuple",
        PerlValue::Array(vec![PerlValue::Integer(10), PerlValue::Integer(20)]),
    );

    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "[0]");
    assert_eq!(children[0].value, "10");
    assert_eq!(children[1].name, "[1]");
    assert_eq!(children[1].value, "20");
    Ok(())
}

#[test]
fn nested_object_in_object() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let inner_obj = PerlValue::object(
        "Address",
        PerlValue::Hash(vec![("city".into(), PerlValue::scalar("NYC"))]),
    );
    let outer = PerlValue::object(
        "Person",
        PerlValue::Hash(vec![
            ("name".into(), PerlValue::scalar("Bob")),
            ("addr".into(), inner_obj),
        ]),
    );

    let rendered = renderer.render("$p", &outer);
    assert_eq!(rendered.type_name.as_deref(), Some("Person"));

    let children = renderer.render_children(&outer, 0, 10);
    assert_eq!(children.len(), 2);
    assert_eq!(children[1].name, "addr");
    // The inner object should show its class name as type
    assert_eq!(children[1].type_name.as_deref(), Some("Address"));
    Ok(())
}

#[test]
fn object_value_shows_class_equals_brief() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object(
        "HTTP::Request",
        PerlValue::Hash(vec![
            ("method".into(), PerlValue::scalar("GET")),
            ("uri".into(), PerlValue::scalar("/api/v1")),
        ]),
    );
    let rendered = renderer.render("$req", &val);

    // Value should be "HTTP::Request={method => ...}" format
    assert!(rendered.value.starts_with("HTTP::Request="));
    Ok(())
}

#[test]
fn object_expandable_via_render_with_reference() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val =
        PerlValue::object("Config", PerlValue::Hash(vec![("debug".into(), PerlValue::Integer(1))]));

    let rendered = renderer.render_with_reference("$cfg", &val, 99);
    assert_eq!(rendered.variables_reference, 99);
    assert!(rendered.is_expandable());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Large data structure truncation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn string_truncation_at_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(10);
    let val = PerlValue::scalar("abcdefghij");
    let rendered = renderer.render("$s", &val);

    // Exactly at limit: no truncation
    assert_eq!(rendered.value, "\"abcdefghij\"");
    assert!(!rendered.value.contains("..."));
    Ok(())
}

#[test]
fn string_truncation_over_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(10);
    let val = PerlValue::scalar("abcdefghijk");
    let rendered = renderer.render("$s", &val);

    // Over limit: truncated with ellipsis
    assert!(rendered.value.contains("..."));
    assert!(rendered.value.contains("abcdefghij"));
    Ok(())
}

#[test]
fn string_truncation_utf8_boundary_safe() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(5);
    // Multi-byte characters: each emoji is 4 bytes
    let val = PerlValue::scalar("\u{1F600}\u{1F601}\u{1F602}");
    let rendered = renderer.render("$emoji", &val);

    // Should not panic, and should produce valid output
    assert!(rendered.value.contains("..."));
    // The string should be valid UTF-8
    assert!(rendered.value.is_char_boundary(0));
    Ok(())
}

#[test]
fn string_truncation_two_byte_chars() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(3);
    // Each character is 2 bytes in UTF-8
    let val = PerlValue::scalar("\u{00E9}\u{00E9}\u{00E9}");
    let rendered = renderer.render("$s", &val);

    // Should truncate safely without splitting a multi-byte char
    assert!(rendered.value.contains("..."));
    Ok(())
}

#[test]
fn array_preview_truncation_with_many_elements() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(3);
    let elements: Vec<PerlValue> = (0..100).map(PerlValue::Integer).collect();
    let val = PerlValue::Array(elements);
    let rendered = renderer.render("@big", &val);

    // Should show first 3 elements and total count
    assert!(rendered.value.contains("0, 1, 2"));
    assert!(rendered.value.contains("100 total"));
    assert_eq!(rendered.indexed_variables, Some(100));
    Ok(())
}

#[test]
fn hash_preview_truncation_with_many_keys() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_hash_preview(2);
    let pairs: Vec<(String, PerlValue)> =
        (0..50).map(|i| (format!("key_{}", i), PerlValue::Integer(i))).collect();
    let val = PerlValue::Hash(pairs);
    let rendered = renderer.render("%big", &val);

    // Should show first 2 pairs and total key count
    assert!(rendered.value.contains("key_0 => 0"));
    assert!(rendered.value.contains("key_1 => 1"));
    assert!(rendered.value.contains("50 keys"));
    assert_eq!(rendered.named_variables, Some(50));
    Ok(())
}

#[test]
fn truncated_value_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val =
        PerlValue::Truncated { summary: "ARRAY [1, 2, 3, ...]".into(), total_count: Some(10_000) };
    let rendered = renderer.render("@huge", &val);

    assert!(rendered.value.contains("10000 total"));
    assert!(rendered.value.contains("ARRAY [1, 2, 3, ...]"));
    assert_eq!(rendered.type_name.as_deref(), Some("..."));
    Ok(())
}

#[test]
fn truncated_value_without_count() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Truncated { summary: "deeply nested structure".into(), total_count: None };
    let rendered = renderer.render("$deep", &val);

    assert_eq!(rendered.value, "deeply nested structure");
    Ok(())
}

#[test]
fn large_array_children_pagination() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let elements: Vec<PerlValue> = (0..1000).map(PerlValue::Integer).collect();
    let val = PerlValue::Array(elements);

    // Fetch page from middle
    let children = renderer.render_children(&val, 500, 5);
    assert_eq!(children.len(), 5);
    assert_eq!(children[0].name, "[500]");
    assert_eq!(children[0].value, "500");
    assert_eq!(children[4].name, "[504]");
    assert_eq!(children[4].value, "504");
    Ok(())
}

#[test]
fn large_hash_children_pagination() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let pairs: Vec<(String, PerlValue)> =
        (0..100).map(|i| (format!("k{}", i), PerlValue::Integer(i))).collect();
    let val = PerlValue::Hash(pairs);

    // Fetch a slice
    let children = renderer.render_children(&val, 10, 3);
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].name, "k10");
    assert_eq!(children[1].name, "k11");
    assert_eq!(children[2].name, "k12");
    Ok(())
}

#[test]
fn array_preview_with_max_zero_shows_all_truncated() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(0);
    let val = PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);
    let rendered = renderer.render("@x", &val);

    // With max_array_preview=0, should still show total count
    assert!(rendered.value.contains("2 total"));
    Ok(())
}

#[test]
fn string_truncation_with_length_zero() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(0);
    let val = PerlValue::scalar("any content");
    let rendered = renderer.render("$s", &val);

    // Even with max_string_length=0, should produce valid output
    assert!(rendered.value.contains("..."));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Cross-cutting: combined scenarios
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn reference_to_object_with_nested_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let obj = PerlValue::object(
        "Mojo::Transaction",
        PerlValue::Hash(vec![
            (
                "req".into(),
                PerlValue::object(
                    "Mojo::Message::Request",
                    PerlValue::Hash(vec![
                        ("method".into(), PerlValue::scalar("POST")),
                        ("url".into(), PerlValue::scalar("/api")),
                    ]),
                ),
            ),
            (
                "res".into(),
                PerlValue::object(
                    "Mojo::Message::Response",
                    PerlValue::Hash(vec![("code".into(), PerlValue::Integer(200))]),
                ),
            ),
        ]),
    );
    let val = PerlValue::reference(obj);
    let rendered = renderer.render("$tx", &val);

    // Should show the class name in the value
    assert!(rendered.value.contains("Mojo::Transaction"));
    Ok(())
}

#[test]
fn array_of_objects() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![
        PerlValue::object("Item", PerlValue::Hash(vec![("id".into(), PerlValue::Integer(1))])),
        PerlValue::object("Item", PerlValue::Hash(vec![("id".into(), PerlValue::Integer(2))])),
    ]);

    let rendered = renderer.render("@items", &val);
    assert_eq!(rendered.indexed_variables, Some(2));

    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "[0]");
    assert_eq!(children[0].type_name.as_deref(), Some("Item"));
    assert_eq!(children[1].name, "[1]");
    assert_eq!(children[1].type_name.as_deref(), Some("Item"));
    Ok(())
}

#[test]
fn hash_of_arrays() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![
        (
            "fruits".into(),
            PerlValue::Array(vec![PerlValue::scalar("apple"), PerlValue::scalar("banana")]),
        ),
        ("vegs".into(), PerlValue::Array(vec![PerlValue::scalar("carrot")])),
    ]);

    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "fruits");
    assert_eq!(children[0].indexed_variables, Some(2));
    assert_eq!(children[1].name, "vegs");
    assert_eq!(children[1].indexed_variables, Some(1));
    Ok(())
}
