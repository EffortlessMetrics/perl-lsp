use perl_ast::ast::Node;
use std::ops::Range;

use crate::{PragmaSnapshot, PragmaState, range_builder};

/// Query object describing compile-time pragma state at a byte offset.
#[derive(Debug, Clone, PartialEq)]
pub struct PragmaStateQuery {
    offset: usize,
    snapshot: PragmaSnapshot,
}

impl PragmaStateQuery {
    /// Byte offset this query was created for.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Immutable snapshot at this query position.
    #[must_use]
    pub fn snapshot(&self) -> &PragmaSnapshot {
        &self.snapshot
    }
}

/// Explicit compile-time pragma environment that can answer file-position
/// queries and expose immutable snapshots.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompileTimePragmaEnvironment {
    map: PragmaMap,
}

/// One effective pragma-state transition over a source byte range.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PragmaEntry {
    /// Source byte range covered by this snapshot.
    ///
    /// Lexical scope restores are represented as zero-length ranges at the
    /// scope end so callers can observe the restored state at that byte offset.
    pub range: Range<usize>,
    /// Immutable pragma state active for this transition.
    pub snapshot: PragmaSnapshot,
}

/// Explicit pragma transition timeline.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct PragmaMap {
    entries: Box<[PragmaEntry]>,
}

impl CompileTimePragmaEnvironment {
    /// Build a queryable environment from an AST.
    #[must_use]
    pub fn build(ast: &Node) -> Self {
        let mut ranges = Vec::new();
        let mut current_state = PragmaState::default();
        range_builder::build_ranges(ast, &mut current_state, &mut ranges);
        ranges.sort_by_key(|(range, _)| range.start);

        let entries = ranges
            .into_iter()
            .map(|(range, state)| PragmaEntry { range, snapshot: PragmaSnapshot::from(state) })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self { map: PragmaMap { entries } }
    }

    /// Return a position query object with immutable state snapshot.
    #[must_use]
    pub fn query_at(&self, offset: usize) -> PragmaStateQuery {
        PragmaStateQuery { offset, snapshot: self.snapshot_at(offset) }
    }

    /// Return the immutable snapshot active at the given byte offset.
    #[must_use]
    pub fn snapshot_at(&self, offset: usize) -> PragmaSnapshot {
        self.map.snapshot_at(offset)
    }

    /// Access the underlying transition map.
    #[must_use]
    pub fn map(&self) -> &PragmaMap {
        &self.map
    }

    /// Access the underlying range map for advanced consumers.
    #[must_use]
    pub fn as_map(&self) -> Vec<(Range<usize>, PragmaSnapshot)> {
        self.map.to_tuples()
    }
}

impl PragmaMap {
    /// Return the immutable snapshot active at the given byte offset.
    #[must_use]
    pub fn snapshot_at(&self, offset: usize) -> PragmaSnapshot {
        let idx = self.entries.partition_point(|entry| entry.range.start <= offset);
        let snapshot = if idx > 0 {
            self.entries[idx - 1].snapshot.clone()
        } else {
            PragmaSnapshot::default()
        };

        normalize_snapshot(snapshot)
    }

    /// Return the concrete pragma state active at the given byte offset.
    #[must_use]
    pub fn state_at(&self, offset: usize) -> PragmaState {
        self.snapshot_at(offset).into()
    }

    /// Return the final top-level pragma state after all lexical restores.
    #[must_use]
    pub fn final_state(&self) -> PragmaState {
        let state = self
            .entries
            .last()
            .map_or_else(PragmaState::default, |entry| entry.snapshot.clone().into());

        normalize_state(state)
    }

    /// Create a cursor for monotonic queries against this map.
    #[must_use]
    pub fn cursor(&self) -> PragmaQueryCursor {
        PragmaQueryCursor::new()
    }

    /// Return all transition entries in source order.
    #[must_use]
    pub fn entries(&self) -> &[PragmaEntry] {
        &self.entries
    }

    /// Convert this map to the legacy tuple representation.
    #[must_use]
    pub fn to_tuples(&self) -> Vec<(Range<usize>, PragmaSnapshot)> {
        self.entries.iter().map(|e| (e.range.clone(), e.snapshot.clone())).collect()
    }
}

fn normalize_snapshot(mut snapshot: PragmaSnapshot) -> PragmaSnapshot {
    if snapshot.state.signatures_strict {
        snapshot.state.strict_vars = true;
        snapshot.state.strict_subs = true;
        snapshot.state.strict_refs = true;
    }

    snapshot
}

pub(crate) fn normalize_state(mut state: PragmaState) -> PragmaState {
    if state.signatures_strict {
        state.strict_vars = true;
        state.strict_subs = true;
        state.strict_refs = true;
    }

    state
}

/// Monotonic query cursor for repeated pragma lookups.
///
/// Reuse a single cursor when querying offsets in non-decreasing order to
/// avoid repeated binary searches over the pragma map.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PragmaQueryCursor {
    index: usize,
}

impl PragmaQueryCursor {
    /// Create a new cursor positioned before the start of the map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Query state at `offset` assuming lookups are mostly non-decreasing.
    ///
    /// This is the primary cursor API for the explicit pragma map.
    /// If the caller queries an older offset, this falls back to a binary
    /// search and repositions the cursor.
    pub fn snapshot_at(&mut self, pragma_map: &PragmaMap, offset: usize) -> PragmaSnapshot {
        let snapshot = self
            .entry_for_offset(pragma_map.entries(), offset)
            .map_or_else(PragmaSnapshot::default, |entry| entry.snapshot.clone());

        normalize_snapshot(snapshot)
    }

    /// Query state at `offset` against the explicit pragma map.
    pub fn state_at(&mut self, pragma_map: &PragmaMap, offset: usize) -> PragmaState {
        self.snapshot_at(pragma_map, offset).into()
    }

    /// Query state at `offset` assuming lookups are mostly non-decreasing.
    ///
    /// This legacy tuple API is retained for existing `PragmaTracker` callers.
    /// If the caller queries an older offset, this falls back to a binary
    /// search and repositions the cursor.
    pub fn state_for_offset(
        &mut self,
        pragma_map: &[(Range<usize>, PragmaState)],
        offset: usize,
    ) -> PragmaState {
        if pragma_map.is_empty() {
            return PragmaState::default();
        }

        if self.index >= pragma_map.len() {
            self.index = pragma_map.len() - 1;
        }

        if pragma_map[self.index].0.start > offset {
            self.index = pragma_map.partition_point(|(range, _)| range.start <= offset);
            if self.index > 0 {
                self.index -= 1;
            }
        } else {
            while self.index + 1 < pragma_map.len() && pragma_map[self.index + 1].0.start <= offset
            {
                self.index += 1;
            }
        }

        let state = if pragma_map[self.index].0.start <= offset {
            pragma_map[self.index].1.clone()
        } else {
            PragmaState::default()
        };

        normalize_state(state)
    }

    fn entry_for_offset<'a>(
        &mut self,
        entries: &'a [PragmaEntry],
        offset: usize,
    ) -> Option<&'a PragmaEntry> {
        if entries.is_empty() {
            return None;
        }

        if self.index >= entries.len() {
            self.index = entries.len() - 1;
        }

        if entries[self.index].range.start > offset {
            self.index = entries.partition_point(|entry| entry.range.start <= offset);
            if self.index > 0 {
                self.index -= 1;
            }
        } else {
            while self.index + 1 < entries.len() && entries[self.index + 1].range.start <= offset {
                self.index += 1;
            }
        }

        if entries[self.index].range.start <= offset { Some(&entries[self.index]) } else { None }
    }
}
