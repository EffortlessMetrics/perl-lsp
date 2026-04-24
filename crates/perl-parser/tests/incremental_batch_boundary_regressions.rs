#![cfg(feature = "incremental")]

use perl_parser::incremental_document::IncrementalDocument;
use perl_parser::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_parser::Parser;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn rejects_overlapping_batch_edits() -> TestResult {
    let source = "my $x = 10;\nmy $y = 20;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 9, "$name".to_string()));
    edits.add(IncrementalEdit::new(7, 12, "= 99".to_string()));

    let result = doc.apply_edits(&edits);
    assert!(result.is_err(), "overlapping ranges should be rejected");
    assert_eq!(doc.source, source, "rejected edits must not mutate document source");
    Ok(())
}

#[test]
fn rejects_backwards_range_batch_edit() -> TestResult {
    let source = "my $x = 10;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 3, "broken".to_string()));

    let result = doc.apply_edits(&edits);
    assert!(result.is_err(), "backwards range should be rejected");
    assert_eq!(doc.source, source, "rejected edit must not mutate document source");
    Ok(())
}

#[test]
fn mid_codepoint_batch_edit_is_rejected_without_panicking() -> TestResult {
    let source = "my $x = \"é\";\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let replace_x_start = source.find("$x").ok_or("source missing '$x'")? + 1;
    edits.add(IncrementalEdit::new(replace_x_start, replace_x_start + 1, "z".to_string()));

    let e_start = source.find('é').ok_or("source missing 'é'")?;
    edits.add(IncrementalEdit::new(e_start + 1, e_start + 1, "!".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[test]
fn one_bad_edit_batch_falls_back_without_panicking() -> TestResult {
    let source = "my $x = \"é\";\nmy $y = 20;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let y_start = source.find("$y").ok_or("source missing '$y'")? + 1;
    edits.add(IncrementalEdit::new(y_start, y_start + 1, "k".to_string()));

    let e_start = source.find('é').ok_or("source missing 'é'")?;
    edits.add(IncrementalEdit::new(e_start + 1, e_start + 1, "!".to_string()));

    let result = doc.apply_edits(&edits);
    assert!(result.is_ok(), "batch with one unmappable edit should safely fall back");
    assert_eq!(doc.source, "my $x = \"é\";\nmy $k = 20;\n");
    Ok(())
}

#[test]
fn incremental_batch_matches_fresh_parse_for_supported_case() -> TestResult {
    let source = "my $x = 10;\nmy $y = 20;\nprint $x + $y;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();

    let ten_start = source.find("10").ok_or("source missing '10'")?;
    edits.add(IncrementalEdit::new(ten_start, ten_start + 2, "15".to_string()));

    let print_start = source.find("print").ok_or("source missing 'print'")?;
    edits.add(IncrementalEdit::new(print_start, print_start + 5, "say".to_string()));

    doc.apply_edits(&edits)?;

    let mut parser = Parser::new(&doc.source);
    let fresh = parser.parse()?;

    assert_eq!(doc.root.to_sexp(), fresh.to_sexp());
    Ok(())
}
