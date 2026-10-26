//! Invalid UTF-8 edit endpoints must not round into neighboring text.

use perl_position_tracking::{LineEnding, PositionMapper, WirePosition};

#[test]
fn interior_utf8_edit_endpoints_preserve_document() -> Result<(), Box<dyn std::error::Error>> {
    // text, scalar start byte, scalar width, valid replacement result
    for (text, start, width, expected) in [
        ("a😀b\r\n", 1, 4, "aXb\r\n"),
        ("aé b\r\n", 1, 2, "aX b\r\n"),
        ("a你b\r\n", 1, 3, "aXb\r\n"),
        ("head\r\nx😀y", 7, 4, "head\r\nxXy"),
        ("a😀", 1, 4, "aX"),
    ] {
        let end = start + width;
        for interior in start + 1..end {
            for (edit_start, edit_end) in [(interior, end), (start, interior), (interior, interior)]
            {
                let mut mapper = PositionMapper::new(text);
                let line_count = mapper.len_lines();
                let line_ending = mapper.line_ending();
                let positions: Vec<_> = text
                    .char_indices()
                    .map(|(byte, _)| byte)
                    .chain(std::iter::once(text.len()))
                    .map(|byte| (byte, mapper.byte_to_lsp_pos(byte)))
                    .collect();

                // Nonempty text exposes an invalid end or insertion even when
                // both rounded character indices would otherwise be equal.
                mapper.apply_edit(edit_start, edit_end, "X");

                assert_eq!(
                    mapper.text(),
                    text,
                    "invalid bytes {edit_start}..{edit_end} in {text:?}"
                );
                assert_eq!(mapper.len_bytes(), text.len());
                assert_eq!(mapper.len_lines(), line_count);
                assert_eq!(mapper.line_ending(), line_ending);
                for (byte, position) in positions {
                    assert_eq!(mapper.byte_to_lsp_pos(byte), position);
                    assert_eq!(mapper.lsp_pos_to_byte(position), Some(byte));
                }
            }
        }

        // Paired positive control: rejecting all edits would fail this case.
        let mut mapper = PositionMapper::new(text);
        mapper.apply_edit(start, end, "X");
        assert_eq!(mapper.text(), expected);
    }
    Ok(())
}

#[test]
fn valid_utf16_emoji_edit_and_crlf_deletion_preserve_mapping()
-> Result<(), Box<dyn std::error::Error>> {
    let mut mapper = PositionMapper::new("a😀\r\nb");
    let before_emoji = WirePosition { line: 0, character: 1 };
    let after_emoji = WirePosition { line: 0, character: 3 };
    assert_eq!(mapper.lsp_pos_to_byte(before_emoji), Some(1));
    assert_eq!(mapper.lsp_pos_to_byte(after_emoji), Some(5));
    let start = mapper.lsp_pos_to_byte(before_emoji).ok_or("missing emoji start")?;
    let end = mapper.lsp_pos_to_byte(after_emoji).ok_or("missing emoji end")?;
    mapper.apply_edit(start, end, "é");
    assert_eq!(mapper.text(), "aé\r\nb");
    assert_eq!(mapper.line_ending(), LineEnding::CrLf);
    assert_eq!(mapper.len_lines(), 2);
    assert_eq!(mapper.lsp_pos_to_byte(WirePosition { line: 1, character: 0 }), Some(5));
    assert_eq!(mapper.byte_to_lsp_pos(5), WirePosition { line: 1, character: 0 });

    // The whole CRLF is a valid byte range. Removing it joins the lines.
    mapper.apply_edit(3, 5, "");
    assert_eq!(mapper.text(), "aéb");
    assert_eq!(mapper.line_ending(), LineEnding::Lf);
    assert_eq!(mapper.len_lines(), 1);
    assert_eq!(mapper.byte_to_lsp_pos(3), WirePosition { line: 0, character: 2 });
    Ok(())
}

#[test]
fn valid_insertions_beside_emoji_and_at_eof() -> Result<(), Box<dyn std::error::Error>> {
    for (text, byte, expected) in [
        ("", 0, "X"),
        ("😀", 0, "X😀"),
        ("😀", 4, "😀X"),
        ("😀\r\n", 4, "😀X\r\n"),
        ("😀\r\n", 6, "😀\r\nX"),
    ] {
        let mut mapper = PositionMapper::new(text);
        mapper.apply_edit(byte, byte, "X");
        assert_eq!(mapper.text(), expected);
        assert_eq!(mapper.len_bytes(), expected.len());
    }
    Ok(())
}
