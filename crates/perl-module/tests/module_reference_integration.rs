use perl_module::import::{ModuleImportKind, parse_module_import_head};
use perl_module::path::module_name_to_path;
use perl_module::reference::{
    extract_module_reference, extract_module_reference_extended, find_module_reference,
    find_module_reference_extended,
};

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

// Extended reference tests for use parent / use base

#[test]
fn extended_extracts_parent_module_and_converts_to_path() {
    let line = "use parent 'Base::Class';";
    let cursor = line.find("Base::Class").unwrap_or(0);

    let reference = find_module_reference_extended(line, cursor);
    assert!(reference.is_some());
    if let Some(reference) = reference {
        let canonical = reference.canonical_module_name();
        assert_eq!(canonical, "Base::Class");
        assert_eq!(module_name_to_path(&canonical), "Base/Class.pm");
    }
}

#[test]
fn extended_extracts_base_module_and_converts_to_path() {
    let line = "use base 'Exporter';";
    let cursor = line.find("Exporter").unwrap_or(0);

    let reference = find_module_reference_extended(line, cursor);
    assert!(reference.is_some());
    if let Some(reference) = reference {
        let canonical = reference.canonical_module_name();
        assert_eq!(canonical, "Exporter");
        assert_eq!(module_name_to_path(&canonical), "Exporter.pm");
    }
}

#[test]
fn extended_aligns_with_import_head_parser_for_parent() {
    let line = "use parent 'Demo::Worker';";
    let cursor = line.find("Demo::Worker").unwrap_or(0);

    // The import head parser recognizes this as UseParent
    let parsed = parse_module_import_head(line);
    assert!(parsed.is_some());
    if let Some(parsed) = parsed {
        assert_eq!(parsed.kind, ModuleImportKind::UseParent);
    }

    // The extended reference extractor finds the module name
    let extracted = extract_module_reference_extended(line, cursor);
    assert_eq!(extracted, Some("Demo::Worker".to_string()));
}

#[test]
fn extended_handles_multiline_with_parent_statement() {
    let source = "package MyApp;\nuse parent 'Base::Class';\nuse strict;\n";
    let cursor = source.find("Base::Class").unwrap_or(0);

    assert_eq!(extract_module_reference_extended(source, cursor), Some("Base::Class".to_string()));
}

#[test]
fn extended_selects_correct_module_from_qw_list() {
    let line = "use parent qw(First::Base Second::Base);";
    let cursor = line.find("Second::Base").unwrap_or(0);

    assert_eq!(extract_module_reference_extended(line, cursor), Some("Second::Base".to_string()));
}

#[test]
fn extended_supports_legacy_ascii_separator_in_parent_qw_list() {
    let line = "use parent qw(First'Base Second'Base);";
    let cursor = line.find("Second'Base").unwrap_or(0);

    assert_eq!(extract_module_reference_extended(line, cursor), Some("Second::Base".to_string()));
}

#[test]
fn extended_falls_back_to_direct_use_when_not_parent() {
    let line = "use File::Basename;";
    let cursor = line.find("File::Basename").unwrap_or(0);

    // Both direct and extended should resolve this
    assert_eq!(extract_module_reference(line, cursor), Some("File::Basename".to_string()));
    assert_eq!(extract_module_reference_extended(line, cursor), Some("File::Basename".to_string()));
}
