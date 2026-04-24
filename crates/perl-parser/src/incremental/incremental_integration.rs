//! Integration module for incremental parsing with the main LSP server
//!
//! This module provides the bridge between the existing LspServer and
//! incremental parsing capabilities, controlled by feature flags.

use super::{
    incremental_document::{IncrementalDocument, ParseMetrics},
    incremental_edit::{IncrementalEdit, IncrementalEditSet},
};
use perl_parser_core::{ast::Node, error::ParseResult, parser::Parser};
use ropey::Rope;
use serde_json::Value;
use std::sync::Arc;

/// Configuration for incremental parsing
pub struct IncrementalConfig {
    /// Enable incremental parsing
    pub enabled: bool,
    /// Target parse time in milliseconds
    pub target_parse_time_ms: f64,
    /// Maximum cache size for subtrees
    pub max_cache_size: usize,
}

impl Default for IncrementalConfig {
    fn default() -> Self {
        Self {
            // Check environment variable to enable incremental parsing
            enabled: std::env::var("PERL_LSP_INCREMENTAL").is_ok(),
            target_parse_time_ms: 1.0,
            max_cache_size: 10000,
        }
    }
}

/// Helper to convert LSP ContentChange to IncrementalEdit
pub fn lsp_change_to_edit(change: &Value, rope: &Rope) -> Option<IncrementalEdit> {
    // Check if this is a full document change or incremental
    if let Some(range) = change.get("range") {
        // Incremental change with range
        let start_line = range["start"]["line"].as_u64()? as usize;
        let start_char = range["start"]["character"].as_u64()? as usize;
        let end_line = range["end"]["line"].as_u64()? as usize;
        let end_char = range["end"]["character"].as_u64()? as usize;

        // Convert LSP positions to byte offsets using rope
        let start_byte = lsp_pos_to_byte(rope, start_line, start_char);
        let end_byte = lsp_pos_to_byte(rope, end_line, end_char);

        let new_text = change["text"].as_str()?.to_string();

        // Create position objects
        let start_position = perl_parser_core::position::Position::new(
            start_byte,
            start_line as u32,
            start_char as u32,
        );
        let old_end_position =
            perl_parser_core::position::Position::new(end_byte, end_line as u32, end_char as u32);

        Some(IncrementalEdit::with_positions(
            start_byte,
            end_byte,
            new_text,
            start_position,
            old_end_position,
        ))
    } else {
        // Full document change - return None to trigger full reparse
        None
    }
}

/// Convert LSP position to byte offset using rope
pub fn lsp_pos_to_byte(rope: &Rope, line: usize, character: usize) -> usize {
    if line >= rope.len_lines() {
        return rope.len_bytes();
    }

    let line_start = rope.line_to_byte(line);
    let line = rope.line(line);

    // Handle UTF-16 code units (LSP uses UTF-16)
    let mut utf16_pos = 0;
    let mut byte_pos = 0;

    for ch in line.chars() {
        if utf16_pos >= character {
            break;
        }
        utf16_pos += ch.len_utf16();
        byte_pos += ch.len_utf8();
    }

    line_start + byte_pos
}

/// Convert byte offset to LSP position using rope
pub fn byte_to_lsp_pos(rope: &Rope, byte_offset: usize) -> (usize, usize) {
    let byte_offset = byte_offset.min(rope.len_bytes());
    let line = rope.byte_to_line(byte_offset);
    let line_start = rope.line_to_byte(line);
    let column_bytes = byte_offset - line_start;

    // Convert byte offset to UTF-16 code units
    let line_str = rope.line(line);
    let mut utf16_pos = 0;
    let mut current_bytes = 0;

    for ch in line_str.chars() {
        if current_bytes >= column_bytes {
            break;
        }
        current_bytes += ch.len_utf8();
        utf16_pos += ch.len_utf16();
    }

    (line, utf16_pos)
}

/// Wrapper for document state with incremental parsing support
pub enum DocumentParser {
    /// Full parsing mode (current implementation)
    Full { content: String, ast: Option<Arc<Node>> },
    /// Incremental parsing mode
    Incremental { document: Box<IncrementalDocument>, rope: Rope },
}

impl DocumentParser {
    /// Create a new document parser based on configuration
    pub fn new(content: String, config: &IncrementalConfig) -> ParseResult<Self> {
        if config.enabled {
            // Use incremental parsing
            let document = IncrementalDocument::new(content.clone())?;
            let rope = Rope::from_str(&content);
            Ok(DocumentParser::Incremental { document: Box::new(document), rope })
        } else {
            // Use full parsing
            let mut parser = Parser::new(&content);
            let ast = parser.parse().ok().map(Arc::new);
            Ok(DocumentParser::Full { content, ast })
        }
    }

