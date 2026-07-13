//! Lower-tier incremental token replay for the native parser.
//!
//! This module owns the smallest reusable incremental kernel: it preserves
//! parser tokens outside an edit, re-lexes a checkpoint-bounded window, and
//! drives the normal parser from the assembled token stream. The AST is still
//! rebuilt by the parser; callers must use the metrics and fallback reason
//! before making stronger subtree-reuse claims.

use crate::{
    ast::Node,
    engine::parser::Parser,
    error::{ParseError, ParseResult},
    token_stream::{Token, TokenStream},
};
use perl_lexer::{Checkpointable, LexerCheckpoint, PerlLexer, TokenType};
use std::fmt;
use std::ops::Range;

const CHECKPOINT_INTERVAL: usize = 256;
const MAX_INCREMENTAL_EDIT_BYTES: usize = 4096;

/// A source edit in old-source byte coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IncrementalEdit {
    /// Start byte in the old source.
    pub start_byte: usize,
    /// Exclusive end byte in the old source.
    pub old_end_byte: usize,
    /// Replacement text in the new source.
    pub new_text: String,
}

impl IncrementalEdit {
    /// Create a source edit.
    pub fn new(start_byte: usize, old_end_byte: usize, new_text: impl Into<String>) -> Self {
        Self { start_byte, old_end_byte, new_text: new_text.into() }
    }

    /// Number of bytes touched in the old source or inserted into the new source.
    pub fn touched_bytes(&self) -> usize {
        self.old_end_byte.saturating_sub(self.start_byte).saturating_add(self.new_text.len())
    }
}

/// Why an incremental attempt fell back to a complete parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FallbackReason {
    /// The edit is too large for the bounded replay path.
    EditTooLarge,
    /// No stable checkpoint window was available.
    NoCheckpointWindow,
    /// The cached prefix or suffix could not be assembled safely.
    CacheBoundaryUnavailable,
    /// Parsing the assembled token stream failed.
    TokenReplayFailed,
    /// Format declarations require lexer/parser state that token replay cannot preserve safely.
    ContextSensitiveFormat,
    /// Replayed recovery diagnostics were not safe to preserve with shifted tokens.
    RecoveryDiagnosticsUnstable,
}

impl fmt::Display for FallbackReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            Self::EditTooLarge => "edit too large",
            Self::NoCheckpointWindow => "no checkpoint window",
            Self::CacheBoundaryUnavailable => "cache boundary unavailable",
            Self::TokenReplayFailed => "token replay failed",
            Self::ContextSensitiveFormat => "context-sensitive format declaration",
            Self::RecoveryDiagnosticsUnstable => "replayed recovery diagnostics unstable",
        };
        formatter.write_str(description)
    }
}

/// Measurements for the most recent parse or incremental replay.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IncrementalMetrics {
    /// Whether the most recent operation parsed the complete source.
    pub full_parse: bool,
    /// Number of source bytes re-lexed for the bounded replay window.
    pub reparsed_bytes: usize,
    /// Number of cached parser tokens retained outside that window.
    pub tokens_reused: usize,
    /// Number of parser tokens produced by fresh lexing.
    pub tokens_relexed: usize,
    /// The source range re-lexed by the incremental operation.
    pub changed_range: Range<usize>,
    /// Fallback classification, when the complete-parse path was used.
    pub fallback: Option<FallbackReason>,
}

impl IncrementalMetrics {
    fn full(source_len: usize, fallback: Option<FallbackReason>) -> Self {
        Self {
            full_parse: true,
            reparsed_bytes: source_len,
            tokens_reused: 0,
            tokens_relexed: 0,
            changed_range: 0..source_len,
            fallback,
        }
    }
}

/// Cached lexical state shared by parser facades that support incremental edits.
#[derive(Debug, Clone)]
pub struct IncrementalState {
    source: String,
    tokens: Vec<Token>,
    checkpoints: Vec<LexerCheckpoint>,
    latest_metrics: IncrementalMetrics,
    diagnostics: Vec<ParseError>,
}

impl IncrementalState {
    /// Build a token and checkpoint cache for an already parsed source.
    pub fn new(source: &str) -> Self {
        let (tokens, checkpoints) = lex_full(source);
        Self {
            source: source.to_owned(),
            tokens,
            checkpoints,
            latest_metrics: IncrementalMetrics::full(source.len(), None),
            diagnostics: Vec::new(),
        }
    }

