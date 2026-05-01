pub(crate) fn skip_char_class(bytes: &[u8], pos: &mut usize) -> bool {
    let len = bytes.len();
    if bytes.get(*pos) != Some(&b'[') {
        return false;
    }

    *pos += 1;
    while *pos < len {
        if bytes[*pos] == b'\\' {
            *pos += 2;
        } else if bytes[*pos] == b']' {
            *pos += 1;
            break;
        } else {
            *pos += 1;
        }
    }
    true
}
