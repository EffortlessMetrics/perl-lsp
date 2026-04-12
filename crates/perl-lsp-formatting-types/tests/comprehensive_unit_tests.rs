//! Comprehensive unit tests for perl-lsp-formatting-types.

use perl_lsp_formatting_types::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingOptions,
};

// ---------------------------------------------------------------------------
// FormatPosition
// ---------------------------------------------------------------------------

#[test]
fn position_new_stores_line_and_character() {
    let pos = FormatPosition::new(3, 7);
    assert_eq!(pos.line, 3);
    assert_eq!(pos.character, 7);
}

#[test]
fn position_new_zero_zero() {
    let pos = FormatPosition::new(0, 0);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 0);
}

#[test]
fn position_new_max_values() {
    let pos = FormatPosition::new(u32::MAX, u32::MAX);
    assert_eq!(pos.line, u32::MAX);
    assert_eq!(pos.character, u32::MAX);
}

#[test]
fn position_clone_is_independent() {
    let pos = FormatPosition::new(1, 2);
    let cloned = pos.clone();
    assert_eq!(cloned.line, pos.line);
    assert_eq!(cloned.character, pos.character);
}

#[test]
fn position_debug_contains_fields() {
    let pos = FormatPosition::new(5, 10);
    let dbg = format!("{pos:?}");
    assert!(dbg.contains("line"));
    assert!(dbg.contains("character"));
}

#[test]
fn position_serialize_uses_field_names() -> Result<(), serde_json::Error> {
    let pos = FormatPosition::new(2, 8);
    let json = serde_json::to_string(&pos)?;
    assert!(json.contains("\"line\":2"));
    assert!(json.contains("\"character\":8"));
    Ok(())
}

#[test]
fn position_roundtrip_serde() -> Result<(), serde_json::Error> {
    let pos = FormatPosition::new(100, 200);
    let json = serde_json::to_string(&pos)?;
    let back: FormatPosition = serde_json::from_str(&json)?;
    assert_eq!(back.line, 100);
    assert_eq!(back.character, 200);
    Ok(())
}

#[test]
fn position_deserialize_from_json() -> Result<(), serde_json::Error> {
    let json = r#"{"line":42,"character":99}"#;
    let pos: FormatPosition = serde_json::from_str(json)?;
    assert_eq!(pos.line, 42);
    assert_eq!(pos.character, 99);
    Ok(())
}

// ---------------------------------------------------------------------------
// FormatRange
// ---------------------------------------------------------------------------

#[test]
fn range_new_stores_start_and_end() {
    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(5, 10));
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 5);
    assert_eq!(range.end.character, 10);
}

#[test]
fn range_clone_is_independent() {
    let range = FormatRange::new(FormatPosition::new(1, 2), FormatPosition::new(3, 4));
    let cloned = range.clone();
    assert_eq!(cloned.start.line, range.start.line);
    assert_eq!(cloned.end.character, range.end.character);
}

#[test]
fn range_debug_contains_start_end() {
    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(1, 1));
    let dbg = format!("{range:?}");
    assert!(dbg.contains("start"));
    assert!(dbg.contains("end"));
}

#[test]
fn range_serialize_roundtrip() -> Result<(), serde_json::Error> {
    let range = FormatRange::new(FormatPosition::new(10, 20), FormatPosition::new(30, 40));
    let json = serde_json::to_string(&range)?;
    let back: FormatRange = serde_json::from_str(&json)?;
    assert_eq!(back.start.line, 10);
    assert_eq!(back.start.character, 20);
    assert_eq!(back.end.line, 30);
    assert_eq!(back.end.character, 40);
    Ok(())
}

// ---------------------------------------------------------------------------
// FormatRange::whole_document
// ---------------------------------------------------------------------------

#[test]
fn whole_document_empty_string() {
    let range = FormatRange::whole_document("");
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 0);
}

#[test]
fn whole_document_single_line_no_newline() {
    let range = FormatRange::whole_document("hello");
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 5);
}

