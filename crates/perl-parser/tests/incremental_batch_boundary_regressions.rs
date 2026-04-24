#[cfg(feature = "incremental")]
use perl_parser::Parser;
#[cfg(feature = "incremental")]
use perl_parser::incremental_document::IncrementalDocument;
#[cfg(feature = "incremental")]
use perl_parser::incremental_edit::{IncrementalEdit, IncrementalEditSet};

#[cfg(feature = "incremental")]
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(feature = "incremental")]
#[test]
fn overlapping_batch_edits_fallback_conservatively() -> TestResult {
    let source = "my $value = 123;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 9, "$item".to_string()));
    edits.add(IncrementalEdit::new(7, 12, "456".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[cfg(feature = "incremental")]
#[test]
fn backwards_ranges_are_rejected() {
    let source = "abc";
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(2, 1, "x".to_string()));

    assert!(edits.normalized_for_source(source).is_none());
    assert_eq!(edits.apply_to_string(source), source);
}

#[cfg(feature = "incremental")]
#[test]
fn mid_codepoint_edit_attempt_triggers_batch_fallback() -> TestResult {
    let source = "my $x = \"é\";\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let cp_start = source.find('é').ok_or("expected source to contain é")?;
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(cp_start + 1, cp_start + 1, "X".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[cfg(feature = "incremental")]
#[test]
fn one_bad_edit_causes_batch_fallback() -> TestResult {
    let source = "my $first = 1; my $second = \"é\";\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let good_start = source.find("1").ok_or("expected source to contain '1'")?;
    let bad_start = source.find('é').ok_or("expected source to contain é")?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(good_start, good_start + 1, "9".to_string()));
    edits.add(IncrementalEdit::new(bad_start + 1, bad_start + 1, "X".to_string()));

    doc.apply_edits(&edits)?;

    assert_eq!(doc.source, source);
    Ok(())
}

#[cfg(feature = "incremental")]
#[test]
fn incremental_result_matches_fresh_parse_for_supported_batch() -> TestResult {
    let source = "my $a = 10;\nmy $b = 20;\nprint $a + $b;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let ten = source.find("10").ok_or("expected source to contain '10'")?;
    let twenty = source.find("20").ok_or("expected source to contain '20'")?;
    edits.add(IncrementalEdit::new(ten, ten + 2, "15".to_string()));
    edits.add(IncrementalEdit::new(twenty, twenty + 2, "25".to_string()));

    doc.apply_edits(&edits)?;

    let mut parser = Parser::new(&doc.source);
    let fresh_root = parser.parse()?;

    assert_eq!(format!("{:?}", doc.root), format!("{:?}", fresh_root));
    Ok(())
}
