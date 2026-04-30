/// The difference between two [`crate::checkpoint::LexerCheckpoint`]s, produced by
/// [`crate::checkpoint::LexerCheckpoint::diff`].
#[derive(Debug)]
pub struct CheckpointDiff {
    /// Signed byte-offset difference between the two checkpoint positions.
    pub position_delta: isize,
    /// Whether the lexer mode (term vs. operator) changed.
    pub mode_changed: bool,
    /// Whether the nested delimiter stack differs.
    pub delimiter_stack_changed: bool,
    /// Whether any prototype-tracking state differs.
    pub prototype_state_changed: bool,
    /// Whether the [`crate::checkpoint::CheckpointContext`] variant changed.
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
