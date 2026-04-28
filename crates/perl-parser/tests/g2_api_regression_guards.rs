//! Regression guards for Wave G2 API changes (#4508, #4526).
//!
//! These tests ensure that the API changes introduced during the G2 runtime collapse
//! (removal of `IncrementalState::parse()`, `Edit::text` → `Edit::new_text`,
//! `apply_edits()` signature change) remain properly implemented and don't regress
//! in future refactoring.

#![cfg(feature = "incremental")]

use perl_parser::{Edit, IncrementalState, apply_edits};

/// Regression guard: IncrementalState must expose ast as a public field.
/// Previously, state.parse() returned the AST; now it's accessed via state.ast.
#[test]
fn test_incremental_state_ast_field_exposed() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let state = IncrementalState::new(code.to_string());

    // The ast field must be accessible directly
    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Regression guard: IncrementalState::new() must accept String, not &str.
/// The builder changed the signature to take ownership of the source.
#[test]
fn test_incremental_state_new_takes_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = String::from("sub foo { }");
    let state = IncrementalState::new(code);

    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Regression guard: IncrementalState::new() must accept String from to_string().
/// Ensure both String literals and String from to_string() work correctly.
#[test]
fn test_incremental_state_new_with_to_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @arr = (1, 2, 3);";
    let state = IncrementalState::new(code.to_string());

    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Regression guard: Edit field must be new_text, not text.
/// This is a critical footgun — code constructing Edit with text: will fail to compile.
#[test]
fn test_edit_field_is_new_text() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that Edit can be constructed with new_text field
    let _edit = Edit { start_byte: 0, old_end_byte: 1, new_end_byte: 1, new_text: "x".to_string() };

    Ok(())
}

/// Regression guard: apply_edits() must accept slice, not Vec.
/// The signature changed from Vec<Edit> to &[Edit].
#[test]
fn test_apply_edits_accepts_slice() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut state = IncrementalState::new(code.to_string());

    let edit = Edit { start_byte: 3, old_end_byte: 5, new_end_byte: 5, new_text: "y".to_string() };

    // apply_edits must accept a slice, not a Vec
    let _result = apply_edits(&mut state, &[edit])?;

    Ok(())
}

/// Regression guard: apply_edits() must return Result.
/// The signature changed to return Result<(), Box<dyn std::error::Error>>.
#[test]
fn test_apply_edits_returns_result() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut state = IncrementalState::new(code.to_string());

    let edit = Edit { start_byte: 0, old_end_byte: 0, new_end_byte: 0, new_text: String::new() };

    // apply_edits returns Result, must be handled with ?
    let _result = apply_edits(&mut state, &[edit])?;

    Ok(())
}

/// Regression guard: Empty edits slice must not fail.
/// Edge case: passing an empty slice should not panic or return an error.
#[test]
fn test_apply_edits_empty_slice() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut state = IncrementalState::new(code.to_string());

    let empty_edits: &[Edit] = &[];

    // Applying zero edits should be a no-op
    let _result = apply_edits(&mut state, empty_edits)?;
    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Regression guard: IncrementalState must initialize ast correctly even with malformed code.
/// The parser uses recovery, so state.ast should always be set (Program or Error).
#[test]
fn test_incremental_state_ast_always_set() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ("; // Incomplete, parser should recover
    let state = IncrementalState::new(code.to_string());

    // state.ast should always be set, regardless of parse success
    // The parser uses recovery to produce a valid AST
    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Regression guard: ast field must not require mutable access to read.
/// Ensure state.ast is immutable and accessible on a borrowed state.
#[test]
fn test_incremental_state_ast_immutable_access() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let state = IncrementalState::new(code.to_string());

    // Should be able to read ast multiple times without mut
    let kind1 = &state.ast.kind;
    let kind2 = &state.ast.kind;

    assert_eq!(std::mem::discriminant(kind1), std::mem::discriminant(kind2));

    Ok(())
}

/// Regression guard: Edit struct must have start_byte, old_end_byte, new_end_byte, and new_text fields.
/// The old API had a "text" field; it's now "new_text".
#[test]
fn test_edit_struct_fields() -> Result<(), Box<dyn std::error::Error>> {
    let edit =
        Edit { start_byte: 5, old_end_byte: 10, new_end_byte: 12, new_text: "test".to_string() };

    assert_eq!(edit.start_byte, 5);
    assert_eq!(edit.old_end_byte, 10);
    assert_eq!(edit.new_end_byte, 12);
    assert_eq!(edit.new_text, "test");

    Ok(())
}

