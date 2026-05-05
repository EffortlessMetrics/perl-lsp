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
