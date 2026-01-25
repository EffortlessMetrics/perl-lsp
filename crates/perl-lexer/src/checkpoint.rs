//! Lexer checkpointing for incremental parsing
//!
//! This module provides checkpointing functionality for the Perl lexer,
//! allowing it to save and restore its state for incremental parsing.

use crate::{LexerMode, Position};
use std::fmt;

/// A checkpoint that captures the complete lexer state
#[derive(Debug, Clone, PartialEq)]
pub struct LexerCheckpoint {
    /// Current position in the input
    pub position: usize,
    /// Current lexer mode (`ExpectTerm`, `ExpectOperator`, etc.)
    pub mode: LexerMode,
    /// Stack for nested delimiters in s{}{} constructs
    pub delimiter_stack: Vec<char>,
    /// Whether we're inside prototype parens after 'sub'
    pub in_prototype: bool,
    /// Paren depth to track when we exit prototype
    pub prototype_depth: usize,
    /// Current position with line/column tracking
    pub current_pos: Position,
    /// Additional context for complex states
    pub context: CheckpointContext,
}

/// Additional context that may be needed for certain lexer states
#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointContext {
    /// Normal lexing
    Normal,
    /// Inside a heredoc (tracks the terminator)
    Heredoc { terminator: String, is_interpolated: bool },
    /// Inside a format body
    Format { start_position: usize },
    /// Inside a regex or substitution
    Regex { delimiter: char, flags_position: Option<usize> },
    /// Inside a quote-like operator
    QuoteLike { operator: String, delimiter: char, is_paired: bool },
}

impl LexerCheckpoint {
    /// Create a new checkpoint with default values
    pub fn new() -> Self {
        Self {
            position: 0,
            mode: LexerMode::ExpectTerm,
            delimiter_stack: Vec::new(),
            in_prototype: false,
            prototype_depth: 0,
            current_pos: Position::start(),
            context: CheckpointContext::Normal,
        }
    }

    /// Create a checkpoint at a specific position
    pub fn at_position(position: usize) -> Self {
        Self { position, ..Self::new() }
    }

    /// Check if this checkpoint is at the start of input
    pub fn is_at_start(&self) -> bool {
        self.position == 0
    }

    /// Calculate the difference between two checkpoints
    pub fn diff(&self, other: &Self) -> CheckpointDiff {
        CheckpointDiff {
            position_delta: self.position as isize - other.position as isize,
            mode_changed: self.mode != other.mode,
            delimiter_stack_changed: self.delimiter_stack != other.delimiter_stack,
            prototype_state_changed: self.in_prototype != other.in_prototype
                || self.prototype_depth != other.prototype_depth,
            context_changed: self.context != other.context,
        }
    }

    /// Apply an edit to this checkpoint
    pub fn apply_edit(&mut self, start: usize, old_len: usize, new_len: usize) {
        if self.position > start {
            if self.position >= start + old_len {
                // Checkpoint is after the edit
                self.position = self.position - old_len + new_len;
            } else {
                // Checkpoint is inside the edit - invalidate
                self.position = start;
                self.mode = LexerMode::ExpectTerm;
                self.delimiter_stack.clear();
                self.in_prototype = false;
                self.prototype_depth = 0;
                self.context = CheckpointContext::Normal;
            }
        }

        // Update position tracking
        // In a real implementation, we'd update line/column based on the edit
    }

    /// Validate that this checkpoint is valid for the given input
    pub fn is_valid_for(&self, input: &str) -> bool {
        self.position <= input.len()
    }
}

impl Default for LexerCheckpoint {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LexerCheckpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Checkpoint@{} mode={:?} delims={} proto={}",
            self.position,
            self.mode,
            self.delimiter_stack.len(),
            self.in_prototype
        )
    }
}

/// Represents the difference between two checkpoints
#[derive(Debug)]
pub struct CheckpointDiff {
    pub position_delta: isize,
    pub mode_changed: bool,
    pub delimiter_stack_changed: bool,
    pub prototype_state_changed: bool,
    pub context_changed: bool,
}

impl CheckpointDiff {
    /// Check if any state changed besides position
    pub fn has_state_changes(&self) -> bool {
        self.mode_changed
            || self.delimiter_stack_changed
            || self.prototype_state_changed
            || self.context_changed
    }
}

/// Trait for types that support checkpointing
pub trait Checkpointable {
    /// Create a checkpoint of the current state
    fn checkpoint(&self) -> LexerCheckpoint;

