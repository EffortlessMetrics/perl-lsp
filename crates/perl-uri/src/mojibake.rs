//! Repair helpers for common path mojibake produced by lossy URI clients.

use std::path::PathBuf;

pub(crate) fn repair_path_mojibake(path: PathBuf) -> PathBuf {
    let Some(path_text) = path.to_str() else {
        return path;
    };

    let repaired = repair_mojibake_text(path_text);
    if repaired == path_text { path } else { PathBuf::from(repaired) }
}

fn repair_mojibake_text(text: &str) -> String {
    if !looks_like_mojibake(text) {
        return text.to_string();
    }

    let mut bytes = Vec::with_capacity(text.len());
    for ch in text.chars() {
        let code = u32::from(ch);
        let Ok(byte) = u8::try_from(code) else {
            return text.to_string();
        };
        bytes.push(byte);
    }

    let Ok(candidate) = String::from_utf8(bytes) else {
        return text.to_string();
    };

    if mojibake_marker_count(&candidate) < mojibake_marker_count(text) {
        candidate
    } else {
        text.to_string()
    }
}

fn looks_like_mojibake(text: &str) -> bool {
    mojibake_marker_count(text) > 0
}

fn mojibake_marker_count(text: &str) -> usize {
    text.chars().filter(|ch| matches!(ch, 'Ã' | 'Â' | 'â' | 'ð' | '�')).count()
}

#[cfg(test)]
mod tests {
    use super::{mojibake_marker_count, repair_mojibake_text, repair_path_mojibake};
    use std::path::PathBuf;

    #[test]
    fn clean_text_without_markers_is_returned_unchanged() {
        assert_eq!(repair_mojibake_text("/tmp/plain-module.pm"), "/tmp/plain-module.pm");
        assert_eq!(mojibake_marker_count("/tmp/plain-module.pm"), 0);
    }

    #[test]
    fn double_encoded_latin1_text_repairs_when_marker_count_drops() {
        assert_eq!(repair_mojibake_text("/tmp/cafÃ©.pl"), "/tmp/café.pl");
    }

    #[test]
    fn text_with_marker_and_non_latin1_codepoint_is_left_unchanged() {
        assert_eq!(repair_mojibake_text("/tmp/Ã中.pl"), "/tmp/Ã中.pl");
    }

    #[test]
    fn text_that_would_decode_to_invalid_utf8_is_left_unchanged() {
        assert_eq!(repair_mojibake_text("/tmp/Ãa.pl"), "/tmp/Ãa.pl");
    }

    #[test]
    fn text_with_equal_candidate_marker_count_is_left_unchanged() {
        let input = "/tmp/Ã\u{83}.pl";
        assert_eq!(repair_mojibake_text(input), input);
    }

    #[test]
    fn path_repair_reuses_original_path_when_text_is_unchanged() {
        let path = PathBuf::from("/tmp/plain-module.pm");
        assert_eq!(repair_path_mojibake(path.clone()), path);
    }

    #[test]
    fn path_repair_returns_repaired_path_when_text_changes() {
        assert_eq!(
            repair_path_mojibake(PathBuf::from("/tmp/cafÃ©.pl")),
            PathBuf::from("/tmp/café.pl")
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_is_returned_unchanged() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xff]));
        assert_eq!(repair_path_mojibake(path.clone()), path);
    }
}
