//! Unit tests for source.fixAll code action
//!
//! These tests verify the `get_fix_all_actions` method on `CodeActionsProvider`
//! which produces a `SourceFixAll` action that aggregates all available quick fixes.

use perl_lsp_code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser_core::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_diag(start: usize, end: usize, code: &str, msg: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Error,
        code: Some(code.to_string()),
        message: msg.to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

fn parse_and_get_fix_all(source: &str, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    // The method we're testing - this should exist but currently doesn't
    provider.get_fix_all_actions(&ast, diagnostics)
}

// Helper to apply edits in descending order (like the production handler should)
fn apply_edits(source: &str, edits: Vec<(usize, usize, &str)>) -> String {
    let mut sorted_edits = edits;
    sorted_edits.sort_by(|a, b| b.0.cmp(&a.0)); // Descending by start position

    let mut output = source.to_string();
    for (start, end, new_text) in sorted_edits {
        output.replace_range(start..end, new_text);
    }
    output
}

// ---------------------------------------------------------------------------
// Test: get_fix_all_actions method exists and returns SourceFixAll
// ---------------------------------------------------------------------------

#[test]
fn test_get_fix_all_actions_method_exists() {
    // This test verifies the method exists on CodeActionsProvider
    let source = "use strict;\nprint $x;";
    let diags = [make_diag(12, 14, "PL103", "Undefined variable '$x'")];

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    // This will fail to compile if the method doesn't exist
    let actions = provider.get_fix_all_actions(&ast, &diags);

    // Should have exactly one SourceFixAll action
    assert_eq!(actions.len(), 1, "Expected exactly one SourceFixAll action");
    assert_eq!(actions[0].kind, CodeActionKind::SourceFixAll);
}

#[test]
fn test_source_fix_all_action_has_correct_title() {
    let source = "use strict;\nprint $x;";
    let diags = [make_diag(12, 14, "PL103", "Undefined variable '$x'")];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].title, "Fix All");
}

#[test]
fn test_source_fix_all_is_not_preferred() {
    let source = "use strict;\nprint $x;";
    let diags = [make_diag(12, 14, "PL103", "Undefined variable '$x'")];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    // SourceFixAll is informational, not the single recommended fix
    assert!(!actions[0].is_preferred, "SourceFixAll should not be preferred");
}

// ---------------------------------------------------------------------------
// AC1: SourceFixAll action is produced when fixes exist
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_produced_when_fixes_exist() {
    let source = "use strict;\nprint $x;";
    // PL103: undefined variable at bytes 12-14
    let diags = [make_diag(12, 14, "PL103", "Undefined variable '$x'")];

    let actions = parse_and_get_fix_all(source, &diags);

    assert!(!actions.is_empty(), "Expected SourceFixAll action when fixes exist");
    assert_eq!(actions[0].kind, CodeActionKind::SourceFixAll);
}

// ---------------------------------------------------------------------------
// AC2: SourceFixAll action is NOT produced when no fixes exist
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_not_produced_when_no_diagnostics() {
    let source = "use strict;\nprint 1;";
    // No diagnostics - file is clean
    let diags: [Diagnostic; 0] = [];

    let actions = parse_and_get_fix_all(source, &diags);

    assert!(actions.is_empty(), "Expected no SourceFixAll action when no diagnostics");
}

#[test]
fn test_source_fix_all_not_produced_when_diagnostics_have_no_fixes() {
    let source = "use strict;\nprint 1;";
    // Diagnostic that doesn't have an associated quick fix
    let diags = [make_diag(0, 8, "PL999", "Some unknown diagnostic")];

    let actions = parse_and_get_fix_all(source, &diags);

    assert!(actions.is_empty(), "Expected no SourceFixAll when diagnostics have no fixes");
}

