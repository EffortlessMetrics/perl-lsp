//! Tokenization utilities shared by parser-facing entry points.
//!
//! Helpers in this module identify Perl data-section markers (`__DATA__` and
//! `__END__`) using lexer tokens, so callers can safely split executable code
//! from trailing payload without matching markers embedded in strings,
//! heredocs, or POD content.

/// Find the byte offset of a __DATA__ or __END__ marker in the source text.
/// Uses the lexer to avoid false positives in heredocs/POD.
/// Returns the byte offset of the start of the marker, or None if not found.
const DATA_SECTION_MARKERS: [&str; 2] = ["__DATA__", "__END__"];

fn may_contain_data_marker(text: &str) -> bool {
    DATA_SECTION_MARKERS.iter().any(|marker| text.contains(marker))
}

pub fn find_data_marker_byte_lexed(s: &str) -> Option<usize> {
    // Cheap prefilter: avoid constructing the lexer when marker substrings are absent.
    if !may_contain_data_marker(s) {
        return None;
    }

    use crate::{PerlLexer, TokenType};
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
    split_code_and_data(text).0
}

/// Split source text into executable code and optional trailing data section.
///
/// The data section starts at a lexed `__DATA__` or `__END__` marker and includes
/// the marker line itself.
pub fn split_code_and_data(text: &str) -> (&str, Option<&str>) {
    if let Some(marker_start) = find_data_marker_byte_lexed(text) {
        (&text[..marker_start], Some(&text[marker_start..]))
    } else {
        (text, None)
    }
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
    fn test_may_contain_data_marker_prefilter() {
        assert!(!may_contain_data_marker("print 'hello';\n"));
        assert!(may_contain_data_marker("__DATA__"));
        assert!(may_contain_data_marker("prefix __END__ suffix"));
    }

    #[test]
    fn test_split_code_and_data_handles_crlf_line_endings() {
        let src = "print 'ok';\r\n__DATA__\r\nvalue";
        assert_eq!(split_code_and_data(src), ("print 'ok';\r\n", Some("__DATA__\r\nvalue")));
    }

    #[test]
    fn test_find_data_marker_lexed() {
        // No marker
        assert_eq!(find_data_marker_byte_lexed("print 'hello';\n"), None);

        // __DATA__ marker
        let src = "print 'hello';\n__DATA__\ndata here";
        assert_eq!(find_data_marker_byte_lexed(src), Some(15));

        // __END__ marker at line start
        let src2 = "code;\n__END__\ndata";
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

    #[test]
    fn test_split_code_and_data_prefers_first_marker() {
        let src = "print 'a';\n__DATA__\none\n__END__\ntwo";
        assert_eq!(split_code_and_data(src), ("print 'a';\n", Some("__DATA__\none\n__END__\ntwo")));
    }

    #[test]
    fn test_find_data_marker_ignores_markers_inside_heredoc_and_pod() {
        let heredoc = "my $x = <<'TXT';\n__DATA__\nTXT\nprint $x;\n";
        assert_eq!(find_data_marker_byte_lexed(heredoc), None);

        let pod = "=pod\n__END__\n=cut\nprint 'ok';\n";
        assert_eq!(find_data_marker_byte_lexed(pod), None);
    }

    #[test]
    fn test_split_code_and_data() {
        let no_marker = "print 'hello';\n";
        assert_eq!(split_code_and_data(no_marker), (no_marker, None));

        let with_data = "print 'hello';\n__DATA__\nvalue";
        assert_eq!(split_code_and_data(with_data), ("print 'hello';\n", Some("__DATA__\nvalue")));

        let with_end = "code;\n__END__\nvalue";
        assert_eq!(split_code_and_data(with_end), ("code;\n", Some("__END__\nvalue")));
    }

    #[test]
    fn test_find_data_marker_ignores_pod_and_heredoc_content() {
        let pod = "=head1 NAME\n__DATA__\n=cut\nprint 'done';\n";
        assert_eq!(find_data_marker_byte_lexed(pod), None);

        let heredoc = "my $text = <<\"TXT\";\n__END__\nTXT\nprint $text;\n";
        assert_eq!(find_data_marker_byte_lexed(heredoc), None);
    }

    #[test]
    fn test_split_code_and_data_prefers_first_lexed_marker() {
        let src = "print 'prelude';\n__DATA__\nchunk\n__END__\nignored";
        assert_eq!(
            split_code_and_data(src),
            ("print 'prelude';\n", Some("__DATA__\nchunk\n__END__\nignored"))
        );
    }

    #[test]
    // Allow deprecated call to verify compatibility wrapper behavior.
    #[allow(deprecated)]
    fn test_find_data_marker_deprecated_matches_lexed_helper() {
        let src = "say 1;\n__END__\ntrailer";
        assert_eq!(find_data_marker_byte(src), find_data_marker_byte_lexed(src));
    }
}
