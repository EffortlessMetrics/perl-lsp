//! UTF-8/UTF-16 position conversion functions.
pub fn offset_to_utf16_line_col(text: &str, offset: usize) -> (u32, u32) {
    if offset > text.len() {
        let lines: Vec<&str> = text.lines().collect();
        let last_line = lines.len().saturating_sub(1) as u32;
        let last_col = lines.last().map(|l| l.encode_utf16().count()).unwrap_or(0) as u32;
        return (last_line, last_col);
    }
    if offset == text.len() && (text.ends_with('\n') || text.ends_with("\r\n")) {
        return (text.split_inclusive('\n').count() as u32, 0);
    }
    let mut acc = 0usize;
    for (line_idx, line) in text.split_inclusive('\n').enumerate() {
        let next = acc + line.len();
        if offset < next {
            let rel = offset - acc;
            if rel == 0 {
                return (line_idx as u32, 0);
            }
            if rel >= line.len() {
                return (line_idx as u32, line.encode_utf16().count() as u32);
            }
            if line.is_char_boundary(rel) {
                return (line_idx as u32, line[..rel].encode_utf16().count() as u32);
            }
            let mut cs = rel;
            while cs > 0 && !line.is_char_boundary(cs) {
                cs -= 1;
            }
            return (line_idx as u32, line[..cs].encode_utf16().count() as u32 + 1);
        }
        acc = next;
    }
    let last_line = text.lines().count().saturating_sub(1) as u32;
    (last_line, text.lines().last().map(|l| l.encode_utf16().count()).unwrap_or(0) as u32)
}
pub fn utf16_line_col_to_offset(text: &str, line: u32, col: u32) -> usize {
    let mut offset = 0;
    for (curr, lt) in text.split_inclusive('\n').enumerate() {
        if curr as u32 == line {
            if col == 0 {
                return offset;
            }
            let mut up = 0u32;
            for (bi, ch) in lt.char_indices() {
                if up == col {
                    return offset + bi;
                }
                if up < col && col < up + ch.len_utf16() as u32 {
                    return offset + bi;
                }
                up += ch.len_utf16() as u32;
                if up > col {
                    return offset + bi;
                }
            }
            let lcl = if lt.ends_with('\n') { lt.len() - 1 } else { lt.len() };
            return offset + lcl.min(text.len() - offset);
        }
        offset += lt.len();
    }
    text.len()
}
