use perl_symbol::cursor::{
    CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source, get_symbol_range_at_position,
    token_under_cursor,
};
use perl_tdd_support::must_some;

fn char_position(source: &str, needle: &str) -> Result<usize, String> {
    let byte_idx = source.find(needle).ok_or_else(|| format!("missing '{needle}' in fixture"))?;
    Ok(source[..byte_idx].chars().count())
}

fn char_slice(source: &str, start: usize, end: usize) -> String {
    source.chars().skip(start).take(end - start).collect()
}

mod byte_position_apis {
    use super::*;

    #[test]
    fn extract_handles_multibyte_prefix_near_cursor() -> Result<(), String> {
        let source = "my 😀 $name = 1;";
        let pos = char_position(source, "name")?;
        let (name, kind) = must_some(extract_symbol_from_source(pos, source));

        assert_eq!(name, "name");
        assert_eq!(kind, CursorSymbolKind::Scalar);
        Ok(())
    }

    #[test]
    fn range_handles_multibyte_prefix_near_cursor() -> Result<(), String> {
        let source = "my 😀 $name = 1;";
        let pos = char_position(source, "name")?;
        let (start, end) = must_some(get_symbol_range_at_position(pos, source));

        assert_eq!(char_slice(source, start, end), "$name");
        Ok(())
    }

    #[test]
    fn extract_cursor_in_middle_of_bareword_keeps_suffix() {
        let source = "run_worker();";
        let pos = 4; // middle of "run_worker"
        let (name, kind) = must_some(extract_symbol_from_source(pos, source));

        assert_eq!(name, "worker");
        assert_eq!(kind, CursorSymbolKind::Subroutine);
    }

    #[test]
    fn extract_cursor_on_sigil_and_after_sigil_match() {
        let source = "$value";
        let on_sigil = must_some(extract_symbol_from_source(0, source));
        let after_sigil = must_some(extract_symbol_from_source(1, source));

        assert_eq!(on_sigil, after_sigil);
    }

    #[test]
    fn extract_and_range_split_qualified_names_at_double_colon() -> Result<(), String> {
        let source = "Foo::bar";

        let left_pos = char_position(source, "Foo")?;
        let right_pos = char_position(source, "bar")?;

        let (left_name, _) = must_some(extract_symbol_from_source(left_pos, source));
        let (left_start, left_end) = must_some(get_symbol_range_at_position(left_pos, source));
        assert_eq!(left_name, "Foo");
        assert_eq!(char_slice(source, left_start, left_end), "Foo");

        let (right_name, _) = must_some(extract_symbol_from_source(right_pos, source));
        let (right_start, right_end) = must_some(get_symbol_range_at_position(right_pos, source));
        assert_eq!(right_name, "bar");
        assert_eq!(char_slice(source, right_start, right_end), "bar");

        Ok(())
    }

    #[test]
    fn extract_degrades_conservatively_on_deref_cursored_on_second_sigil() {
        let source = "$$ref";
        assert_eq!(extract_symbol_from_source(1, source), None);
    }

    #[test]
    fn extract_and_range_are_symmetric_for_scalar_symbol() -> Result<(), String> {
        let source = "my $count = 42;";
        let pos = char_position(source, "count")?;

        let (name, kind) = must_some(extract_symbol_from_source(pos, source));
        let (start, end) = must_some(get_symbol_range_at_position(pos, source));

        assert_eq!(kind, CursorSymbolKind::Scalar);
        assert_eq!(char_slice(source, start, end), format!("${name}"));
        assert_eq!(end - start, name.chars().count() + 1);
        Ok(())
    }
}

mod utf16_cursor_api {
    use super::*;

    #[test]
    fn token_under_cursor_handles_multibyte_column_boundaries() {
        let text = "use 😀Demo::Worker;\n";
        // "u"(0), "s"(1), "e"(2), " "(3), "😀"(4-5 UTF-16), "D"(6)
        let token_at_first_surrogate = must_some(token_under_cursor(text, 0, 4));
        let token_at_second_surrogate = must_some(token_under_cursor(text, 0, 5));
        let token_at_module_start = must_some(token_under_cursor(text, 0, 6));

        assert_eq!(token_at_first_surrogate, "");
        assert_eq!(token_at_second_surrogate, "");
        assert_eq!(token_at_module_start, "Demo::Worker");
    }

    #[test]
    fn token_under_cursor_handles_crlf_lines() {
        let text = "my $first = 1;\r\nmy $second = 2;\r\n";
        let token = must_some(token_under_cursor(text, 1, 5));
        assert_eq!(token, "$second");
    }

    #[test]
    fn token_under_cursor_keeps_typeglob_prefix_conservatively() {
        let text = "print *STDOUT;\n";
        let token = must_some(token_under_cursor(text, 0, 8));
        assert_eq!(token, "*STDOUT");
    }

    #[test]
    fn utf16_token_and_range_agree_on_ascii_scalar_span() -> Result<(), String> {
        let source = "my $alpha = 1;";
        let token = must_some(token_under_cursor(source, 0, 5));
        let pos = char_position(source, "alpha")?;
        let (start, end) = must_some(get_symbol_range_at_position(pos, source));

        assert_eq!(token, char_slice(source, start, end));
        Ok(())
    }

    #[test]
    fn byte_offset_utf16_and_token_cursor_agree_at_module_start() {
        let text = "use 😀Demo::Worker;\n";
        let line = "use 😀Demo::Worker;";
        let module_start_utf16 = 6;
        let byte_pos = byte_offset_utf16(line, module_start_utf16);
        let token = must_some(token_under_cursor(text, 0, module_start_utf16));

        assert_eq!(&line[byte_pos..byte_pos + 4], "Demo");
        assert_eq!(token, "Demo::Worker");
    }
}
