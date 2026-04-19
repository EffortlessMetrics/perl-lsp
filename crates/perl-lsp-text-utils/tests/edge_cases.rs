//! Edge-case and boundary behavior tests for TextEditHelpers.
//!
//! These complement the happy-path tests in text_edit_helpers_tests.rs by
//! targeting zero/boundary positions, missing delimiters, and the truncation
//! boundary at max_len == 3.

use perl_lsp_text_utils::TextEditHelpers;

fn helpers<'a>(source: &'a str, lines: &'a [String]) -> TextEditHelpers<'a> {
    TextEditHelpers::new(source, lines)
}

fn make_lines(source: &str) -> Vec<String> {
    source.lines().map(ToString::to_string).collect()
}

// ---------------------------------------------------------------------------
// find_statement_start
// ---------------------------------------------------------------------------

#[test]
fn find_statement_start_at_beginning_of_file_returns_zero() {
    let source = "my $x = 1;";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.find_statement_start(0), 0);
}

#[test]
fn find_statement_start_before_any_delimiter_returns_zero() {
    let source = "my $x = 1;";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // Position 3 is inside "my $x", before any ';'
    assert_eq!(h.find_statement_start(3), 0);
}

#[test]
fn find_statement_start_ignores_newline_as_boundary() {
    // Newlines are NOT statement boundaries in Perl — a multi-line expression is a
    // single statement. There is no ';' before position 9, so the result is 0.
    let source = "line one\nline two";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.find_statement_start(9), 0);
}

#[test]
fn find_statement_start_at_exact_end_of_source() {
    let source = "my $x = 1;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // ';' at byte 9 → raw=10; byte 10 is '\n' → skip → 11 == source.len().
    // The result must be at most source.len() (end-of-string insert is valid).
    let result = h.find_statement_start(source.len());
    assert_eq!(
        result, 11,
        "after skipping trailing newline, result is source.len()"
    );
    assert!(
        result <= source.len(),
        "result must not exceed source length"
    );
}

// ---------------------------------------------------------------------------
// find_pragma_insert_position
// ---------------------------------------------------------------------------

#[test]
fn pragma_insert_position_without_shebang_returns_zero() {
    let source = "use strict;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.find_pragma_insert_position(), 0);
}

#[test]
fn pragma_insert_position_empty_source_returns_zero() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.find_pragma_insert_position(), 0);
}

#[test]
fn pragma_insert_position_shebang_without_newline_returns_zero() {
    // Shebang but no newline — the `find('\n')` call returns None → fallback to 0
    let source = "#!/usr/bin/perl";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // Without a newline, can't place after shebang → returns 0
    assert_eq!(h.find_pragma_insert_position(), 0);
}

// ---------------------------------------------------------------------------
// find_import_insert_position
// ---------------------------------------------------------------------------

#[test]
fn import_insert_position_empty_source_returns_zero() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.find_import_insert_position(), 0);
}

#[test]
fn import_insert_position_no_use_or_require_returns_zero() {
    let source = "my $x = 1;\nsub foo { }\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // No 'use' or 'require' lines; non-empty non-comment line 'my $x' causes break
    assert_eq!(h.find_import_insert_position(), 0);
}

#[test]
fn import_insert_position_single_require_line() {
    let source = "require Carp;\nmy $x = 1;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // After 'require Carp;\n' = 14 chars
    let pos = h.find_import_insert_position();
    assert!(
        pos > 0,
        "import insert position should be after require line"
    );
}

#[test]
fn import_insert_position_with_comment_between_uses() {
    // Comments should not break the import block
    let source = "use strict;\n# a comment\nuse warnings;\nmy $x = 1;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    let pos = h.find_import_insert_position();
    // Should be at or after 'use warnings;\n' end
    assert!(pos >= "use strict;\n# a comment\nuse warnings;\n".len() - 1);
}

