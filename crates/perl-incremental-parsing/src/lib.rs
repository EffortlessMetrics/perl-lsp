//! Incremental parsing compatibility shim for Perl.
//!
//! This crate intentionally re-exports [`perl_parser`] incremental APIs.
//! `perl-parser` is the single source of truth for incremental parsing logic.
//!
//! # Migration
//!
//! Prefer importing incremental APIs directly from `perl_parser` in new code.
//! Existing `perl_incremental_parsing` imports remain supported via re-exports.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

#[doc(inline)]
pub use perl_parser::edit;
#[doc(inline)]
pub use perl_parser::{Node, NodeKind, Parser, SourceLocation, ast, error, parser, position};

/// Compatibility re-export of incremental parsing APIs from [`perl_parser::incremental`].
#[doc(inline)]
pub use perl_parser::incremental;

#[doc(inline)]
pub use perl_parser::incremental::*;

#[cfg(test)]
mod tests {
    use super::*;
    use incremental::{Edit, IncrementalState, LineIndex, MAX_EDIT_SIZE, apply_edits};

    // -------------------------------------------------------------------------
    // Constant sanity
    // -------------------------------------------------------------------------

    #[test]
    fn max_edit_size_is_64kb() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(MAX_EDIT_SIZE, 64 * 1024);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // LineIndex — start-of-file, end-of-file, and CRLF boundary positions
    // -------------------------------------------------------------------------

    #[test]
    fn line_index_byte_zero_is_line_zero_col_zero() -> Result<(), Box<dyn std::error::Error>> {
        let li = LineIndex::new("hello");
        assert_eq!(li.byte_to_position(0), (0, 0));
        Ok(())
    }

    #[test]
    fn line_index_position_to_byte_last_column_on_last_line()
    -> Result<(), Box<dyn std::error::Error>> {
        // "ab" — last column is col 2 (one past 'b'), still within the single line
        let li = LineIndex::new("ab");
        assert_eq!(li.position_to_byte(0, 2), Some(2));
        // col 3 is beyond the text length → None
        assert_eq!(li.position_to_byte(0, 3), None);
        Ok(())
    }

    #[test]
    fn line_index_crlf_line_boundary() -> Result<(), Box<dyn std::error::Error>> {
        // "\r\n" counts as two bytes; only '\n' starts the new line.
        // "a\r\nb" — '\n' is at byte 2, so line 1 starts at byte 3.
        let li = LineIndex::new("a\r\nb");
        assert_eq!(li.byte_to_position(3), (1, 0));
        assert_eq!(li.position_to_byte(1, 0), Some(3));
        Ok(())
    }

    #[test]
    fn line_index_empty_string_position_to_byte() -> Result<(), Box<dyn std::error::Error>> {
        let li = LineIndex::new("");
        // Only line 0 exists with zero width; col 0 maps to byte 0.
        assert_eq!(li.position_to_byte(0, 0), Some(0));
        // No line 1.
        assert_eq!(li.position_to_byte(1, 0), None);
        Ok(())
    }

