use crate::syntax::{
    balanced::extract_balanced_group_pattern, char_class::skip_char_class, escape::skip_escape,
};

use super::parser::{parse_named_capture_name, parse_named_capture_name_from};

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureGroup {
    pub name: String,
    pub index: usize,
    pub pattern: String,
}

pub(crate) fn extract_named_captures(pattern: &str) -> Vec<CaptureGroup> {
    let mut out = Vec::new();
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut capture_index = 0usize;
    while i < bytes.len() {
        if skip_escape(bytes, &mut i) || skip_char_class(bytes, &mut i) {
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
                        let (sub, end) = extract_balanced_group_pattern(bytes, next);
                        i = end;
                        out.push(CaptureGroup { name, index: capture_index, pattern: sub });
                        continue;
                    }
                } else if i < bytes.len() && bytes[i] == b'\'' {
                    if let Some((name, next)) = parse_named_capture_name(bytes, i, b'\'', b'\'') {
                        capture_index += 1;
                        let (sub, end) = extract_balanced_group_pattern(bytes, next);
                        i = end;
                        out.push(CaptureGroup { name, index: capture_index, pattern: sub });
                        continue;
                    }
                } else {
                    continue;
                }
                continue;
            }
            capture_index += 1;
            continue;
        }
        i += 1;
    }
    out
}