// ---------------------------------------------------------------------------
// truncate_expr
// ---------------------------------------------------------------------------

#[test]
fn truncate_expr_max_len_zero_returns_ellipsis() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // max_len <= 3 branch: returns "..."
    assert_eq!(h.truncate_expr("hello", 0), "...");
}

#[test]
fn truncate_expr_max_len_one_returns_ellipsis() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.truncate_expr("hello", 1), "...");
}

#[test]
fn truncate_expr_max_len_three_returns_ellipsis() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // Exactly 3 → still "..."
    assert_eq!(h.truncate_expr("hello", 3), "...");
}

#[test]
fn truncate_expr_max_len_four_truncates_to_one_plus_ellipsis() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // max_len=4 → take(1) + "..." = "h..."
    assert_eq!(h.truncate_expr("hello", 4), "h...");
}

#[test]
fn truncate_expr_expr_fits_within_max_len_is_unchanged() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.truncate_expr("abc", 100), "abc");
}

#[test]
fn truncate_expr_expr_exactly_at_max_len_is_unchanged() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.truncate_expr("hello", 5), "hello");
}

#[test]
fn truncate_expr_unicode_counts_by_chars_not_bytes() {
    let source = "";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // "café" = 4 chars (5 bytes); max_len=3 → "..."
    assert_eq!(h.truncate_expr("café", 3), "...");
    // max_len=10 → fits unchanged
    assert_eq!(h.truncate_expr("café", 10), "café");
}

// ---------------------------------------------------------------------------
// get_indent_at
// ---------------------------------------------------------------------------

#[test]
fn get_indent_at_position_on_first_line_no_prior_newline() {
    let source = "    my $x = 1;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // Position 4 is at 'my'; line starts at 0 (no '\n' before it)
    assert_eq!(h.get_indent_at(4), "    ");
}

#[test]
fn get_indent_at_zero_returns_empty_when_no_leading_whitespace() {
    let source = "hello\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // pos=0, no '\n' before it → line starts at 0; 'h' is not space/tab → empty indent
    assert_eq!(h.get_indent_at(0), "");
}

#[test]
fn get_indent_at_zero_returns_indent_when_line_is_indented() {
    // When the first line itself has leading whitespace, get_indent_at(0) returns it
    let source = "  hello\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.get_indent_at(0), "  ");
}

#[test]
fn get_indent_at_with_tab_indentation() {
    let source = "sub foo {\n\tmy $x = 1;\n}\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    let pos = source.find("my $x").unwrap_or(0);
    assert_eq!(h.get_indent_at(pos), "\t");
}

#[test]
fn get_indent_at_mixed_spaces_and_tabs() {
    let source = "\n    \tmy $x = 1;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    let pos = source.find("my $x").unwrap_or(0);
    let indent = h.get_indent_at(pos);
    assert_eq!(indent, "    \t");
}

#[test]
fn get_indent_at_unindented_line_returns_empty_string() {
    let source = "my $x = 1;\n    my $y = 2;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    // 'my $x' is at position 0 — no indentation
    assert_eq!(h.get_indent_at(0), "");
}

// ---------------------------------------------------------------------------
// has_non_ascii_content
// ---------------------------------------------------------------------------

#[test]
fn has_non_ascii_content_returns_false_for_pure_ascii() {
    let source = "my $x = 'hello world';";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert!(!h.has_non_ascii_content());
}

#[test]
fn has_non_ascii_content_returns_true_for_unicode_comment() {
    let source = "# Ünïcöde comment\nmy $x = 1;\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert!(h.has_non_ascii_content());
}

// ---------------------------------------------------------------------------
// lines() accessor
// ---------------------------------------------------------------------------

#[test]
fn lines_accessor_returns_the_lines_slice() {
    let source = "line one\nline two\n";
    let lines = make_lines(source);
    let h = helpers(source, &lines);
    assert_eq!(h.lines(), lines.as_slice());
}
