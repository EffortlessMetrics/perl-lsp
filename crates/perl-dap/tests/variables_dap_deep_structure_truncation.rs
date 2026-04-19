//! Integration tests for DAP variable truncation with deeply nested structures.
//! Tests #3487: deeply nested data structures, large arrays, cyclic references.
//!
//! This test suite validates that:
//! 1. DAP renders 7+ level nested hashes/arrays without exponential output
//! 2. 200+ element arrays/hashes paginate correctly
//! 3. Cyclic references are marked as truncated safely
//! 4. All variables remain clickable/expandable even when truncated

use perl_dap::variables::{PerlValue, PerlVariableRenderer, VariableParser, VariableRenderer};

#[test]
fn test_render_7level_nested_hash_structure() {
    let renderer = PerlVariableRenderer::new();

    // Build a 7-level nested hash: config { level1 { level2 { ... } } }
    let mut value = PerlValue::Hash(vec![("level7".to_string(), PerlValue::Integer(7))]);
    for level in (1..=6).rev() {
        value = PerlValue::Hash(vec![(format!("level{}", level), value)]);
    }

    let rendered = renderer.render("$config", &value);

    // Should not panic or produce exponential output
    assert!(
        rendered.value.len() < 1000,
        "7-level nested value should be bounded"
    );
    assert_eq!(rendered.type_name, Some("HASH".to_string()));
    assert_eq!(rendered.named_variables, Some(1));

    // Should be expandable
    assert!(
        rendered.variables_reference == 0,
        "render() doesn't set reference, need render_with_reference"
    );
}

#[test]
fn test_render_7level_nested_hash_expandable() {
    let renderer = PerlVariableRenderer::new();

    // Build a 7-level nested hash
    let mut value = PerlValue::Hash(vec![("level7".to_string(), PerlValue::Integer(7))]);
    for level in (1..=6).rev() {
        value = PerlValue::Hash(vec![(format!("level{}", level), value)]);
    }

    // Render with reference for expansion
    let rendered = renderer.render_with_reference("$config", &value, 1);

    assert_eq!(rendered.variables_reference, 1);
    assert_eq!(rendered.type_name, Some("HASH".to_string()));
    assert!(rendered.value.len() < 1000);
}

#[test]
fn test_render_500element_array_with_preview_truncation() {
    let renderer = PerlVariableRenderer::new();

    let elements: Vec<PerlValue> = (0..500).map(PerlValue::Integer).collect();
    let value = PerlValue::Array(elements);

    let rendered = renderer.render("@big", &value);

    assert_eq!(rendered.type_name, Some("ARRAY".to_string()));
    assert_eq!(rendered.indexed_variables, Some(500));

    // Preview should be truncated to max_array_preview (3) + "... (500 total)"
    assert!(
        rendered.value.contains("..."),
        "should have truncation marker"
    );
    assert!(
        rendered.value.contains("500 total"),
        "should show total count"
    );
    assert!(rendered.value.len() < 500, "preview should be bounded");
}

#[test]
fn test_render_500element_array_pagination() {
    let renderer = PerlVariableRenderer::new();

    let elements: Vec<PerlValue> = (0..500).map(PerlValue::Integer).collect();
    let value = PerlValue::Array(elements);

    // Request first page
    let page1 = renderer.render_children(&value, 0, 50);
    assert_eq!(page1.len(), 50);
    assert_eq!(page1[0].name, "[0]");
    assert_eq!(page1[49].name, "[49]");

    // Request middle page
    let page_mid = renderer.render_children(&value, 250, 50);
    assert_eq!(page_mid.len(), 50);
    assert_eq!(page_mid[0].name, "[250]");

    // Request final page
    let page_end = renderer.render_children(&value, 450, 100);
    assert_eq!(page_end.len(), 50, "only 50 items left [450..500]");
    assert_eq!(page_end[0].name, "[450]");
}

#[test]
fn test_render_500key_hash_with_preview_truncation() {
    let renderer = PerlVariableRenderer::new();

    let pairs: Vec<(String, PerlValue)> = (0..500)
        .map(|i| (format!("key_{:03}", i), PerlValue::Integer(i)))
        .collect();
    let value = PerlValue::Hash(pairs);

    let rendered = renderer.render("%big", &value);

    assert_eq!(rendered.type_name, Some("HASH".to_string()));
    assert_eq!(rendered.named_variables, Some(500));

    // Preview should be truncated
    assert!(
        rendered.value.contains("..."),
        "should have truncation marker"
    );
    assert!(rendered.value.contains("500 keys"), "should show key count");
    assert!(rendered.value.len() < 500, "preview should be bounded");
}

