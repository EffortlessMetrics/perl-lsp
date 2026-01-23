//! Trivia (comments and whitespace) handling for the Perl parser
//!
//! This module provides support for preserving comments and whitespace
//! in the AST, which is essential for code formatting and refactoring tools.

use crate::{
    ast_v2::{Node, NodeKind},
    position::Range,
};
use perl_lexer::TokenType;

/// Trivia represents non-semantic tokens like comments and whitespace
#[derive(Debug, Clone, PartialEq)]
pub enum Trivia {
    /// Whitespace (spaces, tabs, etc.)
    Whitespace(String),
    /// Single-line comment starting with #
    LineComment(String),
    /// POD documentation
    PodComment(String),
    /// Newline character(s)
    Newline,
}

impl Trivia {
    /// Convert trivia to a string representation
    pub fn as_str(&self) -> &str {
        match self {
            Trivia::Whitespace(s) => s,
            Trivia::LineComment(s) => s,
            Trivia::PodComment(s) => s,
            Trivia::Newline => "\n",
        }
    }

    /// Get the display name for this trivia type
    pub fn kind_name(&self) -> &'static str {
        match self {
            Trivia::Whitespace(_) => "whitespace",
            Trivia::LineComment(_) => "comment",
            Trivia::PodComment(_) => "pod",
            Trivia::Newline => "newline",
        }
    }
}

/// A node with attached trivia
#[derive(Debug, Clone)]
pub struct NodeWithTrivia {
    /// The actual AST node
    pub node: Node,
    /// Trivia that appears before this node
    pub leading_trivia: Vec<TriviaToken>,
    /// Trivia that appears after this node
    pub trailing_trivia: Vec<TriviaToken>,
}

/// A trivia token with position information
#[derive(Debug, Clone)]
pub struct TriviaToken {
    /// The trivia content
    pub trivia: Trivia,
    /// The source range of this trivia
    pub range: Range,
}

impl TriviaToken {
    /// Create a new trivia token with the given content and range
    pub fn new(trivia: Trivia, range: Range) -> Self {
        TriviaToken { trivia, range }
    }
}

/// Extension trait for collecting trivia.
///
/// Implement this trait to collect leading and trailing trivia during lexing.
pub trait TriviaCollector {
    /// Collect trivia tokens before the next meaningful token
    fn collect_leading_trivia(&mut self) -> Vec<TriviaToken>;

    /// Collect trivia tokens after a node (typically until newline)
    fn collect_trailing_trivia(&mut self) -> Vec<TriviaToken>;
}

/// A lexer wrapper that preserves trivia.
///
/// Wraps the Perl lexer to collect comments and whitespace as trivia tokens.
pub struct TriviaLexer {
    /// The underlying Perl lexer
    lexer: perl_lexer::PerlLexer<'static>,
    /// Source code (owned)
    source: String,
    /// Current position for trivia tracking
    position: usize,
    /// Buffered trivia tokens
    _trivia_buffer: Vec<TriviaToken>,
}

impl TriviaLexer {
    /// Create a new trivia-preserving lexer
    pub fn new(source: String) -> Self {
        // We need to leak the string to get a 'static reference
        // In a real implementation, we'd use a better lifetime strategy
        let source_ref: &'static str = Box::leak(source.clone().into_boxed_str());

        TriviaLexer {
            lexer: perl_lexer::PerlLexer::new(source_ref),
            source,
            position: 0,
            _trivia_buffer: Vec::new(),
        }
    }

    /// Get the next token, collecting any preceding trivia.
    ///
    /// Returns the token along with any whitespace or comments that precede it.
    pub fn next_token_with_trivia(&mut self) -> Option<(perl_lexer::Token, Vec<TriviaToken>)> {
        // First, collect any trivia
        let trivia = self.collect_trivia();

        // Then get the next meaningful token
        let token = self.lexer.next_token()?;

        // Skip EOF tokens
        if matches!(token.token_type, TokenType::EOF) {
            return None;
        }

        Some((token, trivia))
    }

