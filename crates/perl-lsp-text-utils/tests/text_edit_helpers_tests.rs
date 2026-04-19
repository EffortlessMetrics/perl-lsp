use perl_lsp_text_utils::TextEditHelpers;

#[test]
fn find_statement_start_does_not_break_on_newline() {
    // A multi-line call: the newline after '(' is NOT a statement boundary.
    // Extracting $arg1 should find the statement starts at byte 0, not after the '\n'.
    let source = "my $result = some_func(\n    $arg1,\n    $arg2\n);\n";
    let lines: Vec<String> = source.lines().map(ToString::to_string).collect();
    let helpers = TextEditHelpers::new(source, &lines);

    // $arg1 starts at byte 28 (after "my $result = some_func(\n    ")
    let arg1_pos = source.find("$arg1").unwrap_or(0);
    assert_eq!(arg1_pos, 28, "test precondition: $arg1 is at byte 28");

    let start = helpers.find_statement_start(arg1_pos);
    // There is no ';' before $arg1, so the statement starts at the beginning of the source.
    assert_eq!(
        start, 0,
        "newline inside multi-line expression should not be treated as statement boundary"
    );
}

#[test]
fn finds_statement_start_after_semicolon() {
    // source: "my $a = 1;\nmy $b = length($x);\n"
    //            byte 9=';', byte 10='\n', byte 11='m'
    // find_statement_start sees the ';' at 9 → raw = 10, then skips the '\n'
    // at byte 10, returning 11 — the first real character of the next statement.
    // This ensures the extracted declaration is inserted on its own line, not
    // appended to the end of "my $a = 1;".
    let source = "my $a = 1;\nmy $b = length($x);\n";
    let lines: Vec<String> = source.lines().map(ToString::to_string).collect();
    let helpers = TextEditHelpers::new(source, &lines);

    let pos = source.find("length").unwrap_or(0);
    assert_eq!(helpers.find_statement_start(pos), 11);
}

#[test]
fn finds_pragma_and_import_insert_positions() {
    let source = "#!/usr/bin/env perl\nuse strict;\nuse warnings;\nmy $x = 1;\n";
    let lines: Vec<String> = source.lines().map(ToString::to_string).collect();
    let helpers = TextEditHelpers::new(source, &lines);

    assert_eq!(helpers.find_pragma_insert_position(), 20);
    assert_eq!(helpers.find_import_insert_position(), 46);
}

#[test]
fn finds_subroutine_insert_position_or_end() {
    let source = "my $x = 1;\nsub alpha {\n    return 1;\n}\n";
    let lines: Vec<String> = source.lines().map(ToString::to_string).collect();
    let helpers = TextEditHelpers::new(source, &lines);

    assert_eq!(helpers.find_subroutine_insert_position(source.len()), 11);

    let source_no_sub = "my $x = 1;\n";
    let lines_no_sub: Vec<String> = source_no_sub.lines().map(ToString::to_string).collect();
    let helpers_no_sub = TextEditHelpers::new(source_no_sub, &lines_no_sub);
    assert_eq!(
        helpers_no_sub.find_subroutine_insert_position(source_no_sub.len()),
        source_no_sub.len()
    );
}

#[test]
fn indentation_truncation_and_non_ascii() {
    let source = "if (1) {\n    my $x = 3;\n}\n";
    let lines: Vec<String> = source.lines().map(ToString::to_string).collect();
    let helpers = TextEditHelpers::new(source, &lines);

    let pos = source.find("my $x").unwrap_or(0);
    assert_eq!(helpers.get_indent_at(pos), "    ");
    assert_eq!(
        helpers.truncate_expr("abcdefghijklmnopqrstuvwxyz", 10),
        "abcdefg..."
    );
    assert!(!helpers.has_non_ascii_content());

    let non_ascii_source = "say \"café\";";
    let non_ascii_lines: Vec<String> = non_ascii_source.lines().map(ToString::to_string).collect();
    let non_ascii_helpers = TextEditHelpers::new(non_ascii_source, &non_ascii_lines);
    assert!(non_ascii_helpers.has_non_ascii_content());
}