#[test]
fn test_render_500key_hash_pagination() {
    let renderer = PerlVariableRenderer::new();

    let pairs: Vec<(String, PerlValue)> = (0..500)
        .map(|i| (format!("key_{:03}", i), PerlValue::Integer(i)))
        .collect();
    let value = PerlValue::Hash(pairs);

    // Request first page
    let page1 = renderer.render_children(&value, 0, 50);
    assert_eq!(page1.len(), 50);
    assert_eq!(page1[0].name, "key_000");

    // Request middle page
    let page_mid = renderer.render_children(&value, 250, 50);
    assert_eq!(page_mid.len(), 50);
    assert_eq!(page_mid[0].name, "key_250");

    // Request final page
    let page_end = renderer.render_children(&value, 450, 100);
    assert_eq!(page_end.len(), 50);
    assert_eq!(page_end[0].name, "key_450");
}

#[test]
fn test_render_cyclic_reference_safe() {
    let renderer = PerlVariableRenderer::new();

    // Simulate a self-referential hash
    let truncated_marker = PerlValue::Truncated {
        summary: "HASH(0x7f1234567890)".to_string(),
        total_count: None,
    };
    let value = PerlValue::Hash(vec![(
        "self".to_string(),
        PerlValue::Reference(Box::new(truncated_marker)),
    )]);

    let rendered = renderer.render("$c", &value);

    // Should not panic
    assert_eq!(rendered.type_name, Some("HASH".to_string()));
    assert_eq!(rendered.named_variables, Some(1));
    assert!(rendered.value.len() < 500);

    // Expanding children of the hash must also be safe
    let children = renderer.render_children(&value, 0, 10);
    assert_eq!(children.len(), 1, "hash has 1 child key");
    assert_eq!(children[0].name, "self");
    assert!(
        !children[0].value.is_empty(),
        "ref child should render a non-empty value"
    );

    // Expanding the Reference inner value (the Truncated sentinel) must not panic
    let ref_value = PerlValue::Reference(Box::new(PerlValue::Truncated {
        summary: "HASH(0x7f1234567890)".to_string(),
        total_count: None,
    }));
    let ref_children = renderer.render_children(&ref_value, 0, 10);
    assert_eq!(
        ref_children.len(),
        1,
        "Reference expands to its single inner value"
    );
    assert!(
        !ref_children[0].value.is_empty(),
        "Truncated sentinel renders non-empty"
    );
}

#[test]
fn test_render_deep_reference_chain_bounded() {
    let renderer = PerlVariableRenderer::new();

    // Build 150-level deep reference chain
    let mut value = PerlValue::Integer(42);
    for _ in 0..150 {
        value = PerlValue::Reference(Box::new(value));
    }

    let rendered = renderer.render("$deep", &value);

    // Should be bounded, not exponential
    assert!(
        rendered.value.len() < 200,
        "deep reference chain should be truncated"
    );
    assert_eq!(rendered.type_name, Some("REF".to_string()));

    // Should contain truncation marker
    assert!(
        rendered.value.contains("REF(...)"),
        "should truncate with REF(...) marker"
    );
}

#[test]
fn test_parser_max_depth_safety() {
    let parser = VariableParser::new().with_max_depth(3);

    // Try to parse 7-level nested hash
    let text = "$x = { a => { b => { c => { d => 1 } } } }";
    let result = parser.parse_assignment(text);

    // Should fail gracefully due to max_depth
    assert!(result.is_err(), "parser should reject depth > 3");
}

#[test]
fn test_parser_default_max_depth_succeeds_7levels() {
    let parser = VariableParser::new(); // default max_depth=50

    // Try to parse 7-level nested hash
    let text = "$x = { a => { b => { c => { d => { e => { f => { g => 1 } } } } } } }";
    let result = parser.parse_assignment(text);

    // Should succeed with default max_depth
    assert!(
        result.is_ok(),
        "parser with default max_depth=50 should accept 7 levels"
    );
}

#[test]
fn test_mixed_nested_array_hash_rendering() {
    let renderer = PerlVariableRenderer::new();

    // Create: { pools => [ { host => "localhost", port => 5432 } ] }
    let db_hash = PerlValue::Hash(vec![
        (
            "host".to_string(),
            PerlValue::Scalar("localhost".to_string()),
        ),
        ("port".to_string(), PerlValue::Integer(5432)),
    ]);

    let pools_array = PerlValue::Array(vec![db_hash]);

    let value = PerlValue::Hash(vec![("pools".to_string(), pools_array)]);

    let rendered = renderer.render("$config", &value);

    assert_eq!(rendered.type_name, Some("HASH".to_string()));
    assert!(rendered.value.len() < 500);
    assert!(rendered.named_variables.is_some());
}

