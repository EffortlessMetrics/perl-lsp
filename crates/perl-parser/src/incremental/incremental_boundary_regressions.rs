use super::incremental_document::IncrementalDocument;
use super::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_parser_core::{error::ParseResult, parser::Parser};

#[test]
fn overlapping_batch_edits_are_rejected_conservatively() -> ParseResult<()> {
    let source = "my $x = 10;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 8, "$value".to_string()));
    edits.add(IncrementalEdit::new(6, 10, "20".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn backwards_ranges_are_rejected_conservatively() -> ParseResult<()> {
    let source = "my $x = 10;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 6, "11".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn mid_codepoint_edit_attempt_falls_back_safely() -> ParseResult<()> {
    let source = "my $x = \"é\";\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let char_start =
        source.find("é").ok_or_else(|| perl_parser_core::error::ParseError::SyntaxError {
            message: "expected UTF-8 test character".to_string(),
            location: 0,
        })?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(char_start + 1, char_start + 1, "x".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn one_bad_edit_in_batch_uses_fallback_and_applies_mappable_edits() -> ParseResult<()> {
    let source = "my $x = \"é\";\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let good_start =
        source.find('x').ok_or_else(|| perl_parser_core::error::ParseError::SyntaxError {
            message: "expected variable in source".to_string(),
            location: 0,
        })?;
    let char_start =
        source.find("é").ok_or_else(|| perl_parser_core::error::ParseError::SyntaxError {
            message: "expected UTF-8 test character".to_string(),
            location: 0,
        })?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(good_start, good_start + 1, "y".to_string()));
    edits.add(IncrementalEdit::new(char_start + 1, char_start + 1, "x".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, "my $y = \"é\";\n");
    Ok(())
}

#[test]
fn supported_incremental_batch_matches_fresh_parse() -> ParseResult<()> {
    let source = "my $x = 10;\nmy $y = 20;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let x_pos =
        source.find("10").ok_or_else(|| perl_parser_core::error::ParseError::SyntaxError {
            message: "expected 10 literal".to_string(),
            location: 0,
        })?;
    let y_pos =
        source.find("20").ok_or_else(|| perl_parser_core::error::ParseError::SyntaxError {
            message: "expected 20 literal".to_string(),
            location: 0,
        })?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(x_pos, x_pos + 2, "11".to_string()));
    edits.add(IncrementalEdit::new(y_pos, y_pos + 2, "21".to_string()));

    doc.apply_edits(&edits)?;

    let mut parser = Parser::new(&doc.source);
    let fresh = parser.parse()?;

    assert_eq!(format!("{:?}", doc.root), format!("{:?}", fresh));
    Ok(())
}
