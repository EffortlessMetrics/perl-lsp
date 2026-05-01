pub(crate) fn skip_escape(bytes: &[u8], pos: &mut usize) -> bool {
    if bytes.get(*pos) == Some(&b'\\') {
        *pos += 2;
        return true;
    }
    false
}
