//! Incremental parser with lexer checkpointing
//!
//! This module provides a fully incremental parser that uses lexer checkpoints
//! to efficiently re-lex only the changed portions of the input.
//!
//! # Pipeline integration
//!
//! Token caching and `Parser::from_tokens` are now wired together:
//!
//! 1. `parse_with_checkpoints` collects **parser tokens** (trivia-filtered,
//!    kind-converted) and caches them alongside the lexer checkpoints.
//! 2. `reparse_from_checkpoint_two_sided` assembles a mixed token list from:
//!    - cached tokens before the left checkpoint (unchanged prefix)
//!    - freshly-lexed tokens between left and right checkpoints (affected region)
//!    - cached tokens after the right checkpoint, with byte shift applied (unchanged suffix)
//!    - Then calls `Parser::from_tokens` to skip re-lexing the unchanged portions.
//!
//! # Two-Sided Checkpoint Window
//!
//! The incremental parser uses a two-sided checkpoint window approach (#3527):
//!
//! - **Left checkpoint**: The nearest checkpoint at or before the edit start
//! - **Right checkpoint**: The nearest checkpoint at or after the edit end
//!
//! This replaces the previous fixed heuristic (+100 bytes) with checkpoint-based
//! bounds, providing more precise re-lexing regions and better cache utilization.
//!
//! The three-phase reparse algorithm ensures correctness by:
//! 1. Reusing cached tokens from the start up to the left checkpoint
//! 2. Re-lexing from the left checkpoint through the edit to the right checkpoint
//! 3. Reusing cached tokens from the right checkpoint to the end
//!
//! Edge cases are handled gracefully:
//! - No checkpoint before edit → relex from position 0
//! - No checkpoint after edit → relex to `source.len()`
//! - Checkpoint at edit boundary → minimal re-lexing scope
//!
//! # Segment-Level Metrics
//!
//! The incremental parser tracks segment-level metrics to diagnose cache efficiency
//! and fallback behavior. These metrics help understand how well the segment-based
//! caching system is working:
//!
//! - **`segments_reused_before`**: Count of segments reused before the edit.
//!   A high value indicates good cache coverage for the unchanged prefix.
//!
//! - **`segments_reused_after`**: Count of segments reused after the edit.
//!   A high value indicates good cache coverage for the unchanged suffix.
//!
//! - **`segments_invalidated`**: Count of segments invalidated during edit.
//!   A high value indicates high cache churn, which may suggest the need for
//!   more granular segments or different checkpoint placement strategies.
//!
//! - **`full_tail_fallbacks`**: Count of times we had to relex the entire tail
//!   because no cache hit was found after the edit. A high value indicates
//!   cache coverage gaps, which may suggest the need for more checkpoints
//!   in the tail region.
//!
//! These metrics can be accessed via [`CheckpointedIncrementalParser::stats()`]
//! and formatted using the `Display` implementation for debugging and monitoring.

use perl_lexer::{CheckpointCache, Checkpointable, LexerCheckpoint, PerlLexer};
use perl_parser_core::token_stream::{Token, TokenStream};
use perl_parser_core::{ast::Node, edit::Edit as OriginalEdit, error::ParseResult, parser::Parser};

/// Incremental parser with lexer checkpointing
pub struct CheckpointedIncrementalParser {
    /// Current source text
    source: String,
    /// Current parse tree
    tree: Option<Node>,
    /// Lexer checkpoint cache
    checkpoint_cache: CheckpointCache,
    /// Token cache for reuse — stores **parser** tokens (trivia-filtered, kind-converted).
    token_cache: TokenCache,
    /// Statistics
    stats: IncrementalStats,
}

/// A contiguous segment of cached parser tokens.
///
/// Each segment represents a range of source code that has been parsed and
/// whose tokens can be reused during incremental reparses.
#[derive(Debug, Clone)]
struct TokenSegment {
    /// Start byte position of this segment in the source.
    start: usize,
    /// End byte position of this segment in the source.
    end: usize,
    /// Parser tokens for this segment, in source order.
    tokens: Vec<Token>,
}

