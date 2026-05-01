use crate::PerlLexer;

impl<'a> PerlLexer<'a> {
    /// Normalize file start by skipping BOM if present
    pub(crate) fn normalize_file_start(&mut self) {
        // Skip UTF-8 BOM (EF BB BF) if at file start
        if self.position == 0 && self.matches_bytes(&[0xEF, 0xBB, 0xBF]) {
            self.position = 3;
            self.line_start_offset = 3;
        }
    }

    /// Helper to check if remaining bytes on a line are only spaces/tabs
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

    /// Consume a newline sequence (CRLF or LF) and update state
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

    /// Find the end of the current line, returning both raw end and visible end (without trailing CR)
    #[inline]
    pub(crate) fn find_line_end(bytes: &[u8], start: usize) -> (usize, usize) {
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'\n' && bytes[end] != b'\r' {
            end += 1;
        }
        let visible_end = end;
        (end, visible_end)
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn byte_at(bytes: &[u8], index: usize) -> u8 {
        debug_assert!(index < bytes.len());
        match bytes.get(index) {
            Some(&byte) => byte,
            None => 0,
        }
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn current_char(&self) -> Option<char> {
        if self.position < self.input_bytes.len() {
            let byte = Self::byte_at(self.input_bytes, self.position);
            if byte < 128 {
                Some(byte as char)
            } else {
                self.input.get(self.position..).and_then(|s| s.chars().next())
            }
        } else {
            None
        }
    }

    #[inline(always)]
    pub(crate) fn peek_char(&self, offset: usize) -> Option<char> {
        if offset > self.config.max_lookahead {
            return None;
        }

        let pos = self.position.checked_add(offset)?;
        if pos < self.input_bytes.len() {
            let byte = Self::byte_at(self.input_bytes, pos);
            if byte < 128 {
                Some(byte as char)
            } else {
                self.input.get(self.position..).and_then(|s| s.chars().nth(offset))
            }
        } else {
            None
        }
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn advance(&mut self) {
        if self.position < self.input_bytes.len() {
            let byte = Self::byte_at(self.input_bytes, self.position);
            if byte < 128 {
                self.position += 1;
            } else if let Some(ch) = self.input.get(self.position..).and_then(|s| s.chars().next())
            {
                self.position += ch.len_utf8();
            }
        }
    }
}