    /// Apply changes to the document
    pub fn apply_changes(
        &mut self,
        changes: &[Value],
        _config: &IncrementalConfig,
    ) -> ParseResult<()> {
        match self {
            DocumentParser::Full { content, ast } => {
                // Full document replacement
                if let Some(change) = changes.first() {
                    if let Some(text) = change["text"].as_str() {
                        *content = text.to_string();
                        let mut parser = Parser::new(content);
                        *ast = parser.parse().ok().map(Arc::new);
                    }
                }
                Ok(())
            }
            DocumentParser::Incremental { document: boxed_document, rope } => {
                let document = boxed_document.as_mut();
                // Incremental updates
                let mut edits = Vec::new();

                for change in changes {
                    if let Some(edit) = lsp_change_to_edit(change, rope) {
                        edits.push(edit);
                    } else {
                        // Fall back to full document replacement
                        if let Some(text) = change["text"].as_str() {
                            *document = IncrementalDocument::new(text.to_string())?;
                            *rope = Rope::from_str(text);
                            return Ok(());
                        }
                    }
                }

                if !edits.is_empty() {
                    // Apply incremental edits
                    let edit_set = IncrementalEditSet { edits };
                    document.apply_edits(&edit_set)?;

                    // Update rope to match new content
                    *rope = Rope::from_str(&document.source);
                }

                Ok(())
            }
        }
    }

    /// Get the current AST
    pub fn ast(&self) -> Option<Arc<Node>> {
        match self {
            DocumentParser::Full { ast, .. } => ast.clone(),
            DocumentParser::Incremental { document, .. } => Some(document.root.clone()),
        }
    }

    /// Get the current content
    pub fn content(&self) -> &str {
        match self {
            DocumentParser::Full { content, .. } => content,
            DocumentParser::Incremental { document, .. } => &document.source,
        }
    }

