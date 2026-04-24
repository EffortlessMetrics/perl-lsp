#![cfg(feature = "incremental")]

use perl_parser::incremental_document::IncrementalDocument;
use perl_parser::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_parser::Parser;

#[test]
fn overlapping_batch_edits_fallback_safely() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 10;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 8, "$value".to_string()));
    edits.add(IncrementalEdit::new(6, 10, "99".to_string()));

    doc.apply_edits(&edits)?;

    let expected_source = edits.apply_to_string(source);
    assert_eq!(doc.source, expected_source);
    Ok(())
}

#[test]
fn backwards_range_fallback_safely() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 10;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 6, "11".to_string()));

    doc.apply_edits(&edits)?;

    let expected_source = edits.apply_to_string(source);
    assert_eq!(doc.source, expected_source);
    Ok(())
}

#[test]
fn mid_codepoint_edit_attempt_falls_back_without_corruption(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $emoji = \"😀\";";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let emoji_start = source.find('😀').ok_or("emoji must exist")?;
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(emoji_start + 1, emoji_start + 2, "x".to_string()));

    doc.apply_edits(&edits)?;

    let expected_source = edits.apply_to_string(source);
    assert_eq!(doc.source, expected_source);
    Ok(())
}

#[test]
fn one_bad_edit_in_batch_forces_batch_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $emoji = \"😀\"; my $x = 1;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let replacement_start = source.find("1;").ok_or("value must exist")?;
    let emoji_start = source.find('😀').ok_or("emoji must exist")?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(replacement_start, replacement_start + 1, "2".to_string()));
    edits.add(IncrementalEdit::new(emoji_start + 1, emoji_start + 2, "x".to_string()));

    doc.apply_edits(&edits)?;

    let expected_source = edits.apply_to_string(source);
    assert_eq!(doc.source, expected_source);
    Ok(())
}

#[test]
fn supported_batch_edits_match_fresh_parse() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 10; my $y = 20;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let x_pos = source.find("10").ok_or("10 must exist")?;
    let y_pos = source.find("20").ok_or("20 must exist")?;

    edits.add(IncrementalEdit::new(x_pos, x_pos + 2, "11".to_string()));
    edits.add(IncrementalEdit::new(y_pos, y_pos + 2, "21".to_string()));

    doc.apply_edits(&edits)?;

    let mut parser = Parser::new(&doc.source);
    let fresh = parser.parse()?;

    assert_eq!(doc.root.to_sexp(), fresh.to_sexp());
    Ok(())
}
