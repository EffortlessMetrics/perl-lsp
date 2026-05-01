use crate::syntax::balanced::read_balanced_group;

use super::parser::{parse_named_capture_name, parse_named_capture_name_from};

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureGroup {
    pub name: String,
    pub index: usize,
    pub pattern: String,
}

pub(crate) fn extract_named_captures(pattern: &str) -> Vec<CaptureGroup> {
    let mut result = Vec::new();
    let mut capture_index = 0usize;
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }

        if bytes[i] == b'[' {
            i += 1;
            while i < len {
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
            if i < len && bytes[i] == b'?' {
                i += 1;

                if i < len && bytes[i] == b'<' {
                    i += 1;
                    if i < len && (bytes[i] == b'=' || bytes[i] == b'!') {
                        i += 1;
                        continue;
                    }
                    if let Some((name, next_pos)) = parse_named_capture_name_from(bytes, i, b'>') {
                        capture_index += 1;
                        let (sub, end_pos) = read_balanced_group(pattern, next_pos);
                        i = end_pos;
                        result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                        continue;
                    }
                } else if i < len && bytes[i] == b'\'' {
                    if let Some((name, next_pos)) = parse_named_capture_name(bytes, i, b'\'', b'\'') {
                        capture_index += 1;
                        let (sub, end_pos) = read_balanced_group(pattern, next_pos);
                        i = end_pos;
                        result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                        continue;
                    }
                } else if i < len
                    && matches!(bytes[i], b':' | b'=' | b'!' | b'>' | b'|' | b'P' | b'#')
                {
                    continue;
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
