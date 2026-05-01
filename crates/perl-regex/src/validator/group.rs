use crate::error::RegexError;

use super::config::RegexValidationConfig;

pub(crate) enum GroupType {
    Normal,
    Lookbehind,
    BranchReset { branch_count: usize },
}

pub(crate) struct GroupStack {
    stack: Vec<GroupType>,
}

impl GroupStack {
    pub(crate) fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub(crate) fn push(
        &mut self,
        group: GroupType,
        offset: usize,
        start_pos: usize,
        config: &RegexValidationConfig,
    ) -> Result<(), RegexError> {
        match group {
            GroupType::Lookbehind => {
                let depth =
                    self.stack.iter().filter(|g| matches!(g, GroupType::Lookbehind)).count();
                if depth >= config.max_nesting {
                    return Err(RegexError::syntax(
                        "Regex lookbehind nesting too deep",
                        start_pos + offset,
                    ));
                }
            }
            GroupType::BranchReset { .. } => {
                let depth = self
                    .stack
                    .iter()
                    .filter(|g| matches!(g, GroupType::BranchReset { .. }))
                    .count();
                if depth >= config.max_nesting {
                    return Err(RegexError::syntax(
                        "Regex branch reset nesting too deep",
                        start_pos + offset,
                    ));
                }
            }
            GroupType::Normal => {}
        }
        self.stack.push(group);
        Ok(())
    }

    pub(crate) fn pop(&mut self) {
        self.stack.pop();
    }

    pub(crate) fn observe_alternation(
        &mut self,
        offset: usize,
        start_pos: usize,
        config: &RegexValidationConfig,
    ) -> Result<(), RegexError> {
        if let Some(GroupType::BranchReset { branch_count }) = self.stack.last_mut() {
            *branch_count += 1;
            if *branch_count > config.max_branch_reset_branches {
                return Err(RegexError::syntax(
                    format!(
                        "Too many branches in branch reset group (max {})",
                        config.max_branch_reset_branches
                    ),
                    start_pos + offset,
                ));
            }
        }
        Ok(())
    }
}
