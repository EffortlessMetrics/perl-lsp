//! Tests for tabSize plumbing and POD block suppression.
//!
//! These tests are RED until:
//! 1. `compute_on_type_edit` gains an `indent_step: usize` parameter (Change 1)
//! 2. `is_inside_pod` suppression is added (Change 2)

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

// ==================================================================
//  Change 1: tabSize plumbing
//  compute_on_type_edit gains an `indent_step: usize` trailing arg.
// ==================================================================

#[test]
fn newline_after_open_brace_respects_tab_size_4() {
    // With indent_step=4, pressing Enter after `{` should give 4 spaces.
    let text = "if ($x) {\n";
    let edits = compute_on_type_edit(text, 1, 0, '\n', 4);
    assert!(edits.is_some(), "should produce indent edits");
    assert_eq!(
        first_new_text(&edits).as_deref(),
        Some("    "),
        "indent_step=4 should produce 4 spaces"
    );
}

#[test]
fn newline_after_open_brace_respects_tab_size_2() {
    // With indent_step=2, pressing Enter after `{` should give 2 spaces.
    let text = "if ($x) {\n";
    let edits = compute_on_type_edit(text, 1, 0, '\n', 2);
    assert!(edits.is_some(), "should produce indent edits");
    assert_eq!(
        first_new_text(&edits).as_deref(),
        Some("  "),
        "indent_step=2 should produce 2 spaces"
    );
}

#[test]
fn newline_after_open_brace_respects_tab_size_1() {
    // Edge: indent_step=1 — single-space indent.
    let text = "if ($x) {\n";
    let edits = compute_on_type_edit(text, 1, 0, '\n', 1);
    assert!(edits.is_some(), "should produce indent edits");
    assert_eq!(
        first_new_text(&edits).as_deref(),
        Some(" "),
        "indent_step=1 should produce 1 space"
    );
}

#[test]
fn newline_after_open_brace_respects_tab_size_8() {
    // Edge: indent_step=8 — large indent step.
    let text = "if ($x) {\n";
    let edits = compute_on_type_edit(text, 1, 0, '\n', 8);
    assert!(edits.is_some(), "should produce indent edits");
    assert_eq!(
        first_new_text(&edits).as_deref(),
        Some("        "),
        "indent_step=8 should produce 8 spaces"
    );
}

#[test]
fn close_brace_realigns_with_tab_size_4() {
    // `}` on line 2 is at column 8 (8 spaces), opener at column 0.
    // With tab_size=4, close brace should realign to column 0.
    let text = "if ($x) {\n    my $y = 1;\n        }";
    let edits = compute_on_type_edit(text, 2, 9, '}', 4);
    assert!(edits.is_some(), "should produce realign edit");
    assert_eq!(
        first_new_text(&edits).as_deref(),
        Some(""),
        "close brace should align to opener at column 0"
    );
}

#[test]
fn close_brace_fallback_saturating_sub_uses_indent_step() {
    // No opener found; fallback is current_indent.saturating_sub(indent_step).
    // current_indent=4, indent_step=4 → target=0, so edit emits "".
    let text = "    }";
    let edits = compute_on_type_edit(text, 0, 5, '}', 4);
    assert!(edits.is_some(), "should emit edit when indent changes");
    assert_eq!(
        first_new_text(&edits).as_deref(),
        Some(""),
        "saturating_sub with step=4 on indent=4 should give 0"
    );
}

// ==================================================================
//  Change 2: POD block suppression
//  Typing inside POD (=head1 ... =cut) should return None.
// ==================================================================

#[test]
fn pod_head1_suppresses_close_brace() {
    // `}` typed on line 2, which is inside a POD block.
    let text = "=head1 NAME\n\n}\n\n=cut\n";
    let edits = compute_on_type_edit(text, 2, 1, '}', 2);
    assert!(
        edits.is_none(),
        "typing `}}` inside POD should be suppressed"
    );
}

#[test]
fn pod_head1_suppresses_newline() {
    // `\n` typed while inside POD — must not produce indent edits.
    let text = "=head1 NAME\nSome description {\n\n=cut\n";
    let edits = compute_on_type_edit(text, 2, 0, '\n', 2);
    assert!(
        edits.is_none(),
        "pressing Enter inside POD should be suppressed"
    );
}

#[test]
fn pod_body_suppresses_semicolon() {
    // `;` inside POD — must return None (same as heredoc behavior).
    let text = "=pod\n\nsome text;\n\n=cut\n";
    let edits = compute_on_type_edit(text, 2, 10, ';', 2);
    assert!(
        edits.is_none(),
        "typing `;` inside POD should be suppressed"
    );
}

#[test]
fn after_cut_formatting_resumes() {
    // After `=cut`, formatting must resume normally.
    // Line 3 is `    }` — after the POD block.
    let text = "=pod\nsome text\n=cut\n    }";
    let edits = compute_on_type_edit(text, 3, 5, '}', 2);
    // After POD ends, formatting resumes. No opener, falls back to
    // current_indent.saturating_sub(indent_step) = 4 - 2 = 2.
    assert!(
        edits.is_some(),
        "after =cut, formatting should resume (not be suppressed)"
    );
}

#[test]
fn pod_begin_end_suppresses_content() {
    // =begin / =end also delimit a POD block.
    let text = "=begin html\n\n<b>bold</b> {\n\n=end html\n\n=cut\n";
    let edits = compute_on_type_edit(text, 2, 13, '}', 2);
    assert!(
        edits.is_none(),
        "typing `}}` inside =begin..=end block should be suppressed"
    );
}

#[test]
fn pod_at_file_start_line_zero() {
    // POD that starts immediately at line 0 (no preceding code).
    let text = "=head1 Overview\n}\n=cut\n";
    let edits = compute_on_type_edit(text, 1, 1, '}', 2);
    assert!(
        edits.is_none(),
        "POD starting at line 0 should still suppress formatting on line 1"
    );
}

#[test]
fn pod_cut_with_trailing_whitespace_terminates() {
    // `=cut` followed by trailing whitespace must still end the POD block.
    let text = "=pod\nsome text\n=cut   \n    }";
    let edits = compute_on_type_edit(text, 3, 5, '}', 2);
    assert!(
        edits.is_some(),
        "=cut with trailing whitespace should still terminate POD; formatting resumes on line 3"
    );
}

#[test]
fn pod_end_keyword_terminates_begin_block() {
    // =end terminates a =begin block (without needing =cut on its own).
    let text = "=begin text\nsome content {\n=end text\n    }";
    let edits = compute_on_type_edit(text, 3, 5, '}', 2);
    assert!(edits.is_some(), "after =end, formatting should resume");
}
