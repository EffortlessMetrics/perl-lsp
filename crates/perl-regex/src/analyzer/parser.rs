pub(crate) fn parse_named_capture_name(
    bytes: &[u8],
    pos: usize,
    open_delim: u8,
    close_delim: u8,
) -> Option<(String, usize)> {
    if pos >= bytes.len() || bytes[pos] != open_delim {
        return None;
    }

    let mut i = pos + 1;
    let name_start = i;
    while i < bytes.len() && bytes[i] != close_delim {
        i += 1;
    }

    if i == name_start || i >= bytes.len() {
        return None;
    }

    let name = String::from_utf8_lossy(&bytes[name_start..i]).into_owned();
    Some((name, i + 1))
}

pub(crate) fn parse_named_capture_name_from(
    bytes: &[u8],
    start: usize,
    close_delim: u8,
) -> Option<(String, usize)> {
    if start >= bytes.len() {
        return None;
    }

    let mut i = start;
    while i < bytes.len() && bytes[i] != close_delim {
        i += 1;
    }

    if i == start || i >= bytes.len() {
        return None;
    }

    let name = String::from_utf8_lossy(&bytes[start..i]).into_owned();
    Some((name, i + 1))
}
