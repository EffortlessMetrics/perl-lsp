use super::{char_class::skip_char_class, escape::skip_escape};

pub(crate) fn extract_balanced_group_pattern(bytes: &[u8], mut i: usize) -> (String, usize) {
    let start = i;
    let mut depth = 1usize;

    while i < bytes.len() && depth > 0 {
        if skip_escape(bytes, &mut i) || skip_char_class(bytes, &mut i) {
            continue;
        }
        if bytes[i] == b'(' {
            depth += 1;
        } else if bytes[i] == b')' {
            depth -= 1;
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
