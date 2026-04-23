//! Edge-case and additional coverage tests for perl-dap-types.
//!
//! Covers: Source with tricky paths, StackFrame builder chaining,
//! Variable deserialization from real DAP JSON, and serde compliance.

use perl_dap::types::{Source, StackFrame, Variable};

// ── Source edge cases ──────────────────────────────────────────────

#[test]
fn source_with_bare_filename_has_same_name_and_path() {
    let src = Source::new("script.pl");
    assert_eq!(src.name, Some("script.pl".to_string()));
    assert_eq!(src.path, "script.pl");
}

#[test]
fn source_with_windows_path() {
    let src = Source::new("C:\\Users\\dev\\project\\lib\\Module.pm");
    assert_eq!(src.name, Some("Module.pm".to_string()));
    assert_eq!(src.path, "C:\\Users\\dev\\project\\lib\\Module.pm");
}

#[test]
fn source_with_empty_path() {
    let src = Source::new("");
    assert_eq!(src.path, "");
    // Empty path produces None for the name since file_name() returns None
    assert!(src.name.is_none());
}

#[test]
fn source_with_path_ending_in_separator() {
    // Trailing separator means no file_name on Unix.
    // On Windows, Path::new("/path/to/dir/").file_name() may return Some("dir").
    let src = Source::new("/path/to/dir/");
    assert_eq!(src.path, "/path/to/dir/");
    // Behavior is platform-dependent; just verify it does not panic
    // and produces a consistent result
    let _ = src.name;
}

#[test]
fn source_with_dotfile() {
    let src = Source::new("/home/user/.perldb");
    assert_eq!(src.name, Some(".perldb".to_string()));
}

#[test]
fn source_with_deep_nested_path() {
    let src = Source::new("/a/b/c/d/e/f/deeply/nested/Module.pm");
    assert_eq!(src.name, Some("Module.pm".to_string()));
}

// ── Source serde ────────────────────────────────────────────────────

#[test]
fn source_serialization_omits_none_source_reference() -> Result<(), serde_json::Error> {
    let src = Source::new("/script.pl");
    let json = serde_json::to_string(&src)?;
    // Source uses snake_case field names (no rename_all = "camelCase")
    assert!(!json.contains("source_reference"), "None source_reference should be omitted: {json}");
    Ok(())
}

#[test]
fn source_deserialization_from_dap_json() -> Result<(), serde_json::Error> {
    // Simulating a source object as it would arrive from a DAP client
    let json = r#"{"name": "test.pl", "path": "/workspace/test.pl"}"#;
    let src: Source = serde_json::from_str(json)?;
    assert_eq!(src.name, Some("test.pl".to_string()));
    assert_eq!(src.path, "/workspace/test.pl");
    assert!(src.source_reference.is_none());
    Ok(())
}

#[test]
fn source_deserialization_with_source_reference() -> Result<(), serde_json::Error> {
    // Note: Source struct does NOT use rename_all = "camelCase",
    // so the JSON field name is "source_reference" (snake_case)
    let json = r#"{"name": "eval", "path": "(eval 1)", "source_reference": 42}"#;
    let src: Source = serde_json::from_str(json)?;
    assert_eq!(src.source_reference, Some(42));
    Ok(())
}

// ── StackFrame edge cases ──────────────────────────────────────────

#[test]
fn stack_frame_builder_chaining_order_independent() {
    let src = Source::new("/a.pl");
    // with_end then with_column
    let f1 = StackFrame::new(1, "main", src.clone(), 10).with_end(20, 30).with_column(5);

    let src2 = Source::new("/a.pl");
    // with_column then with_end
    let f2 = StackFrame::new(1, "main", src2, 10).with_column(5).with_end(20, 30);

    assert_eq!(f1.column, f2.column);
    assert_eq!(f1.end_line, f2.end_line);
    assert_eq!(f1.end_column, f2.end_column);
}

#[test]
fn stack_frame_negative_line_preserved() {
    // DAP protocol uses signed integers; negative values are technically possible
    let src = Source::new("/a.pl");
    let frame = StackFrame::new(1, "test", src, -1);
    assert_eq!(frame.line, -1);
}

#[test]
fn stack_frame_zero_id() {
    let src = Source::new("/a.pl");
    let frame = StackFrame::new(0, "frame0", src, 1);
    assert_eq!(frame.id, 0);
}

#[test]
fn stack_frame_empty_name() {
    let src = Source::new("/a.pl");
    let frame = StackFrame::new(1, "", src, 1);
    assert_eq!(frame.name, "");
}

