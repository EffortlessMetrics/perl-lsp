use crate::syntax::cursor::RegexCursor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EmbeddedCodeKind {
    Immediate,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddedCodeFinding {
    pub(crate) offset: usize,
    pub(crate) kind: EmbeddedCodeKind,
}

pub(crate) fn detects_code_execution(pattern: &str) -> bool {
    find_code_execution(pattern, 0).is_some()
}

pub(crate) fn find_code_execution(pattern: &str, start_pos: usize) -> Option<EmbeddedCodeFinding> {
    let mut cursor = RegexCursor::new(pattern);
    while let Some(ch) = cursor.current() {
        if cursor.skip_escape() || cursor.skip_char_class() {
            continue;
        }
        if ch == b'(' && cursor.peek(1) == Some(b'?') {
            if cursor.peek(2) == Some(b'{') {
                return Some(EmbeddedCodeFinding {
                    offset: start_pos + cursor.position(),
                    kind: EmbeddedCodeKind::Immediate,
                });
            }
            if cursor.peek(2) == Some(b'?') && cursor.peek(3) == Some(b'{') {
                return Some(EmbeddedCodeFinding {
                    offset: start_pos + cursor.position(),
                    kind: EmbeddedCodeKind::Deferred,
                });
            }
        }
        cursor.bump();
    }
    None
}
