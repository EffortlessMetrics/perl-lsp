pub(crate) fn skip_char_class(bytes: &[u8], i: &mut usize) -> bool {
    if *i >= bytes.len() || bytes[*i] != b'[' {
        return false;
    }

    *i += 1;
    while *i < bytes.len() {
        if bytes[*i] == b'\\' {
            *i += 2;
        } else if bytes[*i] == b']' {
            *i += 1;
            break;
        } else {
            *i += 1;
        }
    }

    true
}