impl TokenSegment {
    /// Create a new token segment.
    fn new(start: usize, end: usize, tokens: Vec<Token>) -> Self {
        TokenSegment { start, end, tokens }
    }

    /// Check if this segment overlaps with the given range.
    fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start < end && self.end > start
    }

    /// Check if this segment is completely before the given position.
    fn is_before(&self, position: usize) -> bool {
        self.end <= position
    }

    /// Check if this segment is completely after the given position.
    fn is_after(&self, position: usize) -> bool {
        self.start >= position
    }
}

/// Cache for parser tokens to avoid re-lexing.
///
/// Stores [`Token`] values (from `perl-token`) rather than raw lexer tokens so
/// that the cached values can be fed directly to [`Parser::from_tokens`].
///
/// The cache is organized as a collection of independent segments, each
/// covering a contiguous range of source code. This allows for partial
/// invalidation when edits affect only part of the source, rather than
/// clearing the entire cache.
///
/// Segments are maintained in sorted order by their start position for
/// efficient binary search operations.
struct TokenCache {
    /// Cached token segments, sorted by start position.
    segments: Vec<TokenSegment>,
}

impl TokenCache {
    /// Create a new empty token cache.
    fn new() -> Self {
        TokenCache { segments: Vec::new() }
    }

    /// Return all segments that end at or before the given position.
    ///
    /// This is used to retrieve cached tokens from the unchanged portion of
    /// source code before an edit.
    ///
    /// # Arguments
    ///
    /// * `position` - The byte position to search up to.
    ///
    /// # Returns
    ///
    /// A vector of segments ending at or before `position`, in source order.
    fn get_segments_before(&self, position: usize) -> Vec<TokenSegment> {
        self.segments.iter().filter(|seg| seg.is_before(position)).cloned().collect()
    }

    /// Return all segments that start at or after the given position.
    ///
    /// This is used to retrieve cached tokens from the unchanged portion of
    /// source code after an edit.
    ///
    /// # Arguments
    ///
    /// * `position` - The byte position to search from.
    ///
    /// # Returns
    ///
    /// A vector of segments starting at or after `position`, in source order.
    fn get_segments_after(&self, position: usize) -> Vec<TokenSegment> {
        self.segments.iter().filter(|seg| seg.is_after(position)).cloned().collect()
    }

    /// Return all segments that overlap with the given range.
    ///
    /// This is useful for determining which segments would be affected by
    /// an edit or for finding cached tokens in a specific region.
    ///
    /// # Arguments
    ///
    /// * `start` - Start byte position of the range.
    /// * `end` - End byte position of the range.
    ///
    /// # Returns
    ///
    /// A vector of segments overlapping the range `[start, end)`, in source order.
    fn get_segments_in_range(&self, start: usize, end: usize) -> Vec<TokenSegment> {
        self.segments.iter().filter(|seg| seg.overlaps(start, end)).cloned().collect()
    }

    /// Add a new segment to the cache, maintaining sorted order.
    ///
    /// If the new segment overlaps with existing segments, those segments are
    /// removed before adding the new one. This ensures the cache maintains
    /// non-overlapping segments.
    ///
    /// # Arguments
    ///
    /// * `segment` - The segment to add.
    fn add_segment(&mut self, segment: TokenSegment) {
        // Remove any overlapping segments
        self.segments.retain(|seg| !seg.overlaps(segment.start, segment.end));

        // Find insertion point to maintain sorted order
        let idx = self.segments.partition_point(|seg| seg.start < segment.start);

        self.segments.insert(idx, segment);
    }

    /// Invalidate segments that overlap with the given range.
    ///
    /// Unlike the monolithic cache which cleared entirely on any overlap,
    /// this only removes the affected segments, preserving unrelated cached
    /// tokens.
    ///
    /// # Arguments
    ///
    /// * `start` - Start byte position of the range to invalidate.
    /// * `end` - End byte position of the range to invalidate.
    fn invalidate_range(&mut self, start: usize, end: usize) {
        self.segments.retain(|seg| !seg.overlaps(start, end));
    }