    /// Build a token cache with diagnostics from the parse that produced it.
    pub fn with_diagnostics(source: &str, diagnostics: &[ParseError]) -> Self {
        let mut state = Self::new(source);
        state.diagnostics = diagnostics.to_vec();
        state
    }

    /// Return the source represented by this cache.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Return measurements for the most recent operation.
    pub fn metrics(&self) -> &IncrementalMetrics {
        &self.latest_metrics
    }

    /// Return diagnostics from the most recent parse represented by this cache.
    pub fn diagnostics(&self) -> &[ParseError] {
        &self.diagnostics
    }

    /// Replay one edit and parse the resulting AST from the assembled tokens.
    ///
    /// If the replay window cannot be proven safe, this method preserves
    /// correctness by falling back to a complete source parse and records why.
    pub fn reparse(&mut self, new_source: &str, edit: &IncrementalEdit) -> ParseResult<Node> {
        self.validate_edit(new_source, edit)?;

        if edit.touched_bytes() > MAX_INCREMENTAL_EDIT_BYTES {
            return self.full_reparse(new_source, Some(FallbackReason::EditTooLarge));
        }
        if contains_format_declaration(&self.source) || contains_format_declaration(new_source) {
            return self.full_reparse(new_source, Some(FallbackReason::ContextSensitiveFormat));
        }

        let Some(old_relex_start) = self.replay_start(edit.start_byte) else {
            return self.full_reparse(new_source, Some(FallbackReason::NoCheckpointWindow));
        };
        let Some(old_relex_end) = self.replay_end(edit.old_end_byte) else {
            return self.full_reparse(new_source, Some(FallbackReason::NoCheckpointWindow));
        };

        if old_relex_end < old_relex_start || old_relex_end > self.source.len() {
            return self.full_reparse(new_source, Some(FallbackReason::NoCheckpointWindow));
        }

        let delta = edit.new_text.len() as isize
            - edit.old_end_byte.saturating_sub(edit.start_byte) as isize;
        let new_relex_end = shift_offset(old_relex_end, delta);
        let old_tokens = &self.tokens;

        let prefix = old_tokens
            .iter()
            .filter(|token| token.end <= old_relex_start)
            .cloned()
            .collect::<Vec<_>>();
        let suffix = old_tokens
            .iter()
            .filter(|token| token.start >= old_relex_end)
            .map(|token| shift_token(token, delta))
            .collect::<Option<Vec<_>>>();
        let Some(suffix) = suffix else {
            return self.full_reparse(new_source, Some(FallbackReason::CacheBoundaryUnavailable));
        };

        let left_checkpoint = self
            .checkpoints
            .iter()
            .find(|checkpoint| checkpoint.position == old_relex_start)
            .cloned();
        let mut lexer = PerlLexer::new(new_source);
        if let Some(checkpoint) = &left_checkpoint {
            lexer.restore(checkpoint);
        }

        let mut raw_relexed = Vec::new();
        let mut replay_checkpoints = Vec::new();
        let mut next_checkpoint = old_relex_start.saturating_add(CHECKPOINT_INTERVAL);
        while let Some(token) = lexer.next_token() {
            if token.token_type == TokenType::EOF {
                break;
            }
            if token.start >= new_relex_end {
                break;
            }
            raw_relexed.push(token.clone());
            if token.end >= next_checkpoint {
                replay_checkpoints.push(lexer.checkpoint());
                next_checkpoint = token.end.saturating_add(CHECKPOINT_INTERVAL);
            }
            if token.end >= new_relex_end {
                replay_checkpoints.push(lexer.checkpoint());
                break;
            }
        }

        if raw_relexed.last().is_some_and(|token| token.end > new_relex_end) {
            return self.full_reparse(new_source, Some(FallbackReason::CacheBoundaryUnavailable));
        }
        let reparsed = TokenStream::lexer_tokens_to_parser_tokens(raw_relexed);
        if replay_crosses_cached_suffix(&reparsed, &suffix) {
            return self.full_reparse(new_source, Some(FallbackReason::CacheBoundaryUnavailable));
        }
        let tokens_relexed = reparsed.len();
        let mut assembled = prefix;
        assembled.extend(reparsed);
        assembled.extend(suffix);

        let mut parser = Parser::from_tokens(assembled.clone(), new_source);
        let root = match parser.parse() {
            Ok(root) => root,
            Err(_) => {
                return self.full_reparse(new_source, Some(FallbackReason::TokenReplayFailed));
            }
        };
        let diagnostics = parser.errors().to_vec();
        if !diagnostics.is_empty() {
            return self
                .full_reparse(new_source, Some(FallbackReason::RecoveryDiagnosticsUnstable));
        }

        self.source = new_source.to_owned();
        self.tokens = assembled;
        self.checkpoints = merge_checkpoints(
            &self.checkpoints,
            replay_checkpoints,
            edit,
            old_relex_start,
            old_relex_end,
            new_source.len(),
        );
        self.latest_metrics = IncrementalMetrics {
            full_parse: false,
            reparsed_bytes: new_relex_end.saturating_sub(old_relex_start),
            tokens_reused: self.tokens.len().saturating_sub(tokens_relexed),
            tokens_relexed,
            changed_range: old_relex_start..new_relex_end,
            fallback: None,
        };
        self.diagnostics = diagnostics;
        Ok(root)
    }

