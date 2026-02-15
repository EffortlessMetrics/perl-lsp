//! Trivia-preserving parser implementation
//!
//! This module provides a parser that preserves comments and whitespace
//! by attaching them to AST nodes as leading/trailing trivia.

use crate::trivia::{NodeWithTrivia, Trivia, TriviaToken};
use perl_ast::v2::{Node, NodeIdGenerator, NodeKind};
use perl_lexer::{PerlLexer, Token, TokenType};
use perl_position_tracking::{Position, Range};
use std::collections::VecDeque;

/// Token with trivia information
#[derive(Debug, Clone)]
pub(crate) struct TokenWithTrivia {
    /// The actual token
    token: Token,
    /// Leading trivia (comments/whitespace before this token)
    leading_trivia: Vec<TriviaToken>,
    /// Token range
    range: Range,
}

/// Parser context that preserves trivia
pub struct TriviaParserContext {
    /// Source text
    _source: String,
    /// Tokens with trivia
    tokens: VecDeque<TokenWithTrivia>,
    /// Current token index
    current: usize,
    /// Node ID generator
    id_generator: NodeIdGenerator,
    /// Position tracker for accurate line/column info
    position_tracker: PositionTracker,
}

/// Tracks position in source for accurate line/column information
struct PositionTracker {
    /// Line start offsets
    line_starts: Vec<usize>,
}

impl PositionTracker {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }
        PositionTracker { line_starts }
    }

    fn offset_to_position(&self, offset: usize) -> Position {
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));
        let line_start = self.line_starts[line];
        let column = offset - line_start + 1;
        Position::new(offset, (line + 1) as u32, column as u32)
    }
}

impl TriviaParserContext {
    /// Create a new trivia-preserving parser context
    pub fn new(source: String) -> Self {
        let position_tracker = PositionTracker::new(&source);
        let mut tokens = VecDeque::new();

        // Custom tokenization that preserves trivia
        let mut position = 0;
        let _source_bytes = source.as_bytes();

        while position < source.len() {
            // Collect leading trivia
            let _trivia_start = position;
            let leading_trivia = Self::collect_trivia_at(&source, &mut position);

            if position >= source.len() {
                break;
            }

            // Get next meaningful token using the lexer
            let token_source = &source[position..];
            let mut lexer = PerlLexer::new(token_source);

            if let Some(token) = lexer.next_token() {
                // Skip EOF tokens
                if matches!(token.token_type, TokenType::EOF) {
                    break;
                }

                // Adjust token positions to be relative to the full source
                let adjusted_token = Token::new(
                    token.token_type.clone(),
                    token.text.clone(),
                    position + token.start,
                    position + token.end,
                );

                // Create range with proper line/column info
                let start_pos = position_tracker.offset_to_position(adjusted_token.start);
                let end_pos = position_tracker.offset_to_position(adjusted_token.end);
                let range = Range::new(start_pos, end_pos);

                tokens.push_back(TokenWithTrivia {
                    token: adjusted_token.clone(),
                    leading_trivia,
                    range,
                });

                // Advance position
                position = adjusted_token.end;
            } else {
                break;
            }
        }

        // Handle remaining trivia at EOF, or source that was entirely trivia
        if tokens.is_empty() || position < source.len() {
            let remaining_trivia = if position < source.len() {
                Self::collect_trivia_at(&source, &mut position)
            } else {
                Vec::new()
            };
            if !remaining_trivia.is_empty() || tokens.is_empty() {
                let trivia = if tokens.is_empty() {
                    // Source was entirely trivia — re-collect from start
                    let mut pos = 0;
                    Self::collect_trivia_at(&source, &mut pos)
                } else {
                    remaining_trivia
                };
                if !trivia.is_empty() {
                    let eof_pos = position_tracker.offset_to_position(source.len());
                    let eof_token =
                        Token::new(TokenType::EOF, String::new(), source.len(), source.len());
                    tokens.push_back(TokenWithTrivia {
                        token: eof_token,
                        leading_trivia: trivia,
                        range: Range::new(eof_pos, eof_pos),
                    });
                }
            }
        }

        TriviaParserContext {
            _source: source,
            tokens,
            current: 0,
            id_generator: NodeIdGenerator::new(),
            position_tracker,
        }
    }

