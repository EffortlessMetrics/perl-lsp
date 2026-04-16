use perl_dap::types::{Source, StackFrame, Variable};

#[test]
fn source_derives_name_from_path() {
    let source = Source::new("/tmp/example.pl");

    assert_eq!(source.name.as_deref(), Some("example.pl"));
    assert_eq!(source.path, "/tmp/example.pl");
    assert_eq!(source.source_reference, None);
}

#[test]
fn stack_frame_builders_preserve_state() {
    let frame = StackFrame::new(7, "main::run", Source::new("/tmp/example.pl"), 42)
        .with_column(3)
        .with_end(44, 9);

    assert_eq!(frame.id, 7);
    assert_eq!(frame.name, "main::run");
    assert_eq!(frame.source.name.as_deref(), Some("example.pl"));
    assert_eq!(frame.column, 3);
    assert_eq!(frame.end_line, Some(44));
    assert_eq!(frame.end_column, Some(9));
}

#[test]
fn variable_serializes_type_field() -> Result<(), serde_json::Error> {
    let variable = Variable {
        name: "$answer".to_string(),
        value: "42".to_string(),
        type_: Some("scalar".to_string()),
        variables_reference: 0,
        named_variables: None,
        indexed_variables: None,
    };

    let json = serde_json::to_string(&variable)?;
    assert!(json.contains("\"type\":\"scalar\""));
    Ok(())
}
