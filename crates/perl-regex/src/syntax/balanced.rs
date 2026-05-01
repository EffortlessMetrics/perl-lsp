use super::cursor::RegexCursor;

pub(crate) fn extract_balanced_group_content(pattern: &str, start: usize) -> (String, usize) {
    let bytes = pattern.as_bytes();
    let mut cursor = RegexCursor::new(pattern);
    while cursor.pos() < start {
        cursor.bump();
    }

    let mut depth = 1usize;
    while let Some(ch) = cursor.current() {
        if cursor.skip_escape() || cursor.skip_char_class() {
            continue;
        }
        if ch == b'(' {
            depth += 1;
        } else if ch == b')' {
            depth -= 1;
            if depth == 0 {
                let end = cursor.pos();
                let sub = if start < end {
                    String::from_utf8_lossy(&bytes[start..end]).into_owned()
                } else {
                    String::new()
                };
                cursor.bump();
                return (sub, cursor.pos());
            }
        }
        cursor.bump();
    }

    (String::new(), cursor.pos())
}
