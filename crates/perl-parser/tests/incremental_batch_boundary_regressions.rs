#![cfg(feature = "incremental")]

use perl_parser::Parser;
use perl_parser::incremental::incremental_document::IncrementalDocument;
use perl_parser::incremental::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use std::error::Error;
use std::io;

fn find_index(haystack: &str, needle: &str) -> Result<usize, io::Error> {
    haystack
        .find(needle)
        .ok_or_else(|| io::Error::other(format!("'{needle}' should exist in test source")))
}

#[test]
fn overlapping_batch_edits_fallback_without_partial_application() -> Result<(), Box<dyn Error>> {
    let source = "my $x = 12345;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let first = find_index(source, "123")?;
    let second = find_index(source, "345")?;
    edits.add(IncrementalEdit::new(first, first + 3, "ABC".to_string()));
    edits.add(IncrementalEdit::new(second, second + 3, "XYZ".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn backwards_range_batch_edits_fallback_without_mutation() -> Result<(), Box<dyn Error>> {
    let source = "my $x = 10;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 6, "42".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn mid_codepoint_single_edit_is_ignored_safely() -> Result<(), Box<dyn Error>> {
    let source = "my $greeting = \"é\";";
    let mut doc = IncrementalDocument::new(source.to_string())?;
    let codepoint_start = find_index(source, "é")?;

    // Target the middle byte of a 2-byte UTF-8 codepoint.
    let invalid_mid_byte = codepoint_start + 1;
    let edit = IncrementalEdit::new(invalid_mid_byte, invalid_mid_byte, "x".to_string());
    doc.apply_edit(edit)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn one_bad_edit_forces_batch_fallback() -> Result<(), Box<dyn Error>> {
    let source = "my $name = \"é\"; my $value = 1;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let value_pos = find_index(source, "1")?;
    edits.add(IncrementalEdit::new(value_pos, value_pos + 1, "2".to_string()));

    let multibyte_start = find_index(source, "é")?;
    let invalid_mid_byte = multibyte_start + 1;
    edits.add(IncrementalEdit::new(invalid_mid_byte, invalid_mid_byte, "!".to_string()));

    doc.apply_edits(&edits)?;

    // Entire batch should conservatively fall back without partially applying edits.
    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn supported_batch_edits_match_fresh_parse() -> Result<(), Box<dyn Error>> {
    let source = "my $x = 1; my $y = 2;";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let x_pos = find_index(source, "1")?;
    let y_pos = find_index(source, "2")?;
    edits.add(IncrementalEdit::new(x_pos, x_pos + 1, "10".to_string()));
    edits.add(IncrementalEdit::new(y_pos, y_pos + 1, "20".to_string()));
    edits.add(IncrementalEdit::new(source.len(), source.len(), " # trailing".to_string()));

    doc.apply_edits(&edits)?;

    let expected_source = "my $x = 10; my $y = 20; # trailing";
    let mut parser = Parser::new(expected_source);
    let fresh = parser.parse()?;

    assert_eq!(doc.source, expected_source);
    assert_eq!(&*doc.root, &fresh);
    Ok(())
}
