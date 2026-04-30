//! Lexer checkpointing for incremental parsing.

mod cache;
mod types;

pub use cache::CheckpointCache;
pub use types::{CheckpointContext, CheckpointDiff, Checkpointable, LexerCheckpoint};

#[cfg(test)]
mod tests;
