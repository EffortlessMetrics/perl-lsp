use crate::{LexerMode, Position};
use std::fmt;

/// A checkpoint that captures the complete lexer state
#[derive(Debug, Clone, PartialEq)]
pub struct LexerCheckpoint {
    pub position: usize,
    pub mode: LexerMode,
    pub delimiter_stack: Vec<char>,
    pub in_prototype: bool,
    pub prototype_depth: usize,
    pub after_sub: bool,
    pub after_arrow: bool,
    pub hash_brace_depth: usize,
    pub after_var_subscript: bool,
    pub paren_depth: usize,
    pub current_pos: Position,
    pub context: CheckpointContext,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointContext {
    Normal,
    Heredoc { terminator: String, is_interpolated: bool },
    Format { start_position: usize },
    Regex { delimiter: char, flags_position: Option<usize> },
    QuoteLike { operator: String, delimiter: char, is_paired: bool },
}

impl LexerCheckpoint {
    pub fn new() -> Self {
        Self {
            position: 0,
            mode: LexerMode::ExpectTerm,
            delimiter_stack: Vec::new(),
            in_prototype: false,
            prototype_depth: 0,
            after_sub: false,
            after_arrow: false,
            hash_brace_depth: 0,
            after_var_subscript: false,
            paren_depth: 0,
            current_pos: Position::start(),
            context: CheckpointContext::Normal,
        }
    }

    pub fn at_position(position: usize) -> Self {
        Self { position, ..Self::new() }
    }

    pub fn is_at_start(&self) -> bool {
        self.position == 0
    }

    pub fn diff(&self, other: &Self) -> CheckpointDiff {
        CheckpointDiff {
            position_delta: self.position as isize - other.position as isize,
            mode_changed: self.mode != other.mode,
            delimiter_stack_changed: self.delimiter_stack != other.delimiter_stack,
            prototype_state_changed: self.in_prototype != other.in_prototype
                || self.prototype_depth != other.prototype_depth
                || self.after_sub != other.after_sub
                || self.after_arrow != other.after_arrow
                || self.hash_brace_depth != other.hash_brace_depth
                || self.after_var_subscript != other.after_var_subscript
                || self.paren_depth != other.paren_depth,
            context_changed: self.context != other.context,
        }
    }

    pub fn apply_edit(&mut self, start: usize, old_len: usize, new_len: usize) {
        if self.position > start {
            if self.position >= start + old_len {
                self.position = self.position - old_len + new_len;
            } else {
                self.position = start;
                self.mode = LexerMode::ExpectTerm;
                self.delimiter_stack.clear();
                self.in_prototype = false;
                self.prototype_depth = 0;
                self.after_sub = false;
                self.after_arrow = false;
                self.hash_brace_depth = 0;
                self.after_var_subscript = false;
                self.paren_depth = 0;
                self.context = CheckpointContext::Normal;
            }
        }
    }

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
            "Checkpoint@{} mode={:?} delims={} proto={} after_sub={}",
            self.position,
            self.mode,
            self.delimiter_stack.len(),
            self.in_prototype,
            self.after_sub
        )
    }
}

#[derive(Debug)]
pub struct CheckpointDiff {
    pub position_delta: isize,
    pub mode_changed: bool,
    pub delimiter_stack_changed: bool,
    pub prototype_state_changed: bool,
    pub context_changed: bool,
}

impl CheckpointDiff {
    pub fn has_state_changes(&self) -> bool {
        self.mode_changed
            || self.delimiter_stack_changed
            || self.prototype_state_changed
            || self.context_changed
    }
}

pub trait Checkpointable {
    fn checkpoint(&self) -> LexerCheckpoint;
    fn restore(&mut self, checkpoint: &LexerCheckpoint);
    fn can_restore(&self, checkpoint: &LexerCheckpoint) -> bool;
}
