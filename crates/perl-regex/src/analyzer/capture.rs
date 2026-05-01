use crate::syntax::balanced::extract_balanced_group_content;

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
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
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
                    if let Some((name, next_pos)) = parse_named_capture_name_from(bytes, i, b'>') {
                        capture_index += 1;
                        let (sub, end_pos) = extract_balanced_group_content(pattern, next_pos);
                        result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                        i = end_pos;
                        continue;
                    }
                } else if i < bytes.len() && bytes[i] == b'\'' {
                    if let Some((name, next_pos)) = parse_named_capture_name(bytes, i, b'\'', b'\'')
                    {
                        capture_index += 1;
                        let (sub, end_pos) = extract_balanced_group_content(pattern, next_pos);
                        result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                        i = end_pos;
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