    /// Collect trivia at the given position
    fn collect_trivia_at(source: &str, position: &mut usize) -> Vec<TriviaToken> {
        let mut trivia = Vec::new();
        let bytes = source.as_bytes();

        while *position < source.len() {
            let _start = *position;
            let ch = bytes[*position];

            match ch {
                // Whitespace
                b' ' | b'\t' | b'\r' => {
                    let ws_start = *position;
                    while *position < source.len()
                        && matches!(bytes[*position], b' ' | b'\t' | b'\r')
                    {
                        *position += 1;
                    }

                    let ws = &source[ws_start..*position];
                    trivia.push(TriviaToken::new(
                        Trivia::Whitespace(ws.to_string()),
                        Range::new(Position::new(ws_start, 0, 0), Position::new(*position, 0, 0)),
                    ));
                }

                // Newline
                b'\n' => {
                    trivia.push(TriviaToken::new(
                        Trivia::Newline,
                        Range::new(
                            Position::new(*position, 0, 0),
                            Position::new(*position + 1, 0, 0),
                        ),
                    ));
                    *position += 1;
                }

                // Comment
                b'#' => {
                    let comment_start = *position;
                    // Find end of line
                    while *position < source.len() && bytes[*position] != b'\n' {
                        *position += 1;
                    }

                    let comment = &source[comment_start..*position];
                    trivia.push(TriviaToken::new(
                        Trivia::LineComment(comment.to_string()),
                        Range::new(
                            Position::new(comment_start, 0, 0),
                            Position::new(*position, 0, 0),
                        ),
                    ));
                }

                // POD documentation
                b'=' if *position == 0 || (*position > 0 && bytes[*position - 1] == b'\n') => {
                    // Check if this starts a POD section
                    let remaining = &source[*position..];
                    if remaining.starts_with("=pod")
                        || remaining.starts_with("=head")
                        || remaining.starts_with("=over")
                        || remaining.starts_with("=item")
                        || remaining.starts_with("=back")
                        || remaining.starts_with("=begin")
                        || remaining.starts_with("=end")
                        || remaining.starts_with("=for")
                        || remaining.starts_with("=encoding")
                    {
                        let pod_start = *position;

                        // Edge case fix: Find =cut at start of line (including position 0 or after newline)
                        let mut found_cut = false;
                        while *position < source.len() {
                            // Check for =cut at the start of a line
                            if (*position == 0 || (*position > 0 && bytes[*position - 1] == b'\n'))
                                && source[*position..].starts_with("=cut")
                            {
                                *position += 4; // Skip "=cut"
                                // Skip to end of line
                                while *position < source.len() && bytes[*position] != b'\n' {
                                    *position += 1;
                                }
                                if *position < source.len() {
                                    *position += 1; // Skip newline
                                }
                                found_cut = true;
                                break;
                            }
                            *position += 1;
                        }

                        // Edge case fix: If no =cut found, POD extends to end of file
                        if !found_cut {
                            *position = source.len();
                        }

                        let pod = &source[pod_start..*position];
                        trivia.push(TriviaToken::new(
                            Trivia::PodComment(pod.to_string()),
                            Range::new(
                                Position::new(pod_start, 0, 0),
                                Position::new(*position, 0, 0),
                            ),
                        ));
                    } else {
                        // Not POD, this is a regular token
                        break;
                    }
                }

                // Non-trivia character
                _ => {
                    // Check for Unicode whitespace
                    if ch >= 128 {
                        let ch_str = &source[*position..];
                        if let Some(unicode_ch) = ch_str.chars().next() {
                            if unicode_ch.is_whitespace() {
                                let ch_len = unicode_ch.len_utf8();
                                trivia.push(TriviaToken::new(
                                    Trivia::Whitespace(unicode_ch.to_string()),
                                    Range::new(
                                        Position::new(*position, 0, 0),
                                        Position::new(*position + ch_len, 0, 0),
                                    ),
                                ));
                                *position += ch_len;
                                continue;
                            }
                        }
                    }

                    // Not trivia, stop collecting
                    break;
                }
            }
        }

        trivia
    }

    /// Get current token with trivia
    pub(crate) fn current_token(&self) -> Option<&TokenWithTrivia> {
        self.tokens.get(self.current)
    }

    /// Advance to next token
    pub(crate) fn advance(&mut self) -> Option<&TokenWithTrivia> {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
        self.current_token()
    }

    /// Check if at end of tokens
    pub fn is_eof(&self) -> bool {
        self.current >= self.tokens.len()
    }
}

