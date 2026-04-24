#![cfg(feature = "incremental")]

use perl_parser::incremental::incremental_document::IncrementalDocument;
use perl_parser::incremental::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_parser_core::error::{ParseError, ParseResult};
use perl_parser_core::parser::Parser;

fn find_or_err(haystack: &str, needle: &str) -> ParseResult<usize> {
    haystack.find(needle).ok_or_else(|| ParseError::SyntaxError {
        message: format!("expected test source to contain '{needle}'"),
        location: 0,
    })
}

#[test]
fn start_end_beyond_source_len_falls_back_safely() -> ParseResult<()> {
    let source = "my $x = 42;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(source.len() + 1, source.len() + 2, "100".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn start_greater_than_end_after_normalization_falls_back_safely() -> ParseResult<()> {
    let source = "my $x = 42;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(source.len() + 10, 3, "100".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn mid_codepoint_insert_or_delete_falls_back_safely() -> ParseResult<()> {
    let source = "my $x = \"é\";\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let codepoint_start = find_or_err(source, "é")?;
    let mid_codepoint = codepoint_start + 1;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(mid_codepoint, mid_codepoint + 1, "e".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn batch_with_one_unmappable_edit_uses_safe_fallback() -> ParseResult<()> {
    let source = "my $x = 42;\nmy $y = 10;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let pos_42 = find_or_err(source, "42")?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(pos_42, pos_42 + 2, "43".to_string()));
    edits.add(IncrementalEdit::new(source.len() + 100, source.len() + 101, "oops".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn incremental_result_matches_fresh_full_parse_on_supported_batch() -> ParseResult<()> {
    let source = "my $x = 42;\nmy $y = 10;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let pos_42 = find_or_err(source, "42")?;
    let pos_10 = find_or_err(source, "10")?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(pos_42, pos_42 + 2, "43".to_string()));
    edits.add(IncrementalEdit::new(pos_10, pos_10 + 2, "11".to_string()));

    doc.apply_edits(&edits)?;

    let mut parser = Parser::new(&doc.source);
    let fresh = parser.parse()?;
    assert_eq!(*doc.root, fresh);

    Ok(())
}