    /// Get parsing metrics (if available)
    pub fn metrics(&self) -> Option<&ParseMetrics> {
        match self {
            DocumentParser::Full { .. } => None,
            DocumentParser::Incremental { document, .. } => Some(document.metrics()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn incremental_config() -> IncrementalConfig {
        IncrementalConfig { enabled: true, ..Default::default() }
    }

    fn parse_debug(source: &str) -> ParseResult<String> {
        let mut parser = Parser::new(source);
        Ok(format!("{:?}", parser.parse()?))
    }

    fn parser_ast_debug(parser: &DocumentParser) -> String {
        parser
            .ast()
            .map(|node| format!("{node:?}"))
            .unwrap_or_else(|| "<none>".to_string())
    }

    #[test]
    fn test_lsp_pos_to_byte() {
        let text = "Hello\nWorld\n";
        let rope = Rope::from_str(text);

        // Start of document
        assert_eq!(lsp_pos_to_byte(&rope, 0, 0), 0);

        // Start of second line
        assert_eq!(lsp_pos_to_byte(&rope, 1, 0), 6);

        // Middle of second line
        assert_eq!(lsp_pos_to_byte(&rope, 1, 3), 9);
    }

    #[test]
    fn test_byte_to_lsp_pos() {
        let text = "Hello\nWorld\n";
        let rope = Rope::from_str(text);

        // Start of document
        assert_eq!(byte_to_lsp_pos(&rope, 0), (0, 0));

        // Start of second line
        assert_eq!(byte_to_lsp_pos(&rope, 6), (1, 0));

        // Middle of second line
        assert_eq!(byte_to_lsp_pos(&rope, 9), (1, 3));
    }

    #[test]
    fn test_crlf_handling() {
        let text = "Hello\r\nWorld\r\n";
        let rope = Rope::from_str(text);

        // Start of second line (after CRLF)
        assert_eq!(lsp_pos_to_byte(&rope, 1, 0), 7);
        assert_eq!(byte_to_lsp_pos(&rope, 7), (1, 0));
    }

    #[test]
    fn test_utf16_handling() {
        let text = "Hello 😀 World"; // Emoji is 2 UTF-16 code units
        let rope = Rope::from_str(text);

        // Position after emoji
        let byte_after_emoji = "Hello 😀".len();
        let (line, char) = byte_to_lsp_pos(&rope, byte_after_emoji);
        assert_eq!(line, 0);
        assert_eq!(char, 8); // "Hello " = 6 + emoji = 2 = 8 UTF-16 units
    }

    #[test]
    fn test_incremental_large_deletion_matches_full_parse() -> ParseResult<()> {
        let source = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\nmy $d = 4;\n";
        let config = incremental_config();
        let mut parser = DocumentParser::new(source.to_string(), &config)?;

        let change = json!({
            "range": {
                "start": {"line": 1, "character": 0},
                "end": {"line": 3, "character": 0}
            },
            "text": ""
        });

        parser.apply_changes(&[change], &config)?;

        let expected = parse_debug(parser.content())?;
        assert_eq!(parser_ast_debug(&parser), expected);
        Ok(())
    }

    #[test]
    fn test_incremental_insertion_invalidation_matches_full_parse() -> ParseResult<()> {
        let source = "my $value = 1;\n";
        let config = incremental_config();
        let mut parser = DocumentParser::new(source.to_string(), &config)?;

        let change = json!({
            "range": {
                "start": {"line": 0, "character": 13},
                "end": {"line": 0, "character": 13}
            },
            "text": " + (2 * 3)"
        });

        parser.apply_changes(&[change], &config)?;

        let expected = parse_debug(parser.content())?;
        assert_eq!(parser_ast_debug(&parser), expected);
        Ok(())
    }

    #[test]
    fn test_incremental_whitespace_only_edit_matches_full_parse() -> ParseResult<()> {
        let source = "my $total=1+2;\n";
        let config = incremental_config();
        let mut parser = DocumentParser::new(source.to_string(), &config)?;

        let change = json!({
            "range": {
                "start": {"line": 0, "character": 9},
                "end": {"line": 0, "character": 9}
            },
            "text": " "
        });

        parser.apply_changes(&[change], &config)?;

        let expected = parse_debug(parser.content())?;
        assert_eq!(parser_ast_debug(&parser), expected);
        Ok(())
    }

    #[test]
    fn test_incremental_multibyte_boundary_edit_matches_full_parse() -> ParseResult<()> {
        let source = "my $emoji = \"😀x\";\n";
        let config = incremental_config();
        let mut parser = DocumentParser::new(source.to_string(), &config)?;
        let rope = Rope::from_str(source);
        let insertion_byte = source.find('x').unwrap_or(0);
        let (line, character) = byte_to_lsp_pos(&rope, insertion_byte);

        let change = json!({
            "range": {
                "start": {"line": line, "character": character},
                "end": {"line": line, "character": character}
            },
            "text": "y"
        });

        parser.apply_changes(&[change], &config)?;

        let expected = parse_debug(parser.content())?;
        assert_eq!(parser_ast_debug(&parser), expected);
        Ok(())
    }

    #[test]
    fn test_incremental_batch_edits_with_independent_shifts_match_full_parse() -> ParseResult<()> {
        let source = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\n";
        let config = incremental_config();
        let mut parser = DocumentParser::new(source.to_string(), &config)?;

        let changes = vec![
            json!({
                "range": {
                    "start": {"line": 0, "character": 9},
                    "end": {"line": 0, "character": 10}
                },
                "text": "11"
            }),
            json!({
                "range": {
                    "start": {"line": 2, "character": 9},
                    "end": {"line": 2, "character": 10}
                },
                "text": "33"
            }),
        ];

        parser.apply_changes(&changes, &config)?;

        let expected = parse_debug(parser.content())?;
        assert_eq!(parser_ast_debug(&parser), expected);
        Ok(())
    }

    #[test]
    fn test_unmappable_edit_falls_back_to_full_document_replace() -> ParseResult<()> {
        let source = "my $x = 1;\n";
        let config = incremental_config();
        let mut parser = DocumentParser::new(source.to_string(), &config)?;
        let replacement = "my $x = ;\n";
        let change = json!({"text": replacement});

        parser.apply_changes(&[change], &config)?;

        assert_eq!(parser.content(), replacement);
        let expected = parse_debug(parser.content())?;
        assert_eq!(parser_ast_debug(&parser), expected);
        Ok(())
    }

    #[test]
    fn test_malformed_incremental_change_does_not_panic() -> ParseResult<()> {
        let source = "my $x = 1;\n";
        let config = incremental_config();
        let mut parser = DocumentParser::new(source.to_string(), &config)?;

        let malformed_change = json!({
            "range": {
                "start": {"line": "zero", "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "text": "broken"
        });

        parser.apply_changes(&[malformed_change], &config)?;
        assert_eq!(parser.content(), source);
        Ok(())
    }
}