/// Parser that preserves trivia
pub struct TriviaPreservingParser {
    context: TriviaParserContext,
}

impl TriviaPreservingParser {
    /// Create a new trivia-preserving parser
    pub fn new(source: String) -> Self {
        TriviaPreservingParser { context: TriviaParserContext::new(source) }
    }

    /// Parse the source, preserving trivia
    pub fn parse(mut self) -> NodeWithTrivia {
        let start_pos = Position::new(0, 1, 1);
        let mut statement_nodes = Vec::new();

        // Collect any leading trivia before first statement
        let mut leading_trivia = Vec::new();
        if let Some(first_token) = self.context.current_token() {
            leading_trivia = first_token.leading_trivia.clone();
        }

        // Parse statements
        while !self.context.is_eof() {
            if let Some(stmt) = self.parse_statement() {
                statement_nodes.push(stmt.node);
            }
        }

        let end_pos = if let Some(last_token) = self.context.tokens.back() {
            last_token.range.end
        } else {
            start_pos
        };

        let program = Node::new(
            self.context.id_generator.next_id(),
            NodeKind::Program { statements: statement_nodes },
            Range::new(start_pos, end_pos),
        );

        NodeWithTrivia { node: program, leading_trivia, trailing_trivia: Vec::new() }
    }

    /// Parse a statement with trivia
    fn parse_statement(&mut self) -> Option<NodeWithTrivia> {
        let (token, leading_trivia, _token_range) = {
            let token_with_trivia = self.context.current_token()?;
            (
                token_with_trivia.token.clone(),
                token_with_trivia.leading_trivia.clone(),
                token_with_trivia.range,
            )
        };

        // Simple demonstration: parse variable declarations
        match &token.token_type {
            TokenType::Keyword(kw) if matches!(kw.as_ref(), "my" | "our" | "local" | "state") => {
                let start_pos = self.context.position_tracker.offset_to_position(token.start);

                let declarator = kw.to_string();
                self.context.advance();

                // For demonstration, create a simple node
                let end_pos = self.context.position_tracker.offset_to_position(token.end);

                let node = Node::new(
                    self.context.id_generator.next_id(),
                    NodeKind::Identifier { name: declarator },
                    Range::new(start_pos, end_pos),
                );

                // Skip to next statement for demo
                while !self.context.is_eof() {
                    if let Some(t) = self.context.current_token() {
                        if matches!(t.token.token_type, TokenType::Semicolon) {
                            self.context.advance();
                            break;
                        }
                    }
                    self.context.advance();
                }

                Some(NodeWithTrivia { node, leading_trivia, trailing_trivia: Vec::new() })
            }
            _ => {
                // Skip unknown tokens for now
                self.context.advance();
                None
            }
        }
    }
}

/// Format an AST with trivia back to source code
pub fn format_with_trivia(node: &NodeWithTrivia) -> String {
    let mut result = String::new();

    // Add leading trivia
    for trivia in &node.leading_trivia {
        result.push_str(trivia.trivia.as_str());
    }

    // Add node content (simplified)
    result.push_str(&format!("{:?}", node.node.kind));

    // Add trailing trivia
    for trivia in &node.trailing_trivia {
        result.push_str(trivia.trivia.as_str());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use perl_tdd_support::must_some;

    #[test]
    fn test_trivia_preservation() {
        let source = r#"#!/usr/bin/perl
# This is a comment
  
my $x = 42;  # end of line comment

=pod
This is POD documentation
=cut

our $y;"#
            .to_string();

        let parser = TriviaPreservingParser::new(source);
        let result = parser.parse();

        // Check that we have leading trivia
        assert!(!result.leading_trivia.is_empty());

        // First trivia should be the shebang comment
        assert!(matches!(
            &result.leading_trivia[0].trivia,
            Trivia::LineComment(s) if s.starts_with("#!/usr/bin/perl")
        ));
    }

    #[test]
    fn test_whitespace_preservation() {
        let source = "  \t  my $x;".to_string();
        let ctx = TriviaParserContext::new(source);

        let first_token = must_some(ctx.current_token());
        assert!(!first_token.leading_trivia.is_empty());
        assert!(matches!(
            &first_token.leading_trivia[0].trivia,
            Trivia::Whitespace(ws) if ws == "  \t  "
        ));
    }
}
