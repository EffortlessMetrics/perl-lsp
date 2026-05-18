//! Repair helpers for common URI/path mojibake artifacts.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn repair_path_mojibake(path: std::path::PathBuf) -> std::path::PathBuf {
    let Some(path_text) = path.to_str() else {
        return path;
    };

    let repaired = repair_mojibake_text(path_text);
    if repaired == path_text { path } else { std::path::PathBuf::from(repaired) }
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
fn looks_like_mojibake(text: &str) -> bool {
    mojibake_marker_count(text) > 0
}

#[cfg(not(target_arch = "wasm32"))]
fn mojibake_marker_count(text: &str) -> usize {
    text.chars().filter(|ch| matches!(ch, 'Ã' | 'Â' | 'â' | 'ð' | '�')).count()
}