    /// Adjust segment positions after an edit.
    ///
    /// Segments that start after `edit_start` have their positions shifted
    /// by the difference between `new_len` and `old_len`. This keeps the
    /// cache consistent with the modified source.
    ///
    /// # Arguments
    ///
    /// * `edit_start` - Start byte position of the edit.
    /// * `old_len` - Length of the removed text.
    /// * `new_len` - Length of the inserted text.
    fn adjust_positions(&mut self, edit_start: usize, old_len: usize, new_len: usize) {
        let delta = new_len as isize - old_len as isize;

        if delta == 0 {
            return;
        }

        for segment in &mut self.segments {
            if segment.start >= edit_start {
                segment.start = (segment.start as isize + delta) as usize;
                segment.end = (segment.end as isize + delta) as usize;
            }
        }
    }

    /// Return cached tokens whose `start` is `>= position`.
    ///
    /// This method maintains backward compatibility with the original
    /// monolithic cache API by collecting tokens from all relevant segments.
    ///
    /// # Arguments
    ///
    /// * `position` - The byte position to search from.
    ///
    /// # Returns
    ///
    /// A slice of tokens starting at or after `position`, or `None` if no
    /// cached tokens exist in that range.
    fn get_tokens_from(&self, position: usize) -> Option<Vec<Token>> {
        let segments = self.get_segments_after(position);
        if segments.is_empty() {
            return None;
        }

        // Collect all tokens from segments after position
        let mut all_tokens = Vec::new();
        for segment in segments {
            // Filter tokens within the segment that start at or after position
            for token in &segment.tokens {
                if token.start >= position {
                    all_tokens.push(token.clone());
                }
            }
        }

        if all_tokens.is_empty() { None } else { Some(all_tokens) }
    }

    /// Return cached tokens that end at or before `position`.
    ///
    /// This method maintains backward compatibility with the original
    /// monolithic cache API by collecting tokens from all relevant segments.
    ///
    /// # Arguments
    ///
    /// * `position` - The byte position to search up to.
    ///
    /// # Returns
    ///
    /// A slice of tokens ending at or before `position`, or `None` if no
    /// cached tokens exist in that range.
    fn get_tokens_before(&self, position: usize) -> Option<Vec<Token>> {
        let segments = self.get_segments_before(position);
        if segments.is_empty() {
            return None;
        }

        // Collect all tokens from segments before position
        let mut all_tokens = Vec::new();
        for segment in segments {
            // Filter tokens within the segment that end at or before position
            for token in &segment.tokens {
                if token.end <= position {
                    all_tokens.push(token.clone());
                }
            }
        }

        if all_tokens.is_empty() { None } else { Some(all_tokens) }
    }

    /// Add a new segment to the cache.
    ///
    /// This replaces the original `cache_tokens` method which replaced the
    /// entire cache. The new implementation adds tokens as an independent
    /// segment, allowing for partial invalidation.
    ///
    /// # Arguments
    ///
    /// * `start` - Start byte position of the segment.
    /// * `end` - End byte position of the segment.
    /// * `tokens` - Parser tokens for this segment.
    fn cache_tokens(&mut self, start: usize, end: usize, tokens: Vec<Token>) {
        if tokens.is_empty() {
            return;
        }

        let segment = TokenSegment::new(start, end, tokens);
        self.add_segment(segment);
    }
}

/// Statistics for incremental parsing
#[derive(Debug, Default)]
pub struct IncrementalStats {
    pub total_parses: usize,
    pub incremental_parses: usize,
    pub tokens_reused: usize,
    pub tokens_relexed: usize,
    pub checkpoints_used: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    /// Distance from edit start to the left checkpoint (in bytes)
    pub left_checkpoint_distance: usize,
    /// Distance from edit end to the right checkpoint (in bytes)
    pub right_checkpoint_distance: usize,
    /// Total bytes relexed during incremental parsing
    pub bytes_relexed: usize,
    /// Count of segments reused before the edit (cache efficiency metric)
    pub segments_reused_before: usize,
    /// Count of segments reused after the edit (cache efficiency metric)
    pub segments_reused_after: usize,
    /// Count of segments invalidated during edit (cache churn metric)
    pub segments_invalidated: usize,
    /// Count of times we had to relex the entire tail (cache coverage gaps)
    pub full_tail_fallbacks: usize,
}