    #[test]
    fn line_index_trailing_newline_empty_final_line() -> Result<(), Box<dyn std::error::Error>> {
        // "x\n" — line 1 is empty; col 0 maps to byte 2 (end of string).
        let li = LineIndex::new("x\n");
        assert_eq!(li.position_to_byte(1, 0), Some(2));
        // col 1 is beyond the empty final line → None
        assert_eq!(li.position_to_byte(1, 1), None);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Edit::from_lsp_change — None branch (out-of-range line)
    // -------------------------------------------------------------------------

    #[test]
    fn edit_from_lsp_change_returns_none_for_out_of_range_line()
    -> Result<(), Box<dyn std::error::Error>> {
        use lsp_types::{Range as LspRange, TextDocumentContentChangeEvent};

        let old = "hello";
        let li = LineIndex::new(old);
        let change = TextDocumentContentChangeEvent {
            range: Some(LspRange {
                start: lsp_types::Position { line: 99, character: 0 },
                end: lsp_types::Position { line: 99, character: 1 },
            }),
            range_length: None,
            text: "x".to_string(),
        };
        let result = Edit::from_lsp_change(&change, &li, old);
        assert!(result.is_none());
        Ok(())
    }

    // -------------------------------------------------------------------------
    // IncrementalState — clone preserves source and token count
    // -------------------------------------------------------------------------

    #[test]
    fn incremental_state_clone_equals_original() -> Result<(), Box<dyn std::error::Error>> {
        let src = "my $x = 1;";
        let state = IncrementalState::new(src.to_string());
        let cloned = state.clone();
        assert_eq!(cloned.source, state.source);
        assert_eq!(cloned.tokens.len(), state.tokens.len());
        assert_eq!(cloned.lex_checkpoints.len(), state.lex_checkpoints.len());
        Ok(())
    }

    // -------------------------------------------------------------------------
    // apply_edits — identity edit (empty replacement) leaves source unchanged
    // -------------------------------------------------------------------------

    #[test]
    fn apply_edits_identity_edit_leaves_source_unchanged() -> Result<(), Box<dyn std::error::Error>>
    {
        let src = "my $x = 1;";
        let mut state = IncrementalState::new(src.to_string());
        // Replace bytes 4..4 (zero-width) with "" — pure identity
        let edit =
            Edit { start_byte: 4, old_end_byte: 4, new_end_byte: 4, new_text: String::new() };
        apply_edits(&mut state, &[edit])?;
        assert_eq!(state.source, src);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // apply_edits — single-char insertion at start of file (byte 0)
    // -------------------------------------------------------------------------

    #[test]
    fn apply_edits_single_char_at_start_of_file() -> Result<(), Box<dyn std::error::Error>> {
        let src = "x = 1;";
        let mut state = IncrementalState::new(src.to_string());
        // Prepend '#' — comment out the line
        let edit =
            Edit { start_byte: 0, old_end_byte: 0, new_end_byte: 1, new_text: "#".to_string() };
        apply_edits(&mut state, &[edit])?;
        assert_eq!(state.source.as_bytes().first(), Some(&b'#'));
        Ok(())
    }

    // -------------------------------------------------------------------------
    // apply_edits — single-char deletion at end of file
    // -------------------------------------------------------------------------

    #[test]
    fn apply_edits_single_char_deletion_at_end_of_file() -> Result<(), Box<dyn std::error::Error>> {
        let src = "my $x = 1;";
        let end = src.len();
        let mut state = IncrementalState::new(src.to_string());
        // Delete the trailing semicolon
        let edit = Edit {
            start_byte: end - 1,
            old_end_byte: end,
            new_end_byte: end - 1,
            new_text: String::new(),
        };
        apply_edits(&mut state, &[edit])?;
        assert!(!state.source.ends_with(';'));
        Ok(())
    }

    // -------------------------------------------------------------------------
    // apply_edits — UTF-8 multi-byte boundary: edit touching a multi-byte char
    // -------------------------------------------------------------------------

    #[test]
    fn apply_edits_replaces_ascii_adjacent_to_multibyte_char()
    -> Result<(), Box<dyn std::error::Error>> {
        // "é" is two bytes (0xC3 0xA9).  We insert ASCII after it, leaving é intact.
        let src = "éx";
        let mut state = IncrementalState::new(src.to_string());
        // Replace 'x' (bytes 2..3) with 'y'
        let edit =
            Edit { start_byte: 2, old_end_byte: 3, new_end_byte: 3, new_text: "y".to_string() };
        apply_edits(&mut state, &[edit])?;
        assert_eq!(&state.source, "éy");
        Ok(())
    }

    // -------------------------------------------------------------------------
    // apply_edits — ReparseResult fields after full reparse
    // -------------------------------------------------------------------------

    #[test]
    fn apply_edits_full_reparse_reparsed_bytes_matches_source_len()
    -> Result<(), Box<dyn std::error::Error>> {
        let src = "my $x = 1;";
        let mut state = IncrementalState::new(src.to_string());
        // A multi-line edit forces a full reparse.
        let new_text = "my $a = 1;\n".repeat(15);
        let new_len = new_text.len();
        let edit = Edit { start_byte: 0, old_end_byte: src.len(), new_end_byte: new_len, new_text };
        let result = apply_edits(&mut state, &[edit])?;
        // After full reparse the result covers the whole new source.
        assert_eq!(result.reparsed_bytes, state.source.len());
        assert_eq!(result.changed_ranges.len(), 1);
        assert_eq!(result.changed_ranges[0].start, 0);
        assert_eq!(result.changed_ranges[0].end, state.source.len());
        Ok(())
    }
}
