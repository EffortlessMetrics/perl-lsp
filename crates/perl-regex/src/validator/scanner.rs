use crate::syntax::cursor::RegexCursor;

use super::group::GroupType;

pub(crate) enum RegexEvent {
    UnicodeProperty { offset: usize },
    GroupStart { kind: GroupType, offset: usize },
    Alternation { offset: usize },
    GroupEnd,
    Other,
}

pub(crate) struct RegexScanner<'a> { cursor: RegexCursor<'a> }

impl<'a> RegexScanner<'a> {
    pub(crate) fn new(pattern: &'a str) -> Self { Self { cursor: RegexCursor::new(pattern) } }

    pub(crate) fn next_event(&mut self) -> Option<RegexEvent> {
        let ch = self.cursor.current()?;
        let offset = self.cursor.pos();

        if ch == b'\\' && (self.cursor.peek(1) == Some(b'p') || self.cursor.peek(1) == Some(b'P')) && self.cursor.peek(2) == Some(b'{') {
            self.cursor.bump();
            self.cursor.bump();
            return Some(RegexEvent::UnicodeProperty { offset });
        }
        if self.cursor.skip_escape() || self.cursor.skip_char_class() {
            return Some(RegexEvent::Other);
        }

        let event = match ch {
            b'(' => {
                let mut kind = GroupType::Normal;
                if self.cursor.peek(1) == Some(b'?') {
                    if self.cursor.peek(2) == Some(b'<') && matches!(self.cursor.peek(3), Some(b'=' | b'!')) {
                        kind = GroupType::Lookbehind;
                        self.cursor.bump(); self.cursor.bump(); self.cursor.bump();
                    } else if self.cursor.peek(2) == Some(b'|') {
                        kind = GroupType::BranchReset { branch_count: 1 };
                        self.cursor.bump(); self.cursor.bump();
                    }
                }
                RegexEvent::GroupStart { kind, offset }
            }
            b'|' => RegexEvent::Alternation { offset },
            b')' => RegexEvent::GroupEnd,
            _ => RegexEvent::Other,
        };
        self.cursor.bump();
        Some(event)
    }
}
