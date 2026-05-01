pub(crate) fn skip_escape(bytes: &[u8], pos: &mut usize) -> bool {
    if *pos < bytes.len() && bytes[*pos] == b'\\' {
        *pos += 2;
        return true;
    }
    false
}
