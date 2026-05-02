//! Canonical convenience imports for `perl-parser` consumers.

pub use crate::analysis;
pub use crate::core::{
    Node, NodeKind, ParseError, ParseOutput, ParseResult, Parser, SourceLocation,
};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::dead_code::{
    DeadCode, DeadCodeAnalysis, DeadCodeDetector, DeadCodeStats, DeadCodeType,
};
#[cfg(feature = "incremental")]
pub use crate::incremental::{Edit, IncrementalState, apply_edits};
pub use crate::refactor;
pub use crate::workspace;