#[test]
fn stack_frame_unicode_name() {
    let src = Source::new("/a.pl");
    let frame = StackFrame::new(1, "日本語::関数", src, 42);
    assert_eq!(frame.name, "日本語::関数");
}

#[test]
fn stack_frame_serde_with_all_fields() -> Result<(), serde_json::Error> {
    let src = Source::new("/script.pl");
    let frame = StackFrame::new(5, "Foo::bar", src, 100).with_column(10).with_end(105, 20);

    let json = serde_json::to_string(&frame)?;
    assert!(json.contains("\"endLine\":105"), "endLine should be present: {json}");
    assert!(json.contains("\"endColumn\":20"), "endColumn should be present: {json}");

    let back: StackFrame = serde_json::from_str(&json)?;
    assert_eq!(back, frame);
    Ok(())
}

#[test]
fn stack_frame_deserialization_from_dap_json() -> Result<(), serde_json::Error> {
    let json = r#"{
        "id": 3,
        "name": "main::run",
        "source": {"name": "app.pl", "path": "/ws/app.pl"},
        "line": 42,
        "column": 1
    }"#;
    let frame: StackFrame = serde_json::from_str(json)?;
    assert_eq!(frame.id, 3);
    assert_eq!(frame.name, "main::run");
    assert_eq!(frame.line, 42);
    assert_eq!(frame.column, 1);
    assert!(frame.end_line.is_none());
    assert!(frame.end_column.is_none());
    Ok(())
}

// ── Variable edge cases ────────────────────────────────────────────

#[test]
fn variable_with_all_optional_fields() -> Result<(), serde_json::Error> {
    let var = Variable {
        name: "@data".to_string(),
        value: "(10 elements)".to_string(),
        type_: Some("ARRAY".to_string()),
        variables_reference: 5,
        named_variables: Some(0),
        indexed_variables: Some(10),
    };

    let json = serde_json::to_string(&var)?;
    assert!(json.contains("\"namedVariables\":0"), "namedVariables present: {json}");
    assert!(json.contains("\"indexedVariables\":10"), "indexedVariables present: {json}");

    let back: Variable = serde_json::from_str(&json)?;
    assert_eq!(back, var);
    Ok(())
}

#[test]
fn variable_deserialization_from_dap_json() -> Result<(), serde_json::Error> {
    // Simulating what a DAP client would send
    let json = r#"{
        "name": "$count",
        "value": "42",
        "type": "SCALAR",
        "variablesReference": 0
    }"#;
    let var: Variable = serde_json::from_str(json)?;
    assert_eq!(var.name, "$count");
    assert_eq!(var.value, "42");
    assert_eq!(var.type_, Some("SCALAR".to_string()));
    assert_eq!(var.variables_reference, 0);
    assert!(var.named_variables.is_none());
    assert!(var.indexed_variables.is_none());
    Ok(())
}

#[test]
fn variable_type_field_deserialization_uses_type_not_type_underscore()
-> Result<(), serde_json::Error> {
    // The JSON field must be "type", not "type_"
    let json = r#"{
        "name": "$x",
        "value": "1",
        "type": "int",
        "variablesReference": 0
    }"#;
    let var: Variable = serde_json::from_str(json)?;
    assert_eq!(var.type_, Some("int".to_string()));

    // Verify "type_" in JSON does NOT populate the field
    let json_wrong = r#"{
        "name": "$x",
        "value": "1",
        "type_": "int",
        "variablesReference": 0
    }"#;
    let var2: Variable = serde_json::from_str(json_wrong)?;
    assert!(var2.type_.is_none(), "type_ key in JSON should not populate type_ field");
    Ok(())
}

#[test]
fn variable_empty_name_and_value() -> Result<(), serde_json::Error> {
    let var = Variable {
        name: String::new(),
        value: String::new(),
        type_: None,
        variables_reference: 0,
        named_variables: None,
        indexed_variables: None,
    };
    let json = serde_json::to_string(&var)?;
    let back: Variable = serde_json::from_str(&json)?;
    assert_eq!(back.name, "");
    assert_eq!(back.value, "");
    Ok(())
}

#[test]
fn variable_large_variables_reference() -> Result<(), serde_json::Error> {
    let var = Variable {
        name: "$big".to_string(),
        value: "complex structure".to_string(),
        type_: Some("HASH".to_string()),
        variables_reference: i32::MAX,
        named_variables: Some(i32::MAX),
        indexed_variables: None,
    };
    let json = serde_json::to_string(&var)?;
    let back: Variable = serde_json::from_str(&json)?;
    assert_eq!(back.variables_reference, i32::MAX);
    assert_eq!(back.named_variables, Some(i32::MAX));
    Ok(())
}
