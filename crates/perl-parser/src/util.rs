//! Utility functions for the Perl parser

/// Find the byte offset of a __DATA__ or __END__ marker in the source text.
/// Uses the lexer to avoid false positives in heredocs/POD.
/// Returns the byte offset of the start of the marker, or None if not found.
pub fn find_data_marker_byte_lexed(s: &str) -> Option<usize> {
    use perl_lexer::{PerlLexer, TokenType};
    let mut lx = PerlLexer::new(s);
    while let Some(tok) = lx.next_token() {
        match tok.token_type {
            TokenType::DataMarker(_) => return Some(tok.start),
            TokenType::EOF => break,
            _ => {}
        }
    }
    None
}

/// Helper to get the code portion of text (before __DATA__/__END__)
pub fn code_slice(text: &str) -> &str {
    find_data_marker_byte_lexed(text).map(|i| &text[..i]).unwrap_or(text)
}

/// Find the byte offset of a __DATA__ or __END__ marker in the source text.
/// Returns the byte offset of the start of the marker line, or None if not found.
#[deprecated(note = "Use find_data_marker_byte_lexed to avoid false positives in heredocs/POD")]
pub fn find_data_marker_byte(s: &str) -> Option<usize> {
    find_data_marker_byte_lexed(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_data_marker_lexed() {
        // No marker
        assert_eq!(find_data_marker_byte_lexed("print 'hello';\n"), None);

        // __DATA__ marker
        let src = "print 'hello';\n__DATA__\ndata here";
        assert_eq!(find_data_marker_byte_lexed(src), Some(15));

        // __END__ marker with spaces
        let src2 = "code;\n  __END__\ndata";
        assert_eq!(find_data_marker_byte_lexed(src2), Some(6));

        // Marker not at line start (should not match)
        let src3 = "print '__DATA__';\n";
        assert_eq!(find_data_marker_byte_lexed(src3), None);
    }

    #[test]
    fn test_code_slice() {
        // No marker - returns full text
        assert_eq!(code_slice("print 'hello';\n"), "print 'hello';\n");

        // With __DATA__ marker
        let src = "print 'hello';\n__DATA__\ndata here";
        assert_eq!(code_slice(src), "print 'hello';\n");

        // With __END__ marker
        let src2 = "code;\n__END__\ndata";
        assert_eq!(code_slice(src2), "code;\n");
    }
}
