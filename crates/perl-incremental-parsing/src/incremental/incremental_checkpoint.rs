//! Incremental parser with lexer checkpointing
//!
//! This module provides a fully incremental parser that uses lexer checkpoints
//! to efficiently re-lex only the changed portions of the input.

use crate::{ast::Node, edit::Edit as OriginalEdit, error::ParseResult, parser::Parser};
use perl_lexer::{CheckpointCache, Checkpointable, LexerCheckpoint, PerlLexer, Token};
use std::collections::HashMap;

/// Incremental parser with lexer checkpointing
pub struct CheckpointedIncrementalParser {
    /// Current source text
    source: String,
    /// Current parse tree
    tree: Option<Node>,
    /// Lexer checkpoint cache
    checkpoint_cache: CheckpointCache,
    /// Token cache for reuse
    token_cache: TokenCache,
    /// Statistics
    stats: IncrementalStats,
}

/// Cache for tokens to avoid re-lexing
struct TokenCache {
    /// Tokens indexed by start position
    tokens: HashMap<usize, Vec<Token>>,
    /// Valid range for cached tokens
    valid_range: Option<(usize, usize)>,
}

impl TokenCache {
    fn new() -> Self {
        TokenCache { tokens: HashMap::new(), valid_range: None }
    }

    /// Get cached tokens starting at position
    fn get_tokens_at(&self, position: usize) -> Option<&[Token]> {
        if let Some((start, end)) = self.valid_range {
            if position >= start && position < end {
                return self.tokens.get(&position).map(|v| v.as_slice());
            }
        }
        None
    }

    /// Cache tokens for a range
    fn cache_tokens(&mut self, start: usize, end: usize, tokens: Vec<Token>) {
        // Group tokens by start position
        self.tokens.clear();
        let mut current_pos = start;
        let mut token_groups = Vec::new();
        let mut current_group = Vec::new();

        for token in tokens {
            if token.start != current_pos && !current_group.is_empty() {
                token_groups.push((current_pos, current_group));
                current_group = Vec::new();
                current_pos = token.start;
            }
            current_group.push(token);
        }

        if !current_group.is_empty() {
            token_groups.push((current_pos, current_group));
        }

        // Store in map
        for (pos, tokens) in token_groups {
            self.tokens.insert(pos, tokens);
        }

        self.valid_range = Some((start, end));
    }

    /// Invalidate cache for an edit, preserving tokens that end before the edit start.
    ///
    /// Rather than wiping the entire cache on any overlapping edit, we keep token
    /// groups that are entirely before `edit_start`. This allows `get_tokens_before`
    /// to return pre-edit tokens for reuse on the next incremental parse.
    fn invalidate_range(&mut self, edit_start: usize, edit_end: usize) {
        if let Some((valid_start, valid_end)) = self.valid_range {
            if edit_start <= valid_end && edit_end >= valid_start {
                // Edit overlaps with the cached range.
                // Keep only token groups whose tokens all end at or before edit_start.
                self.tokens.retain(|_pos, tokens| tokens.iter().all(|t| t.end <= edit_start));

                // Shrink the valid range to [valid_start, edit_start).
                if edit_start > valid_start {
                    self.valid_range = Some((valid_start, edit_start));
                } else {
                    // Edit covers the very beginning of the cached range — nothing remains.
                    self.valid_range = None;
                    self.tokens.clear();
                }
            }
        }
    }

