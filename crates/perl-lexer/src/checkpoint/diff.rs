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

#[cfg(test)]
mod tests {
    use super::CheckpointDiff;

    #[test]
    fn has_state_changes_returns_false_when_only_position_differs() {
        let diff = CheckpointDiff {
            position_delta: 7,
            mode_changed: false,
            delimiter_stack_changed: false,
            prototype_state_changed: false,
            context_changed: false,
        };

        assert!(!diff.has_state_changes());
    }

    #[test]
    fn has_state_changes_returns_true_when_mode_changes() {
        let diff = CheckpointDiff {
            position_delta: 0,
            mode_changed: true,
            delimiter_stack_changed: false,
            prototype_state_changed: false,
            context_changed: false,
        };

        assert!(diff.has_state_changes());
    }

    #[test]
    fn has_state_changes_returns_true_when_any_non_position_flag_changes() {
        for diff in [
            CheckpointDiff {
                position_delta: 0,
                mode_changed: false,
                delimiter_stack_changed: true,
                prototype_state_changed: false,
                context_changed: false,
            },
            CheckpointDiff {
                position_delta: 0,
                mode_changed: false,
                delimiter_stack_changed: false,
                prototype_state_changed: true,
                context_changed: false,
            },
            CheckpointDiff {
                position_delta: 0,
                mode_changed: false,
                delimiter_stack_changed: false,
                prototype_state_changed: false,
                context_changed: true,
            },
        ] {
            assert!(diff.has_state_changes());
        }
    }
}