    fn validate_edit(&self, new_source: &str, edit: &IncrementalEdit) -> ParseResult<()> {
        if edit.start_byte > edit.old_end_byte || edit.old_end_byte > self.source.len() {
            return Err(ParseError::syntax(
                format!(
                    "invalid incremental edit range {}..{} for source length {}",
                    edit.start_byte,
                    edit.old_end_byte,
                    self.source.len()
                ),
                edit.start_byte,
            ));
        }
        if !self.source.is_char_boundary(edit.start_byte)
            || !self.source.is_char_boundary(edit.old_end_byte)
        {
            return Err(ParseError::syntax(
                "incremental edit is not aligned to UTF-8 boundaries",
                edit.start_byte,
            ));
        }
        let expected_len = self
            .source
            .len()
            .saturating_sub(edit.old_end_byte - edit.start_byte)
            .saturating_add(edit.new_text.len());
        if new_source.len() != expected_len {
            return Err(ParseError::syntax(
                format!(
                    "new source length mismatch: expected {expected_len} bytes, got {}",
                    new_source.len()
                ),
                edit.start_byte,
            ));
        }
        let Some(new_edit_end) = edit.start_byte.checked_add(edit.new_text.len()) else {
            return Err(ParseError::syntax(
                "incremental edit range overflows the new source",
                edit.start_byte,
            ));
        };
        if edit.start_byte > new_source.len()
            || new_edit_end > new_source.len()
            || !new_source.is_char_boundary(edit.start_byte)
            || !new_source.is_char_boundary(new_edit_end)
        {
            return Err(ParseError::syntax(
                "new source edit is not aligned to UTF-8 boundaries",
                edit.start_byte,
            ));
        }
        if new_source.get(edit.start_byte..new_edit_end) != Some(edit.new_text.as_str()) {
            return Err(ParseError::syntax(
                "new source does not contain the replacement text at the edit range",
                edit.start_byte,
            ));
        }
        Ok(())
    }

    fn replay_start(&self, edit_start: usize) -> Option<usize> {
        let checkpoint = self.find_before(edit_start)?;
        if checkpoint.position == 0 {
            return Some(0);
        }

        let prefix_token =
            self.tokens.iter().rev().find(|token| token.end <= checkpoint.position)?;
        let preceding_checkpoint = self.find_before(prefix_token.start)?;
        (preceding_checkpoint.position < checkpoint.position)
            .then_some(preceding_checkpoint.position)
    }

    fn replay_end(&self, edit_end: usize) -> Option<usize> {
        let Some(checkpoint) = self.find_after(edit_end) else {
            return Some(self.source.len());
        };
        if checkpoint.position != edit_end {
            return Some(checkpoint.position);
        }

        self.checkpoints
            .iter()
            .find(|candidate| candidate.position > checkpoint.position)
            .map_or(Some(self.source.len()), |next| Some(next.position))
    }

    fn full_reparse(
        &mut self,
        source: &str,
        fallback: Option<FallbackReason>,
    ) -> ParseResult<Node> {
        let mut parser = Parser::new(source);
        let root = parser.parse()?;
        let diagnostics = parser.errors().to_vec();
        let (tokens, checkpoints) = lex_full(source);
        self.source = source.to_owned();
        self.tokens = tokens;
        self.checkpoints = checkpoints;
        let mut metrics = IncrementalMetrics::full(source.len(), fallback);
        metrics.tokens_relexed = self.tokens.len();
        self.latest_metrics = metrics;
        self.diagnostics = diagnostics;
        Ok(root)
    }

