#[cfg(test)]
mod tests {
    use crate::incremental::incremental_document::IncrementalDocument;
    use crate::incremental::incremental_edit::{IncrementalEdit, IncrementalEditSet};
    use perl_parser_core::{
        error::{ParseError, ParseResult},
        parser::Parser,
    };

    fn missing_fragment(fragment: &str) -> ParseError {
        ParseError::SyntaxError {
            message: format!("test source should contain `{fragment}`"),
            location: 0,
        }
    }

    fn fresh_parse_debug(source: &str) -> ParseResult<String> {
        let mut parser = Parser::new(source);
        let root = parser.parse()?;
        Ok(format!("{root:?}"))
    }

    #[test]
    fn overlapping_batch_edits_fall_back_conservatively() -> ParseResult<()> {
        let mut doc = IncrementalDocument::new("abcdef".to_string())?;
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(0, 3, "X".to_string()));
        edits.add(IncrementalEdit::new(2, 5, "Y".to_string()));

        doc.apply_edits(&edits)?;

        // Overlap causes fallback; conservative application keeps deterministic
        // reverse-order behavior by applying the later-starting edit.
        assert_eq!(doc.source, "abYf");
        Ok(())
    }

    #[test]
    fn backwards_ranges_are_rejected() -> ParseResult<()> {
        let original = "my $x = 10;";
        let mut doc = IncrementalDocument::new(original.to_string())?;
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(7, 3, "oops".to_string()));

        doc.apply_edits(&edits)?;

        assert_eq!(doc.source, original);
        Ok(())
    }

    #[test]
    fn mid_codepoint_batch_edit_is_ignored() -> ParseResult<()> {
        let original = "my $s = \"é\";";
        let mut doc = IncrementalDocument::new(original.to_string())?;
        let mut edits = IncrementalEditSet::new();
        let char_start = original.find("é").ok_or_else(|| missing_fragment("é"))?;
        // Edit starts inside UTF-8 codepoint for "é"
        edits.add(IncrementalEdit::new(char_start + 1, char_start + 2, "x".to_string()));

        doc.apply_edits(&edits)?;

        assert_eq!(doc.source, original);
        Ok(())
    }

    #[test]
    fn batch_fallback_when_one_edit_is_unmappable() -> ParseResult<()> {
        let original = "my $s = \"é\";\nmy $x = 1;\n";
        let mut doc = IncrementalDocument::new(original.to_string())?;
        let mut edits = IncrementalEditSet::new();
        let valid_start = original.find("1;").ok_or_else(|| missing_fragment("1;"))?;
        // Valid replacement for "1" -> "2"
        edits.add(IncrementalEdit::new(valid_start, valid_start + 1, "2".to_string()));
        let invalid_start = original.find("é").ok_or_else(|| missing_fragment("é"))?;
        // Invalid UTF-8 boundary edit (inside "é")
        edits.add(IncrementalEdit::new(
            invalid_start + 1,
            invalid_start + 2,
            "x".to_string(),
        ));

        doc.apply_edits(&edits)?;

        assert_eq!(doc.source, "my $s = \"é\";\nmy $x = 2;\n");
        Ok(())
    }

    #[test]
    fn incremental_matches_fresh_parse_for_supported_edit() -> ParseResult<()> {
        let original = "my $x = 1;\nmy $y = 2;\n";
        let mut doc = IncrementalDocument::new(original.to_string())?;
        let line_break = original.find('\n').ok_or_else(|| missing_fragment("\\n"))?;
        let edit = IncrementalEdit::new(line_break, line_break, "\n".to_string());

        doc.apply_edit(edit)?;

        let fresh = fresh_parse_debug(&doc.source)?;
        assert_eq!(format!("{:?}", doc.root), fresh);
        Ok(())
    }
}
