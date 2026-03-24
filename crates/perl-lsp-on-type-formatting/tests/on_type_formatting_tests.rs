use perl_lsp_on_type_formatting::compute_on_type_edit;
use serde_json::Value;

// ------------------------------------------------------------------
// Helper: extract the "newText" from the first edit in the result.
// ------------------------------------------------------------------
fn first_new_text(edits: &Option<Vec<Value>>) -> Option<String> {
    edits
        .as_ref()
        .and_then(|v| v.first())
        .and_then(|e| e.get("newText"))
        .and_then(|t| t.as_str())
        .map(String::from)
}

fn first_edit_range(edits: &Option<Vec<Value>>) -> Option<(u64, u64, u64, u64)> {
    edits.as_ref().and_then(|v| v.first()).and_then(|e| {
        let r = e.get("range")?;
        let sl = r.pointer("/start/line")?.as_u64()?;
        let sc = r.pointer("/start/character")?.as_u64()?;
        let el = r.pointer("/end/line")?.as_u64()?;
        let ec = r.pointer("/end/character")?.as_u64()?;
        Some((sl, sc, el, ec))
    })
}

// ==================================================================
//  `}` — auto-indent to match opening `{`
// ==================================================================

#[test]
fn close_brace_aligns_to_opening_brace() {
    // The `}` on line 2 is indented 4 spaces, but the opening `{` is
    // on a line with 0-space indent, so `}` should be re-indented to 0.
    let text = "if ($x) {\n  print 1;\n    }";
    let edits = compute_on_type_edit(text, 2, 5, '}', 2);
    assert!(edits.is_some());
    let new_text = first_new_text(&edits);
    assert_eq!(new_text.as_deref(), Some(""));
    // Range should replace the leading 4 spaces.
    assert_eq!(first_edit_range(&edits), Some((2, 0, 2, 4)));
}

#[test]
fn close_brace_already_correct_indentation() {
    let text = "if ($x) {\n  print 1;\n}";
    let edits = compute_on_type_edit(text, 2, 1, '}', 2);
    // Already at correct indent (0), so no edits.
    assert!(edits.is_none());
}

