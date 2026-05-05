use crate::syntax::cursor::RegexCursor;

pub(crate) fn detects_code_execution(pattern: &str) -> bool {
    find_code_execution_offset(pattern).is_some()
}

pub(crate) fn find_code_execution_offset(pattern: &str) -> Option<usize> {
    let mut cursor = RegexCursor::new(pattern);
    while let Some(ch) = cursor.current() {
        if cursor.skip_escape() || cursor.skip_char_class() {
            continue;
        }
        if ch == b'(' && cursor.peek(1) == Some(b'?') {
            if cursor.peek(2) == Some(b'{')
                || (cursor.peek(2) == Some(b'?') && cursor.peek(3) == Some(b'{'))
            {
                return Some(cursor.position());
            }
        }
        cursor.bump();
    }
    None
}
