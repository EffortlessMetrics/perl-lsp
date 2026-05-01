use crate::syntax::cursor::RegexCursor;

pub(crate) fn read_balanced_group(pattern: &str, start: usize) -> (String, usize) {
    let bytes = pattern.as_bytes();
    let mut i = start;
    let mut depth = 1usize;

    while i < bytes.len() && depth > 0 {
        let mut cursor = RegexCursor::new(&pattern[i..]);
        if cursor.skip_escape() || cursor.skip_char_class() {
            i += cursor.pos();
            continue;
        }
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        i += 1;
    }

    let sub = if i > 0 && start < i - 1 {
        String::from_utf8_lossy(&bytes[start..i - 1]).into_owned()
    } else {
        String::new()
    };

    (sub, i)
}