#[test]
fn whole_document_single_line_with_newline() {
    // "hello\n" — .lines() yields ["hello"], last_line=0, len=5
    let range = FormatRange::whole_document("hello\n");
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 5);
}

#[test]
fn whole_document_multiple_lines() {
    let content = "line0\nline1\nline2";
    let range = FormatRange::whole_document(content);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.character, 5); // "line2".len() == 5
}

#[test]
fn whole_document_multiple_lines_trailing_newline() {
    let content = "aaa\nbbb\nccc\n";
    let range = FormatRange::whole_document(content);
    // .lines() yields ["aaa","bbb","ccc"] → last_line=2, char=3
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.character, 3);
}

#[test]
fn whole_document_unicode_content() {
    // LSP positions use UTF-16 code units, not byte lengths.
    // "héllo": é (U+00E9) is 2 UTF-8 bytes but 1 UTF-16 code unit.
    // "wörld": ö (U+00F6) is 2 UTF-8 bytes but 1 UTF-16 code unit.
    // Both lines: 5 chars, 5 UTF-16 units, 6 bytes.
    let content = "héllo\nwörld";
    let range = FormatRange::whole_document(content);
    assert_eq!(range.end.line, 1);
    // "wörld" = 5 UTF-16 code units (ö is BMP, counts as 1 unit)
    assert_eq!(range.end.character, 5);
}

#[test]
fn whole_document_single_newline() {
    // "\n" — .lines() yields [""] (one empty line), last_line=0, len=0
    let range = FormatRange::whole_document("\n");
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 0);
}

#[test]
fn whole_document_blank_lines() {
    let content = "\n\n\n";
    let range = FormatRange::whole_document(content);
    // .lines() yields ["","",""] → last_line=2, char=0
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.character, 0);
}

// ---------------------------------------------------------------------------
// FormatTextEdit
// ---------------------------------------------------------------------------

#[test]
fn text_edit_construction() {
    let edit = FormatTextEdit {
        range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 5)),
        new_text: "replaced".to_string(),
    };
    assert_eq!(edit.new_text, "replaced");
    assert_eq!(edit.range.start.line, 0);
    assert_eq!(edit.range.end.character, 5);
}

#[test]
fn text_edit_empty_new_text() {
    let edit = FormatTextEdit {
        range: FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(1, 10)),
        new_text: String::new(),
    };
    assert!(edit.new_text.is_empty());
}

#[test]
fn text_edit_clone_is_independent() {
    let edit = FormatTextEdit {
        range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 1)),
        new_text: "a".to_string(),
    };
    let cloned = edit.clone();
    assert_eq!(cloned.new_text, edit.new_text);
    assert_eq!(cloned.range.start.line, edit.range.start.line);
}

#[test]
fn text_edit_debug_contains_fields() {
    let edit = FormatTextEdit {
        range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 1)),
        new_text: "x".to_string(),
    };
    let dbg = format!("{edit:?}");
    assert!(dbg.contains("new_text"));
    assert!(dbg.contains("range"));
}

#[test]
fn text_edit_serialize_uses_camel_case() -> Result<(), serde_json::Error> {
    let edit = FormatTextEdit {
        range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 3)),
        new_text: "foo".to_string(),
    };
    let json = serde_json::to_string(&edit)?;
    // serde rename: new_text → newText
    assert!(json.contains("\"newText\""));
    assert!(!json.contains("\"new_text\""));
    Ok(())
}

#[test]
fn text_edit_deserialize_camel_case() -> Result<(), serde_json::Error> {
    let json = r#"{
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 0, "character": 5}
        },
        "newText": "hello"
    }"#;
    let edit: FormatTextEdit = serde_json::from_str(json)?;
    assert_eq!(edit.new_text, "hello");
    assert_eq!(edit.range.start.line, 0);
    assert_eq!(edit.range.end.character, 5);
    Ok(())
}

