# Issue #3527 Re-Scope: Incremental Token Cache is Coarse-Grained and Uses Heuristic Splice Boundaries

**Original Issue Title:** "Incremental parsing token caching not implemented"

**Re-Scope Title:** "Incremental token cache is coarse-grained and uses heuristic splice boundaries"

**Issue Number:** #3527

---

## Current State Analysis

The incremental parsing implementation already has working token caching infrastructure. The following components exist and are functional:

### TokenCache Implementation
**Location:** [`crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:39-44`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:39-44)

```rust
struct TokenCache {
    /// All cached parser tokens in source order.
    tokens: Vec<Token>,
    /// The byte range `[start, end)` that the cached tokens cover.
    valid_range: Option<(usize, usize)>,
}
```

The `TokenCache` provides:
- Storage for parser tokens in source order
- Byte range tracking for validity
- Methods for token retrieval by position:
  - [`get_tokens_from()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:55-62) - tokens starting at or after position
  - [`get_tokens_before()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:65-72) - tokens ending at or before position
  - [`cache_tokens()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:75-78) - replace entire cache
  - [`invalidate_range()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:81-88) - invalidate on overlap

### Cache Invalidation
**Location:** [`crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:81-88`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:81-88)

```rust
/// Invalidate the cache if the given byte range overlaps with the cached range.
fn invalidate_range(&mut self, start: usize, end: usize) {
    if let Some((valid_start, valid_end)) = self.valid_range {
        if start <= valid_end && end >= valid_start {
            self.valid_range = None;
            self.tokens.clear();
        }
    }
}
```

**Current behavior:** All-or-nothing invalidation - any overlap clears the entire cache.

### Incremental Reparse Path
**Location:** [`crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:238-321`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:238-321)

The [`reparse_from_checkpoint()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:238-321) method implements a three-phase approach:

1. **Phase 1:** Reuse cached tokens before the checkpoint ([`lines 252-255`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:252-255))
2. **Phase 2:** Re-lex the affected region with a fixed heuristic window ([`lines 257-275`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:257-275))
3. **Phase 3:** Reuse cached tokens after the affected region ([`lines 277-306`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:277-306))

The method drives parsing via [`Parser::from_tokens()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:316), avoiding re-lexing for the pre-assembled token stream.

### CheckpointCache
**Location:** [`crates/perl-lexer/src/checkpoint.rs:239-245`](../../crates/perl-lexer/src/checkpoint.rs:239-245)

```rust
pub fn find_before(&self, position: usize) -> Option<&LexerCheckpoint> {
    let idx = self.checkpoints.partition_point(|(pos, _)| *pos <= position);
    if idx == 0 { None } else { self.checkpoints.get(idx - 1).map(|(_, cp)| cp) }
}
```

**Current state:** Only provides `find_before()` - no `find_after()` method exists.

### Statistics Tracking
**Location:** [`crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:91-94`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:91-94)

The [`IncrementalStats`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:91-94) struct tracks:
- `total_parses`
- `cache_hits`
- `cache_misses`
- `tokens_reused`
- `tokens_relexed`

---

## What is Actually Missing

The implementation works but has significant limitations that reduce its effectiveness:

### 1. Coarse-Grained Cache Granularity
- **Problem:** The `TokenCache` stores a single monolithic `Vec<Token>` covering the entire valid range
- **Impact:** Any edit overlap invalidates the entire cache, even for unrelated regions
- **Example:** A small change in line 1 invalidates cache for a 10,000-line file

### 2. All-or-Nothing Invalidation
- **Problem:** [`invalidate_range()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:81-88) clears the entire cache on any overlap
- **Impact:** No partial reuse possible when edits affect only a subset of the cached range
- **Missing:** Segment-based invalidation that preserves unaffected cache segments

### 3. Fixed Heuristic Window
- **Problem:** [`reparse_from_checkpoint()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:258) uses a fixed `+100` byte lookahead:
  ```rust
  let relex_end = edit.start + edit.new_text.len() + 100; // small lookahead
  ```