#[test]
fn close_brace_nested_blocks() {
    let text = "sub foo {\n  if ($x) {\n    print 1;\n      }\n}";
    // Line 3: "      }" has 6 spaces, should align to "  if ($x) {" = 2 spaces.
    let edits = compute_on_type_edit(text, 3, 7, '}', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
    assert_eq!(first_edit_range(&edits), Some((3, 0, 3, 6)));
}

#[test]
fn close_brace_on_first_line_uses_saturating_sub() {
    // `}` on line 0 with no matching opener — falls back to saturating_sub.
    let text = "  }";
    let edits = compute_on_type_edit(text, 0, 3, '}', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some(""));
}

#[test]
fn close_brace_skips_braces_in_strings() {
    // The `{` inside the string should not count as the matching opener.
    let text = "my $x = \"{\";\nif ($y) {\n  1;\n    }";
    let edits = compute_on_type_edit(text, 3, 5, '}', 2);
    assert!(edits.is_some());
    // Should align to line 1 ("if ($y) {") which has indent 0.
    assert_eq!(first_new_text(&edits).as_deref(), Some(""));
}

#[test]
fn close_brace_skips_braces_in_comments() {
    // The `{` in the comment should be ignored.
    let text = "# {\nif ($y) {\n  1;\n    }";
    let edits = compute_on_type_edit(text, 3, 5, '}', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some(""));
}

// ==================================================================
//  `;` — does not change indentation
// ==================================================================

#[test]
fn semicolon_does_not_change_indentation() {
    let text = "  my $x = 1;";
    let edits = compute_on_type_edit(text, 0, 12, ';', 2);
    assert!(edits.is_none());
}

#[test]
fn semicolon_at_end_of_indented_line_no_edit() {
    let text = "sub foo {\n    my $x = 1;";
    let edits = compute_on_type_edit(text, 1, 14, ';', 2);
    assert!(edits.is_none());
}

#[test]
fn semicolon_preserves_deeper_indentation() {
    let text = "        return 1;";
    let edits = compute_on_type_edit(text, 0, 17, ';', 2);
    assert!(edits.is_none());
}

// ==================================================================
//  `\n` — indent new line after `{`
// ==================================================================

#[test]
fn newline_after_open_brace_indents() {
    let text = "if ($x) {\n";
    let edits = compute_on_type_edit(text, 1, 0, '\n', 2);
    assert!(edits.is_some());
    // New line should get 2-space indent (0 + 2).
    assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
}

#[test]
fn newline_after_indented_open_brace() {
    let text = "  sub foo {\n";
    let edits = compute_on_type_edit(text, 1, 0, '\n', 2);
    assert!(edits.is_some());
    // Previous line has 2-space indent, so new line gets 4.
    assert_eq!(first_new_text(&edits).as_deref(), Some("    "));
}

#[test]
fn newline_after_plain_statement_keeps_indent() {
    let text = "    my $x = 1;\n";
    let edits = compute_on_type_edit(text, 1, 0, '\n', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some("    "));
}

#[test]
fn newline_after_close_brace_keeps_its_indent() {
    let text = "  if ($x) {\n    1;\n  }\n";
    // Line 3 is after "  }", which has 2-space indent.
    let edits = compute_on_type_edit(text, 3, 0, '\n', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
}

#[test]
fn newline_on_first_line_returns_none() {
    let text = "\n";
    let edits = compute_on_type_edit(text, 0, 0, '\n', 2);
    assert!(edits.is_none());
}

#[test]
fn newline_replaces_existing_wrong_indent() {
    let text = "if ($x) {\n      ";
    // Line 1 has 6-space indent but should be 2.
    let edits = compute_on_type_edit(text, 1, 6, '\n', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
    assert_eq!(first_edit_range(&edits), Some((1, 0, 1, 6)));
}

// ==================================================================
//  Heredoc — formatting is disabled
// ==================================================================

#[test]
fn heredoc_suppresses_close_brace() {
    let text = "my $x = <<END;\n}\nEND";
    let edits = compute_on_type_edit(text, 1, 1, '}', 2);
    assert!(edits.is_none());
}

#[test]
fn heredoc_suppresses_semicolon() {
    let text = "my $x = <<END;\nsome text;\nEND";
    let edits = compute_on_type_edit(text, 1, 10, ';', 2);
    assert!(edits.is_none());
}

#[test]
fn heredoc_suppresses_newline() {
    let text = "my $x = <<END;\nsome text\n\nEND";
    let edits = compute_on_type_edit(text, 2, 0, '\n', 2);
    assert!(edits.is_none());
}

#[test]
fn heredoc_single_quoted_tag() {
    let text = "my $x = <<'END';\n}\nEND";
    let edits = compute_on_type_edit(text, 1, 1, '}', 2);
    assert!(edits.is_none());
}

#[test]
fn heredoc_double_quoted_tag() {
    let text = "my $x = <<\"END\";\n}\nEND";
    let edits = compute_on_type_edit(text, 1, 1, '}', 2);
    assert!(edits.is_none());
}

#[test]
fn heredoc_indented_form() {
    let text = "my $x = <<~END;\n  }\n  END";
    let edits = compute_on_type_edit(text, 1, 3, '}', 2);
    assert!(edits.is_none());
}

#[test]
fn after_heredoc_terminator_formatting_resumes() {
    let text = "my $x = <<END;\nheredoc body\nEND\n    }";
    // Line 3 ("    }") is after the heredoc ends, so formatting should work.
    let edits = compute_on_type_edit(text, 3, 5, '}', 2);
    // No matching opener, so falls back to saturating_sub: 4 - 2 = 2.
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
}

// ==================================================================
//  Edge cases
// ==================================================================

#[test]
fn out_of_bounds_line_is_none() {
    let edits = compute_on_type_edit("", 42, 0, '}', 2);
    assert!(edits.is_none());
}

#[test]
fn returns_none_for_unknown_trigger() {
    let text = "my $x = 1;";
    let edits = compute_on_type_edit(text, 0, 5, 'a', 2);
    assert!(edits.is_none());
}

#[test]
fn empty_document_newline() {
    let edits = compute_on_type_edit("\n", 0, 0, '\n', 2);
    assert!(edits.is_none());
}

#[test]
fn multiple_heredocs_on_one_line() {
    // Two heredocs on one line: body of first, then body of second.
    let text = "my ($a, $b) = (<<A, <<B);\nfirst\nA\nsecond\nB\nnormal;";
    // Line 1 ("first") is inside heredoc A.
    assert!(compute_on_type_edit(text, 1, 5, ';', 2).is_none());
    // Line 3 ("second") is inside heredoc B.
    assert!(compute_on_type_edit(text, 3, 6, ';', 2).is_none());
    // Line 5 ("normal;") is after both heredocs.
    assert!(compute_on_type_edit(text, 5, 7, ';', 2).is_none()); // `;` is always None
}

// ==================================================================
//  Regex quantifiers — `}` must not be misidentified as a block closer
// ==================================================================

#[test]
fn close_brace_after_regex_quantifier_does_not_misindent() {
    // The `{3}` in the regex is a quantifier, not a block. The real block
    // opener is `{` on line 0. Typing `}` on line 2 should align with line 0.
    let text = "if ($str =~ /\\w{3}/) {\n  do_something();\n    }";
    let edits = compute_on_type_edit(text, 2, 5, '}', 2);
    assert!(edits.is_some());
    // Should align to line 0 ("if ...") which has indent 0.
    assert_eq!(first_new_text(&edits).as_deref(), Some(""));
}

#[test]
fn close_brace_after_regex_range_quantifier_does_not_misindent() {
    // The `{2,5}` is a range quantifier; the block opener is on line 0.
    let text = "if ($x =~ /\\d{2,5}/) {\n  ok();\n    }";
    let edits = compute_on_type_edit(text, 2, 5, '}', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some(""));
}

#[test]
fn close_brace_with_open_range_quantifier_does_not_misindent() {
    // {2,} is an open-ended quantifier.
    let text = "if ($x =~ /a{2,}/) {\n  ok();\n    }";
    let edits = compute_on_type_edit(text, 2, 5, '}', 2);
    assert!(edits.is_some());
    assert_eq!(first_new_text(&edits).as_deref(), Some(""));
}

// ==================================================================
//  qw{} — braces inside word-list must not affect indent tracking
// ==================================================================

#[test]
fn close_brace_after_qw_block_does_not_misindent() {
    // The `qw{foo bar}` braces should be ignored; the block opener is on line 0.
    let text = "foreach my $x (qw{foo bar}) {\n  use($x);\n    }";
    let edits = compute_on_type_edit(text, 2, 5, '}', 2);
    assert!(edits.is_some());
    // Should align to line 0 which has indent 0.
    assert_eq!(first_new_text(&edits).as_deref(), Some(""));
}

#[test]
fn qw_block_already_correct_indent_no_edit() {
    // When the `}` is already at the right indent, no edit should be emitted.
    let text = "foreach my $x (qw{a b c}) {\n  use($x);\n}";
    let edits = compute_on_type_edit(text, 2, 1, '}', 2);
    assert!(edits.is_none());
}