#[test]
fn text_edit_roundtrip_serde() -> Result<(), serde_json::Error> {
    let edit = FormatTextEdit {
        range: FormatRange::new(FormatPosition::new(7, 3), FormatPosition::new(7, 15)),
        new_text: "replacement text".to_string(),
    };
    let json = serde_json::to_string(&edit)?;
    let back: FormatTextEdit = serde_json::from_str(&json)?;
    assert_eq!(back.new_text, "replacement text");
    assert_eq!(back.range.start.line, 7);
    assert_eq!(back.range.end.character, 15);
    Ok(())
}

// ---------------------------------------------------------------------------
// FormattingOptions
// ---------------------------------------------------------------------------

#[test]
fn formatting_options_all_fields_set() {
    let opts = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(false),
    };
    assert_eq!(opts.tab_size, 4);
    assert!(opts.insert_spaces);
    assert_eq!(opts.trim_trailing_whitespace, Some(true));
    assert_eq!(opts.insert_final_newline, Some(true));
    assert_eq!(opts.trim_final_newlines, Some(false));
}

#[test]
fn formatting_options_optional_fields_none() {
    let opts = FormattingOptions {
        tab_size: 2,
        insert_spaces: false,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    assert_eq!(opts.tab_size, 2);
    assert!(!opts.insert_spaces);
    assert!(opts.trim_trailing_whitespace.is_none());
    assert!(opts.insert_final_newline.is_none());
    assert!(opts.trim_final_newlines.is_none());
}

#[test]
fn formatting_options_clone() {
    let opts = FormattingOptions {
        tab_size: 8,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: None,
        trim_final_newlines: Some(true),
    };
    let cloned = opts.clone();
    assert_eq!(cloned.tab_size, opts.tab_size);
    assert_eq!(cloned.insert_spaces, opts.insert_spaces);
    assert_eq!(cloned.trim_trailing_whitespace, opts.trim_trailing_whitespace);
}

#[test]
fn formatting_options_debug() {
    let opts = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let dbg = format!("{opts:?}");
    assert!(dbg.contains("tab_size"));
    assert!(dbg.contains("insert_spaces"));
}

#[test]
fn formatting_options_deserialize_camel_case() -> Result<(), serde_json::Error> {
    let json = r#"{
        "tabSize": 2,
        "insertSpaces": false,
        "trimTrailingWhitespace": true,
        "insertFinalNewline": false,
        "trimFinalNewlines": null
    }"#;
    let opts: FormattingOptions = serde_json::from_str(json)?;
    assert_eq!(opts.tab_size, 2);
    assert!(!opts.insert_spaces);
    assert_eq!(opts.trim_trailing_whitespace, Some(true));
    assert_eq!(opts.insert_final_newline, Some(false));
    assert!(opts.trim_final_newlines.is_none());
    Ok(())
}

#[test]
fn formatting_options_deserialize_missing_optionals() -> Result<(), serde_json::Error> {
    let json = r#"{"tabSize": 4, "insertSpaces": true}"#;
    let opts: FormattingOptions = serde_json::from_str(json)?;
    assert_eq!(opts.tab_size, 4);
    assert!(opts.insert_spaces);
    assert!(opts.trim_trailing_whitespace.is_none());
    assert!(opts.insert_final_newline.is_none());
    assert!(opts.trim_final_newlines.is_none());
    Ok(())
}

#[test]
fn formatting_options_deserialize_rejects_missing_required() {
    // tabSize is required
    let json = r#"{"insertSpaces": true}"#;
    let result = serde_json::from_str::<FormattingOptions>(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// FormattedDocument
// ---------------------------------------------------------------------------

#[test]
fn formatted_document_construction() {
    let doc = FormattedDocument {
        text: "formatted content".to_string(),
        edits: vec![FormatTextEdit {
            range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 7)),
            new_text: "formatted".to_string(),
        }],
    };
    assert_eq!(doc.text, "formatted content");
    assert_eq!(doc.edits.len(), 1);
}

