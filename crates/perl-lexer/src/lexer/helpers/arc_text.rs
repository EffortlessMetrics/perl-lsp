use std::sync::{Arc, OnceLock};

// Pre-allocated empty Arc to avoid repeated allocations.
static EMPTY_ARC: OnceLock<Arc<str>> = OnceLock::new();

#[inline(always)]
pub(crate) fn empty_arc() -> Arc<str> {
    EMPTY_ARC.get_or_init(|| Arc::from("")).clone()
}

pub(crate) fn truncate_preview(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}...", &text[..idx]),
        None => text.to_string(),
    }
}
