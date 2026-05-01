use crate::syntax::{char_class::skip_char_class, escape::skip_escape};

pub(crate) fn detect_nested_quantifiers(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut group_stack = Vec::new();
    let mut last_type = 0;

    while i < bytes.len() {
        if skip_escape(bytes, &mut i) || skip_char_class(bytes, &mut i) {
            last_type = 0;
            continue;
        }
        match bytes[i] {
            b'(' => {
                i += 1;
                if i < bytes.len() && bytes[i] == b'?' {
                    i += 1;
                    if i < bytes.len()
                        && matches!(bytes[i], b':' | b'=' | b'!' | b'<' | b'>' | b'|' | b'P' | b'#')
                    {
                        i += 1;
                    }
                }
                group_stack.push(false);
                last_type = 0;
                continue;
            }
            b')' => {
                last_type = if group_stack.pop().unwrap_or(false) { 2 } else { 0 };
            }
            b'+' | b'*' | b'?' | b'{' => {
                if last_type == 2 {
                    if bytes[i] == b'{' {
                        let mut p = i + 1;
                        if is_brace_quantifier(bytes, &mut p) {
                            return true;
                        }
                        i += 1;
                        last_type = 0;
                        continue;
                    }
                    return true;
                }
                if let Some(last) = group_stack.last_mut() {
                    *last = true;
                }
                last_type = 1;
            }
            _ => last_type = 0,
        }
        i += 1;
    }
    false
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
