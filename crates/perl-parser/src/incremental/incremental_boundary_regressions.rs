#![cfg(test)]

use super::incremental_document::IncrementalDocument;
use super::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use anyhow::{anyhow, Result};
use perl_parser_core::parser::Parser;

fn parse_to_sexp(source: &str) -> Result<String> {
    let mut parser = Parser::new(source);
    let node = parser.parse()?;
    Ok(node.to_sexp())
}

#[test]
fn overlapping_batch_edits_fallback_to_safe_application() -> Result<()> {
    let source = "my $x = 1;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 8, "$value".to_string()));
    edits.add(IncrementalEdit::new(6, 9, "$oops".to_string()));

    let apply_result = document.apply_edits(&edits);
    assert!(apply_result.is_ok());

    // Overlapping ranges are rejected by normalization, so source is derived by
    // conservative per-edit application that skips invalid overlaps.
    let expected_source = edits.apply_to_string(&source);
    assert_eq!(document.source, expected_source);
    assert_eq!(document.root.to_sexp(), parse_to_sexp(&expected_source)?);
    Ok(())
}

#[test]
fn backwards_range_is_rejected_and_source_is_unchanged() -> Result<()> {
    let source = "my $x = 1;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 3, "2".to_string()));

    let apply_result = document.apply_edits(&edits);
    assert!(apply_result.is_ok());
    assert_eq!(document.source, source);
    assert_eq!(document.root.to_sexp(), parse_to_sexp(&document.source)?);
    Ok(())
}

#[test]
fn mid_codepoint_edit_is_skipped_without_panicking() -> Result<()> {
    let source = "my $x = \"é\";\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;
    let e_start = source.find('é').ok_or_else(|| anyhow!("test setup missing UTF-8 codepoint"))?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(e_start + 1, e_start + 1, "x".to_string()));

    let apply_result = document.apply_edits(&edits);
    assert!(apply_result.is_ok());
    assert_eq!(document.source, source);
    assert_eq!(document.root.to_sexp(), parse_to_sexp(&source)?);
    Ok(())
}

#[test]
fn one_bad_edit_in_batch_falls_back_and_preserves_good_edit() -> Result<()> {
    let source = "my $x = \"é\";\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;
    let e_start = source.find('é').ok_or_else(|| anyhow!("test setup missing UTF-8 codepoint"))?;
    let x_pos = source
        .find("$x")
        .map(|idx| idx + 1) // Replace identifier name only.
        .ok_or_else(|| anyhow!("test setup missing variable"))?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(e_start + 1, e_start + 1, "x".to_string())); // invalid boundary
    edits.add(IncrementalEdit::new(x_pos, x_pos + 1, "name".to_string())); // valid

    let apply_result = document.apply_edits(&edits);
    assert!(apply_result.is_ok());

    let expected_source = edits.apply_to_string(&source);
    assert_eq!(document.source, expected_source);
    assert!(document.source.contains("$name"));
    assert_eq!(document.root.to_sexp(), parse_to_sexp(&expected_source)?);
    Ok(())
}

#[test]
fn supported_batch_edits_match_fresh_parse() -> Result<()> {
    let source = "my $value = 1;\n$value = $value + 1;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 9, "$total".to_string()));
    edits.add(IncrementalEdit::new(22, 27, "$total".to_string()));

    let apply_result = document.apply_edits(&edits);
    assert!(apply_result.is_ok());

    let fresh = parse_to_sexp(&document.source)?;
    assert_eq!(document.root.to_sexp(), fresh);
    Ok(())
}