impl std::fmt::Display for IncrementalStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Incremental Parsing Statistics:")?;
        writeln!(f, "  Total parses: {}", self.total_parses)?;
        writeln!(f, "  Incremental parses: {}", self.incremental_parses)?;
        writeln!(f, "  Tokens reused: {}", self.tokens_reused)?;
        writeln!(f, "  Tokens relexed: {}", self.tokens_relexed)?;
        writeln!(f, "  Checkpoints used: {}", self.checkpoints_used)?;
        writeln!(f, "  Cache hits: {}", self.cache_hits)?;
        writeln!(f, "  Cache misses: {}", self.cache_misses)?;
        writeln!(f, "  Left checkpoint distance: {} bytes", self.left_checkpoint_distance)?;
        writeln!(f, "  Right checkpoint distance: {} bytes", self.right_checkpoint_distance)?;
        writeln!(f, "  Bytes relexed: {}", self.bytes_relexed)?;
        writeln!(f, "  Segments reused before edit: {}", self.segments_reused_before)?;
        writeln!(f, "  Segments reused after edit: {}", self.segments_reused_after)?;
        writeln!(f, "  Segments invalidated: {}", self.segments_invalidated)?;
        writeln!(f, "  Full tail fallbacks: {}", self.full_tail_fallbacks)?;
        Ok(())
    }
}

/// Simple edit structure for demos
#[derive(Debug, Clone)]
pub struct SimpleEdit {
    pub start: usize,
    pub end: usize,
    pub new_text: String,
}

impl SimpleEdit {
    /// Convert to original Edit format if needed
    pub fn to_original_edit(&self) -> OriginalEdit {
        // Simplified conversion - would need proper position tracking
        OriginalEdit::new(
            self.start,
            self.end,
            self.start + self.new_text.len(),
            perl_parser_core::position::Position::new(self.start, 0, 0),
            perl_parser_core::position::Position::new(self.end, 0, 0),
            perl_parser_core::position::Position::new(self.start + self.new_text.len(), 0, 0),
        )
    }
}

impl Default for CheckpointedIncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckpointedIncrementalParser {
    /// Create a new incremental parser
    pub fn new() -> Self {
        CheckpointedIncrementalParser {
            source: String::new(),
            tree: None,
            checkpoint_cache: CheckpointCache::new(50), // Keep 50 checkpoints for large files (#2080)
            token_cache: TokenCache::new(),
            stats: IncrementalStats::default(),
        }
    }

    /// Parse the initial source
    pub fn parse(&mut self, source: String) -> ParseResult<Node> {
        self.source = source;
        self.stats.total_parses += 1;

        // Full parse with checkpoint collection
        let tree = self.parse_with_checkpoints()?;
        self.tree = Some(tree.clone());

        Ok(tree)
    }

