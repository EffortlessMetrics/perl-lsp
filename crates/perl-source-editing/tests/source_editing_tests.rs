use perl_source_editing::{
    find_import_insert_position, find_pragma_insert_position, find_statement_start, get_indent_at,
    has_non_ascii_content, truncate_expr,
};

#[test]
fn finds_statement_start_after_semicolon() {
    let source = "my $x = 1;\nmy $y = 2;";
    let pos = source.find("$y").unwrap_or(source.len());
    assert_eq!(find_statement_start(source, pos), 11);
}

#[test]
fn extracts_indent_for_line() {
    let source = "sub test {\n    my $x = 1;\n}\n";
    let pos = source.find("my").unwrap_or(0);
    assert_eq!(get_indent_at(source, pos), "    ");
}

#[test]
fn shebang_sets_pragma_insert_position() {
    let source = "#!/usr/bin/env perl\nuse strict;\n";
    assert_eq!(find_pragma_insert_position(source), 20);
}

#[test]
fn imports_insert_after_existing_imports() {
    let source = "#!/usr/bin/env perl\nuse strict;\nuse warnings;\nmy $x = 1;\n";
    let lines = source.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    assert_eq!(find_import_insert_position(source, &lines), 46);
}

#[test]
fn truncates_small_limits_safely() {
    assert_eq!(truncate_expr("abcdef", 3), "...");
    assert_eq!(truncate_expr("abcdef", 2), "..");
    assert_eq!(truncate_expr("abcdef", 1), ".");
}

#[test]
fn detects_non_ascii_content() {
    assert!(has_non_ascii_content("my $x = 'é';"));
    assert!(!has_non_ascii_content("my $x = 'e';"));
}
