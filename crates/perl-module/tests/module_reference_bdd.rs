use perl_module::reference::{
    ModuleReferenceKind, extract_module_reference, find_module_reference,
};

#[test]
fn given_use_statement_when_cursor_is_inside_module_then_reference_is_extracted() {
    let line = "use Demo::Worker;";
    let cursor = line.find("Worker").unwrap_or(0) + 2;

    let reference = find_module_reference(line, cursor);
    assert!(reference.is_some());
    if let Some(reference) = reference {
        assert_eq!(reference.kind, ModuleReferenceKind::Use);
        assert_eq!(reference.module_name, "Demo::Worker");
        assert_eq!(extract_module_reference(line, cursor), Some("Demo::Worker".to_string()));
    }
}

#[test]
fn given_require_statement_when_cursor_is_inside_module_then_reference_is_extracted() {
    let line = "require Demo::Worker;";
    let cursor = line.find("Demo").unwrap_or(0) + 1;

    assert_eq!(extract_module_reference(line, cursor), Some("Demo::Worker".to_string()));
}

#[test]
fn given_legacy_separator_when_cursor_is_inside_module_then_name_is_canonicalized() {
    let line = "use Demo'Worker;";
    let cursor = line.find("Worker").unwrap_or(0) + 1;

    assert_eq!(extract_module_reference(line, cursor), Some("Demo::Worker".to_string()));
}

#[test]
fn given_parent_statement_when_cursor_is_inside_argument_then_no_reference_is_extracted() {
    let line = "use parent 'Demo::Worker';";
    let cursor = line.find("Worker").unwrap_or(0);

    assert_eq!(extract_module_reference(line, cursor), None);
}

#[test]
fn given_non_import_context_when_cursor_is_inside_module_like_text_then_no_reference_is_extracted()
{
    let line = "my $x = Demo::Worker->new();";
    let cursor = line.find("Worker").unwrap_or(0);

    assert_eq!(extract_module_reference(line, cursor), None);
}