- **Impact:** May not re-lex enough context for complex edits, or may over-relex for simple edits
- **Missing:** Adaptive window sizing based on edit characteristics

### 4. Single-Sided Checkpoint Lookup
- **Problem:** [`CheckpointCache`](../../crates/perl-lexer/src/checkpoint.rs:239-245) only provides `find_before()`
- **Impact:** Cannot find checkpoints after a position for two-sided splice windows
- **Missing:** `find_after()` method to enable bidirectional checkpoint selection

### 5. Limited Metrics
- **Problem:** Current statistics don't track cache segment utilization or invalidation patterns
- **Impact:** Difficult to measure effectiveness of incremental parsing in production
- **Missing:** Segment-level metrics, invalidation cause tracking, window size effectiveness

---

## Re-Scored Implementation Plan

### Phase 1: Segment-Based Token Cache

**Goal:** Replace monolithic cache with segment-based storage to enable partial invalidation.

**Changes:**
1. Refactor `TokenCache` to store `Vec<TokenSegment>` instead of `Vec<Token>`
2. Implement `TokenSegment` struct with:
   - `tokens: Vec<Token>`
   - `range: (usize, usize)`
   - `checksum: Option<u64>` (for validation)
3. Add segment management methods:
   - `invalidate_segment(start, end)` - removes overlapping segments
   - `merge_adjacent_segments()` - combines contiguous segments
   - `find_overlapping_segments(start, end)` - returns affected segments
4. Update `get_tokens_from()` and `get_tokens_before()` to work with segments

**Benefits:**
- Partial cache reuse on edits
- Better cache hit rates for large files
- Foundation for advanced invalidation strategies

### Phase 2: Two-Sided Checkpoint Windows

**Goal:** Enable bidirectional checkpoint selection for optimal splice boundaries.

**Changes:**
1. Add `find_after(position)` to [`CheckpointCache`](../../crates/perl-lexer/src/checkpoint.rs:239-245)
2. Implement `find_checkpoint_window(start, end)` that returns:
   - `before: Option<LexerCheckpoint>`
   - `after: Option<LexerCheckpoint>`
