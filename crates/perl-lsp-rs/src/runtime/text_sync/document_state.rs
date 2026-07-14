//! DocumentState constructors for text synchronization.
//!
//! Keeps raw state assembly separate from LSP notification handlers so the
//! handlers can focus on protocol flow and parse/index decisions.

use super::*;

pub(super) fn minimal_state(text: &str, version: i32) -> DocumentState {
    // No parse ever runs for this document (large-file/binary/template
    // guards), so there is no `ParsedSnapshot` to publish -- `parsed` stays
    // `None`, and `current_parsed()`/`latest_parsed()` correctly report
    // nothing available (degradation-tier consumers fall back to `Minimal`).
    DocumentState::new(text, version)
}

pub(super) fn empty_state(version: i32) -> DocumentState {
    DocumentState::new("", version)
}

pub(super) fn minimal_state_from_rope(
    rope: ropey::Rope,
    text: String,
    version: i32,
    generation: Arc<AtomicU32>,
) -> DocumentState {
    DocumentState::from_parts(rope, text, version, generation)
}
