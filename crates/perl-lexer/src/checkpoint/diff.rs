/// The difference between two [`crate::checkpoint::LexerCheckpoint`]s, produced by
/// [`crate::checkpoint::LexerCheckpoint::diff`].
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
