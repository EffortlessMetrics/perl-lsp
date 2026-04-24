use super::incremental_document::IncrementalDocument;
use super::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_parser_core::{error::ParseResult, parser::Parser};

#[test]
fn overlapping_batch_edits_fall_back_safely() -> ParseResult<()> {
    let source = "my $x = 10;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 10, "20".to_string()));
    edits.add(IncrementalEdit::new(9, 10, "5".to_string()));

    let expected = edits.apply_to_string(&source);
    document.apply_edits(&edits)?;

    assert_eq!(document.source, expected);
    assert_eq!(document.metrics.nodes_reused, 0);

    Ok(())
}

#[test]
fn backwards_range_batch_edit_is_rejected() -> ParseResult<()> {
    let source = "my $x = 10;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(9, 7, "5".to_string()));

    document.apply_edits(&edits)?;

    assert_eq!(document.source, source);
    assert_eq!(document.metrics.nodes_reused, 0);

    Ok(())
}

#[test]
fn mid_codepoint_edit_attempt_falls_back() -> ParseResult<()> {
    let source = "my $x = \"é\";\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let accent_start =
        source.find('é').ok_or_else(|| perl_parser_core::error::ParseError::SyntaxError {
            message: "test source should contain 'é'".to_string(),
            location: 0,
        })?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(accent_start + 1, accent_start + 1, "x".to_string()));

    document.apply_edits(&edits)?;

    assert_eq!(document.source, source);
    assert_eq!(document.metrics.nodes_reused, 0);

    Ok(())
}

#[test]
fn batch_with_one_unmappable_edit_uses_fallback() -> ParseResult<()> {
    let source = "my $x = \"é\";\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let accent_start =
        source.find('é').ok_or_else(|| perl_parser_core::error::ParseError::SyntaxError {
            message: "test source should contain 'é'".to_string(),
            location: 0,
        })?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 6, "$value".to_string()));
    edits.add(IncrementalEdit::new(accent_start + 1, accent_start + 1, "x".to_string()));

    let expected = edits.apply_to_string(&source);
    document.apply_edits(&edits)?;

    assert_eq!(document.source, expected);
    assert!(document.source.contains("$value"));
    assert_eq!(document.metrics.nodes_reused, 0);

    Ok(())
}

#[test]
fn supported_batch_edits_match_fresh_parse() -> ParseResult<()> {
    let source = "my $x = 10;\nmy $y = 20;\n".to_string();
    let mut document = IncrementalDocument::new(source)?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 10, "11".to_string()));
    edits.add(IncrementalEdit::new(20, 22, "21".to_string()));

    document.apply_edits(&edits)?;

    let mut parser = Parser::new(&document.source);
    let parsed_fresh = parser.parse()?;

    assert_eq!(format!("{:?}", document.root), format!("{:?}", parsed_fresh));

    Ok(())
}