    fn find_before(&self, position: usize) -> Option<&LexerCheckpoint> {
        self.checkpoints.iter().rev().find(|checkpoint| checkpoint.position <= position)
    }

    fn find_after(&self, position: usize) -> Option<&LexerCheckpoint> {
        self.checkpoints.iter().find(|checkpoint| checkpoint.position >= position)
    }
}

fn lex_full(source: &str) -> (Vec<Token>, Vec<LexerCheckpoint>) {
    let mut lexer = PerlLexer::new(source);
    let mut checkpoints = vec![lexer.checkpoint()];
    let mut next_checkpoint = CHECKPOINT_INTERVAL;
    let mut raw_tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        if token.token_type == TokenType::EOF {
            break;
        }
        raw_tokens.push(token.clone());
        if token.end >= next_checkpoint {
            checkpoints.push(lexer.checkpoint());
            next_checkpoint = token.end.saturating_add(CHECKPOINT_INTERVAL);
        }
    }

    (TokenStream::lexer_tokens_to_parser_tokens(raw_tokens), checkpoints)
}

fn merge_checkpoints(
    old: &[LexerCheckpoint],
    replay: Vec<LexerCheckpoint>,
    edit: &IncrementalEdit,
    old_relex_start: usize,
    old_relex_end: usize,
    new_source_len: usize,
) -> Vec<LexerCheckpoint> {
    let mut checkpoints = old
        .iter()
        .filter_map(|checkpoint| {
            if checkpoint.position > old_relex_start && checkpoint.position < old_relex_end {
                return None;
            }
            let mut shifted = checkpoint.clone();
            if shifted.position >= edit.old_end_byte {
                shifted.apply_edit(
                    edit.start_byte,
                    edit.old_end_byte - edit.start_byte,
                    edit.new_text.len(),
                );
            }
            (shifted.position <= new_source_len).then_some(shifted)
        })
        .collect::<Vec<_>>();
    checkpoints.extend(replay);
    if checkpoints.is_empty() {
        checkpoints.push(LexerCheckpoint::new());
    }
    checkpoints.sort_by_key(|checkpoint| checkpoint.position);
    checkpoints.dedup_by_key(|checkpoint| checkpoint.position);
    checkpoints
}

fn shift_offset(offset: usize, delta: isize) -> usize {
    (offset as isize).saturating_add(delta).max(0) as usize
}

fn contains_format_declaration(source: &str) -> bool {
    source.lines().any(|line| line.trim_start().starts_with("format "))
}

fn shift_token(token: &Token, delta: isize) -> Option<Token> {
    let start = shift_offset(token.start, delta);
    let end = shift_offset(token.end, delta);
    (start <= end).then(|| Token::new(token.kind, token.text.clone(), start, end))
}