    /// Apply an edit and reparse incrementally
    pub fn apply_edit(&mut self, edit: &SimpleEdit) -> ParseResult<Node> {
        self.stats.total_parses += 1;
        self.stats.incremental_parses += 1;

        // Apply edit to source
        let new_content = &edit.new_text;
        self.source.replace_range(edit.start..edit.end, new_content);

        // Track segments that will be invalidated before invalidating
        let invalidated_segments = self.token_cache.get_segments_in_range(edit.start, edit.end);
        self.stats.segments_invalidated += invalidated_segments.len();

        // Invalidate token cache for edited range
        self.token_cache.invalidate_range(edit.start, edit.end);

        // Update checkpoint cache
        let old_len = edit.end - edit.start;
        let new_len = new_content.len();
        self.checkpoint_cache.apply_edit(edit.start, old_len, new_len);

        // Adjust token cache segment positions for the edit
        self.token_cache.adjust_positions(edit.start, old_len, new_len);

        // Find nearest checkpoints before and after the edit for two-sided window
        let left_checkpoint = self.checkpoint_cache.find_before(edit.start);
        let right_checkpoint = self.checkpoint_cache.find_after(edit.start + new_len);

        if left_checkpoint.is_some() || right_checkpoint.is_some() {
            self.stats.checkpoints_used += 1;
            self.reparse_from_checkpoint_two_sided(
                left_checkpoint.cloned(),
                right_checkpoint.cloned(),
                edit,
            )
        } else {
            // No checkpoint found, full reparse
            self.parse_with_checkpoints()
        }
    }

    /// Parse with checkpoint collection and parser-token caching.
    ///
    /// Collects lexer checkpoints at pre-defined positions and caches the full
    /// set of **parser** tokens (trivia-filtered) so they can be reused during
    /// subsequent incremental reparses.
    fn parse_with_checkpoints(&mut self) -> ParseResult<Node> {
        // A full reparse must rebuild both caches from the current source
        // state; carrying forward shifted checkpoints from a prior incremental
        // edit can poison later checkpoint selection.
        self.checkpoint_cache.clear();
        self.token_cache = TokenCache::new();

        let mut lexer = PerlLexer::new(&self.source);
        let mut raw_tokens = Vec::new();
        let mut checkpoint_positions = vec![0, 100, 500, 1000, 5000];

        // Collect raw lexer tokens and save checkpoints at specific positions
        let mut position = 0;
        while let Some(token) = lexer.next_token() {
            // Save checkpoint at specific positions
            if checkpoint_positions.first() == Some(&position) {
                checkpoint_positions.remove(0);
                let checkpoint = lexer.checkpoint();
                self.checkpoint_cache.add(checkpoint);
            }

            position = token.end;

            // Stop at EOF
            if matches!(token.token_type, perl_lexer::TokenType::EOF) {
                break;
            }

            raw_tokens.push(token);
        }

        // Convert raw lexer tokens to parser tokens (trivia-filtered + kind-mapped)
        // and cache them for reuse in incremental reparses.
        let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw_tokens);

        if let (Some(first), Some(last)) = (parser_tokens.first(), parser_tokens.last()) {
            let start = first.start;
            let end = last.end;
            self.token_cache.cache_tokens(start, end, parser_tokens);
        }

