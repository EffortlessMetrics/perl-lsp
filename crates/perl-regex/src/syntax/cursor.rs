pub(crate) fn quoted_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'\\') || bytes.get(start + 1) != Some(&b'Q') {
        return None;
    }

    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\\' && bytes[i + 1] == b'E' {
            return Some(i + 2);
        }
        i += 1;
    }
    Some(bytes.len())
}

pub(crate) struct RegexCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> RegexCursor<'a> {
    pub(crate) fn new(pattern: &'a str) -> Self {
        Self { bytes: pattern.as_bytes(), pos: 0 }
    }

    pub(crate) fn current(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(crate) fn peek(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    pub(crate) fn bump(&mut self) {
        self.pos += 1;
    }

    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    pub(crate) fn skip_quoted_literal(&mut self) -> bool {
        if let Some(end) = quoted_literal_end(self.bytes, self.pos) {
            self.pos = end;
            return true;
        }
        false
    }

    pub(crate) fn skip_escape(&mut self) -> bool {
        if self.current() == Some(b'\\') {
            self.pos += 2;
            return true;
        }
        false
    }

    pub(crate) fn skip_char_class(&mut self) -> bool {
        if self.current() != Some(b'[') {
            return false;
        }
        self.pos += 1;
        while let Some(ch) = self.current() {
            if ch == b'\\' {
                self.pos += 2;
            } else if ch == b']' {
                self.pos += 1;
                break;
            } else {
                self.pos += 1;
            }
        }
        true
    }
}