fn replay_crosses_cached_suffix(replayed: &[Token], suffix: &[Token]) -> bool {
    let Some(replayed_end) = replayed.last().map(|token| token.end) else {
        return false;
    };
    suffix.first().is_some_and(|token| token.start < replayed_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_stream::TokenKind;
    use perl_tdd_support::{must, must_some};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn assert_incremental_matches_fresh(
        source: &str,
        new_source: &str,
        edit: &IncrementalEdit,
    ) -> IncrementalMetrics {
        let mut state = IncrementalState::new(source);
        let incremental = must(state.reparse(new_source, edit));
        let mut fresh_parser = Parser::new(new_source);
        let fresh = must(fresh_parser.parse());
        let (fresh_tokens, _) = lex_full(new_source);

        assert_eq!(incremental.to_sexp(), fresh.to_sexp(), "source: {new_source:?}");
        assert_eq!(state.diagnostics(), fresh_parser.errors(), "source: {new_source:?}");
        assert_eq!(state.tokens, fresh_tokens, "source: {new_source:?}");
        state.metrics().clone()
    }

    fn identifier_checkpoint_source() -> (String, usize) {
        let identifier = "x".repeat(254);
        let source = format!("my ${identifier};\nmy $tail = 1;\n");
        let boundary = 4 + identifier.len();
        assert_eq!(boundary, 258);

        let state = IncrementalState::new(&source);
        assert!(state.checkpoints.iter().any(|checkpoint| checkpoint.position == boundary));
        assert!(
            state.tokens.iter().any(|token| token.end == boundary),
            "token ends: {:?}",
            state.tokens.iter().map(|token| token.end).collect::<Vec<_>>()
        );
        (source, boundary)
    }

    #[test]
    fn local_edit_reuses_tokens_and_matches_fresh_parse() {
        let source = "my $value = 1;\n".repeat(40);
        let mut state = IncrementalState::new(&source);
        let start = must_some(source.find('1'));
        let new_source = source.replacen('1', "22", 1);
        let edit = IncrementalEdit::new(start, start + 1, "22");

        let incremental = must(state.reparse(&new_source, &edit));
        let mut fresh_parser = Parser::new(&new_source);
        let fresh = must(fresh_parser.parse());
        let (fresh_tokens, _) = lex_full(&new_source);

        assert_eq!(incremental.to_sexp(), fresh.to_sexp());
        assert_eq!(state.diagnostics(), fresh_parser.errors());
        assert_eq!(state.tokens, fresh_tokens);
        assert!(!state.metrics().full_parse);
        assert!(state.metrics().tokens_reused > 0);
        assert!(state.metrics().tokens_relexed > 0);
    }

    #[test]
    fn sequential_edits_preserve_fresh_equivalence() {
        let source = "my $value = 1;\n".repeat(40);
        let first_start = must_some(source.find('1'));
        let first_new_source = source.replacen('1', "22", 1);
        let first_edit = IncrementalEdit::new(first_start, first_start + 1, "22");
        let mut state = IncrementalState::new(&source);

        let first_incremental = must(state.reparse(&first_new_source, &first_edit));
        let mut first_fresh_parser = Parser::new(&first_new_source);
        let first_fresh = must(first_fresh_parser.parse());
        let (first_fresh_tokens, _) = lex_full(&first_new_source);
        assert_eq!(first_incremental.to_sexp(), first_fresh.to_sexp());
        assert_eq!(state.diagnostics(), first_fresh_parser.errors());
        assert_eq!(state.tokens, first_fresh_tokens);

        let second_start = must_some(first_new_source.find("22"));
        let second_new_source = first_new_source.replacen("22", "3", 1);
        let second_edit = IncrementalEdit::new(second_start, second_start + 2, "3");

        let second_incremental = must(state.reparse(&second_new_source, &second_edit));
        let mut second_fresh_parser = Parser::new(&second_new_source);
        let second_fresh = must(second_fresh_parser.parse());
        let (second_fresh_tokens, _) = lex_full(&second_new_source);
        assert_eq!(second_incremental.to_sexp(), second_fresh.to_sexp());
        assert_eq!(state.diagnostics(), second_fresh_parser.errors());
        assert_eq!(state.tokens, second_fresh_tokens);
    }

    #[test]
    fn insertion_after_identifier_at_checkpoint_relexes_the_prefix_token() {
        let (source, boundary) = identifier_checkpoint_source();
        let identifier = "x".repeat(254);
        let new_source = format!("my ${identifier}z;\nmy $tail = 1;\n");
        let edit = IncrementalEdit::new(boundary, boundary, "z");

        let metrics = assert_incremental_matches_fresh(&source, &new_source, &edit);

        assert!(!metrics.full_parse);
        assert_eq!(metrics.changed_range.start, 0);
    }

    #[test]
    fn left_boundary_insertions_match_fresh_parse_for_lexical_delimiters() {
        let (source, boundary) = identifier_checkpoint_source();
        let identifier = "x".repeat(254);

        for insertion in ["z", "0", "#", "\"", "$"] {
            let new_source = format!("my ${identifier}{insertion};\nmy $tail = 1;\n");
            let edit = IncrementalEdit::new(boundary, boundary, insertion);
            assert_incremental_matches_fresh(&source, &new_source, &edit);
        }
    }

    #[test]
    fn edit_inside_the_first_replayed_token_matches_fresh_parse() {
        let (source, boundary) = identifier_checkpoint_source();
        let start = boundary - 1;
        let new_source = format!("my ${}y;\nmy $tail = 1;\n", "x".repeat(253));
        let edit = IncrementalEdit::new(start, boundary, "y");

        let metrics = assert_incremental_matches_fresh(&source, &new_source, &edit);

        assert!(!metrics.full_parse);
        assert_eq!(metrics.changed_range.start, 0);
    }

    #[test]
    fn number_and_operator_left_boundaries_match_fresh_parse() {
        let number_prefix = "my $value = ";
        let number = "1".repeat(256 - number_prefix.len());
        let number_source = format!("{number_prefix}{number};\nmy $tail = 1;\n");
        let number_new_source = format!("{number_prefix}{number}0;\nmy $tail = 1;\n");
        let number_edit = IncrementalEdit::new(256, 256, "0");
        assert_incremental_matches_fresh(&number_source, &number_new_source, &number_edit);

        let operator_prefix = "my $value = 1 ";
        let padding = " ".repeat(255 - operator_prefix.len());
        let operator_source = format!("{operator_prefix}{padding}+;\nmy $tail = 1;\n");
        let operator_new_source = format!("{operator_prefix}{padding}+=;\nmy $tail = 1;\n");
        let operator_edit = IncrementalEdit::new(256, 256, "=");
        assert_incremental_matches_fresh(&operator_source, &operator_new_source, &operator_edit);
    }

    #[test]
    fn unicode_edit_is_replayed_at_character_boundaries() {
        let source = "my $name = 'old';\n".to_owned();
        let mut state = IncrementalState::new(&source);
        let start = must_some(source.find("old"));
        let new_source = source.replacen("old", "é", 1);
        let edit = IncrementalEdit::new(start, start + 3, "é");

        let incremental = must(state.reparse(&new_source, &edit));
        let mut fresh_parser = Parser::new(&new_source);
        let fresh = must(fresh_parser.parse());

        assert_eq!(incremental.to_sexp(), fresh.to_sexp());
    }

    #[test]
    fn malformed_new_source_boundaries_return_an_error_without_panicking() {
        let cases =
            [("é", IncrementalEdit::new(1, 1, "")), ("aé", IncrementalEdit::new(1, 1, "x"))];

        for (new_source, edit) in cases {
            let mut state = IncrementalState::new("a;");
            let result = catch_unwind(AssertUnwindSafe(|| state.reparse(new_source, &edit)));
            assert!(matches!(result, Ok(Err(_))));
        }

        let mut state = IncrementalState::new("a;");
        let error = must_some(state.reparse("a", &IncrementalEdit::new(1, 1, "x")).err());
        let message = error.to_string();
        assert!(message.contains("expected 3 bytes"));
        assert!(message.contains("got 1"));
    }

    #[test]
    fn replay_crossing_cached_suffix_is_detected() {
        let replayed = [Token::new(TokenKind::Identifier, "long", 10, 20)];
        let overlapping_suffix = [Token::new(TokenKind::Identifier, "suffix", 18, 24)];
        let adjacent_suffix = [Token::new(TokenKind::Identifier, "suffix", 20, 24)];

        assert!(replay_crosses_cached_suffix(&replayed, &overlapping_suffix));
        assert!(!replay_crosses_cached_suffix(&replayed, &adjacent_suffix));
    }

    #[test]
    fn replay_preserves_parser_diagnostics() {
        let source = "my $value = 1;\n".to_owned();
        let mut state = IncrementalState::new(&source);
        let start = must_some(source.find("1"));
        let new_source = source.replacen("1", "", 1);
        let edit = IncrementalEdit::new(start, start + 1, "");

        let _ = must(state.reparse(&new_source, &edit));

        assert!(!state.diagnostics().is_empty());
        assert_eq!(state.metrics().fallback, Some(FallbackReason::RecoveryDiagnosticsUnstable));
        assert!(state.metrics().full_parse);
    }

    #[test]
    fn format_declaration_records_a_context_sensitive_fallback() {
        let source = "format REPORT =\nName: @<<<\n$x\n.\n";
        let mut state = IncrementalState::new(source);
        let start = must_some(source.find("Name"));
        let new_source = source.replacen("Name", "Value", 1);
        let edit = IncrementalEdit::new(start, start + 4, "Value");

        let _ = must(state.reparse(&new_source, &edit));

        assert_eq!(state.metrics().fallback, Some(FallbackReason::ContextSensitiveFormat));
        assert!(state.metrics().full_parse);
    }

    #[test]
    fn oversized_edit_records_a_full_parse_fallback() {
        let source = "my $value = 1;\n".to_owned();
        let mut state = IncrementalState::new(&source);
        let replacement = "x".repeat(MAX_INCREMENTAL_EDIT_BYTES + 1);
        let edit = IncrementalEdit::new(0, source.len(), replacement.clone());
        let new_source = replacement;

        let _ = must(state.reparse(&new_source, &edit));

        assert_eq!(state.metrics().fallback, Some(FallbackReason::EditTooLarge));
        assert!(state.metrics().full_parse);
        assert!(state.metrics().tokens_relexed > 0);
    }
}
