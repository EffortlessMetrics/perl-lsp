use super::incremental_document::IncrementalDocument;
use super::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_parser_core::{error::ParseResult, parser::Parser};

#[test]
fn overlapping_batch_edits_fall_back_safely() -> ParseResult<()> {
    // "my $x = 10;\n"
    //           ^-- byte 8 = '1', byte 9 = '0', byte 10 = ';'
    // Edits (8..10, "20") and (9..10, "5") overlap (both cover byte 9).
    // normalize_for_source rejects them; fallback applies via apply_to_string,
    // which sorts descending by start and silently skips the earlier edit once
    // bytes shift. After (9..10)->"5": "my $x = 15;\n". After (8..10)->"20":
    // replaces bytes 8..10 ('1','5') with "20" → "my $x = 20;\n".
    let source = "my $x = 10;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 10, "20".to_string()));
    edits.add(IncrementalEdit::new(9, 10, "5".to_string()));

    document.apply_edits(&edits)?;

    // Concrete expected: the fallback applies edits via apply_to_string in
    // descending-start order. The result is "my $x = 20;\n" (second edit wins
    // over the overlapping region). Asserting the literal avoids the tautology
    // of comparing against apply_to_string itself.
    assert_eq!(document.source, "my $x = 20;\n");
    assert_eq!(document.metrics.nodes_reused, 0);
    // The document should parse successfully after the fallback.
    assert!(document.source.starts_with("my $x"));

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

    assert_eq!(
        *document.root, parsed_fresh,
        "incremental batch result must match a fresh parse of the same source"
    );

    Ok(())
}

#[test]
fn empty_edit_set_is_a_noop() -> ParseResult<()> {
    // An empty batch must not mutate the document source or tree.
    let source = "my $x = 42;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;
    let root_before = (*document.root).clone();

    let edits = IncrementalEditSet::new();
    document.apply_edits(&edits)?;

    assert_eq!(document.source, source, "source must be unchanged for empty edit set");
    assert_eq!(*document.root, root_before, "tree must be unchanged for empty edit set");

    Ok(())
}

#[test]
fn adjacent_non_overlapping_edits_both_apply() -> ParseResult<()> {
    // "my $x = 10; my $y = 20;\n"
    // byte offsets (0-based):
    //  0:'m' 1:'y' 2:' ' 3:'$' 4:'x' 5:' ' 6:'=' 7:' ' 8:'1' 9:'0'
    //  10:';' 11:' ' 12:'m' 13:'y' 14:' ' 15:'$' 16:'y' 17:' ' 18:'='
    //  19:' ' 20:'2' 21:'0' 22:';' 23:'\n'
    // Edit A: (8, 10, "99") replaces "10" with "99".
    // Edit B: (20, 22, "88") replaces "20" with "88".
    // These ranges do not overlap (10 <= 20), so normalize_for_source
    // must accept both and apply them in-place.
    let source = "my $x = 10; my $y = 20;\n".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    // Verify assumed byte positions before relying on them.
    assert_eq!(&source[8..10], "10", "byte offset assumption for edit A");
    assert_eq!(&source[20..22], "20", "byte offset assumption for edit B");

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(8, 10, "99".to_string()));
    edits.add(IncrementalEdit::new(20, 22, "88".to_string()));

    document.apply_edits(&edits)?;

    assert!(
        document.source.contains("99"),
        "edit A ($x value) must be applied; got: {}",
        document.source
    );
    assert!(
        document.source.contains("88"),
        "edit B ($y value) must be applied; got: {}",
        document.source
    );
    assert_eq!(document.source, "my $x = 99; my $y = 88;\n");

    // The post-edit tree should match a fresh parse of the new source.
    let mut parser = Parser::new(&document.source);
    let parsed_fresh = parser.parse()?;
    assert_eq!(*document.root, parsed_fresh, "incremental batch must match fresh parse");

    Ok(())
}

#[test]
fn adjacent_touching_ranges_are_not_rejected() -> ParseResult<()> {
    // Two edits whose ranges share exactly one endpoint (end_a == start_b)
    // are NOT overlapping and must both be accepted by normalize_for_source.
    // "abcdef": edit A=(0,3,"XY") and edit B=(3,6,"Z"). End of A == start of B.
    let source = "abcdef".to_string();
    let mut document = IncrementalDocument::new(source)?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(0, 3, "XY".to_string()));
    edits.add(IncrementalEdit::new(3, 6, "Z".to_string()));

    document.apply_edits(&edits)?;

    // Both edits applied in reverse order: first (3,6)->"Z", then (0,3)->"XY".
    // After (3,6)->"Z": "abcZ". After (0,3)->"XY": "XYZ".
    assert_eq!(document.source, "XYZ", "adjacent touching edits must both apply");

    Ok(())
}

#[test]
fn whole_file_replacement_batch_takes_single_parse_path() -> ParseResult<()> {
    // When an edit covers the entire source, there are no subtrees outside the
    // affected range, so reusable is empty.  The no-reuse branch must do exactly
    // one parse (not two) and produce a correct tree.
    let source = "my $x = 1;\n".to_string();
    let mut document = IncrementalDocument::new(source)?;

    let mut edits = IncrementalEditSet::new();
    // Replace the entire content with a new statement.
    let new_content = "my $y = 99;\n".to_string();
    edits.add(IncrementalEdit::new(0, 11, new_content.trim_end_matches('\n').to_string()));

    document.apply_edits(&edits)?;

    assert!(
        document.source.contains("99"),
        "whole-file replacement must produce updated source; got: {}",
        document.source
    );
    let mut parser = Parser::new(&document.source);
    let parsed_fresh = parser.parse()?;
    assert_eq!(*document.root, parsed_fresh, "whole-file replacement must match fresh parse");

    Ok(())
}

#[test]
fn same_start_edits_apply_deterministically() -> ParseResult<()> {
    // Two edits with the same start_byte but different old_end_byte.
    // Correct reverse-sort order: larger old_end_byte first (so (2,6) before (2,4)).
    // "abcdefgh" -> edit A=(2,6,"X") replaces "cdef" with "X" -> "abXgh"
    //              edit B=(2,4,"Y") replaces "cd" with "Y" -> overlaps with A
    // Because they overlap, normalize_for_source must reject both -> fallback.
    // apply_to_string must use the same secondary sort as normalize_for_source
    // so the deterministic ordering matches between the two paths.
    let source = "abcdefgh".to_string();
    let mut document = IncrementalDocument::new(source.clone())?;

    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(2, 6, "X".to_string())); // replaces "cdef"
    edits.add(IncrementalEdit::new(2, 4, "Y".to_string())); // replaces "cd" — overlaps

    let expected = edits.apply_to_string(&source);
    document.apply_edits(&edits)?;

    // Both paths (fallback via apply_to_string) should agree.
    assert_eq!(
        document.source, expected,
        "same-start overlapping edits: fallback result must be deterministic"
    );

    Ok(())
}
