use crate::syntax::cursor::RegexCursor;

use super::RegexFinding;

pub(crate) fn detect_nested_quantifiers(pattern: &str) -> bool {
    find_nested_quantifier(pattern, 0).is_some()
}

pub(crate) fn find_nested_quantifier(pattern: &str, start_pos: usize) -> Option<RegexFinding> {
    let mut cursor = RegexCursor::new(pattern);
    let bytes = pattern.as_bytes();
    let mut group_stack = Vec::new();
    let mut last_type = 0;
    while let Some(ch) = cursor.current() {
        if cursor.skip_escape() || cursor.skip_char_class() {
            last_type = 0;
            continue;
        }

        match ch {
            b'(' => {
                cursor.bump();
                if cursor.current() == Some(b'?') {
                    cursor.bump();
                    if let Some(marker) = cursor.current()
                        && matches!(marker, b':' | b'=' | b'!' | b'<' | b'>' | b'|' | b'P' | b'#')
                    {
                        cursor.bump();
                    }
                }
                group_stack.push(false);
                continue;
            }
            b')' => {
                if let Some(has_quantifier) = group_stack.pop() {
                    last_type = if has_quantifier { 2 } else { 0 };
                }
            }
            b'+' | b'*' | b'?' | b'{' => {
                if last_type == 2 {
                    if ch == b'{' {
                        let mut j = cursor.position() + 1;
                        if is_brace_quantifier(bytes, &mut j) {
                            return Some(RegexFinding { offset: start_pos + cursor.position() });
                        }
                    } else {
                        return Some(RegexFinding { offset: start_pos + cursor.position() });
                    }
                }
                if let Some(last) = group_stack.last_mut() {
                    *last = true;
                }
                last_type = 1;
            }
            _ => last_type = 0,
        }
        cursor.bump();
    }
    None
}

fn is_brace_quantifier(bytes: &[u8], i: &mut usize) -> bool {
    let mut has_digit = false;
    let mut has_comma = false;
    while *i < bytes.len() {
        let ch = bytes[*i];
        *i += 1;
        if ch.is_ascii_digit() {
            has_digit = true;
        } else if ch == b',' && !has_comma {
            has_comma = true;
        } else if ch == b'}' && has_digit {
            return true;
        } else {
            break;
        }
    }
    false
}
