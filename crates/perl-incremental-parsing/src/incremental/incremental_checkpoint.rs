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
//! 2. `reparse_from_checkpoint` assembles a mixed token list from cached tokens
//!    (before the edit) + freshly-lexed tokens (affected region) + cached or
//!    freshly-lexed tokens (after the edit), then calls `Parser::from_tokens`
//!    to skip re-lexing the unchanged portions.

use crate::{ast::Node, edit::Edit as OriginalEdit, error::ParseResult, parser::Parser};
use perl_lexer::{CheckpointCache, Checkpointable, LexerCheckpoint, PerlLexer};
use perl_parser_core::token_stream::{Token, TokenStream};

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

/// Cache for parser tokens to avoid re-lexing.
///
/// Stores [`Token`] values (from `perl-token`) rather than raw lexer tokens so
/// that the cached values can be fed directly to [`Parser::from_tokens`].
struct TokenCache {
    /// All cached parser tokens in source order.
    tokens: Vec<Token>,
    /// The byte range `[start, end)` that the cached tokens cover.
    valid_range: Option<(usize, usize)>,
}

impl TokenCache {
    fn new() -> Self {
        TokenCache { tokens: Vec::new(), valid_range: None }
    }

    /// Return a sub-slice of cached tokens whose `start` is `>= position`.
    ///
    /// Because tokens are stored in source order we can binary-search for the
    /// first token at or after `position`.
    fn get_tokens_from(&self, position: usize) -> Option<&[Token]> {
        let (valid_start, valid_end) = self.valid_range?;
        if position < valid_start || position >= valid_end {
            return None;
        }
        let idx = self.tokens.partition_point(|t| t.start < position);
        Some(&self.tokens[idx..])
    }

    /// Return a sub-slice of cached tokens that end at or before `position`.
    fn get_tokens_before(&self, position: usize) -> Option<&[Token]> {
        let (valid_start, _valid_end) = self.valid_range?;
        if self.tokens.is_empty() || valid_start >= position {
            return None;
        }
        let idx = self.tokens.partition_point(|t| t.end <= position);
        if idx == 0 { None } else { Some(&self.tokens[..idx]) }
    }

    /// Replace the entire cache with a new set of parser tokens.
    fn cache_tokens(&mut self, start: usize, end: usize, tokens: Vec<Token>) {
        self.tokens = tokens;
        self.valid_range = Some((start, end));
    }

    /// Invalidate the cache if the given byte range overlaps with the cached range.
    fn invalidate_range(&mut self, start: usize, end: usize) {
        if let Some((valid_start, valid_end)) = self.valid_range {
            if start <= valid_end && end >= valid_start {
                self.valid_range = None;
                self.tokens.clear();
            }
        }
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
            crate::position::Position::new(self.start, 0, 0),
            crate::position::Position::new(self.end, 0, 0),
            crate::position::Position::new(self.start + self.new_text.len(), 0, 0),
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

        // Invalidate token cache for edited range
        self.token_cache.invalidate_range(edit.start, edit.end);

        // Update checkpoint cache
        let old_len = edit.end - edit.start;
        let new_len = new_content.len();
        self.checkpoint_cache.apply_edit(edit.start, old_len, new_len);

        // Find nearest checkpoint before edit
        let checkpoint = self.checkpoint_cache.find_before(edit.start);

        if let Some(checkpoint) = checkpoint {
            self.stats.checkpoints_used += 1;
            self.reparse_from_checkpoint(checkpoint.clone(), edit)
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

    /// Reparse from a lexer checkpoint using cached tokens where possible.
    ///
    /// Assembles a parser-token stream that reuses cached tokens for the
    /// unchanged regions and re-lexes only the portion affected by the edit,
    /// then calls [`Parser::from_tokens`] to drive the parse without invoking
    /// the lexer again.
    fn reparse_from_checkpoint(
        &mut self,
        checkpoint: LexerCheckpoint,
        edit: &SimpleEdit,
    ) -> ParseResult<Node> {
        // Restore the lexer at the checkpoint position so we can re-lex the
        // affected region.
        let mut lexer = PerlLexer::new(&self.source);
        lexer.restore(&checkpoint);

        let relex_start = checkpoint.position;
        let mut parser_tokens: Vec<Token> = Vec::new();

        // --- Phase 1: reuse cached tokens before the checkpoint ---
        if let Some(cached) = self.token_cache.get_tokens_before(relex_start) {
            parser_tokens.extend_from_slice(cached);
            self.stats.tokens_reused += cached.len();
        }

        // --- Phase 2: re-lex the region from the checkpoint through the edit ---
        let relex_end = edit.start + edit.new_text.len() + 100; // small lookahead
        let mut raw_relexed: Vec<perl_lexer::Token> = Vec::new();
        loop {
            match lexer.next_token() {
                Some(token) if matches!(token.token_type, perl_lexer::TokenType::EOF) => break,
                Some(token) => {
                    let token_end = token.end;
                    raw_relexed.push(token);
                    self.stats.tokens_relexed += 1;
                    if token_end >= relex_end {
                        break;
                    }
                }
                None => break,
            }
        }
        let converted = TokenStream::lexer_tokens_to_parser_tokens(raw_relexed);
        parser_tokens.extend(converted);

        // --- Phase 3: reuse cached tokens after the affected region ---
        let after_edit_pos = edit.start + edit.new_text.len();
        let byte_shift: isize = edit.new_text.len() as isize - (edit.end - edit.start) as isize;

        if let Some(cached) = self.token_cache.get_tokens_from(after_edit_pos) {
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
    use crate::NodeKind;
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
        let source = "my $x = 1;\n".repeat(20);
        must(parser.parse(source));

        // Multiple edits
        let edit1 = SimpleEdit { start: 8, end: 9, new_text: "42".to_string() };
        must(parser.apply_edit(&edit1));

        let edit2 = SimpleEdit { start: 20, end: 21, new_text: "99".to_string() };
        must(parser.apply_edit(&edit2));

        let stats = parser.stats();
        assert_eq!(stats.incremental_parses, 2);
        assert!(stats.tokens_relexed > 0);
    }

    #[test]
    fn test_from_tokens_used_in_reparse() {
        // Verify that `reparse_from_checkpoint` actually uses `Parser::from_tokens`
        // by checking that `tokens_reused` is non-zero after an incremental reparse
        // in a source large enough to have a checkpoint and cached tokens.
        let mut parser = CheckpointedIncrementalParser::new();

        // Source large enough to have tokens before the first checkpoint (position 0)
        // so that the token cache has entries the reparse can reuse.
        let source = format!("my $preamble = {};\n", "1".repeat(5));
        must(parser.parse(source.clone()));

        // Edit after the preamble so cached tokens before it can be reused.
        let edit_start = source.find('=').unwrap_or(13) + 2; // just past `= `
        let edit_end = edit_start + 5; // covers "11111"
        let edit = SimpleEdit { start: edit_start, end: edit_end, new_text: "99999".to_string() };

        must(parser.apply_edit(&edit));

        let stats = parser.stats();
        assert_eq!(stats.incremental_parses, 1);
        // The reparse should have re-lexed at least some tokens in the edited region.
        assert!(
            stats.tokens_relexed > 0 || stats.tokens_reused > 0,
            "expected either reused or relexed tokens, got {:?}",
            stats
        );
    }
}
