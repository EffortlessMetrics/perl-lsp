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
    /// Whether we just saw 'sub' and are waiting for a possible prototype
    pub after_sub: bool,
    /// Whether we just saw '->' (suppresses s/tr/y as substitution)
    pub after_arrow: bool,
    /// Depth of hash-subscript brace nesting.
    /// When > 0, suppresses quote-op detection inside hash subscripts/slices.
    pub hash_brace_depth: usize,
    /// Whether the lexer just emitted a complete $var/@var/%var token.
    /// Used by the `{` handler to distinguish hash subscript openers from block openers.
    pub after_var_subscript: bool,
    /// Depth of open parentheses (used to guard heredoc vs bitshift disambiguation)
    pub paren_depth: usize,
    /// Current position with line/column tracking
    pub current_pos: Position,
    /// Whether the terminal EOF token has already been emitted.
    pub eof_emitted: bool,
    /// Additional context for complex states
    pub context: CheckpointContext,
}

/// Additional context that may be needed for certain lexer states
#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointContext {
    /// Normal lexing
    Normal,
    /// Inside a heredoc (tracks the terminator)
    Heredoc {
        /// The terminator label (e.g. `END` in `<<END`)
        terminator: String,
        /// Whether the heredoc body is interpolated (double-quoted style)
        is_interpolated: bool,
    },
    /// Inside a format body
    Format {
        /// Byte offset where the format body begins
        start_position: usize,
    },
    /// Inside a regex or substitution
    Regex {
        /// The delimiter character (e.g. `/` in `/pattern/`)
        delimiter: char,
        /// Byte offset where the flags begin, if already scanned
        flags_position: Option<usize>,
    },
    /// Inside a quote-like operator
    QuoteLike {
        /// The operator name (e.g. `q`, `qq`, `qw`)
        operator: String,
        /// The delimiter character (e.g. `(` in `qw(...)`)
        delimiter: char,
        /// Whether the delimiter is a paired bracket (e.g. `(` / `)`)
        is_paired: bool,
    },
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
            after_sub: false,
            after_arrow: false,
            hash_brace_depth: 0,
            after_var_subscript: false,
            paren_depth: 0,
            current_pos: Position::start(),
            eof_emitted: false,
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
    pub fn diff(&self, other: &Self) -> super::CheckpointDiff {
        super::CheckpointDiff {
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
            eof_state_changed: self.eof_emitted != other.eof_emitted,
            context_changed: self.context != other.context,
        }
    }

    /// Apply an edit to this checkpoint.
    ///
    /// # Behavior
    ///
    /// * Edit before the checkpoint and ending strictly before it: the byte
    ///   `position` is shifted by `new_len - old_len`. The `current_pos`
    ///   line/column tracker is reset to `Position::start()` because we
    ///   cannot recompute line/column without rescanning the input.
    /// * Edit overlapping the checkpoint: the checkpoint is invalidated --
    ///   `position` is rewound to `start`, lexer mode and stacks are reset to
    ///   defaults, and `current_pos` is reset to `Position::start()`.
    /// * Edit at or after the checkpoint: no change.
    ///
    /// `current_pos` is intentionally reset in both the "shifted" and
    /// "invalidated" branches so callers always observe a known sentinel value
    /// and must rescan from `position` to recover accurate line/column data.
    pub fn apply_edit(&mut self, start: usize, old_len: usize, new_len: usize) {
        if self.position > start {
            if self.position >= start.saturating_add(old_len) {
                self.position = self.position.saturating_sub(old_len).saturating_add(new_len);
                self.current_pos = Position::start();
                self.eof_emitted = false;
            } else {
                self.position = start;
                self.current_pos = Position::start();
                self.eof_emitted = false;
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
            "Checkpoint@{} mode={:?} delims={} proto={} after_sub={}",
            self.position,
            self.mode,
            self.delimiter_stack.len(),
            self.in_prototype,
            self.after_sub
        )
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
