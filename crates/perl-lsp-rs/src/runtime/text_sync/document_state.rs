//! DocumentState constructors for text synchronization.
//!
//! Keeps raw state assembly separate from LSP notification handlers so the
//! handlers can focus on protocol flow and parse/index decisions.

use super::*;
use crate::state::DegradationTier;

pub(super) fn minimal_state(text: &str, version: i32) -> DocumentState {
    let rope = ropey::Rope::from_str(text);
    minimal_state_from_rope(rope, text.to_string(), version, Arc::new(AtomicU32::new(0)))
}

pub(super) fn empty_state(version: i32) -> DocumentState {
    DocumentState {
        rope: ropey::Rope::new(),
        text: String::new(),
        version,
        ast: None,
        parse_errors: vec![],
        parent_map: ParentMap::default(),
        line_starts: LineStartsCache::new(""),
        generation: Arc::new(AtomicU32::new(0)),
        degradation_tier: DegradationTier::Minimal,
        #[cfg(feature = "incremental")]
        incremental_doc: None,
        #[cfg(feature = "incremental")]
        incremental_state: None,
    }
}

pub(super) fn minimal_state_from_rope(
    rope: ropey::Rope,
    text: String,
    version: i32,
    generation: Arc<AtomicU32>,
) -> DocumentState {
    let line_starts = LineStartsCache::new_rope(&rope);
    DocumentState {
        rope,
        text,
        version,
        ast: None,
        parse_errors: vec![],
        parent_map: ParentMap::default(),
        line_starts,
        generation,
        degradation_tier: DegradationTier::Minimal,
        #[cfg(feature = "incremental")]
        incremental_doc: None,
        #[cfg(feature = "incremental")]
        incremental_state: None,
    }
}
