use crate::PerlLexer;

impl PerlLexer<'_> {
    #[inline]
    pub(crate) fn trailing_ws_only(bytes: &[u8], mut p: usize) -> bool {
        while let Some(&byte) = bytes.get(p) {
            if byte == b'\n' || byte == b'\r' {
                return true;
            }

            match byte {
                b' ' | b'\t' => p += 1,
                _ => return false,
            }
        }
        true
    }

    #[inline]
    pub(crate) fn consume_newline(&mut self) {
        if self.position >= self.input.len() {
            return;
        }

        match self.input_bytes[self.position] {
            b'\r' => {
                self.position += 1;
                if self.position < self.input.len() && self.input_bytes[self.position] == b'\n' {
                    self.position += 1;
                }
            }
            b'\n' => self.advance(),
            _ => return,
        }

        self.after_newline = true;
        self.line_start_offset = self.position;
    }

    #[inline]
    pub(crate) fn find_line_end(bytes: &[u8], start: usize) -> usize {
        if start >= bytes.len() {
            return start;
        }

        bytes[start..]
            .iter()
            .position(|&byte| byte == b'\n' || byte == b'\r')
            .map_or(bytes.len(), |offset| start + offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_ws_only_stops_at_newline() {
        let bytes = b"\t  \nnot checked";
        assert!(PerlLexer::trailing_ws_only(bytes, 0));
    }

    #[test]
    fn trailing_ws_only_rejects_non_whitespace_before_newline() {
        let bytes = b"\t x\n";
        assert!(!PerlLexer::trailing_ws_only(bytes, 0));
    }

    #[test]
    fn find_line_end_handles_crlf_and_bounds() {
        let bytes = b"first\r\nsecond";
        assert_eq!(PerlLexer::find_line_end(bytes, 0), 5);
        assert_eq!(PerlLexer::find_line_end(bytes, 7), 13);
        assert_eq!(PerlLexer::find_line_end(bytes, bytes.len()), 13);
        assert_eq!(PerlLexer::find_line_end(bytes, bytes.len() + 1), 14);
    }
}
