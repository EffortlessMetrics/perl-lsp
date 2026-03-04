//! LSP wire types and UTF-8/UTF-16 conversion helpers.

pub use convert::{offset_to_utf16_line_col, utf16_line_col_to_offset};
pub use wire::{WireLocation, WirePosition, WireRange};

mod convert;
mod wire;

#[cfg(test)]
mod tests {
    use crate::{WirePosition, offset_to_utf16_line_col, utf16_line_col_to_offset};

    #[test]
    fn roundtrips_multibyte_character_position() {
        let source = "ok\n💚x\n";
        let byte_offset = source.find('💚').unwrap_or_default();
        let wire = WirePosition::from_byte_offset(source, byte_offset + '💚'.len_utf8());

        assert_eq!(wire.line, 1);
        assert_eq!(wire.character, 2);
        assert_eq!(wire.to_byte_offset(source), byte_offset + '💚'.len_utf8());
    }

    #[test]
    fn converts_line_column_to_offset_within_surrogate_pair() {
        let source = "💚";
        assert_eq!(utf16_line_col_to_offset(source, 0, 1), 0);
    }

    #[test]
    fn clamps_offset_past_end_to_last_position() {
        let source = "a\nβ";
        assert_eq!(offset_to_utf16_line_col(source, source.len() + 50), (1, 1));
    }
}
