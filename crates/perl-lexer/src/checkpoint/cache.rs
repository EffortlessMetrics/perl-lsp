use crate::checkpoint::LexerCheckpoint;

/// A checkpoint cache for efficient incremental parsing
pub struct CheckpointCache {
    /// Cached checkpoints at various positions
    checkpoints: Vec<(usize, LexerCheckpoint)>,
    /// Maximum number of checkpoints to cache
    max_checkpoints: usize,
}

impl CheckpointCache {
    /// Create a new checkpoint cache.
    pub fn new(max_checkpoints: usize) -> Self {
        Self { checkpoints: Vec::new(), max_checkpoints }
    }

    /// Number of checkpoints currently held in the cache.
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    /// Whether the cache currently holds zero checkpoints.
    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }

    /// Add a checkpoint to the cache.
    ///
    /// If a checkpoint already exists at the same byte position, it is
    /// replaced in place. Otherwise the new checkpoint is inserted at the
    /// correct sorted index so [`Self::find_before`] and [`Self::find_after`]
    /// can use binary search.
    ///
    /// When the cache exceeds `max_checkpoints`, entries are evicted while
    /// preserving the earliest and latest checkpoints as boundary anchors for
    /// incremental parsing windows.
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

    /// Find the nearest checkpoint at or before a given position.
    ///
    /// Uses binary search over the sorted `checkpoints` vector (invariant
    /// maintained by [`Self::add`]) for O(log N) rather than the previous
    /// O(N) linear scan. This matters when the checkpoint limit is large
    /// (50+) for big documents (#2080).
    pub fn find_before(&self, position: usize) -> Option<&LexerCheckpoint> {
        // `partition_point` returns the index of the first element for which
        // the predicate is false (i.e. the first entry with pos > position).
        // The entry just before that index is the last one with pos <= position.
        let idx = self.checkpoints.partition_point(|(pos, _)| *pos <= position);
        if idx == 0 { None } else { self.checkpoints.get(idx - 1).map(|(_, cp)| cp) }
    }

    /// Find the nearest checkpoint at or after a given position.
    ///
    /// Uses binary search over the sorted `checkpoints` vector (invariant
    /// maintained by [`Self::add`]) for O(log N) performance. This is the
    /// counterpart to [`Self::find_before`] and is needed for two-sided
    /// checkpoint windows in incremental parsing (#3527).
    ///
    /// # Arguments
    ///
    /// * `position` - The byte position to search from
    ///
    /// # Returns
    ///
    /// * `Some(&LexerCheckpoint)` - The checkpoint at or after the given position
    /// * `None` - If no checkpoint exists at or after the position
    ///
    /// # Examples
    ///
    /// ```
    /// # use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};
    /// let mut cache = CheckpointCache::new(10);
    /// cache.add(LexerCheckpoint::at_position(100));
    /// cache.add(LexerCheckpoint::at_position(200));
    /// cache.add(LexerCheckpoint::at_position(300));
    ///
    /// // Find checkpoint at or after position 150
    /// let cp = cache.find_after(150);
    /// assert!(matches!(cp, Some(found) if found.position == 200));
    ///
    /// // Find checkpoint at exact position
    /// let cp = cache.find_after(200);
    /// assert!(matches!(cp, Some(found) if found.position == 200));
    ///
    /// // Position beyond last checkpoint returns None
    /// let cp = cache.find_after(400);
    /// assert!(cp.is_none());
    /// ```
    pub fn find_after(&self, position: usize) -> Option<&LexerCheckpoint> {
        // `partition_point` returns the index of the first element for which
        // the predicate is false (i.e. the first entry with pos >= position).
        let idx = self.checkpoints.partition_point(|(pos, _)| *pos < position);
        self.checkpoints.get(idx).map(|(_, cp)| cp)
    }

    /// Clear all cached checkpoints.
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    /// Apply an edit to all cached checkpoints, shifting or invalidating each
    /// entry and re-sorting to preserve the binary-search invariant.
    pub fn apply_edit(&mut self, start: usize, old_len: usize, new_len: usize) {
        for (pos, checkpoint) in &mut self.checkpoints {
            checkpoint.apply_edit(start, old_len, new_len);
            *pos = checkpoint.position;
        }

        // Edits can move checkpoints backward (or to the same position), so
        // restore the sorted-order invariant required by binary-search lookups.
        // NOTE: do NOT drop checkpoints that landed at position 0 — a Normal
        // checkpoint shifted to byte 0 by a deletion is a valid start-of-file
        // anchor and must be preserved for incremental re-lexing (#7508).
        self.checkpoints.sort_unstable_by_key(|(pos, _)| *pos);
    }
}