    /// Collect trivia tokens at current position
    fn collect_trivia(&mut self) -> Vec<TriviaToken> {
        let mut trivia = Vec::new();

        while self.position < self.source.len() {
            let remaining = &self.source[self.position..];

            // Check for whitespace
            if let Some(ws_len) = self.whitespace_length(remaining) {
                let ws = &remaining[..ws_len];
                let start = self.position;
                let end = start + ws_len;

                // Check if it's just newlines
                if ws.chars().all(|c| c == '\n' || c == '\r') {
                    trivia.push(TriviaToken::new(
                        Trivia::Newline,
                        Range::new(
                            crate::position::Position::new(start, 0, 0),
                            crate::position::Position::new(end, 0, 0),
                        ),
                    ));
                } else {
                    trivia.push(TriviaToken::new(
                        Trivia::Whitespace(ws.to_string()),
                        Range::new(
                            crate::position::Position::new(start, 0, 0),
                            crate::position::Position::new(end, 0, 0),
                        ),
                    ));
                }

                self.position += ws_len;
                continue;
            }

            // Check for comments
            if remaining.starts_with('#') {
                let comment_end = remaining.find('\n').unwrap_or(remaining.len());
                let comment = &remaining[..comment_end];
                let start = self.position;
                let end = start + comment_end;

                trivia.push(TriviaToken::new(
                    Trivia::LineComment(comment.to_string()),
                    Range::new(
                        crate::position::Position::new(start, 0, 0),
                        crate::position::Position::new(end, 0, 0),
                    ),
                ));

                self.position += comment_end;
                continue;
            }

            // Check for POD
            if remaining.starts_with("=")
                && (self.position == 0 || self.source.as_bytes()[self.position - 1] == b'\n')
            {
                if let Some(pod_end) = self.find_pod_end(remaining) {
                    let pod = &remaining[..pod_end];
                    let start = self.position;
                    let end = start + pod_end;

                    trivia.push(TriviaToken::new(
                        Trivia::PodComment(pod.to_string()),
                        Range::new(
                            crate::position::Position::new(start, 0, 0),
                            crate::position::Position::new(end, 0, 0),
                        ),
                    ));

                    self.position += pod_end;
                    continue;
                }
            }

            // No more trivia
            break;
        }

        // Sync lexer position
        if self.position > 0 {
            // The lexer will skip whitespace internally, so we need to ensure
            // our position tracking stays in sync
        }

        trivia
    }

    /// Calculate the length of whitespace at the start of the string
    fn whitespace_length(&self, s: &str) -> Option<usize> {
        let mut len = 0;
        for ch in s.chars() {
            if ch.is_whitespace() && ch != '\n' && ch != '\r' {
                len += ch.len_utf8();
            } else if ch == '\n' || ch == '\r' {
                // Handle newlines separately
                len += ch.len_utf8();
                // Handle \r\n
                if ch == '\r' && s[len..].starts_with('\n') {
                    len += 1;
                }
                break;
            } else {
                break;
            }
        }

        if len > 0 { Some(len) } else { None }
    }

    /// Find the end of a POD section
    fn find_pod_end(&self, s: &str) -> Option<usize> {
        // POD ends with =cut at the beginning of a line
        let mut pos = 0;
        for line in s.lines() {
            if line.trim() == "=cut" {
                return Some(pos + line.len());
            }
            pos += line.len() + 1; // +1 for newline
        }

        // If no =cut found, POD extends to end of string
        Some(s.len())
    }
}

/// Parser that preserves trivia.
///
/// A parser that attaches comments and whitespace to AST nodes for formatting.
pub struct TriviaPreservingParser {
    /// Trivia-aware lexer
    lexer: TriviaLexer,
    /// Current lookahead token
    current: Option<(perl_lexer::Token, Vec<TriviaToken>)>,
    /// Node ID generator
    id_generator: crate::ast_v2::NodeIdGenerator,
}

impl TriviaPreservingParser {
    /// Create a new trivia-preserving parser
    pub fn new(source: String) -> Self {
        let mut parser = TriviaPreservingParser {
            lexer: TriviaLexer::new(source),
            current: None,
            id_generator: crate::ast_v2::NodeIdGenerator::new(),
        };
        // Prime the lookahead
        parser.advance();
        parser
    }

    /// Advance to the next token
    fn advance(&mut self) {
        self.current = self.lexer.next_token_with_trivia();
    }

    /// Parse and return AST with trivia preserved.
    ///
    /// Returns a node with leading and trailing trivia attached.
    pub fn parse(mut self) -> NodeWithTrivia {
        let leading_trivia =
            if let Some((_, trivia)) = &self.current { trivia.clone() } else { Vec::new() };

        // For now, create a simple demonstration node
        let node = Node::new(
            self.id_generator.next_id(),
            NodeKind::Program { statements: Vec::new() },
            Range::new(
                crate::position::Position::new(0, 1, 1),
                crate::position::Position::new(0, 1, 1),
            ),
        );

        NodeWithTrivia { node, leading_trivia, trailing_trivia: Vec::new() }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_trivia_collection() {
        let source = "  # comment\n  my $x = 42;".to_string();
        let mut lexer = TriviaLexer::new(source);

        let (_token, trivia) = lexer.next_token_with_trivia().unwrap();

        // Should have whitespace and comment as trivia
        eprintln!("Trivia count: {}", trivia.len());
        for (i, t) in trivia.iter().enumerate() {
            eprintln!("Trivia[{}]: {:?}", i, t.trivia);
        }
        assert!(trivia.len() >= 2); // At least whitespace and comment
        assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::Whitespace(_))));
        assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::LineComment(_))));
    }

    #[test]
    fn test_pod_preservation() {
        let source = "=head1 NAME\n\nTest\n\n=cut\n\nmy $x;".to_string();
        let mut lexer = TriviaLexer::new(source);

        let (_, trivia) = lexer.next_token_with_trivia().unwrap();

        // Should have POD as trivia
        assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_))));
    }
}
