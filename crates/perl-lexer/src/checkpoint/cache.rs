use crate::checkpoint::{CheckpointContext, LexerCheckpoint};

/// A checkpoint cache for efficient incremental parsing
pub struct CheckpointCache {
    /// Cached checkpoints at various positions
    pub(crate) checkpoints: Vec<(usize, LexerCheckpoint)>,
    /// Maximum number of checkpoints to cache
    max_checkpoints: usize,
}

impl CheckpointCache {
    pub fn new(max_checkpoints: usize) -> Self {
        Self { checkpoints: Vec::new(), max_checkpoints }
    }

    pub fn add(&mut self, checkpoint: LexerCheckpoint) {
        if self.max_checkpoints == 0 {
            return;
        }

        let position = checkpoint.position;
        match self.checkpoints.binary_search_by_key(&position, |(pos, _)| *pos) {
            Ok(idx) => self.checkpoints[idx] = (position, checkpoint),
            Err(idx) => self.checkpoints.insert(idx, (position, checkpoint)),
        }

        if self.checkpoints.len() > self.max_checkpoints {
            let total = self.checkpoints.len();
            if self.max_checkpoints == 1 {
                if let Some(last) = self.checkpoints.last().cloned() {
                    self.checkpoints = vec![last];
                }
                return;
            }

            let denominator = self.max_checkpoints - 1;
            let mut kept = Vec::with_capacity(self.max_checkpoints);
            for i in 0..self.max_checkpoints {
                let idx = i * (total - 1) / denominator;
                kept.push(self.checkpoints[idx].clone());
            }
            self.checkpoints = kept;
        }
    }

    pub fn find_before(&self, position: usize) -> Option<&LexerCheckpoint> {
        let idx = self.checkpoints.partition_point(|(pos, _)| *pos <= position);
        if idx == 0 { None } else { self.checkpoints.get(idx - 1).map(|(_, cp)| cp) }
    }

    pub fn find_after(&self, position: usize) -> Option<&LexerCheckpoint> {
        let idx = self.checkpoints.partition_point(|(pos, _)| *pos < position);
        self.checkpoints.get(idx).map(|(_, cp)| cp)
    }

    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    pub fn apply_edit(&mut self, start: usize, old_len: usize, new_len: usize) {
        for (pos, checkpoint) in &mut self.checkpoints {
            checkpoint.apply_edit(start, old_len, new_len);
            *pos = checkpoint.position;
        }

        self.checkpoints
            .retain(|(_, cp)| !matches!(cp.context, CheckpointContext::Normal) || cp.position > 0);
    }
}
