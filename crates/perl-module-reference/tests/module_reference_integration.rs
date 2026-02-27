use perl_module_import::{ModuleImportKind, parse_module_import_head};
use perl_module_path::module_name_to_path;
use perl_module_reference::{extract_module_reference, find_module_reference};

#[test]
fn extracted_module_aligns_with_import_head_parser() {
    let line = "use Demo::Worker;";
    let cursor = line.find("Worker").unwrap_or(0);

    let extracted = extract_module_reference(line, cursor);
    let parsed = parse_module_import_head(line);

    assert_eq!(extracted, Some("Demo::Worker".to_string()));
    assert!(parsed.is_some());
    if let Some(parsed) = parsed {
        assert_eq!(parsed.kind, ModuleImportKind::Use);
        assert_eq!(parsed.token, "Demo::Worker");
    }
}

#[test]
fn extracted_module_name_converts_to_expected_module_path() {
    let line = "require Demo'Worker;";
    let cursor = line.find("Worker").unwrap_or(0);

    let reference = find_module_reference(line, cursor);
    assert!(reference.is_some());
    if let Some(reference) = reference {
        let canonical = reference.canonical_module_name();
        assert_eq!(canonical, "Demo::Worker");
        assert_eq!(module_name_to_path(&canonical), "Demo/Worker.pm");
    }
}

#[test]
fn multiline_cursor_lookup_resolves_line_local_reference_only() {
    let source = "package Demo::App;\nuse Demo::Worker;\nmy $x = 1;\n";
    let worker_cursor = source.find("Worker").unwrap_or(0);
    let package_cursor = source.find("Demo::App").unwrap_or(0);

    assert_eq!(extract_module_reference(source, worker_cursor), Some("Demo::Worker".to_string()));
    assert_eq!(extract_module_reference(source, package_cursor), None);
}
