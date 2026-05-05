pub(crate) fn find_nested_quantifier(pattern: &str) -> Option<usize> {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut group_stack = Vec::new();
    let mut last_type = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 2;
                last_type = 0;
                continue;
            }
            b'(' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'?' {
                    i += 2;
                    if i < bytes.len()
                        && matches!(bytes[i], b':' | b'=' | b'!' | b'<' | b'>' | b'|' | b'P' | b'#')
                    {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
                group_stack.push(false);
                last_type = 0;
                continue;
            }
            b')' => {
                if let Some(has_quantifier) = group_stack.pop() {
                    last_type = if has_quantifier { 2 } else { 0 };
                }
            }
            b'+' | b'*' | b'?' | b'{' => {
                if last_type == 2 {
                    if bytes[i] == b'{' {
                        let mut j = i + 1;
                        if is_brace_quantifier(bytes, &mut j) {
                            return Some(i);
                        }
                        last_type = 0;
                        i += 1;
                        continue;
                    }
                    return Some(i);
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
    None
}

pub(crate) fn detect_nested_quantifiers(pattern: &str) -> bool {
    find_nested_quantifier(pattern).is_some()
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
