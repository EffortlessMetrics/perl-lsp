use crate::syntax::cursor::RegexCursor;

use super::group::GroupType;

pub(crate) enum RegexEvent {
    UnicodeProperty { offset: usize },
    GroupStart { kind: GroupType, offset: usize },
    Alternation { offset: usize },
    GroupEnd,
}

pub(crate) struct RegexScanner<'a> {
    cursor: RegexCursor<'a>,
}

impl<'a> RegexScanner<'a> {
    pub(crate) fn new(pattern: &'a str) -> Self {
        Self { cursor: RegexCursor::new(pattern) }
    }

    pub(crate) fn next_event(&mut self) -> Option<RegexEvent> {
        while let Some(ch) = self.cursor.current() {
            let offset = self.cursor.pos();
            if ch == b'\\' {
                if matches!(self.cursor.peek(1), Some(b'p') | Some(b'P'))
                    && self.cursor.peek(2) == Some(b'{')
                {
                    self.cursor.bump();
                    return Some(RegexEvent::UnicodeProperty { offset });
                }
                self.cursor.skip_escape();
                continue;
            }
            if self.cursor.skip_char_class() {
                continue;
            }

            match ch {
                b'(' => {
                    let (kind, consumed) = match (self.cursor.peek(1), self.cursor.peek(2), self.cursor.peek(3)) {
                        (Some(b'?'), Some(b'<'), Some(b'=') | Some(b'!')) => (GroupType::Lookbehind, 4),
                        (Some(b'?'), Some(b'|'), _) => (GroupType::BranchReset { branch_count: 1 }, 3),
                        _ => (GroupType::Normal, 1),
                    };
                    for _ in 0..consumed {
                        self.cursor.bump();
                    }
                    return Some(RegexEvent::GroupStart { kind, offset });
                }
                b'|' => {
                    self.cursor.bump();
                    return Some(RegexEvent::Alternation { offset });
                }
                b')' => {
                    self.cursor.bump();
                    return Some(RegexEvent::GroupEnd);
                }
                _ => self.cursor.bump(),
            }
        }

        None
    }
}