3. Update [`reparse_from_checkpoint()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:238-321) to:
   - Use checkpoint window instead of single checkpoint
   - Re-lex only between `before` and `after` checkpoints
   - Reuse cached segments outside the window
4. Add adaptive window sizing based on:
   - Edit type (insert/delete/replace)
   - Edit size
   - Token density in affected region

**Benefits:**
- More precise re-lex boundaries
- Reduced re-lex work
- Better cache utilization

### Phase 3: Enhanced Metrics and Validation (Optional)

**Goal:** Add observability and validation for incremental parsing.

**Changes:**
1. Extend `IncrementalStats` with:
   - `segments_invalidate_count`
   - `segments_merged_count`
   - `avg_window_size`
   - `cache_hit_by_segment_size` (histogram)
2. Add validation mode that:
   - Compares incremental parse results to full parse
   - Reports discrepancies
   - Can be enabled via feature flag
3. Add benchmark harness for:
   - Cache hit rate vs. file size
   - Invalidation pattern analysis
   - Window size effectiveness

**Note:** This phase is optional and can be deferred based on production needs.

---

## Recommended PR Sequence

### PR 1: Segment-Based Token Cache
- Implement `TokenSegment` struct
- Refactor `TokenCache` to use segments
- Add segment management methods
- Update existing callers
- Add unit tests for segment operations
- **No breaking changes to public API**

### PR 2: Two-Sided Checkpoint Windows
- Add `find_after()` to `CheckpointCache`
- Implement `find_checkpoint_window()`
- Update `reparse_from_checkpoint()` to use checkpoint windows
- Add adaptive window sizing logic
- Add integration tests
- **May require minor updates to public API**

### PR 3: Enhanced Metrics and Validation (Optional)
- Extend `IncrementalStats` with new metrics
- Add validation mode
- Add benchmark harness
- Update documentation
- **Pure additions, no breaking changes**

---

## Acceptance Criteria (for PR 2 - Segment Cache + Checkpoint Window)

### Functional Requirements

1. **Segment-Based Invalidation**
   - [ ] Edits only invalidate overlapping cache segments
   - [ ] Non-overlapping segments remain valid after invalidation
   - [ ] Adjacent segments are merged when appropriate
   - [ ] Cache hit rate improves by at least 20% for typical edit patterns

2. **Two-Sided Checkpoint Windows**
   - [ ] `CheckpointCache::find_after()` returns checkpoints after a position
   - [ ] `CheckpointCache::find_checkpoint_window()` returns both before and after checkpoints
   - [ ] `reparse_from_checkpoint()` uses checkpoint windows when available
   - [ ] Re-lex region is bounded by checkpoint window, not fixed heuristic

3. **Adaptive Window Sizing**
   - [ ] Window size adapts based on edit characteristics
   - [ ] Small edits use smaller windows
   - [ ] Large edits use larger windows
   - [ ] Window size is logged in statistics

4. **Correctness**
   - [ ] Incremental parse results match full parse results in all test cases
   - [ ] No token position errors after cache reuse
   - [ ] Byte offset adjustments are correct for insert/delete operations
   - [ ] All existing tests pass

### Performance Requirements

1. **Cache Utilization**
   - [ ] Cache hit rate > 60% for files > 1000 lines
   - [ ] Cache hit rate > 40% for files > 100 lines
   - [ ] Average tokens reused > 10x tokens relexed for typical edits

2. **Latency**
   - [ ] Incremental parse latency < 50ms for 1000-line files
   - [ ] Incremental parse latency < 200ms for 10000-line files
   - [ ] No regression in full parse latency

### Code Quality Requirements

1. **Testing**
   - [ ] Unit tests for all new public methods
   - [ ] Integration tests for end-to-end incremental parsing
   - [ ] Regression tests for edge cases (empty files, single-token files, etc.)
   - [ ] Performance benchmarks for cache hit rates

2. **Documentation**
   - [ ] All new public APIs have doc comments
   - [ ] Updated module-level documentation
   - [ ] Examples of segment-based cache usage
   - [ ] Explanation of adaptive window sizing algorithm

3. **Code Style**
   - [ ] Passes `cargo fmt --all`
   - [ ] Passes `cargo clippy --all-targets`
   - [ ] No new `unwrap()`, `expect()`, `panic!()` in production code
   - [ ] Follows project coding conventions

---

## References

### Code Locations
- [`TokenCache`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:39-44) - Current monolithic cache implementation
- [`invalidate_range()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:81-88) - All-or-nothing invalidation
- [`reparse_from_checkpoint()`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:238-321) - Three-phase incremental reparse
- [`CheckpointCache::find_before()`](../../crates/perl-lexer/src/checkpoint.rs:239-245) - Single-sided checkpoint lookup
- [`IncrementalStats`](../../crates/perl-incremental-parsing/src/incremental/incremental_checkpoint.rs:91-94) - Current statistics tracking

### Related Documentation
- [`docs/project/CURRENT_STATUS.md`](CURRENT_STATUS.md) - Current project status
- [`docs/project/ROADMAP.md`](ROADMAP.md) - Project roadmap
- [`docs/reference/LSP_IMPLEMENTATION_GUIDE.md`](../reference/LSP_IMPLEMENTATION_GUIDE.md) - LSP implementation details
- [`docs/reference/CRATE_ARCHITECTURE_GUIDE.md`](../reference/CRATE_ARCHITECTURE_GUIDE.md) - Crate architecture

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-04-10 | Initial re-scope documentation | - |

---

## Notes

- This re-scope removes references to `ContentTokenCache`, LRU dependency, and content-hash dedup as these were identified as Phase 3 optional features
- The focus is on improving the existing working implementation rather than adding new infrastructure
- All changes should maintain backward compatibility where possible
- Performance benchmarks should be run before and after each PR to validate improvements
