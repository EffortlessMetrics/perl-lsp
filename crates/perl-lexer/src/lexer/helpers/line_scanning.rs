use crate::PerlLexer;

impl PerlLexer<'_> {
    #[inline]
    pub(crate) fn trailing_ws_only(bytes: &[u8], mut p: usize) -> bool {
        while p < bytes.len() && bytes[p] != b'\n' && bytes[p] != b'\r' {
            match bytes[p] {
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
    pub(crate) fn find_line_end(bytes: &[u8], start: usize) -> (usize, usize) {
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'\n' && bytes[end] != b'\r' {
            end += 1;
        }
        (end, end)
    }
}