    /// Restore state from a checkpoint
    fn restore(&mut self, checkpoint: &LexerCheckpoint);

    /// Check if we can restore to a given checkpoint
    fn can_restore(&self, checkpoint: &LexerCheckpoint) -> bool;
}

/// A checkpoint cache for efficient incremental parsing
pub struct CheckpointCache {
    /// Cached checkpoints at various positions
    checkpoints: Vec<(usize, LexerCheckpoint)>,
    /// Maximum number of checkpoints to cache
    max_checkpoints: usize,
}

impl CheckpointCache {
    /// Create a new checkpoint cache
    pub fn new(max_checkpoints: usize) -> Self {
        Self { checkpoints: Vec::new(), max_checkpoints }
    }

    /// Add a checkpoint to the cache
    pub fn add(&mut self, checkpoint: LexerCheckpoint) {
        let position = checkpoint.position;

        // Remove any existing checkpoint at this position
        self.checkpoints.retain(|(pos, _)| *pos != position);

        // Add the new checkpoint
        self.checkpoints.push((position, checkpoint));

        // Sort by position
        self.checkpoints.sort_by_key(|(pos, _)| *pos);

        // Trim to max size
        if self.checkpoints.len() > self.max_checkpoints {
            // Keep checkpoints evenly distributed
            let total = self.checkpoints.len();
            let step = total as f64 / self.max_checkpoints as f64;
            let mut kept = Vec::new();
            for i in 0..self.max_checkpoints {
                let idx = (i as f64 * step) as usize;
                if idx < total {
                    kept.push(self.checkpoints[idx].clone());
                }
            }
            self.checkpoints = kept;
        }
    }

    /// Find the nearest checkpoint before a given position
    pub fn find_before(&self, position: usize) -> Option<&LexerCheckpoint> {
        self.checkpoints.iter().rev().find(|(pos, _)| *pos <= position).map(|(_, cp)| cp)
    }

    /// Clear all cached checkpoints
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    /// Apply an edit to all cached checkpoints
    pub fn apply_edit(&mut self, start: usize, old_len: usize, new_len: usize) {
        // Update all checkpoints
        for (pos, checkpoint) in &mut self.checkpoints {
            checkpoint.apply_edit(start, old_len, new_len);
            *pos = checkpoint.position;
        }

        // Remove invalid checkpoints
        self.checkpoints
            .retain(|(_, cp)| !matches!(cp.context, CheckpointContext::Normal) || cp.position > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let cp = LexerCheckpoint::new();
        assert_eq!(cp.position, 0);
        assert_eq!(cp.mode, LexerMode::ExpectTerm);
        assert!(cp.delimiter_stack.is_empty());
    }

    #[test]
    fn test_checkpoint_diff() {
        let cp1 = LexerCheckpoint::at_position(10);
        let mut cp2 = cp1.clone();
        cp2.position = 20;
        cp2.mode = LexerMode::ExpectOperator;

        let diff = cp2.diff(&cp1);
        assert_eq!(diff.position_delta, 10);
        assert!(diff.mode_changed);
        assert!(!diff.delimiter_stack_changed);
    }

    #[test]
    fn test_checkpoint_edit() {
        let mut cp = LexerCheckpoint::at_position(50);

        // Edit before checkpoint
        cp.apply_edit(10, 5, 10);
        assert_eq!(cp.position, 55); // Shifted by +5

        // Edit after checkpoint
        let mut cp2 = LexerCheckpoint::at_position(50);
        cp2.apply_edit(60, 10, 5);
        assert_eq!(cp2.position, 50); // No change

        // Edit containing checkpoint
        let mut cp3 = LexerCheckpoint::at_position(50);
        cp3.apply_edit(45, 10, 5);
        assert_eq!(cp3.position, 45); // Reset to edit start
    }

    #[test]
    fn test_checkpoint_cache() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut cache = CheckpointCache::new(3);

        // Add checkpoints
        cache.add(LexerCheckpoint::at_position(10));
        cache.add(LexerCheckpoint::at_position(20));
        cache.add(LexerCheckpoint::at_position(30));
        cache.add(LexerCheckpoint::at_position(40));

        // Should keep 3 evenly distributed
        assert_eq!(cache.checkpoints.len(), 3);

        // Find nearest before position 25
        let cp = cache.find_before(25).ok_or("Expected checkpoint before position 25")?;
        assert_eq!(cp.position, 20);
        Ok(())
    }
}