// ---------------------------------------------------------------------------
// AC3: All preferred quick fixes are merged into SourceFixAll
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_merges_multiple_preferred_fixes() {
    // File with: missing strict + undefined variable
    let source = "print $x;";
    let diags = [
        make_diag(0, 9, "PL100", "Missing use strict"),
        make_diag(6, 8, "PL103", "Undefined variable '$x'"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Should have edits for both: use strict insertion + variable declaration
    assert!(
        fix_all.edit.changes.len() >= 2,
        "Expected at least 2 edits (use strict + declare variable), got {}",
        fix_all.edit.changes.len()
    );

    // Verify edits contain the expected new_text
    let edit_texts: Vec<&str> = fix_all.edit.changes.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        edit_texts.iter().any(|t| t.contains("use strict")),
        "Expected 'use strict' in merged edits"
    );
    assert!(
        edit_texts.iter().any(|t| t.contains("my") && t.contains("$x")),
        "Expected 'my $x' declaration in merged edits"
    );
}

#[test]
fn test_source_fix_all_includes_unused_variable_remove() {
    let source = "my $unused = 42;\nprint 1;";
    let diags = [make_diag(0, 20, "PL102", "Unused variable '$unused'")];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // The remove action should replace the line with empty string
    assert!(
        fix_all.edit.changes.iter().any(|e| e.new_text.is_empty()),
        "Expected at least one edit that removes text (empty new_text)"
    );
}

// ---------------------------------------------------------------------------
// AC4: Edits are sorted by descending offset
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_edits_sorted_descending() {
    // File with undefined variable (needs declaration at some position)
    // and missing strict (inserts at position 0)
    let source = "print $x;";
    let diags = [
        make_diag(0, 9, "PL100", "Missing use strict"),
        make_diag(6, 8, "PL103", "Undefined variable '$x'"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Extract start positions
    let start_positions: Vec<usize> =
        fix_all.edit.changes.iter().map(|e| e.location.start).collect();

    // Verify descending order (each position should be >= the next)
    for window in start_positions.windows(2) {
        assert!(
            window[0] >= window[1],
            "Edits should be in descending order by start position, but {} < {}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn test_source_fix_all_edits_apply_without_offset_shift() {
    // Source where applying edits in wrong order would cause issues
    let source = "a $x";
    let diags = [
        // Edit at position 0 (insert "my ")
        make_diag(0, 0, "PL103", "Undefined variable '$x'"),
        // Edit at position 3 (insert ";")
        make_diag(3, 3, "PL001", "Missing semicolon"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Collect edits as tuples for the apply function
    let edits: Vec<(usize, usize, &str)> = fix_all
        .edit
        .changes
        .iter()
        .map(|e| (e.location.start, e.location.end, e.new_text.as_str()))
        .collect();

    // Apply edits in descending order
    let result = apply_edits(source, edits);

    // If edits are properly sorted and applied, we should get valid Perl
    // The result should contain both "my $x" and the semicolon
    assert!(result.contains("my $x"), "Expected 'my $x' in result, got: {}", result);
    assert!(result.contains(';'), "Expected semicolon in result, got: {}", result);
}

// ---------------------------------------------------------------------------
// AC5: Duplicate pragmas are deduplicated
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_deduplicates_use_strict() {
    // Both PL100 and PL502 would add "use strict;"
    let source = "BEGIN { use strict; }\nprint 1;";
    let diags = [
        // PL100: missing strict
        make_diag(0, 9, "PL100", "Missing use strict"),
        // PL502: phase-scoped strict (also adds use strict)
        make_diag(0, 24, "PL502", "Phase-scoped strict pragma"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Count how many edits insert "use strict"
    let use_strict_edits: Vec<_> =
        fix_all.edit.changes.iter().filter(|e| e.new_text.contains("use strict")).collect();

    assert_eq!(
        use_strict_edits.len(),
        1,
        "Expected exactly 1 'use strict' edit after deduplication, got {}",
        use_strict_edits.len()
    );
}

#[test]
fn test_source_fix_all_deduplicates_use_warnings() {
    // Both PL101 and PL503 would add "use warnings;"
    let source = "BEGIN { use warnings; }\nprint 1;";
    let diags = [
        // PL101: missing warnings
        make_diag(0, 9, "PL101", "Missing use warnings"),
        // PL503: phase-scoped warnings (also adds use warnings)
        make_diag(0, 25, "PL503", "Phase-scoped warnings pragma"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Count how many edits insert "use warnings"
    let use_warnings_edits: Vec<_> =
        fix_all.edit.changes.iter().filter(|e| e.new_text.contains("use warnings")).collect();

    assert_eq!(
        use_warnings_edits.len(),
        1,
        "Expected exactly 1 'use warnings' edit after deduplication, got {}",
        use_warnings_edits.len()
    );
}

#[test]
fn test_source_fix_all_both_strict_and_warnings_kept_separate() {
    // PL100 (strict) and PL101 (warnings) should both be included
    let source = "print 1;";
    let diags = [
        make_diag(0, 9, "PL100", "Missing use strict"),
        make_diag(0, 9, "PL101", "Missing use warnings"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    let has_strict = fix_all.edit.changes.iter().any(|e| e.new_text.contains("use strict"));
    let has_warnings = fix_all.edit.changes.iter().any(|e| e.new_text.contains("use warnings"));

    assert!(has_strict, "Expected 'use strict' to be present");
    assert!(has_warnings, "Expected 'use warnings' to be present");
}

// ---------------------------------------------------------------------------
// AC6: Only preferred fixes are included
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_only_includes_preferred_fixes() {
    // Undefined variable has 2 options: "my" (preferred) and "our" (not preferred)
    let source = "print $x;";
    let diags = [make_diag(6, 8, "PL103", "Undefined variable '$x'")];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Should only contain the preferred fix ("Declare with my")
    // Should NOT contain the non-preferred fix ("Declare with our")
    let edit_texts: String = fix_all.edit.changes.iter().map(|e| e.new_text.as_str()).collect();

    assert!(edit_texts.contains("my"), "Expected 'my' (preferred) in fixes, got: {}", edit_texts);
    assert!(
        !edit_texts.contains("our"),
        "Should NOT contain 'our' (non-preferred) in SourceFixAll, got: {}",
        edit_texts
    );
}

#[test]
fn test_source_fix_all_assignment_in_condition_prefers_comparison() {
    // Assignment in condition has 2 options: "==" (preferred) and parentheses (not preferred)
    let source = "if ($x = 5) { }";
    let diags = [make_diag(4, 10, "PL403", "Assignment in condition")];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Should only contain the preferred fix (change to ==)
    let edit_texts: String = fix_all.edit.changes.iter().map(|e| e.new_text.as_str()).collect();

    assert!(edit_texts.contains("=="), "Expected '==' (preferred) in fixes, got: {}", edit_texts);
    assert!(
        !edit_texts.contains("()") || !edit_texts.contains("parentheses"),
        "Should NOT contain parentheses fix (non-preferred) in SourceFixAll, got: {}",
        edit_texts
    );
}

// ---------------------------------------------------------------------------
// AC7: Empty result after deduplication returns no action
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_returns_empty_when_all_fixes_deduplicated() {
    // File already has strict and warnings, only duplicate pragmas
    // If all edits are duplicates, we should get empty list
    let source = "use strict;\nuse warnings;\nprint 1;";
    let diags = [
        // Both would try to add use strict (already present)
        make_diag(0, 11, "PL100", "Missing use strict"),
        make_diag(0, 11, "PL502", "Phase-scoped strict pragma"),
        // Both would try to add use warnings (already present)
        make_diag(0, 11, "PL101", "Missing use warnings"),
        make_diag(0, 11, "PL503", "Phase-scoped warnings pragma"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    // No new fixes needed since pragmas already exist
    // After deduplication, there should be no action
    assert!(
        actions.is_empty(),
        "Expected no SourceFixAll when all fixes are redundant, got {} actions",
        actions.len()
    );
}

// ---------------------------------------------------------------------------
// Multiple diagnostics with different fix types
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_handles_mixed_fix_types() {
    // Undefined variable + missing semicolon + bareword
    let source = "my $x = foo";
    let diags = [
        // Undefined variable (will get "my $x")
        make_diag(4, 5, "PL103", "Undefined variable '$x'"),
        // Missing semicolon at end
        make_diag(11, 11, "parse-error-missingsemicolon", "Missing semicolon"),
        // Bareword
        make_diag(8, 11, "PL109", "Bareword 'foo'"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Should have at least 3 edits: declare variable, add semicolon, quote bareword
    assert!(
        fix_all.edit.changes.len() >= 3,
        "Expected at least 3 edits, got {}",
        fix_all.edit.changes.len()
    );
}

// ---------------------------------------------------------------------------
// Diagnostics referenced in action
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_references_its_diagnostics() {
    let source = "use strict;\nprint $x;";
    let diags = [
        make_diag(12, 14, "PL103", "Undefined variable '$x'"),
        make_diag(0, 11, "PL100", "Missing use strict"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // SourceFixAll should reference the diagnostics it resolves
    assert!(!fix_all.diagnostics.is_empty(), "Expected SourceFixAll to reference its diagnostics");
    assert!(
        fix_all.diagnostics.contains(&"PL103".to_string()),
        "Expected PL103 in diagnostics list"
    );
    assert!(
        fix_all.diagnostics.contains(&"PL100".to_string()),
        "Expected PL100 in diagnostics list"
    );
}

// ---------------------------------------------------------------------------
// Test that action can be applied correctly
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_action_can_be_applied() {
    let source = "print $x;";
    let diags = [
        make_diag(0, 9, "PL100", "Missing use strict"),
        make_diag(6, 8, "PL103", "Undefined variable '$x'"),
    ];

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Apply the edits in descending order
    let edits: Vec<(usize, usize, &str)> = fix_all
        .edit
        .changes
        .iter()
        .map(|e| (e.location.start, e.location.end, e.new_text.as_str()))
        .collect();

    let result = apply_edits(source, edits);

    // Result should be valid Perl with strict and declared variable
    assert!(result.contains("use strict"), "Expected 'use strict' in result");
    assert!(
        result.contains("my $x") || result.contains("our $x"),
        "Expected variable declaration in result"
    );
    assert!(result.contains(';'), "Expected semicolon in result");
}

// ---------------------------------------------------------------------------
// Performance: doesn't iterate diagnostics twice unnecessarily
// ---------------------------------------------------------------------------

#[test]
fn test_source_fix_all_with_large_number_of_diagnostics() {
    // Create a source with many variables
    let source = "use strict;\nprint $a $b $c $d $e $f $g $h;";
    let mut diags = Vec::new();

    // Add undefined variable diagnostics for each
    for (i, var) in ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'].iter().enumerate() {
        let pos = 12 + (i * 3); // Approximate positions
        let msg = format!("Undefined variable '${}'", var);
        diags.push(make_diag(pos, pos + 2, "PL103", &msg));
    }

    let actions = parse_and_get_fix_all(source, &diags);

    assert_eq!(actions.len(), 1);
    let fix_all = &actions[0];

    // Should have one edit per variable declaration (8 declarations)
    // Plus possibly use strict if missing
    assert!(
        fix_all.edit.changes.len() >= 8,
        "Expected at least 8 edits for variable declarations, got {}",
        fix_all.edit.changes.len()
    );
}
