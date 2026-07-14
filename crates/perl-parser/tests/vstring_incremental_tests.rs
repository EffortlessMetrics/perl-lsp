//! ripr+ gap 2: IncrementalDocument must produce NodeKind::VString (not
//! NodeKind::String) when editing a document that contains a v-string literal.
//!
//! These tests exercise the NodeKind::VString arms added in `incremental_document.rs`:
//!   - `is_single_token_edit` (line ~345): covered by `vstring_edit_inside_literal`
//!   - `hash_node` (line ~926): covered by both tests (cache_node runs on every node)
//!   - `get_symbol_priority` (line ~1042): covered by both tests (cache_node runs on every node)
//!
//! All tests FAIL if the PR's fix to `primary.rs`
//! (TokenKind::VString → NodeKind::VString) were reverted: the sexp would
//! contain `(string "v1.2.3")` instead of `(vstring "v1.2.3")`.
//!
//! CI feature detection: `feature_cfgs_in_source` scans for inner `#![cfg(...]` lines
//! to auto-add `--features incremental` to the coverage command.
#![cfg(feature = "incremental")]

use perl_parser::incremental::incremental_document::IncrementalDocument;
use perl_parser::incremental::incremental_edit::IncrementalEdit;
use perl_tdd_support::must;

/// Parse a VString document and verify VString is emitted as NodeKind::VString.
///
/// Exercises the `hash_node` VString arm (incremental_document.rs ~line 926) and
/// the `get_symbol_priority` VString arm (~line 1042), both called from `cache_node`
/// during `IncrementalDocument::new`.
///
/// The test FAILS if `primary.rs`'s VString arm is reverted because
/// `(vstring "v1.2.3")` would become `(string "v1.2.3")`.
#[test]
fn vstring_initial_parse_produces_vstring_node() {
    let source = "my $v = v1.2.3;".to_string();
    let doc = must(IncrementalDocument::new(source));

    let sexp = doc.root.to_sexp();
    assert!(
        sexp.contains("(vstring \"v1.2.3\")"),
        "IncrementalDocument::new must produce NodeKind::VString for v1.2.3; \
         reverting primary.rs VString arm would produce (string \"v1.2.3\") instead. \
         Got sexp: {}",
        sexp
    );
    assert!(
        !sexp.contains("(string \"v1.2.3\")"),
        "v-string must not be represented as NodeKind::String; got sexp: {}",
        sexp
    );
}

/// Apply an edit AFTER the VString and verify the VString is preserved.
///
/// This exercises the `hash_node` and `get_symbol_priority` VString arms via
/// `cache_subtrees` (called after `apply_edit`), and drives `find_reusable_subtrees`
/// to correctly identify the VString as OUTSIDE the edit range.
///
/// The test FAILS if `primary.rs`'s VString arm is reverted because after the
/// incremental reparse the node would be `(string "v1.2.3")`.
#[test]
fn vstring_node_survives_incremental_edit_outside_literal() {
    // `my $v = v1.2.3;`
    //  0123456789012345
    //          ^------^  VString "v1.2.3" at bytes 8..14
    //                 ^--- semicolon at byte 14, source length = 15
    let source = "my $v = v1.2.3;".to_string();
    let mut doc = must(IncrementalDocument::new(source.clone()));

    // Verify initial parse produces VString (baseline sanity).
    let initial_sexp = doc.root.to_sexp();
    assert!(
        initial_sexp.contains("(vstring \"v1.2.3\")"),
        "initial parse must produce NodeKind::VString, got: {}",
        initial_sexp
    );

    // Apply an edit that appends a comment after the statement.
    // byte 15 = one past the semicolon = end of source = valid insertion point.
    // This edit is OUTSIDE the VString (8..14), so the VString is reusable.
    // After the edit the source becomes: `my $v = v1.2.3; # ok`
    let after_semicolon = source.len(); // byte 15
    let edit = IncrementalEdit::new(after_semicolon, after_semicolon, " # ok".to_string());
    must(doc.apply_edit(edit));

    // After incremental reparse the VString literal must still be NodeKind::VString.
    // Reverting the primary.rs VString arm causes `v1.2.3` to become NodeKind::String.
    let updated_sexp = doc.root.to_sexp();
    assert!(
        updated_sexp.contains("(vstring \"v1.2.3\")"),
        "after incremental edit, NodeKind::VString must be preserved; \
         reverting primary.rs VString arm would produce (string ...) here. \
         Got sexp: {}",
        updated_sexp
    );
    assert!(
        !updated_sexp.contains("(string \"v1.2.3\")"),
        "v-string must not be downgraded to NodeKind::String after incremental edit; \
         got sexp: {}",
        updated_sexp
    );
}

/// Apply an edit with start_byte INSIDE the VString literal.
///
/// This specifically exercises the `is_single_token_edit` VString arm
/// (incremental_document.rs ~line 345).  `find_node_at_position(9)` walks the
/// AST and returns the VString node at bytes 8..14.  The match then hits the
/// `NodeKind::VString { .. }` arm, returning `true`.
///
/// The fast-path update rewrites the VString node in place. The result is a
/// v-string containing the inserted character.
///
/// Non-vacuous guarantee: the sexp must contain `(vstring "v10.2.3")` — the
/// v-string expanded by the insertion.  Reverting the primary.rs VString arm
/// would produce `(string "v10.2.3")` instead.
#[test]
fn vstring_edit_inside_literal_exercises_is_single_token_edit_arm() {
    // `my $v = v1.2.3;`
    //  0123456789012345
    //           ^ byte 9 is the '1' inside "v1.2.3"
    //  VString spans bytes 8..14 (exclusive end)
    let source = "my $v = v1.2.3;".to_string();
    let mut doc = must(IncrementalDocument::new(source));

    // Insert '0' at byte 9 (inside the VString, after the leading 'v').
    // New source: "my $v = v10.2.3;"
    // edit.start_byte = 9, which is inside VString (8..14), so
    // is_single_token_edit calls find_node_at_position(9) → finds VString →
    // VString arm at incremental_document.rs:345 is executed → returns true.
    let edit = IncrementalEdit::new(9, 9, "0".to_string());
    must(doc.apply_edit(edit));

    // After the reparse the new source is "my $v = v10.2.3;"
    // v10.2.3 is a valid v-string and must be NodeKind::VString.
    // Reverting primary.rs VString arm would produce (string "v10.2.3") here.
    let sexp = doc.root.to_sexp();
    assert!(
        sexp.contains("(vstring \"v10.2.3\")"),
        "after inserting '0' inside VString, result must contain the exact edited vstring; \
         reverting primary.rs VString arm would produce a string node instead. \
         Got: {}",
        sexp
    );
    assert!(
        !sexp.contains("(string \"v10.2.3\")"),
        "v10.2.3 must be NodeKind::VString not NodeKind::String; got: {}",
        sexp
    );
}