        // Full parse from source — this initial parse still uses the lexer
        // directly so that context-sensitive constructs (e.g. regex vs division)
        // are correctly disambiguated.
        let mut parser = Parser::new(&self.source);
        parser.parse()
    }

    /// Reparse using two-sided checkpoint windows.
    ///
    /// This method implements a two-sided checkpoint window approach for incremental
    /// parsing, which provides more precise bounds for re-lexing than the previous
    /// fixed heuristic (+100 bytes).
    ///
    /// # Two-Sided Checkpoint Window Algorithm
    ///
    /// The algorithm works in three phases:
    ///
    /// 1. **Phase 1 - Before the edit**: Find the nearest checkpoint before the edit
    ///    and reuse all cached tokens from the beginning of the source up to that
    ///    checkpoint position.
    ///
    /// 2. **Phase 2 - Between checkpoints**: Re-lex the region between the left
    ///    checkpoint (before the edit) and the right checkpoint (after the edit).
    ///    This ensures we capture any context-sensitive changes that might affect
    ///    tokenization beyond the immediate edit region.
    ///
    /// 3. **Phase 3 - After the edit**: Reuse all cached tokens from the right
    ///    checkpoint position to the end of the source, applying a byte shift to
    ///    account for the edit's size change.
    ///
    /// # Edge Cases
    ///
    /// - **No checkpoint before edit**: The relex region starts at position 0.
    /// - **No checkpoint after edit**: The relex region ends at `source.len()`.
    /// - **Checkpoint at edit boundary**: The checkpoint is used as-is, providing
    ///   minimal re-lexing scope.
    /// - **Edit spans multiple checkpoint boundaries**: All affected segments are
    ///   invalidated, and the relex region spans from the leftmost to rightmost
    ///   checkpoint boundaries.
    ///
    /// # Correctness
    ///
    /// The two-sided checkpoint window ensures correctness by:
    ///
    /// - Re-lexing from a stable checkpoint state (left checkpoint) to capture
    ///   context-sensitive tokenization changes.
    /// - Re-lexing through the edit and continuing until the next checkpoint to
    ///   ensure any ripple effects are captured.
    /// - Reusing cached tokens only from regions that are guaranteed to be
    ///   unaffected by the edit.
    ///
    /// # Arguments
    ///
    /// * `left_checkpoint` - Optional checkpoint before the edit (used to start re-lexing)
    /// * `right_checkpoint` - Optional checkpoint after the edit (used to stop re-lexing)
    /// * `edit` - The edit being applied
    ///
    /// # Returns
    ///
    /// The parsed AST node.
    fn reparse_from_checkpoint_two_sided(
        &mut self,
        left_checkpoint: Option<LexerCheckpoint>,
        right_checkpoint: Option<LexerCheckpoint>,
        edit: &SimpleEdit,
    ) -> ParseResult<Node> {
        // Calculate relex bounds using checkpoint positions
        let relex_start = left_checkpoint.as_ref().map(|cp| cp.position).unwrap_or(0);
        let relex_end =
            right_checkpoint.as_ref().map(|cp| cp.position).unwrap_or(self.source.len());

        // Track checkpoint distances for statistics
        let edit_end = edit.start + edit.new_text.len();
        if edit.start >= relex_start {
            self.stats.left_checkpoint_distance = edit.start - relex_start;
        }
        if relex_end >= edit_end {
            self.stats.right_checkpoint_distance = relex_end - edit_end;
        }

        let mut parser_tokens: Vec<Token> = Vec::new();

        // --- Phase 1: reuse cached tokens before the left checkpoint ---
        let segments_before = self.token_cache.get_segments_before(relex_start);
        self.stats.segments_reused_before += segments_before.len();
        let cached_before = self.token_cache.get_tokens_before(relex_start);

        // The cache is still populated as a single whole-file segment. After an
        // interior edit, invalidation can remove that entire segment and leave us
        // with no prefix tokens for a nonzero checkpoint window. In that state,
        // feeding a suffix-only token stream into `Parser::from_tokens` would
        // drop the unchanged prefix, so preserve correctness by falling back to a
        // full reparse until the cache becomes genuinely segment-granular.
        if relex_start > 0 && cached_before.is_none() {
            self.stats.cache_misses += 1;
            return self.parse_with_checkpoints();
        }

        if let Some(cached) = cached_before {
            self.stats.cache_hits += 1;
            let reused_count = cached.len();
            parser_tokens.extend(cached);
            self.stats.tokens_reused += reused_count;
        }

        // --- Phase 2: re-lex the region between checkpoints ---
        // Restore lexer at left checkpoint position (or start of source)
        let mut lexer = PerlLexer::new(&self.source);
        if let Some(ref cp) = left_checkpoint {
            lexer.restore(cp);
        }

        let mut raw_relexed: Vec<perl_lexer::Token> = Vec::new();
        let mut bytes_relexed_this_phase = 0usize;

        loop {
            match lexer.next_token() {
                Some(token) if matches!(token.token_type, perl_lexer::TokenType::EOF) => break,
                Some(token) => {
                    let token_end = token.end;
                    let token_start = token.start;
                    raw_relexed.push(token);
                    self.stats.tokens_relexed += 1;
                    bytes_relexed_this_phase += token_end - token_start;
                    if token_end >= relex_end {
                        break;
                    }
                }
                None => break,
            }
        }
        self.stats.bytes_relexed += bytes_relexed_this_phase;

        let converted = TokenStream::lexer_tokens_to_parser_tokens(raw_relexed);
        parser_tokens.extend(converted);

        // --- Phase 3: reuse cached tokens after the right checkpoint ---
        let byte_shift: isize = edit.new_text.len() as isize - (edit.end - edit.start) as isize;

        let segments_after = self.token_cache.get_segments_after(relex_end);
        self.stats.segments_reused_after += segments_after.len();

        if let Some(cached) = self.token_cache.get_tokens_from(relex_end) {
            self.stats.cache_hits += 1;
            for token in cached {
                // Adjust byte positions to account for the inserted/removed bytes.
                let adjusted = Token {
                    kind: token.kind,
                    text: token.text.clone(),
                    start: (token.start as isize + byte_shift) as usize,
                    end: (token.end as isize + byte_shift) as usize,
                };
                parser_tokens.push(adjusted);
                self.stats.tokens_reused += 1;
            }
        } else {
            self.stats.cache_misses += 1;
            // Track full tail fallback when no segments are found after relex_end
            if segments_after.is_empty() {
                self.stats.full_tail_fallbacks += 1;
            }
            // No cache hit — lex the remainder of the source.
            let mut raw_tail: Vec<perl_lexer::Token> = Vec::new();
            while let Some(token) = lexer.next_token() {
                if matches!(token.token_type, perl_lexer::TokenType::EOF) {
                    break;
                }
                raw_tail.push(token);
                self.stats.tokens_relexed += 1;
            }
            parser_tokens.extend(TokenStream::lexer_tokens_to_parser_tokens(raw_tail));
        }

        // Update token cache with the final merged token list.
        if let (Some(first), Some(last)) = (parser_tokens.first(), parser_tokens.last()) {
            let start = first.start;
            let end = last.end;
            self.token_cache.cache_tokens(start, end, parser_tokens.clone());
        }

        // Drive the parse from the pre-assembled token stream — no re-lexing.
        let mut parser = Parser::from_tokens(parser_tokens, &self.source);
        let tree = parser.parse()?;
        self.tree = Some(tree.clone());

        Ok(tree)
    }

    /// Get parsing statistics
    pub fn stats(&self) -> &IncrementalStats {
        &self.stats
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        self.checkpoint_cache.clear();
        self.token_cache = TokenCache::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::NodeKind;
    use perl_tdd_support::must;

    #[test]
    fn test_checkpoint_incremental_parsing() {
        let mut parser = CheckpointedIncrementalParser::new();

        // Initial parse
        let source = "my $x = 42;\nmy $y = 99;\n".to_string();
        let tree1 = must(parser.parse(source));

        // Edit: change 42 to 4242
        let edit = SimpleEdit { start: 8, end: 10, new_text: "4242".to_string() };

        let tree2 = must(parser.apply_edit(&edit));

        // Check stats
        let stats = parser.stats();
        assert_eq!(stats.total_parses, 2);
        assert_eq!(stats.incremental_parses, 1);
        assert!(stats.checkpoints_used > 0 || stats.tokens_relexed > 0);

        // Trees should be structurally similar
        if let (NodeKind::Program { statements: s1 }, NodeKind::Program { statements: s2 }) =
            (&tree1.kind, &tree2.kind)
        {
            assert_eq!(s1.len(), s2.len());
        } else {
            unreachable!("Expected program nodes");
        }
    }

    #[test]
    fn test_checkpoint_cache_update() {
        let mut parser = CheckpointedIncrementalParser::new();

        // Parse a larger file
        let mut expected_source = "my $x = 1;\n".repeat(20);
        must(parser.parse(expected_source.clone()));

        // Multiple edits
        let edit1 = SimpleEdit { start: 8, end: 9, new_text: "42".to_string() };
        must(parser.apply_edit(&edit1));
        expected_source.replace_range(edit1.start..edit1.end, &edit1.new_text);
        let checkpoints_after_first = parser.stats().checkpoints_used;
        let cache_events_after_first = parser.stats().cache_hits + parser.stats().cache_misses;

        let edit2 = SimpleEdit { start: 20, end: 21, new_text: "99".to_string() };
        let incremental_tree = must(parser.apply_edit(&edit2));
        expected_source.replace_range(edit2.start..edit2.end, &edit2.new_text);

        let stats = parser.stats();
        assert_eq!(stats.incremental_parses, 2);
        assert!(
            stats.checkpoints_used > checkpoints_after_first,
            "expected second edit to exercise checkpoint bookkeeping, got {stats:?}"
        );
        assert!(
            stats.cache_hits + stats.cache_misses > cache_events_after_first,
            "expected second edit to consult cache bookkeeping, got {stats:?}"
        );

        let mut full = CheckpointedIncrementalParser::new();
        let full_tree = must(full.parse(expected_source));
        assert_eq!(
            format!("{incremental_tree:?}"),
            format!("{full_tree:?}"),
            "incremental tree diverged from fresh full parse"
        );
    }

    #[test]
    fn test_checkpointed_reparse_tracks_cache_or_fallback_path() {
        // The current cache is still monolithic, so an interior edit can
        // conservatively fall back to a full reparse once the unchanged prefix
        // can no longer be proven safe. What remains guaranteed is that the
        // incremental checkpoint/cache path is exercised and accounted for.
        let mut parser = CheckpointedIncrementalParser::new();

        // Source large enough to establish checkpoints and cached tokens.
        let source = format!("my $preamble = {};\n", "1".repeat(5));
        must(parser.parse(source.clone()));

        // Edit after the preamble so the incremental path consults the checkpoint window.
        let edit_start = source.find('=').unwrap_or(13) + 2; // just past `= `
        let edit_end = edit_start + 5; // covers "11111"
        let edit = SimpleEdit { start: edit_start, end: edit_end, new_text: "99999".to_string() };

        let checkpoints_before = parser.stats().checkpoints_used;
        let cache_events_before = parser.stats().cache_hits + parser.stats().cache_misses;

        let incremental_tree = must(parser.apply_edit(&edit));
        let mut expected_source = source;
        expected_source.replace_range(edit.start..edit.end, &edit.new_text);

        let stats = parser.stats();
        assert_eq!(stats.incremental_parses, 1);
        assert!(
            stats.checkpoints_used > checkpoints_before,
            "expected checkpoint bookkeeping from incremental reparse, got {stats:?}"
        );
        assert!(
            stats.cache_hits + stats.cache_misses > cache_events_before,
            "expected cache bookkeeping from incremental reparse or conservative fallback, got {stats:?}"
        );

        let mut full = CheckpointedIncrementalParser::new();
        let full_tree = must(full.parse(expected_source));
        assert_eq!(
            format!("{incremental_tree:?}"),
            format!("{full_tree:?}"),
            "incremental tree diverged from fresh full parse"
        );
    }

    #[test]
    fn test_full_fallback_rebuilds_checkpoint_cache() {
        let source = "my $value = 1;\n".repeat(80);
        let edit = SimpleEdit { start: 125, end: 126, new_text: "999".to_string() };

        let mut edited_source = source.clone();
        edited_source.replace_range(edit.start..edit.end, &edit.new_text);

        let mut incremental = CheckpointedIncrementalParser::new();
        must(incremental.parse(source));
        must(incremental.apply_edit(&edit));

        let mut full = CheckpointedIncrementalParser::new();
        must(full.parse(edited_source.clone()));

        for query in (0..=edited_source.len()).step_by(17) {
            let incremental_before =
                incremental.checkpoint_cache.find_before(query).map(|cp| cp.position);
            let full_before = full.checkpoint_cache.find_before(query).map(|cp| cp.position);
            assert_eq!(incremental_before, full_before, "mismatched left checkpoint at {query}");

            let incremental_after =
                incremental.checkpoint_cache.find_after(query).map(|cp| cp.position);
            let full_after = full.checkpoint_cache.find_after(query).map(|cp| cp.position);
            assert_eq!(incremental_after, full_after, "mismatched right checkpoint at {query}");
        }
    }
}
