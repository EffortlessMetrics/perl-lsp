use crate::syntax::cursor::quoted_literal_end;

use super::parser::{parse_named_capture_name, parse_named_capture_name_from};

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureGroup {
    pub name: String,
    pub index: usize,
    pub pattern: String,
}

pub(crate) fn extract_named_captures(pattern: &str) -> Vec<CaptureGroup> {
    let bytes = pattern.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    let mut capture_index = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
            if let Some(end) = quoted_literal_end(bytes, i) {
                i = end;
                continue;
            }
            i += 2;
            continue;
        }

        if bytes[i] == b'[' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                } else if bytes[i] == b']' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        if bytes[i] == b'(' {
            i += 1;
            if i < bytes.len() && bytes[i] == b'?' {
                i += 1;
                if i < bytes.len() && bytes[i] == b'<' {
                    i += 1;
                    if i < bytes.len() && (bytes[i] == b'=' || bytes[i] == b'!') {
                        i += 1;
                        continue;
                    }

                    if let Some((name, next)) = parse_named_capture_name_from(bytes, i, b'>') {
                        capture_index += 1;
                        i = next;
                        let (subpattern, next_i) = collect_subpattern(bytes, i);
                        i = next_i;
                        result.push(CaptureGroup {
                            name,
                            index: capture_index,
                            pattern: subpattern,
                        });
                        continue;
                    }
                } else if i < bytes.len() && bytes[i] == b'\'' {
                    if let Some((name, next)) = parse_named_capture_name(bytes, i, b'\'', b'\'') {
                        capture_index += 1;
                        i = next;
                        let (subpattern, next_i) = collect_subpattern(bytes, i);
                        i = next_i;
                        result.push(CaptureGroup {
                            name,
                            index: capture_index,
                            pattern: subpattern,
                        });
                        continue;
                    }
                } else if i + 1 < bytes.len() && bytes[i] == b'P' && bytes[i + 1] == b'<' {
                    i += 1;
                    if let Some((name, next)) = parse_named_capture_name(bytes, i, b'<', b'>') {
                        capture_index += 1;
                        i = next;
                        let (subpattern, next_i) = collect_subpattern(bytes, i);
                        i = next_i;
                        result.push(CaptureGroup {
                            name,
                            index: capture_index,
                            pattern: subpattern,
                        });
                        continue;
                    }
                }
                continue;
            }

            capture_index += 1;
            continue;
        }

        i += 1;
    }

    result
}

fn collect_subpattern(bytes: &[u8], mut i: usize) -> (String, usize) {
    let start = i;
    let mut depth = 1usize;
    while i < bytes.len() && depth > 0 {
        if bytes[i] == b'\\' {
            if let Some(end) = quoted_literal_end(bytes, i) {
                i = end;
                continue;
            }
            i += 2;
            continue;
        }

        if bytes[i] == b'[' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                } else if bytes[i] == b']' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        if bytes[i] == b'(' {
            depth += 1;
        } else if bytes[i] == b')' {
            depth -= 1;
        }
        i += 1;
    }

    let subpattern = if i > 0 && start < i - 1 {
        String::from_utf8_lossy(&bytes[start..i - 1]).into_owned()
    } else {
        String::new()
    };

    (subpattern, i)
}