/// Regression guard: IncrementalState source field must be accessible.
/// Verifies that the source string is preserved in the state.
#[test]
fn test_incremental_state_preserves_source() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub test { my $x = 1; }";
    let state = IncrementalState::new(code.to_string());

    assert_eq!(state.source, code);

    Ok(())
}

/// Regression guard: Boundary condition — zero-length edit.
/// Zero-byte operations should work correctly.
#[test]
fn test_apply_edits_zero_length_edit() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x;";
    let mut state = IncrementalState::new(code.to_string());

    let edit = Edit { start_byte: 0, old_end_byte: 0, new_end_byte: 0, new_text: String::new() };

    let _result = apply_edits(&mut state, &[edit])?;

    Ok(())
}

/// Regression guard: Multiple edits in sequence should be handled.
/// Verifies that a slice with multiple Edit objects can be applied.
#[test]
fn test_apply_edits_multiple() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x; my $y;";
    let mut state = IncrementalState::new(code.to_string());

    let edit1 = Edit { start_byte: 3, old_end_byte: 4, new_end_byte: 4, new_text: "a".to_string() };

    let edit2 =
        Edit { start_byte: 10, old_end_byte: 11, new_end_byte: 11, new_text: "b".to_string() };

    let _result = apply_edits(&mut state, &[edit1, edit2])?;

    Ok(())
}

/// Regression guard: IncrementalState must have all expected fields.
/// Verifies rope, line_index, and other fields are public.
#[test]
fn test_incremental_state_fields_accessible() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let state = IncrementalState::new(code.to_string());

    // All these fields must be accessible
    let _ = &state.rope;
    let _ = &state.line_index;
    let _ = &state.ast;
    let _ = &state.source;
    let _ = &state.tokens;

    Ok(())
}

/// Regression guard: IncrementalState must preserve tokens.
/// Tokens should be available in the state after creation.
#[test]
fn test_incremental_state_tokens_populated() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let state = IncrementalState::new(code.to_string());

    // Tokens should not be empty for non-empty code
    assert!(!state.tokens.is_empty());

    Ok(())
}

/// Regression guard: Edit with insertion text.
/// Verify that new_text field accepts non-empty strings.
#[test]
fn test_edit_with_insertion_text() -> Result<(), Box<dyn std::error::Error>> {
    let edit =
        Edit { start_byte: 5, old_end_byte: 5, new_end_byte: 10, new_text: "hello".to_string() };

    assert_eq!(edit.new_text, "hello");
    assert_eq!(edit.new_end_byte - edit.old_end_byte, 5);

    Ok(())
}

/// Regression guard: Edit with deletion (empty new_text).
/// Verify that new_text can be empty for deletions.
#[test]
fn test_edit_with_deletion() -> Result<(), Box<dyn std::error::Error>> {
    let edit = Edit { start_byte: 5, old_end_byte: 10, new_end_byte: 5, new_text: String::new() };

    assert_eq!(edit.new_text, "");
    assert!(edit.old_end_byte > edit.new_end_byte);

    Ok(())
}

/// Regression guard: IncrementalState::new() with empty string.
/// Edge case: creating state from empty source should work.
#[test]
fn test_incremental_state_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let state = IncrementalState::new(String::new());

    // Even empty source should create an AST node (possibly Program or Error)
    assert!(!matches!(state.ast.kind, perl_parser::NodeKind::Error { .. }));

    Ok(())
}

/// Regression guard: IncrementalState::new() with complex code.
/// Boundary: verify it handles complex Perl code correctly.
#[test]
fn test_incremental_state_complex_code() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
        package Test::Module;
        use strict;
        use warnings;

        sub new {
            my ($class) = @_;
            my $self = {};
            bless $self, $class;
            return $self;
        }

        sub method {
            my ($self, $arg) = @_;
            return $self->{value} = $arg;
        }

        1;
    "#;

    let state = IncrementalState::new(code.to_string());

    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}