#[test]
fn test_large_array_child_access() {
    let renderer = PerlVariableRenderer::new();

    // Create a 1000-element array
    let elements: Vec<PerlValue> = (0..1000).map(PerlValue::Integer).collect();
    let value = PerlValue::Array(elements);

    // Access specific indices
    let children_100 = renderer.render_children(&value, 100, 10);
    assert_eq!(children_100.len(), 10);
    assert_eq!(children_100[0].name, "[100]");
    assert_eq!(children_100[0].value, "100");
    assert_eq!(children_100[9].value, "109");

    let children_500 = renderer.render_children(&value, 500, 10);
    assert_eq!(children_500[0].name, "[500]");
    assert_eq!(children_500[0].value, "500");

    let children_end = renderer.render_children(&value, 990, 100);
    assert_eq!(children_end.len(), 10, "only 10 items from 990 to 1000");
    assert_eq!(children_end[9].name, "[999]");

    // Past-end start: DAP may request a page beyond the array bounds — must return empty, not panic
    let past_end = renderer.render_children(&value, 1000, 10);
    assert_eq!(past_end.len(), 0, "start == len should return empty");

    let way_past = renderer.render_children(&value, 9999, 10);
    assert_eq!(way_past.len(), 0, "start far past len should return empty");
}

#[test]
fn test_string_truncation_in_large_structures() {
    let renderer = PerlVariableRenderer::new().with_max_string_length(50);

    let long_string = "a".repeat(200);
    let value = PerlValue::Scalar(long_string);

    let rendered = renderer.render("$s", &value);

    // Should truncate at 50 characters + "..."
    assert!(
        rendered.value.contains("..."),
        "should have truncation marker"
    );
    assert!(rendered.value.len() < 100, "should be bounded");
}

#[test]
fn test_nested_hash_in_array_in_hash() {
    let renderer = PerlVariableRenderer::new();

    // { data => [ { id => 1, name => "Alice" }, { id => 2, name => "Bob" } ] }
    let person1 = PerlValue::Hash(vec![
        ("id".to_string(), PerlValue::Integer(1)),
        ("name".to_string(), PerlValue::Scalar("Alice".to_string())),
    ]);

    let person2 = PerlValue::Hash(vec![
        ("id".to_string(), PerlValue::Integer(2)),
        ("name".to_string(), PerlValue::Scalar("Bob".to_string())),
    ]);

    let people_array = PerlValue::Array(vec![person1, person2]);

    let value = PerlValue::Hash(vec![("data".to_string(), people_array)]);

    let rendered = renderer.render("$db", &value);

    assert_eq!(rendered.type_name, Some("HASH".to_string()));
    assert_eq!(rendered.named_variables, Some(1));
    assert!(rendered.value.len() < 500);
}

#[test]
fn test_object_with_deep_hash_backing() {
    let renderer = PerlVariableRenderer::new();

    // A blessed object backed by a deep hash structure
    let inner = PerlValue::Hash(vec![
        (
            "config".to_string(),
            PerlValue::Hash(vec![
                (
                    "host".to_string(),
                    PerlValue::Scalar("localhost".to_string()),
                ),
                ("port".to_string(), PerlValue::Integer(5432)),
            ]),
        ),
        (
            "state".to_string(),
            PerlValue::Scalar("connected".to_string()),
        ),
    ]);

    let value = PerlValue::Object {
        class: "Database::Connection".to_string(),
        value: Box::new(inner),
    };

    let rendered = renderer.render("$db", &value);

    assert_eq!(rendered.type_name, Some("Database::Connection".to_string()));
    assert!(rendered.value.len() < 500);
    assert!(rendered.named_variables.is_some());
}

#[test]
fn test_reference_to_large_array() {
    let renderer = PerlVariableRenderer::new();

    let elements: Vec<PerlValue> = (0..200).map(PerlValue::Integer).collect();
    let array = PerlValue::Array(elements);
    let value = PerlValue::Reference(Box::new(array));

    let rendered = renderer.render("$aref", &value);

    // Should show as REF with expandable children
    assert_eq!(rendered.type_name, Some("REF".to_string()));
    // The reference renders the dereferenced array, which shows as "[...]"
    assert!(
        rendered.value.contains("[") && rendered.value.contains("]"),
        "ref to array should show array notation, got: {}",
        rendered.value
    );
}

#[test]
fn test_empty_arrays_and_hashes_dont_truncate() {
    let renderer = PerlVariableRenderer::new();

    let empty_array = PerlValue::Array(vec![]);
    let rendered_arr = renderer.render("@empty", &empty_array);
    assert_eq!(rendered_arr.value, "[]");

    let empty_hash = PerlValue::Hash(vec![]);
    let rendered_hash = renderer.render("%empty", &empty_hash);
    assert_eq!(rendered_hash.value, "{}");
}

#[test]
fn test_undef_values_safe() {
    let renderer = PerlVariableRenderer::new();

    let value = PerlValue::Undef;
    let rendered = renderer.render("$u", &value);

    // Undef renders as "undef" string
    assert_eq!(rendered.value, "undef");
    // Type should reflect it's undefined, not SCALAR
    assert_eq!(
        rendered.type_name,
        Some("undef".to_string()),
        "undef type_name should be 'undef', not 'SCALAR'"
    );
}