    /// Return all cached tokens whose start position is strictly less than `position`,
    /// sorted by start position.
    ///
    /// This replaces the previous `get_tokens_at(0)` call in `reparse_from_checkpoint`
    /// which only retrieved tokens keyed at position 0 rather than the full prefix.
    fn get_tokens_before(&self, position: usize) -> Vec<&Token> {
        let mut result: Vec<&Token> = self
            .tokens
            .iter()
            .filter(|(k, _)| **k < position)
            .flat_map(|(_, v)| v.iter())
            .collect();
        result.sort_by_key(|t| t.start);
        result
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

    /// Parse with checkpoint collection
    fn parse_with_checkpoints(&mut self) -> ParseResult<Node> {
        let mut lexer = PerlLexer::new(&self.source);
        let mut tokens = Vec::new();
        let mut checkpoint_positions = vec![0, 100, 500, 1000, 5000];

        // Collect tokens and checkpoints
        let mut position = 0;
        while let Some(token) = lexer.next_token() {
            // Save checkpoint at specific positions
            if checkpoint_positions.first() == Some(&position) {
                checkpoint_positions.remove(0);
                let checkpoint = lexer.checkpoint();
                self.checkpoint_cache.add(checkpoint);
            }

            position = token.end;

            // Skip EOF
            if matches!(token.token_type, perl_lexer::TokenType::EOF) {
                break;
            }

            tokens.push(token);
        }

        // Cache all tokens
        if let (Some(first), Some(last)) = (tokens.first(), tokens.last()) {
            let start = first.start;
            let end = last.end;
            self.token_cache.cache_tokens(start, end, tokens);
        }

        // Parse using regular parser
        let mut parser = Parser::new(&self.source);
        parser.parse()
    }

    /// Reparse from a checkpoint
    fn reparse_from_checkpoint(
        &mut self,
        checkpoint: LexerCheckpoint,
        edit: &SimpleEdit,
    ) -> ParseResult<Node> {
        // Create lexer and restore checkpoint
        let mut lexer = PerlLexer::new(&self.source);
        lexer.restore(&checkpoint);

        let mut tokens = Vec::new();
        let relex_start = checkpoint.position;

        // Reuse all cached tokens that end at or before the checkpoint position.
        // `get_tokens_before` returns every token group whose start key is < relex_start,
        // sorted by start position, replacing the previous `get_tokens_at(0)` call which
        // only retrieved the single group keyed at position 0.
        for token in self.token_cache.get_tokens_before(relex_start) {
            if token.end <= relex_start {
                tokens.push(token.clone());
                self.stats.tokens_reused += 1;
            }
        }

        // Lex from checkpoint to end of affected region
        let relex_end = edit.start + edit.new_text.len() + 100; // Some lookahead
        loop {
            if let Some(token) = lexer.next_token() {
                if matches!(token.token_type, perl_lexer::TokenType::EOF) {
                    break;
                }
                let token_end = token.end;
                tokens.push(token);
                self.stats.tokens_relexed += 1;

                // Check if we've lexed past the affected region
                if token_end >= relex_end {
                    break;
                }
            } else {
                break;
            }
        }

        // Try to reuse tokens after the affected region
        let after_edit_pos = edit.start + edit.new_text.len();
        if let Some(cached) = self.token_cache.get_tokens_at(after_edit_pos) {
            self.stats.cache_hits += 1;
            let shift = edit.new_text.len() as isize - (edit.end - edit.start) as isize;
            for token in cached {
                // Guard against integer underflow: if the shift would move the token
                // start below zero (e.g. a deletion larger than the token's start
                // position), skip rather than wrapping.
                let new_start = token.start as isize + shift;
                let new_end = token.end as isize + shift;
                if new_start < 0 || new_end < 0 {
                    continue;
                }
                let mut adjusted_token = token.clone();
                adjusted_token.start = new_start as usize;
                adjusted_token.end = new_end as usize;
                tokens.push(adjusted_token);
                self.stats.tokens_reused += 1;
            }
        } else {
            self.stats.cache_misses += 1;
            // Lex the rest
            while let Some(token) = lexer.next_token() {
                if matches!(token.token_type, perl_lexer::TokenType::EOF) {
                    break;
                }
                tokens.push(token);
                self.stats.tokens_relexed += 1;
            }
        }

        // Cache the new tokens
        if let (Some(first), Some(last)) = (tokens.first(), tokens.last()) {
            let start = first.start;
            let end = last.end;
            self.token_cache.cache_tokens(start, end, tokens);
        }

        // TODO(#3021): feed the assembled token stream to the parser once
        // `Parser::from_tokens(Vec<Token>)` is implemented in perl-parser-core.
        // Until then the mixed pre/post-edit token stream built above is used only
        // for the token-reuse statistics; the parser still does a full re-lex from
        // source. This correctly increments `tokens_reused` as a count of reusable
        // tokens, but the parser does not benefit from the saved lexing work yet.
        let mut parser = Parser::new(&self.source);
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
        assert!(stats.tokens_reused > 0);
        assert!(stats.tokens_relexed > 0);
    }

    #[test]
    fn test_tokens_reused_after_single_char_edit() {
        let mut parser = CheckpointedIncrementalParser::new();
        // Use a source long enough that pre-edit tokens exist.
        let source = "my $x = 42;\nmy $y = 99;\nmy $z = 0;\n".to_string();
        must(parser.parse(source));
        // Edit a value in the middle: change 99 to 9999.
        let edit = SimpleEdit { start: 20, end: 22, new_text: "9999".to_string() };
        must(parser.apply_edit(&edit));
        let stats = parser.stats();
        assert!(stats.tokens_reused > 0, "expected tokens before edit to be reused, got 0");
        assert!(stats.tokens_relexed > 0, "expected some re-lexing around the edit");
    }

    #[test]
    fn test_large_deletion_no_underflow() {
        let mut parser = CheckpointedIncrementalParser::new();
        must(parser.parse("my $x = 42;\n".to_string()));
        // Delete more bytes than exist before edit point — must not panic.
        let edit = SimpleEdit { start: 3, end: 11, new_text: "".to_string() };
        must(parser.apply_edit(&edit));
    }

    #[test]
    fn test_tokens_reused_multiple_edits() {
        let mut parser = CheckpointedIncrementalParser::new();
        let source = "my $x = 1;\n".repeat(20);
        must(parser.parse(source));
        let edit1 = SimpleEdit { start: 8, end: 9, new_text: "42".to_string() };
        must(parser.apply_edit(&edit1));
        let edit2 = SimpleEdit { start: 20, end: 21, new_text: "99".to_string() };
        must(parser.apply_edit(&edit2));
        assert!(parser.stats().tokens_reused > 0);
    }
}
