use crate::syntax::{char_class, escape};

pub(crate) struct RegexCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> RegexCursor<'a> {
    pub(crate) fn new(pattern: &'a str) -> Self {
        Self { bytes: pattern.as_bytes(), pos: 0 }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn current(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(crate) fn peek(&self, n: usize) -> Option<u8> {
        self.bytes.get(self.pos + n).copied()
    }

    pub(crate) fn bump(&mut self) {
        self.pos += 1;
    }

    pub(crate) fn skip_escape(&mut self) -> bool {
        escape::skip_escape(self.bytes, &mut self.pos)
    }

    pub(crate) fn skip_char_class(&mut self) -> bool {
        char_class::skip_char_class(self.bytes, &mut self.pos)
    }
}