#[test]
fn formatted_document_empty_edits() {
    let doc = FormattedDocument { text: "unchanged".to_string(), edits: vec![] };
    assert!(doc.edits.is_empty());
    assert_eq!(doc.text, "unchanged");
}

#[test]
fn formatted_document_multiple_edits() {
    let doc = FormattedDocument {
        text: "result".to_string(),
        edits: vec![
            FormatTextEdit {
                range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 1)),
                new_text: "a".to_string(),
            },
            FormatTextEdit {
                range: FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(1, 1)),
                new_text: "b".to_string(),
            },
        ],
    };
    assert_eq!(doc.edits.len(), 2);
}

#[test]
fn formatted_document_clone_is_independent() {
    let doc = FormattedDocument {
        text: "text".to_string(),
        edits: vec![FormatTextEdit {
            range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 4)),
            new_text: "TEXT".to_string(),
        }],
    };
    let cloned = doc.clone();
    assert_eq!(cloned.text, doc.text);
    assert_eq!(cloned.edits.len(), doc.edits.len());
}

#[test]
fn formatted_document_debug() {
    let doc = FormattedDocument { text: "hello".to_string(), edits: vec![] };
    let dbg = format!("{doc:?}");
    assert!(dbg.contains("text"));
    assert!(dbg.contains("edits"));
}

// ---------------------------------------------------------------------------
// Integration: whole_document used in a FormatTextEdit
// ---------------------------------------------------------------------------

#[test]
fn whole_document_edit_replaces_all_content() -> Result<(), serde_json::Error> {
    let original = "sub foo {\n    return 1;\n}\n";
    let range = FormatRange::whole_document(original);
    let edit = FormatTextEdit { range, new_text: "sub foo {\n  return 1;\n}\n".to_string() };

    // Verify serialization preserves camelCase
    let json = serde_json::to_string(&edit)?;
    assert!(json.contains("\"newText\""));

    // Verify range covers full document
    assert_eq!(edit.range.start.line, 0);
    assert_eq!(edit.range.start.character, 0);
    // .lines() yields ["sub foo {", "    return 1;", "}"]  last_line=2, char=1
    assert_eq!(edit.range.end.line, 2);
    assert_eq!(edit.range.end.character, 1);
    Ok(())
}

#[test]
fn formatted_document_with_whole_document_range() {
    let content = "my $x = 1;\nmy $y = 2;\n";
    let formatted = "my $x = 1;\nmy $y = 2;\n";
    let doc = FormattedDocument {
        text: formatted.to_string(),
        edits: vec![FormatTextEdit {
            range: FormatRange::whole_document(content),
            new_text: formatted.to_string(),
        }],
    };
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].range.start.line, 0);
}

// ---------------------------------------------------------------------------
// Serde edge cases
// ---------------------------------------------------------------------------

#[test]
fn position_deserialize_extra_fields_ignored() -> Result<(), serde_json::Error> {
    let json = r#"{"line":1,"character":2,"extra":"ignored"}"#;
    let pos: FormatPosition = serde_json::from_str(json)?;
    assert_eq!(pos.line, 1);
    assert_eq!(pos.character, 2);
    Ok(())
}

#[test]
fn range_deserialize_nested_json() -> Result<(), serde_json::Error> {
    let json = r#"{"start":{"line":0,"character":0},"end":{"line":99,"character":80}}"#;
    let range: FormatRange = serde_json::from_str(json)?;
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 99);
    assert_eq!(range.end.character, 80);
    Ok(())
}

#[test]
fn text_edit_deserialize_multiline_new_text() -> Result<(), serde_json::Error> {
    let json = r#"{
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 0, "character": 0}
        },
        "newText": "line1\nline2\nline3"
    }"#;
    let edit: FormatTextEdit = serde_json::from_str(json)?;
    assert_eq!(edit.new_text, "line1\nline2\nline3");
    Ok(())
}
