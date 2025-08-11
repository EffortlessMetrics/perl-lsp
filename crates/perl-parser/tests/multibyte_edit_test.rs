//! Test for multibyte character handling in incremental edits

#[cfg(test)]
mod multibyte_tests {
    use perl_parser::position_mapper::{PositionMapper, Position};
    use ropey::Rope;

    #[test]
    fn edit_after_multibyte_earlier_in_file() {
        let s = String::from("é\nhello world\n"); // multibyte before target
        let mapper = PositionMapper::new(&s);

        // Replace "world" with "Rust" on line 1
        let start = Position { line: 1, character: 6 };
        let end = Position { line: 1, character: 11 };
        let (sb, eb) = (
            mapper.lsp_pos_to_byte(start).unwrap(),
            mapper.lsp_pos_to_byte(end).unwrap()
        );

        // Apply via Rope using byte_to_char (the bug is passing sb/eb directly)
        let mut rope = Rope::from_str(&s);
        let (sc, ec) = (rope.byte_to_char(sb), rope.byte_to_char(eb));
        rope.remove(sc..ec);
        rope.insert(sc, "Rust");

        assert_eq!(rope.to_string(), "é\nhello Rust\n");
    }

    #[test]
    fn edit_with_emoji() {
        let s = String::from("👋 Hello\nWorld 🌍\n");
        let mapper = PositionMapper::new(&s);

        // Replace "Hello" with "Hi" on line 0
        // Note: emoji takes 2 UTF-16 code units
        let start = Position { line: 0, character: 3 }; // After "👋 "
        let end = Position { line: 0, character: 8 };   // After "Hello"
        
        let (sb, eb) = (
            mapper.lsp_pos_to_byte(start).unwrap(),
            mapper.lsp_pos_to_byte(end).unwrap()
        );

        let mut rope = Rope::from_str(&s);
        let (sc, ec) = (rope.byte_to_char(sb), rope.byte_to_char(eb));
        rope.remove(sc..ec);
        rope.insert(sc, "Hi");

        assert_eq!(rope.to_string(), "👋 Hi\nWorld 🌍\n");
    }

    #[test]
    fn edit_multiple_emojis() {
        let s = String::from("let $😀 = 1;\nlet $💖 = 2;\n");
        let mapper = PositionMapper::new(&s);

        // Replace "1" with "42" on line 0
        let start = Position { line: 0, character: 10 }; // Position of "1"
        let end = Position { line: 0, character: 11 };
        
        let (sb, eb) = (
            mapper.lsp_pos_to_byte(start).unwrap(),
            mapper.lsp_pos_to_byte(end).unwrap()
        );

        let mut rope = Rope::from_str(&s);
        let (sc, ec) = (rope.byte_to_char(sb), rope.byte_to_char(eb));
        rope.remove(sc..ec);
        rope.insert(sc, "42");

        assert_eq!(rope.to_string(), "let $😀 = 42;\nlet $💖 = 2;\n");
    }

    #[test]
    fn position_mapper_char_conversion() {
        let s = String::from("🦀 Rust\n💖 Perl\n");
        let mapper = PositionMapper::new(&s);

        // Test lsp_pos_to_char
        let pos = Position { line: 1, character: 3 }; // After "💖 "
        let char_idx = mapper.lsp_pos_to_char(pos).unwrap();
        
        // Convert back and verify round-trip
        let pos2 = mapper.char_to_lsp_pos(char_idx);
        assert_eq!(pos, pos2);
    }

}