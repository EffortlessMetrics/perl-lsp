use crate::syntax::cursor::RegexCursor;

pub(crate) fn detects_code_execution(pattern: &str) -> bool {
    let mut cursor = RegexCursor::new(pattern);

    while let Some(ch) = cursor.current() {
        if cursor.skip_escape() || cursor.skip_char_class() {
            continue;
        }

        if ch == b'(' && cursor.peek(1) == Some(b'?') {
            match (cursor.peek(2), cursor.peek(3)) {
                (Some(b'{'), _) => return true,
                (Some(b'?'), Some(b'{')) => return true,
                _ => {}
            }
        }

        cursor.bump();
    }

    false
}
