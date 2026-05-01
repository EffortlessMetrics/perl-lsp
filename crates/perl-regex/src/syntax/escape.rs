pub(crate) fn skip_escape(bytes: &[u8], i: &mut usize) -> bool {
    if *i < bytes.len() && bytes[*i] == b'\\' {
        *i += 2;
        return true;
    }
    false
}
