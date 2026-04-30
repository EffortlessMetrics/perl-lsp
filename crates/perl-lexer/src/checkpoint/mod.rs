//! Lexer checkpointing for incremental parsing
//!
//! This module provides checkpointing functionality for the Perl lexer,
//! allowing it to save and restore its state for incremental parsing.

mod cache;
mod diff;
mod model;
mod traits;

pub use cache::CheckpointCache;
pub use diff::CheckpointDiff;
pub use model::{CheckpointContext, LexerCheckpoint};
pub use traits::Checkpointable;

#[cfg(test)]
mod tests;
