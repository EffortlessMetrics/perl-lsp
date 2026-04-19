//! Test for multibyte character handling in incremental edits

#[cfg(test)]
mod multibyte_tests {
    use perl_parser::position::{PositionMapper, WirePosition as Position};
    use ropey::Rope;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn edit_after_multibyte_earlier_in_file() -> TestResult {
        let s = String::from("é\nhello world\n"); // multibyte before target
        let mapper = PositionMapper::new(&s);

        // Replace "world" with "Rust" on line 1
        let start = Position {
            line: 1,
            character: 6,
        };
        let end = Position {
            line: 1,
            character: 11,
        };
        let sb = mapper
            .lsp_pos_to_byte(start)
            .ok_or("invalid start position")?;
        let eb = mapper.lsp_pos_to_byte(end).ok_or("invalid end position")?;

        // Apply via Rope using byte_to_char (the bug is passing sb/eb directly)
        let mut rope = Rope::from_str(&s);
        let (sc, ec) = (rope.byte_to_char(sb), rope.byte_to_char(eb));
        rope.remove(sc..ec);
        rope.insert(sc, "Rust");

        assert_eq!(rope.to_string(), "é\nhello Rust\n");
        Ok(())
    }

    #[test]
    fn edit_with_emoji() -> TestResult {
        let s = String::from("👋 Hello\nWorld 🌍\n");
        let mapper = PositionMapper::new(&s);

        // Replace "Hello" with "Hi" on line 0
        // Note: emoji takes 2 UTF-16 code units
        let start = Position {
            line: 0,
            character: 3,
        }; // After "👋 "
        let end = Position {
            line: 0,
            character: 8,
        }; // After "Hello"

        let sb = mapper
            .lsp_pos_to_byte(start)
            .ok_or("invalid start position")?;
        let eb = mapper.lsp_pos_to_byte(end).ok_or("invalid end position")?;

        let mut rope = Rope::from_str(&s);
        let (sc, ec) = (rope.byte_to_char(sb), rope.byte_to_char(eb));
        rope.remove(sc..ec);
        rope.insert(sc, "Hi");

        assert_eq!(rope.to_string(), "👋 Hi\nWorld 🌍\n");
        Ok(())
    }

    #[test]
    fn edit_multiple_emojis() -> TestResult {
        let s = String::from("let $😀 = 1;\nlet $💖 = 2;\n");
        let mapper = PositionMapper::new(&s);

        // Replace "1" with "42" on line 0
        let start = Position {
            line: 0,
            character: 10,
        }; // Position of "1"
        let end = Position {
            line: 0,
            character: 11,
        };

        let sb = mapper
            .lsp_pos_to_byte(start)
            .ok_or("invalid start position")?;
        let eb = mapper.lsp_pos_to_byte(end).ok_or("invalid end position")?;

        let mut rope = Rope::from_str(&s);
        let (sc, ec) = (rope.byte_to_char(sb), rope.byte_to_char(eb));
        rope.remove(sc..ec);
        rope.insert(sc, "42");

        assert_eq!(rope.to_string(), "let $😀 = 42;\nlet $💖 = 2;\n");
        Ok(())
    }

    #[test]
    fn position_mapper_with_rope() -> TestResult {
        let s = String::from("my $café = 1;\n");
        let mapper = PositionMapper::new(&s);
        let rope = Rope::from_str(&s);

        let pos = Position {
            line: 0,
            character: 8,
        }; // After "café"

        // Test byte-to-char conversion through rope
        let byte_offset = mapper.lsp_pos_to_byte(pos).ok_or("invalid position")?;
        let char_idx = rope.byte_to_char(byte_offset);
        assert_eq!(char_idx, 8); // "my $café" = 8 chars

        // Verify the byte offset is correct
        let pos2 = mapper.byte_to_lsp_pos(byte_offset);
        assert_eq!(pos, pos2);
        Ok(())
    }

    #[test]
    fn sequential_changes_apply_after_each_other_with_multibyte() -> TestResult {
        let original = "é\nhello world\n"; // multibyte before target
        let mut mapper = PositionMapper::new(original);
        let mut rope = Rope::from_str(original);

        // 1) Replace "world" → "Rust"
        let s1 = Position {
            line: 1,
            character: 6,
        };
        let e1 = Position {
            line: 1,
            character: 11,
        };
        let sb1 = mapper.lsp_pos_to_byte(s1).ok_or("invalid s1 position")?;
        let eb1 = mapper.lsp_pos_to_byte(e1).ok_or("invalid e1 position")?;
        let (sc1, ec1) = (rope.byte_to_char(sb1), rope.byte_to_char(eb1));
        rope.remove(sc1..ec1);
        rope.insert(sc1, "Rust");
        mapper.apply_edit(sb1, eb1, "Rust");

        // 2) Insert "!" at end of line (now after previous edit)
        let end = Position {
            line: 1,
            character: 10,
        }; // "hello Rust" → len 10
        let be = mapper.lsp_pos_to_byte(end).ok_or("invalid end position")?;
        let ce = rope.byte_to_char(be);
        rope.insert(ce, "!");

        assert_eq!(rope.to_string(), "é\nhello Rust!\n");
        Ok(())
    }

    #[test]
    fn multi_change_crlf_with_multibyte() -> TestResult {
        let text = "café\r\nhello world\r\n";
        let mut mapper = PositionMapper::new(text);
        let mut rope = Rope::from_str(text);

        // 1) Replace "world" -> "Rust"
        let s1 = Position {
            line: 1,
            character: 6,
        };
        let e1 = Position {
            line: 1,
            character: 11,
        };
        let sb1 = mapper.lsp_pos_to_byte(s1).ok_or("invalid s1 position")?;
        let eb1 = mapper.lsp_pos_to_byte(e1).ok_or("invalid e1 position")?;
        let (sc1, ec1) = (rope.byte_to_char(sb1), rope.byte_to_char(eb1));
        rope.remove(sc1..ec1);
        rope.insert(sc1, "Rust");
        mapper.apply_edit(sb1, eb1, "Rust");

        // 2) Insert "!" at end of line 1 (after CRLF accounting)
        let end = Position {
            line: 1,
            character: 10,
        }; // "hello Rust"
        let be = mapper.lsp_pos_to_byte(end).ok_or("invalid end position")?;
        let ce = rope.byte_to_char(be);
        rope.insert(ce, "!");
        mapper.apply_edit(be, be, "!");

        assert_eq!(rope.to_string(), "café\r\nhello Rust!\r\n");
        Ok(())
    }
}
