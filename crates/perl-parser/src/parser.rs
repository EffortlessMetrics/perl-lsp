//! Main Perl parser implementation for Perl parsing workflow pipeline
//!
//! This module implements a high-performance recursive descent parser with operator precedence
//! handling that consumes tokens from perl-lexer and produces comprehensive ASTs for email
//! script analysis throughout the Parse → Index → Navigate → Complete → Analyze workflow.
//!
//! # LSP Workflow Integration
//!
//! The parser serves as the entry point for the Parse stage, converting raw Perl script
//! content into structured ASTs that flow through subsequent pipeline stages:
//!
//! - **Extract**: Parses Perl scripts embedded in PST Perl code
//! - **Normalize**: Provides AST foundation for standardization transformations
//! - **Thread**: Enables control flow and dependency analysis across Perl scripts
//! - **Render**: Supports AST-to-source reconstruction with formatting preservation
//! - **Index**: Facilitates symbol extraction and searchable metadata generation
//!
//! # Performance Characteristics
//!
//! Optimized for enterprise-scale Perl parsing:
//! - Handles 50GB+ Perl files with efficient memory management
//! - Recursive descent with configurable depth limits for safety
//! - Token stream abstraction minimizes memory allocation during parsing
//! - Error recovery enables continued processing of malformed Perl scripts
//!
//! # Usage Example
//!
//! ```rust
//! use perl_parser::Parser;
//!
//! let mut parser = Parser::new("my $var = 42; sub hello { print $var; }");
//! match parser.parse() {
//!     Ok(ast) => {
//!         // AST ready for LSP workflow processing
//!         println!("Parsed Perl script: {}", ast.to_sexp());
//!     }
//!     Err(e) => {
//!         // Handle parsing errors with recovery strategies
//!         eprintln!("Parse error in Perl script: {}", e);
//!     }
//! }
//! ```

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    error::{ParseError, ParseResult},
    heredoc_collector::{self, HeredocContent, PendingHeredoc, collect_all},
    quote_parser,
    token_stream::{Token, TokenKind, TokenStream},
};
use std::collections::VecDeque;
use std::sync::Arc;

/// High-performance Perl parser for Perl script analysis within LSP workflow
///
/// The parser processes Perl script content through recursive descent parsing with
/// operator precedence handling, producing comprehensive ASTs suitable for analysis
/// across all LSP workflow stages. Designed for enterprise-scale performance with
/// 50GB+ Perl file processing capabilities.
///
/// # Email Processing Context
///
/// This parser specializes in handling Perl scripts commonly found in Perl code:
/// - Email filtering and routing scripts
/// - Message processing automation code
/// - Configuration and setup scripts embedded in emails
/// - Inline Perl code within email templates and forms
///
/// # Performance Features
///
/// - Configurable recursion depth limits prevent stack overflow on malformed content
/// - Token stream abstraction minimizes memory allocation during large file processing
/// - Error recovery strategies maintain parsing progress despite syntax issues
/// - Position tracking enables precise error reporting for debugging complex Perl scripts
pub struct Parser<'a> {
    /// Token stream providing access to lexed Perl script content
    tokens: TokenStream<'a>,
    /// Current recursion depth for overflow protection during complex Perl script parsing
    recursion_depth: usize,
    /// Position tracking for error reporting and AST location information
    last_end_position: usize,
    /// Context flag for disambiguating for-loop initialization syntax
    in_for_loop_init: bool,
    /// Statement boundary tracking for indirect object syntax detection
    at_stmt_start: bool,
    /// FIFO queue of pending heredoc declarations awaiting content collection
    pending_heredocs: VecDeque<PendingHeredoc>,
    /// Source bytes for heredoc content collection (shared with token stream)
    src_bytes: &'a [u8],
    /// Byte cursor tracking position for heredoc content collection
    byte_cursor: usize,
}

const MAX_RECURSION_DEPTH: usize = 500;

/// Advance byte offset to just after the next line break (handles \n and \r\n)
fn after_line_break(src: &[u8], mut off: usize) -> usize {
    // Skip to newline if in middle of line
    while off < src.len() && src[off] != b'\n' && src[off] != b'\r' {
        off += 1;
    }
    if off < src.len() {
        if src[off] == b'\r' {
            off += 1;
            if off < src.len() && src[off] == b'\n' {
                off += 1;
            }
        } else if src[off] == b'\n' {
            off += 1;
        }
    }
    off
}

impl<'a> Parser<'a> {
    /// Create a new parser for processing Perl script content within LSP workflow
    ///
    /// # Arguments
    ///
    /// * `input` - Email script source code to be parsed during Parse stage
    ///
    /// # Returns
    ///
    /// A configured parser ready for Perl script analysis with optimal settings
    /// for enterprise-scale Perl codebase processing workflows.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::Parser;
    ///
    /// let script = "use strict; my $filter = qr/important/;";
    /// let mut parser = Parser::new(script);
    /// // Parser ready for LSP workflow processing
    /// ```
    pub fn new(input: &'a str) -> Self {
        Parser {
            tokens: TokenStream::new(input),
            recursion_depth: 0,
            last_end_position: 0,
            in_for_loop_init: false,
            at_stmt_start: true,
            pending_heredocs: VecDeque::new(),
            src_bytes: input.as_bytes(),
            byte_cursor: 0,
        }
    }

    /// Parse Perl script content and return comprehensive AST for LSP workflow processing
    ///
    /// This method performs complete parsing of Perl script content, producing an AST
    /// suitable for analysis throughout the Parse → Index → Navigate → Complete → Analyze
    /// pipeline stages. Designed for robust processing of complex Perl scripts found
    /// in enterprise Perl files.
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - Successfully parsed AST with Program root node containing all statements
    /// * `Err(ParseError)` - Parsing failure with detailed error context for recovery strategies
    ///
    /// # Errors
    ///
    /// Returns `ParseError` when:
    /// - Email script syntax is malformed or incomplete
    /// - Unexpected end of input during parsing
    /// - Recursion depth limit exceeded (protects against deeply nested structures)
    /// - Invalid token sequences that cannot be recovered from
    ///
    /// Recovery strategy: Use error classifier to categorize failures and apply
    /// appropriate fallback parsing strategies for continued Perl parsing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::Parser;
    ///
    /// let mut parser = Parser::new("my $email_count = scalar(@emails);");
    /// match parser.parse() {
    ///     Ok(ast) => {
    ///         // AST ready for LSP workflow stages
    ///         assert!(matches!(ast.kind, perl_parser::NodeKind::Program { .. }));
    ///     }
    ///     Err(e) => {
    ///         // Handle parsing errors with appropriate recovery
    ///         eprintln!("Email script parsing failed: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Email Processing Context
    ///
    /// This method is optimized for parsing Perl scripts commonly found in email environments:
    /// - Email filtering and routing logic
    /// - Message processing automation scripts
    /// - Configuration scripts embedded in Perl code
    /// - Template processing code within email systems
    pub fn parse(&mut self) -> ParseResult<Node> {
        self.parse_program()
    }

    // Helper functions for cleaner pattern matching

    #[inline]
    fn is_statement_terminator(kind: Option<TokenKind>) -> bool {
        matches!(kind, Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None)
    }

    #[inline]
    fn is_stmt_modifier_kind(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::If
                | TokenKind::Unless
                | TokenKind::While
                | TokenKind::Until
                | TokenKind::For
                | TokenKind::When
                | TokenKind::Foreach
        )
    }

    #[inline]
    fn is_logical_or(kind: Option<TokenKind>) -> bool {
        matches!(kind, Some(TokenKind::Or) | Some(TokenKind::DefinedOr))
    }

    #[inline]
    fn is_postfix_op(kind: Option<TokenKind>) -> bool {
        matches!(kind, Some(TokenKind::Increment) | Some(TokenKind::Decrement))
    }

    #[inline]
    fn is_variable_sigil(kind: Option<TokenKind>) -> bool {
        matches!(
            kind,
            Some(TokenKind::ScalarSigil) | Some(TokenKind::ArraySigil) | Some(TokenKind::HashSigil)
        )
    }

    /// Check recursion depth with optimized hot path
    #[inline(always)]
    fn check_recursion(&mut self) -> ParseResult<()> {
        self.recursion_depth += 1;
        // Fast path: avoid expensive comparisons in the common case
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            return Err(ParseError::RecursionLimit);
        }
        Ok(())
    }

    fn exit_recursion(&mut self) {
        self.recursion_depth = self.recursion_depth.saturating_sub(1);
    }

    // ——— Heredoc collector integration helpers (Sprint A Day 4) ———

    /// Enqueue a heredoc declaration for later content collection
    fn push_heredoc_decl(
        &mut self,
        label: String,
        allow_indent: bool,
        quote: heredoc_collector::QuoteKind,
        decl_start: usize,
        decl_end: usize,
    ) {
        self.pending_heredocs.push_back(PendingHeredoc {
            label: Arc::from(label.as_str()),
            allow_indent,
            quote,
            decl_span: heredoc_collector::Span { start: decl_start, end: decl_end },
        });
    }

    /// Drain all pending heredocs after statement completion (FIFO order)
    fn drain_pending_heredocs(&mut self, root: &mut Node) {
        if self.pending_heredocs.is_empty() {
            return;
        }
        // Advance to first content line (handle newline after statement terminator)
        self.byte_cursor = after_line_break(self.src_bytes, self.byte_cursor);

        // Keep a copy of the declarations so we can match outputs back to inputs
        let pending: Vec<_> = self.pending_heredocs.iter().cloned().collect();

        let out = collect_all(
            self.src_bytes,
            self.byte_cursor,
            std::mem::take(&mut self.pending_heredocs),
        );

        // Zip 1:1 in order (collector preserves input order)
        for (decl, body) in pending.into_iter().zip(out.contents.into_iter()) {
            let _attached = self.try_attach_heredoc_at_node(root, decl.decl_span, &body);

            // Defensive guardrail: warn if heredoc node wasn't found at expected span
            #[cfg(debug_assertions)]
            if !_attached {
                eprintln!(
                    "[WARNING] drain_pending_heredocs: Failed to attach heredoc content at span {}..{} - no matching Heredoc node found in AST",
                    decl.decl_span.start, decl.decl_span.end
                );
            }
        }
        self.byte_cursor = out.next_offset;
    }

    /// Attach collected heredoc content to its declaration node by matching declaration span
    /// Returns true if a matching Heredoc node was found and updated, false otherwise
    fn try_attach_heredoc_at_node(
        &self,
        root: &mut Node,
        decl_span: heredoc_collector::Span,
        body: &HeredocContent,
    ) -> bool {
        // Depth-first search for the Heredoc node with matching declaration span
        self.try_attach_at_node(root, decl_span, body)
    }

    /// Try to attach heredoc content at this node or its children
    fn try_attach_at_node(
        &self,
        node: &mut Node,
        decl_span: heredoc_collector::Span,
        body: &HeredocContent,
    ) -> bool {
        // Check if this node's span matches the declaration span
        let node_matches =
            node.location.start == decl_span.start && node.location.end == decl_span.end;

        if node_matches {
            // Try to attach at this node
            if let NodeKind::Heredoc { content, .. } = &mut node.kind {
                // Reify the body bytes from src_bytes using the collector's segments
                let mut s = String::new();
                for (i, seg) in body.segments.iter().enumerate() {
                    if seg.end > seg.start {
                        let bytes = &self.src_bytes[seg.start..seg.end];
                        // Source is valid UTF-8 (enforced by lexer)
                        s.push_str(std::str::from_utf8(bytes).unwrap_or_default());
                    }
                    if i + 1 < body.segments.len() {
                        // Normalize line breaks for AST convenience
                        s.push('\n');
                    }
                }
                *content = s;
                return true;
            }
        }

        // Recursively search children (DFS) using for_each_child_mut
        let mut found = false;
        node.for_each_child_mut(|child| {
            if !found && self.try_attach_at_node(child, decl_span, body) {
                found = true;
            }
        });

        #[cfg(debug_assertions)]
        if !found && node_matches {
            eprintln!(
                "warn: no Heredoc node found for decl span {}..{} (matched span but not Heredoc kind)",
                decl_span.start, decl_span.end
            );
        }

        found
    }

    /// Parse a complete program
    fn parse_program(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let mut statements = Vec::new();

        while !self.tokens.is_eof() {
            // Check for UnknownRest token (lexer budget exceeded)
            if matches!(self.peek_kind(), Some(TokenKind::UnknownRest)) {
                let t = self.consume_token()?;
                statements.push(Node::new(
                    NodeKind::UnknownRest,
                    SourceLocation { start: t.start, end: t.end },
                ));
                break; // Stop parsing but preserve earlier nodes
            }

            statements.push(self.parse_statement()?);
        }

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Program { statements }, SourceLocation { start, end }))
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let result = self.parse_statement_inner();
        self.exit_recursion();
        result
    }

    fn parse_statement_inner(&mut self) -> ParseResult<Node> {
        // Every new statement begins here
        self.at_stmt_start = true;

        let token = self.tokens.peek()?.clone();

        // Don't check for labels here - it breaks regular identifier parsing
        // Labels will be handled differently

        let mut stmt = match token.kind {
            // Empty statement (lone semicolon) - just consume and return a no-op
            TokenKind::Semicolon => {
                let pos = self.current_position();
                self.consume_token()?;
                // Return an empty block as a no-op placeholder
                return Ok(Node::new(
                    NodeKind::Block { statements: vec![] },
                    SourceLocation { start: pos, end: pos },
                ));
            }

            // Variable declarations
            TokenKind::My | TokenKind::Our | TokenKind::State => self.parse_variable_declaration(),
            TokenKind::Local => self.parse_local_statement(),

            // Control flow
            TokenKind::If => self.parse_if_statement(),
            TokenKind::Unless => self.parse_unless_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Until => self.parse_until_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Foreach => self.parse_foreach_statement(),
            TokenKind::Given => self.parse_given_statement(),
            TokenKind::Default => self.parse_default_statement(),
            TokenKind::Try => self.parse_try(),

            // Subroutines and modern OOP
            TokenKind::Sub => {
                let sub_node = self.parse_subroutine()?;
                // Check if this is an anonymous subroutine
                Ok(if let NodeKind::Subroutine { name, .. } = &sub_node.kind {
                    if name.is_none() {
                        // Wrap anonymous subroutines in expression statements
                        let location = sub_node.location;
                        Node::new(
                            NodeKind::ExpressionStatement { expression: Box::new(sub_node) },
                            location,
                        )
                    } else {
                        // Named subroutines are statements by themselves
                        sub_node
                    }
                } else {
                    // Shouldn't happen, but return as-is
                    sub_node
                })
            }
            TokenKind::Class => self.parse_class(),
            TokenKind::Method => self.parse_method(),

            // Package management
            TokenKind::Package => self.parse_package(),
            TokenKind::Use => self.parse_use(),
            TokenKind::No => self.parse_no(),

            // Format declarations
            TokenKind::Format => self.parse_format(),

            // Phase blocks
            TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck => self.parse_phase_block(),

            // Data sections
            TokenKind::DataMarker => self.parse_data_section(),

            // Return statement
            TokenKind::Return => self.parse_return(),

            // Block
            TokenKind::LeftBrace => self.parse_block(),

            // Expression-ish statement
            _ => {
                // Check if this might be a labeled statement
                if self.is_label_start() {
                    return self.parse_labeled_statement();
                }

                // Either build via indirect-object path or the normal expression path
                if let TokenKind::Identifier = token.kind {
                    if self.is_indirect_call_pattern(&token.text) {
                        // Parse indirect call but DON'T return early - let it go through
                        // the same modifier/semicolon handling as other statements
                        self.parse_indirect_call()
                    } else {
                        self.parse_expression_statement()
                    }
                } else {
                    self.parse_expression_statement()
                }
            }
        }?;

        // Check for statement modifiers on ANY statement
        if matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k)) {
            stmt = self.parse_statement_modifier(stmt)?;
        }

        // Check for optional semicolon
        // Don't use peek_fresh_kind() here as it can cause issues with nested blocks
        if self.peek_kind() == Some(TokenKind::Semicolon) {
            let semi_token = self.consume_token()?;
            // Track cursor after semicolon for heredoc content collection
            self.byte_cursor = semi_token.end;
        }

        // Drain pending heredocs after statement completion (Sprint A Day 5 - with AST attachment)
        self.drain_pending_heredocs(&mut stmt);

        Ok(stmt)
    }

    /// Check if this might be an indirect call pattern
    /// We only consider this at statement start to avoid ambiguous mid-expression cases.
    ///
    /// Note: When this is called, the parser has peeked at the function name (e.g., "print")
    /// but not consumed it. So:
    /// - peek() returns the function name (current position)
    /// - peek_second() returns the token after the function name
    /// - peek_third() returns two tokens after the function name
    fn is_indirect_call_pattern(&mut self, name: &str) -> bool {
        // Only check for indirect objects at statement start to avoid false positives
        // in contexts like: my $x = 1; if (1) { print $x; }
        if !self.at_stmt_start {
            return false;
        }

        // print "string" should not be treated as indirect object syntax
        // Note: peek_second() gets the token after "print" since peek() is "print"
        if name == "print" {
            if let Ok(next) = self.tokens.peek_second() {
                if next.kind == TokenKind::String {
                    return false;
                }
            }
        }

        // Known builtins that commonly use indirect object syntax
        let indirect_builtins = [
            "print", "printf", "say", "open", "close", "pipe", "sysopen", "sysread", "syswrite",
            "truncate", "fcntl", "ioctl", "flock", "seek", "tell", "select", "binmode", "exec",
            "system",
        ];

        // Check if it's a known builtin
        if indirect_builtins.contains(&name) {
            // Peek at the token AFTER the function name (use peek_second since peek is the function name)
            let (next_kind, next_text) = if let Ok(next) = self.tokens.peek_second() {
                (next.kind, next.text.clone())
            } else {
                return false;
            };

            // These tokens *cannot* start an indirect object
            match next_kind {
                TokenKind::Semicolon
                | TokenKind::RightBrace
                | TokenKind::RightParen
                | TokenKind::Comma
                | TokenKind::Eof => return false,
                _ => {}
            }

            // Check for print $fh $x pattern first (variable followed by another arg)
            // This must be checked before the STDOUT pattern because $fh is also an Identifier
            if next_text.starts_with('$') {
                // Only treat $var as an indirect object if a typical argument follows
                // without a comma. A comma means it's a regular argument list.
                // This prevents misclassifying `print $x, $y` as indirect object.
                // Use peek_third() to look at the token after $fh
                if let Ok(third) = self.tokens.peek_third() {
                    // A comma after $fh means regular argument list, NOT indirect object
                    // e.g., print $x, $y; is print both to STDOUT
                    if third.kind == TokenKind::Comma {
                        return false;
                    }

                    // Allow classic argument starts and sigiled variables ($x, @arr, %hash)
                    let third_text = third.text.as_str();
                    return matches!(
                        third.kind,
                        TokenKind::String       // print $fh "x"
                        | TokenKind::LeftParen    // print $fh ($x)
                        | TokenKind::LeftBracket  // print $fh [$x]
                        | TokenKind::LeftBrace    // print $fh { ... }
                    ) || third_text.starts_with('$')    // print $fh $x
                      || third_text.starts_with('@')    // print $fh @array
                      || third_text.starts_with('%'); // print $fh %hash
                }
                return false; // Can't see more; be conservative
            }

            // print STDOUT ... (uppercase bareword filehandle)
            if next_kind == TokenKind::Identifier {
                if next_text.chars().next().is_some_and(|c| c.is_uppercase()) {
                    return true;
                }
            }
        }

        // Check for "new ClassName" pattern
        if name == "new" {
            // peek_second() gets the token after "new"
            if let Ok(next) = self.tokens.peek_second() {
                if let TokenKind::Identifier = next.kind {
                    // Uppercase identifier after "new" suggests constructor
                    if next.text.chars().next().is_some_and(|c| c.is_uppercase()) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Mark that we're no longer at statement start (called after consuming statement head)
    fn mark_not_stmt_start(&mut self) {
        self.at_stmt_start = false;
    }

    /// Parse indirect object/method call
    fn parse_indirect_call(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let method_token = self.consume_token()?; // consume method name
        let method = method_token.text.clone();

        // We're consuming the function name, no longer at statement start
        self.mark_not_stmt_start();

        // Parse the object/filehandle
        let object = self.parse_primary()?;

        // Parse remaining arguments
        let mut args = vec![];

        // Continue parsing arguments until we hit a statement terminator
        while !Self::is_statement_terminator(self.peek_kind())
            && !self.is_statement_modifier_keyword()
        {
            args.push(self.parse_expression()?);

            // Check if we should continue (comma is optional in indirect syntax)
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
            } else if Self::is_statement_terminator(self.peek_kind())
                || self.is_statement_modifier_keyword()
            {
                break;
            }
        }

        let end = self.previous_position();

        // Return as an indirect call node (using MethodCall with a flag or separate node)
        Ok(Node::new(
            NodeKind::IndirectCall { method, object: Box::new(object), args },
            SourceLocation { start, end },
        ))
    }

    /// Check if current token is a statement modifier keyword
    fn is_statement_modifier_keyword(&mut self) -> bool {
        matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k))
    }

    /// Parse variable declaration (my, our, local, state)
    fn parse_variable_declaration(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?;
        let declarator = declarator_token.text.clone();

        // Check if we have a list declaration like `my ($x, $y)`
        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            let mut variables = Vec::new();

            // Parse comma-separated list of variables with their individual attributes
            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                let var = self.parse_variable()?;

                // Parse optional attributes for this specific variable
                let mut var_attributes = Vec::new();
                while self.peek_kind() == Some(TokenKind::Colon) {
                    self.tokens.next()?; // consume colon
                    let attr_token = self.expect(TokenKind::Identifier)?;
                    var_attributes.push(attr_token.text.clone());
                }

                // Create a node that includes both the variable and its attributes
                let var_with_attrs = if var_attributes.is_empty() {
                    var
                } else {
                    let start = var.location.start;
                    let end = self.previous_position();
                    Node::new(
                        NodeKind::VariableWithAttributes {
                            variable: Box::new(var),
                            attributes: var_attributes,
                        },
                        SourceLocation { start, end },
                    )
                };

                variables.push(var_with_attrs);

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis in variable list",
                        self.current_position(),
                    ));
                }
            }

            self.expect(TokenKind::RightParen)?; // consume )

            // No longer parse attributes here - they're parsed per variable above
            let attributes = Vec::new();

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                self.tokens.next()?; // consume =
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            // Don't consume semicolon here - let parse_statement handle it uniformly

            let end = self.previous_position();
            let node = Node::new(
                NodeKind::VariableListDeclaration {
                    declarator,
                    variables,
                    attributes,
                    initializer,
                },
                SourceLocation { start, end },
            );
            Ok(node)
        } else {
            // Single variable declaration
            // For 'local', we need to parse lvalue expressions (not just simple variables)
            // because local can take complex forms like local $ENV{PATH}
            let variable = if declarator == "local" {
                // For local, parse a general lvalue expression
                self.parse_assignment()?
            } else {
                // For my/our/state, parse a simple variable
                self.parse_variable()?
            };

            // Parse optional attributes
            let mut attributes = Vec::new();
            while self.peek_kind() == Some(TokenKind::Colon) {
                self.tokens.next()?; // consume colon
                let attr_token = self.expect(TokenKind::Identifier)?;
                attributes.push(attr_token.text.clone());
            }

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                self.tokens.next()?; // consume =
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            // Don't consume semicolon here - let parse_statement handle it uniformly

            let end = self.previous_position();
            let node = Node::new(
                NodeKind::VariableDeclaration {
                    declarator,
                    variable: Box::new(variable),
                    attributes,
                    initializer,
                },
                SourceLocation { start, end },
            );
            Ok(node)
        }
    }

    /// Parse local statement (can localize any lvalue, not just simple variables)
    fn parse_local_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?; // consume 'local'
        let declarator = declarator_token.text.clone();

        // Parse the lvalue expression that's being localized
        let variable = Box::new(self.parse_expression()?);

        let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let end = self.previous_position();
        let node = Node::new(
            NodeKind::VariableDeclaration {
                declarator,
                variable,
                attributes: Vec::new(),
                initializer,
            },
            SourceLocation { start, end },
        );
        Ok(node)
    }

    /// Parse a variable ($foo, @bar, %baz)
    fn parse_variable(&mut self) -> ParseResult<Node> {
        let token = self.consume_token()?;

        // The lexer returns variables as identifiers like "$x", "@array", etc.
        // We need to split the sigil from the name
        let text = &token.text;

        // Special handling for @{ and %{ (array/hash dereference)
        if text == "@{" || text == "%{" {
            let sigil = text
                .chars()
                .next()
                .ok_or_else(|| {
                    ParseError::syntax("Empty token text for array/hash dereference", token.start)
                })?
                .to_string();
            let start = token.start;

            // Parse the expression inside the braces
            let expr = self.parse_expression()?;

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(expr) },
                SourceLocation { start, end },
            ));
        }

        let (sigil, name) = if let Some(rest) = text.strip_prefix('$') {
            ("$".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('@') {
            ("@".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('%') {
            ("%".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('&') {
            ("&".to_string(), rest.to_string())
        } else if text.starts_with('*') && text.len() > 1 {
            let rest = &text[1..];
            ("*".to_string(), rest.to_string())
        } else {
            return Err(ParseError::syntax(
                format!("Expected variable, found '{}'", text),
                token.start,
            ));
        };

        // Check if the variable name is followed by :: for package-qualified variables
        let mut full_name = name;
        let mut end = token.end;

        // Handle :: in package-qualified variables
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?; // consume ::
            full_name.push_str("::");

            // The next part might be an identifier or another variable
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let name_token = self.tokens.next()?;
                full_name.push_str(&name_token.text);
                end = name_token.end;
            } else {
                // Handle cases like $Foo::$bar
                return Err(ParseError::syntax(
                    "Expected identifier after :: in package-qualified variable",
                    self.current_position(),
                ));
            }
        }

        Ok(Node::new(
            NodeKind::Variable { sigil, name: full_name },
            SourceLocation { start: token.start, end },
        ))
    }

    /// Parse a variable when we have a sigil token first
    fn parse_variable_from_sigil(&mut self) -> ParseResult<Node> {
        let sigil_token = self.consume_token()?;
        let sigil = match sigil_token.kind {
            TokenKind::BitwiseAnd => "&".to_string(), // Handle & as sigil
            _ => sigil_token.text.clone(),
        };
        let start = sigil_token.start;

        // Check if next token is an identifier or a keyword that should be treated as identifier
        let next_kind = self.peek_kind();
        let can_be_sub_name = |k: TokenKind| {
            matches!(
                k,
                TokenKind::Sub
                    | TokenKind::My
                    | TokenKind::Our
                    | TokenKind::If
                    | TokenKind::Unless
                    | TokenKind::While
                    | TokenKind::For
                    | TokenKind::Return
                    | TokenKind::Do
                    | TokenKind::Eval
                    | TokenKind::Use
                    | TokenKind::Package
            )
        };

        let (name, end) = if next_kind == Some(TokenKind::Identifier) ||
                             // Keywords that can be used as subroutine names with & sigil
                             (sigil == "&" && matches!(next_kind, Some(k) if can_be_sub_name(k)))
        {
            let name_token = self.tokens.next()?;
            let mut name = name_token.text.clone();
            let mut end = name_token.end;

            // Handle :: in package-qualified variables
            while self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.tokens.next()?; // consume ::
                name.push_str("::");

                if self.peek_kind() == Some(TokenKind::Identifier) {
                    let next_token = self.tokens.next()?;
                    name.push_str(&next_token.text);
                    end = next_token.end;
                } else {
                    return Err(ParseError::syntax(
                        "Expected identifier after :: in package-qualified variable",
                        self.current_position(),
                    ));
                }
            }

            (name, end)
        } else {
            // Handle special variables like $$, $@, $!, $?, etc.
            match self.peek_kind() {
                Some(TokenKind::ScalarSigil) => {
                    // $$ - process ID
                    let token = self.tokens.next()?;
                    ("$".to_string(), token.end)
                }
                Some(TokenKind::ArraySigil) => {
                    // $@ - eval error
                    let token = self.tokens.next()?;
                    ("@".to_string(), token.end)
                }
                Some(TokenKind::Not) => {
                    // $! - system error
                    let token = self.tokens.next()?;
                    ("!".to_string(), token.end)
                }
                Some(TokenKind::Unknown) => {
                    // Could be $?, $^, $#, or other special
                    let token = self.tokens.peek()?;
                    match token.text.as_str() {
                        "?" => {
                            let token = self.tokens.next()?;
                            ("?".to_string(), token.end)
                        }
                        "^" => {
                            // Handle $^X variables
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                (format!("^{}", var_token.text), var_token.end)
                            } else {
                                ("^".to_string(), token.end)
                            }
                        }
                        "#" => {
                            // Handle $# (array length)
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                (format!("#{}", var_token.text), var_token.end)
                            } else {
                                // Just $# by itself
                                ("#".to_string(), token.end)
                            }
                        }
                        _ => {
                            return Err(ParseError::syntax(
                                format!("Unexpected character after sigil: {}", token.text),
                                token.start,
                            ));
                        }
                    }
                }
                Some(TokenKind::Number) => {
                    // $0, $1, $2, etc. - numbered capture groups
                    let num_token = self.tokens.next()?;
                    (num_token.text.clone(), num_token.end)
                }
                _ => {
                    // Empty variable name (just the sigil)
                    (String::new(), self.previous_position())
                }
            }
        };

        // Special handling for @ or % sigil followed by { - array/hash dereference
        if (sigil == "@" || sigil == "%") && self.peek_kind() == Some(TokenKind::LeftBrace) {
            self.tokens.next()?; // consume {

            // Parse the expression inside the braces
            let expr = self.parse_expression()?;

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(expr) },
                SourceLocation { start, end },
            ));
        }

        // Special handling for & sigil - it's a function call
        if sigil == "&" {
            // Check if there are parentheses for arguments
            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                self.consume_token()?; // consume (
                let mut args = vec![];

                while self.peek_kind() != Some(TokenKind::RightParen) {
                    args.push(self.parse_expression()?);

                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.consume_token()?; // consume comma
                    } else if self.peek_kind() != Some(TokenKind::RightParen) {
                        return Err(ParseError::syntax(
                            "Expected comma or right parenthesis",
                            self.current_position(),
                        ));
                    }
                }

                let right_paren = self.expect(TokenKind::RightParen)?;
                let _end = right_paren.end;
                args
            } else {
                vec![]
            };

            Ok(Node::new(NodeKind::FunctionCall { name, args }, SourceLocation { start, end }))
        } else {
            Ok(Node::new(NodeKind::Variable { sigil, name }, SourceLocation { start, end }))
        }
    }

    /// Parse if statement
    fn parse_if_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'if'

        self.expect(TokenKind::LeftParen)?;

        // Check if this is a variable declaration in the condition
        let condition = if matches!(
            self.peek_kind(),
            Some(TokenKind::My)
                | Some(TokenKind::Our)
                | Some(TokenKind::Local)
                | Some(TokenKind::State)
        ) {
            self.parse_variable_declaration()?
        } else {
            self.parse_expression()?
        };

        self.expect(TokenKind::RightParen)?;

        let then_branch = self.parse_block()?;

        let mut elsif_branches = Vec::new();
        let mut else_branch = None;

        // Handle elsif chains
        while self.peek_kind() == Some(TokenKind::Elsif) {
            self.tokens.next()?; // consume 'elsif'
            self.expect(TokenKind::LeftParen)?;

            // Check if this is a variable declaration in the condition
            let elsif_cond = if matches!(
                self.peek_kind(),
                Some(TokenKind::My)
                    | Some(TokenKind::Our)
                    | Some(TokenKind::Local)
                    | Some(TokenKind::State)
            ) {
                self.parse_variable_declaration()?
            } else {
                self.parse_expression()?
            };

            self.expect(TokenKind::RightParen)?;
            let elsif_block = self.parse_block()?;
            elsif_branches.push((Box::new(elsif_cond), Box::new(elsif_block)));
        }

        // Handle else
        if self.peek_kind() == Some(TokenKind::Else) {
            self.tokens.next()?; // consume 'else'
            else_branch = Some(Box::new(self.parse_block()?));
        }

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                elsif_branches,
                else_branch,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse unless statement (syntactic sugar for if not)
    fn parse_unless_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'unless'

        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        // Negate the condition
        let negated_condition = Node::new(
            NodeKind::Unary { op: "!".to_string(), operand: Box::new(condition) },
            SourceLocation { start, end: self.previous_position() },
        );

        let then_branch = self.parse_block()?;
        let end = self.previous_position();

        Ok(Node::new(
            NodeKind::If {
                condition: Box::new(negated_condition),
                then_branch: Box::new(then_branch),
                elsif_branches: vec![],
                else_branch: None,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse while loop
    fn parse_while_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'while'

        self.expect(TokenKind::LeftParen)?;

        // Check if this is a variable declaration in the condition
        let condition = if matches!(
            self.peek_kind(),
            Some(TokenKind::My)
                | Some(TokenKind::Our)
                | Some(TokenKind::Local)
                | Some(TokenKind::State)
        ) {
            self.parse_variable_declaration()?
        } else {
            self.parse_expression()?
        };

        self.expect(TokenKind::RightParen)?;

        let body = self.parse_block()?;

        // Handle continue block
        let continue_block = if self.peek_kind() == Some(TokenKind::Continue) {
            self.tokens.next()?; // consume 'continue'
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::While {
                condition: Box::new(condition),
                body: Box::new(body),
                continue_block,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse until loop (while not)
    fn parse_until_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'until'

        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        // Negate the condition
        let negated_condition = Node::new(
            NodeKind::Unary { op: "!".to_string(), operand: Box::new(condition) },
            SourceLocation { start, end: self.previous_position() },
        );

        let body = self.parse_block()?;
        let end = self.previous_position();

        Ok(Node::new(
            NodeKind::While {
                condition: Box::new(negated_condition),
                body: Box::new(body),
                continue_block: None,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse for loop
    fn parse_for_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'for'

        // Check if it's a foreach-style for loop
        if matches!(self.peek_kind(), Some(TokenKind::My)) || self.is_variable_start() {
            return self.parse_foreach_style_for();
        }

        self.expect(TokenKind::LeftParen)?;

        // Parse init (or check if it's a foreach)
        let init = if self.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else if self.peek_kind() == Some(TokenKind::My) {
            // Handle variable declaration in for loop init
            self.in_for_loop_init = true;
            let decl = self.parse_variable_declaration()?;
            self.in_for_loop_init = false;
            // Variable declarations in for loops don't have trailing semicolons
            Some(Box::new(decl))
        } else {
            // Parse expression
            let expr = self.parse_expression()?;

            // If followed by ), it's a foreach loop
            if self.peek_kind() == Some(TokenKind::RightParen) {
                self.tokens.next()?; // consume )
                let body = self.parse_block()?;

                let end = self.previous_position();

                // Create implicit $_ variable
                let implicit_var = Node::new(
                    NodeKind::Variable { sigil: "$".to_string(), name: "_".to_string() },
                    SourceLocation { start, end: start },
                );

                return Ok(Node::new(
                    NodeKind::Foreach {
                        variable: Box::new(implicit_var),
                        list: Box::new(expr),
                        body: Box::new(body),
                    },
                    SourceLocation { start, end },
                ));
            }

            Some(Box::new(expr))
        };
        self.expect(TokenKind::Semicolon)?;

        // Parse condition
        let condition = if self.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        self.expect(TokenKind::Semicolon)?;

        // Parse update
        let update = if self.peek_kind() == Some(TokenKind::RightParen) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };

        self.expect(TokenKind::RightParen)?;
        let body = self.parse_block()?;

        // Handle continue block
        let continue_block = if self.peek_kind() == Some(TokenKind::Continue) {
            self.tokens.next()?; // consume 'continue'
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::For { init, condition, update, body: Box::new(body), continue_block },
            SourceLocation { start, end },
        ))
    }

    /// Parse foreach loop
    fn parse_foreach_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'foreach'

        // Set flag to prevent semicolon consumption in variable declaration
        self.in_for_loop_init = true;
        let variable = if self.peek_kind() == Some(TokenKind::My) {
            self.parse_variable_declaration()?
        } else {
            self.parse_variable()?
        };
        self.in_for_loop_init = false;

        self.expect(TokenKind::LeftParen)?;
        let list = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Foreach {
                variable: Box::new(variable),
                list: Box::new(list),
                body: Box::new(body),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse foreach-style for loop
    fn parse_foreach_style_for(&mut self) -> ParseResult<Node> {
        // Set flag to prevent semicolon consumption in variable declaration
        self.in_for_loop_init = true;
        let variable = if self.peek_kind() == Some(TokenKind::My) {
            self.parse_variable_declaration()?
        } else {
            self.parse_variable()?
        };
        self.in_for_loop_init = false;

        self.expect(TokenKind::LeftParen)?;
        let list = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        let body = self.parse_block()?;

        let start = variable.location.start;
        let end = self.previous_position();

        Ok(Node::new(
            NodeKind::Foreach {
                variable: Box::new(variable),
                list: Box::new(list),
                body: Box::new(body),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse subroutine definition
    fn parse_subroutine(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'sub'

        let (name, name_span) = match self.peek_kind() {
            // Regular identifier
            Some(TokenKind::Identifier)
            | Some(TokenKind::Method)
            | Some(TokenKind::Class)
            | Some(TokenKind::Try)
            | Some(TokenKind::Catch)
            | Some(TokenKind::Finally)
            | Some(TokenKind::Given)
            | Some(TokenKind::When)
            | Some(TokenKind::Default)
            | Some(TokenKind::Continue)
            | Some(TokenKind::Format) => {
                let token = self.tokens.next()?;
                (
                    Some(token.text.clone()),
                    Some(SourceLocation { start: token.start, end: token.end }),
                )
            }
            // No name - anonymous subroutine
            _ => (None, None),
        };

        // Parse optional attributes first (they come before signature in modern Perl)
        let mut attributes = Vec::new();
        while self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume colon

            // Parse one or more space-separated attributes after the colon
            loop {
                // Attributes can be identifiers or certain keywords
                let attr_token = match self.peek_kind() {
                    Some(TokenKind::Identifier | TokenKind::Method) => self.tokens.next()?,
                    _ => {
                        // If it's not an attribute name, we're done with this attribute list
                        break;
                    }
                };

                let mut attr_name = attr_token.text.clone();

                // Check if attribute has a value in parentheses (like :prototype($))
                if self.peek_kind() == Some(TokenKind::LeftParen) {
                    self.consume_token()?; // consume (
                    attr_name.push('(');

                    // Collect tokens until matching )
                    let mut paren_depth = 1;
                    while paren_depth > 0 && !self.tokens.is_eof() {
                        let token = self.tokens.next()?;
                        attr_name.push_str(&token.text);

                        match token.kind {
                            TokenKind::LeftParen => paren_depth += 1,
                            TokenKind::RightParen => {
                                paren_depth -= 1;
                                if paren_depth == 0 {
                                    attr_name.push(')');
                                }
                            }
                            _ => {}
                        }
                    }
                }

                attributes.push(attr_name);

                // Check if there's another attribute (not preceded by colon)
                match self.peek_kind() {
                    Some(TokenKind::Identifier | TokenKind::Method) => {
                        // Continue parsing more attributes
                        continue;
                    }
                    _ => break,
                }
            }
        }

        // Parse optional prototype or signature after attributes
        let (prototype, signature) = if self.peek_kind() == Some(TokenKind::LeftParen) {
            // Look ahead to determine if this is a prototype or signature
            if self.is_likely_prototype()? {
                // Parse as prototype
                let proto_content = self.parse_prototype()?;
                let proto_node = Node::new(
                    NodeKind::Prototype { content: proto_content },
                    SourceLocation { start: self.current_position(), end: self.current_position() },
                );
                (Some(Box::new(proto_node)), None)
            } else {
                // Parse as signature
                let params = self.parse_signature()?;
                let sig_node = Node::new(
                    NodeKind::Signature { parameters: params },
                    SourceLocation { start: self.current_position(), end: self.current_position() },
                );
                (None, Some(Box::new(sig_node)))
            }
        } else {
            (None, None)
        };

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Subroutine {
                name,
                name_span,
                prototype,
                signature,
                attributes,
                body: Box::new(body),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse class declaration (Perl 5.38+)
    fn parse_class(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'class'

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.text.clone();

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Class { name, body: Box::new(body) }, SourceLocation { start, end }))
    }

    /// Parse method declaration (Perl 5.38+)
    fn parse_method(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'method'

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.text.clone();

        // Parse optional signature
        let signature = if self.peek_kind() == Some(TokenKind::LeftParen) {
            let params = self.parse_signature()?;
            Some(Box::new(Node::new(
                NodeKind::Signature { parameters: params },
                SourceLocation { start: self.current_position(), end: self.current_position() },
            )))
        } else {
            None
        };

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Method { name, signature, attributes: Vec::new(), body: Box::new(body) },
            SourceLocation { start, end },
        ))
    }

    /// Parse format declaration
    fn parse_format(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'format'

        // Parse format name (optional - can be anonymous)
        let name = if self.peek_kind() == Some(TokenKind::Assign) {
            // Anonymous format
            String::new()
        } else {
            // Named format
            let name_token = self.expect(TokenKind::Identifier)?;
            name_token.text.clone()
        };

        // Expect =
        self.expect(TokenKind::Assign)?;

        // Tell the lexer to enter format body mode
        self.tokens.enter_format_mode();

        // Get the format body
        let body_token = self.tokens.next()?;
        let body = if body_token.kind == TokenKind::FormatBody {
            body_token.text
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "format body".to_string(),
                found: format!("{:?}", body_token.kind),
                location: body_token.start,
            });
        };

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Format { name, body }, SourceLocation { start, end }))
    }

    /// Parse subroutine signature
    fn parse_signature(&mut self) -> ParseResult<Vec<Node>> {
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut params = Vec::new();

        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            // Parse parameter
            let param = self.parse_signature_param()?;
            params.push(param);

            // Check for comma or end of signature
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
            } else if self.peek_kind() == Some(TokenKind::RightParen) {
                break;
            } else {
                return Err(ParseError::syntax(
                    "Expected comma or closing parenthesis in signature",
                    self.current_position(),
                ));
            }
        }

        self.expect(TokenKind::RightParen)?; // consume )
        Ok(params)
    }

    /// Parse a single signature parameter
    fn parse_signature_param(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Check for named parameter (:$name)
        let named = if self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume :
            true
        } else {
            false
        };

        // Check for type constraint (Type $var)
        let _type_constraint = if self.peek_kind() == Some(TokenKind::Identifier) {
            // Look ahead to see if this is a type constraint
            let token = self.tokens.peek()?;
            if !token.text.starts_with('$')
                && !token.text.starts_with('@')
                && !token.text.starts_with('%')
                && !token.text.starts_with('&')
            {
                // It's likely a type constraint
                Some(self.tokens.next()?.text.clone())
            } else {
                None
            }
        } else {
            None
        };

        // Parse the variable
        let variable = self.parse_variable()?;

        // Check for default value (= expression)
        let default_value = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            // Parse a primary expression for default value to avoid parsing too far
            Some(Box::new(self.parse_primary()?))
        } else {
            None
        };

        let end = if let Some(ref default) = default_value {
            default.location.end
        } else {
            variable.location.end
        };

        // Check if variable is slurpy (@args or %hash)
        let is_slurpy = matches!(&variable.kind, NodeKind::Variable { sigil, .. } if sigil == "@" || sigil == "%");

        // Create the appropriate parameter node type
        let param_kind = if named {
            NodeKind::NamedParameter { variable: Box::new(variable) }
        } else if is_slurpy {
            NodeKind::SlurpyParameter { variable: Box::new(variable) }
        } else if let Some(default) = default_value {
            NodeKind::OptionalParameter { variable: Box::new(variable), default_value: default }
        } else {
            NodeKind::MandatoryParameter { variable: Box::new(variable) }
        };

        Ok(Node::new(param_kind, SourceLocation { start, end }))
    }

    /// Parse package declaration
    fn parse_package(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'package'

        // Parse package name (can include ::)
        let first = self.expect(TokenKind::Identifier)?;
        let mut name = first.text.clone();
        let name_start = first.start;
        let mut name_end = first.end;

        // Handle :: in package names
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                let dc = self.tokens.next()?; // consume ::
                name_end = dc.end;
                name.push_str("::");
            } else if self.peek_kind() == Some(TokenKind::Colon) {
                // Handle two separate Colon tokens as ::
                let _first_colon = self.tokens.next()?; // consume first :
                let second_colon = self.tokens.next()?; // consume second :
                name_end = second_colon.end;
                name.push_str("::");
            }

            // Check if there's an identifier after ::
            // If not, it's a trailing :: which is valid in Perl
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let id = self.tokens.next()?;
                name_end = id.end;
                name.push_str(&id.text);
            } else {
                // Trailing :: is valid, just break
                break;
            }
        }

        let name_span = SourceLocation { start: name_start, end: name_end };

        // Check for optional version number or v-string
        let version = if self.peek_kind() == Some(TokenKind::Number) {
            Some(self.tokens.next()?.text.clone())
        } else if let Some(TokenKind::Identifier) = self.peek_kind() {
            // Check if it's a v-string version
            if let Ok(token) = self.tokens.peek() {
                if token.text.starts_with('v') && token.text.len() > 1 {
                    // It's a v-string like v1 or v5
                    let mut version_str = self.tokens.next()?.text.clone();

                    // Collect the rest of the v-string (e.g., .2.3)
                    while let Some(TokenKind::Number) = self.peek_kind() {
                        if let Ok(num_token) = self.tokens.peek() {
                            if num_token.text.starts_with('.') {
                                version_str.push_str(&self.tokens.next()?.text);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    Some(version_str)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // If we have a version, append it to the name for now
        // (In a real AST, you'd probably want these as separate fields)
        if let Some(ver) = version {
            name.push(' ');
            name.push_str(&ver);
        }

        let block = if self.peek_kind() == Some(TokenKind::LeftBrace) {
            Some(Box::new(self.parse_block()?))
        } else {
            // Don't consume semicolon here - let parse_statement handle it uniformly
            None
        };

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Package { name, name_span, block }, SourceLocation { start, end }))
    }

    /// Parse use statement
    fn parse_use(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.consume_token()?; // consume 'use'

        // Parse module name, version, or identifier
        let mut module = if self.peek_kind() == Some(TokenKind::Number) {
            // Numeric version like 5.036
            self.consume_token()?.text.clone()
        } else {
            let first_token = self.consume_token()?;

            // Check for version strings
            if first_token.kind == TokenKind::Identifier
                && first_token.text.starts_with('v')
                && first_token.text.chars().skip(1).all(|c| c.is_numeric())
            {
                // Version identifier like v5 or v536
                let mut version = first_token.text.clone();

                // Check if followed by dot and more numbers (e.g., v5.36)
                if self.peek_kind() == Some(TokenKind::Unknown) {
                    if let Ok(dot_token) = self.tokens.peek() {
                        if dot_token.text == "." {
                            self.consume_token()?; // consume dot
                            if self.peek_kind() != /* ~ changed by cargo-mutants ~ */ Some(TokenKind::Number)
                            {
                                let num = self.consume_token()?;
                                version.push('.');
                                version.push_str(&num.text);
                            }
                        }
                    }
                }
                version
            } else if first_token.text == "v" && self.peek_kind() == Some(TokenKind::Number) {
                // Version string like v5.36 (tokenized as "v" followed by number)
                let version = self.expect(TokenKind::Number)?;
                format!("v{}", version.text)
            } else if first_token.kind == TokenKind::Identifier {
                first_token.text.clone()
            } else {
                return Err(ParseError::syntax(
                    format!("Expected module name or version, found {:?}", first_token.kind),
                    first_token.start,
                ));
            }
        };

        // Handle :: in module names
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.consume_token()?; // consume ::
                module.push_str("::");
            } else {
                // Handle two separate Colon tokens as ::
                self.consume_token()?; // consume first :
                self.consume_token()?; // consume second :
                module.push_str("::");
            }
            // In Perl, trailing :: is valid (e.g., Foo::Bar::)
            // Only consume identifier if there is one
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let next_part = self.consume_token()?;
                module.push_str(&next_part.text);
            }
            // No error for trailing :: - it's valid in Perl
        }

        // Parse optional version number
        if self.peek_kind() == Some(TokenKind::Number) {
            module.push(' ');
            module.push_str(&self.consume_token()?.text);
        }

        // Parse optional import list
        let mut args = Vec::new();

        // Loop to handle multiple argument groups separated by commas
        // e.g., qw(FOO) => 1, qw(BAR BAZ) => 2
        loop {
            // Special case: ALWAYS check for qw FIRST before any other parsing
            // Check if next token is "qw" - this is critical to handle before bare args
            let is_qw = self.tokens.peek().map(|t| t.text == "qw").unwrap_or(false);
            if is_qw {
                self.consume_token()?; // consume 'qw'

                // Try to parse qw words, but if it fails (e.g., unknown delimiter),
                // fall back to simple token consumption
                let list = match self.parse_qw_words() {
                    Ok(words) => words,
                    Err(_) => {
                        // Fallback: just consume tokens until semicolon
                        let mut words = Vec::new();
                        while !Self::is_statement_terminator(self.peek_kind())
                            && !self.tokens.is_eof()
                        {
                            if let Ok(tok) = self.tokens.next() {
                                if matches!(tok.kind, TokenKind::Identifier | TokenKind::Number) {
                                    words.push(tok.text.clone());
                                }
                            } else {
                                break;
                            }
                        }
                        words
                    }
                };
                // Format as "qw(FOO BAR BAZ)" so DeclarationProvider can recognize it
                // We use parentheses regardless of original delimiter for consistency
                let qw_str = format!("qw({})", list.join(" "));
                args.push(qw_str);
                // optional: qw(...) => <value>
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    self.consume_token()?; // =>
                    if let Some(TokenKind::String | TokenKind::Number | TokenKind::Identifier) =
                        self.peek_kind()
                    {
                        args.push(self.consume_token()?.text.clone());
                    } else {
                        // best-effort: slurp tokens until ',' or ';'
                        while !Self::is_statement_terminator(self.peek_kind())
                            && self.peek_kind() != Some(TokenKind::Comma)
                        {
                            args.push(self.consume_token()?.text.clone());
                        }
                    }
                }
                // Check if there's a comma and more args
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume ','
                    continue; // Loop to parse next argument group
                } else {
                    // No more args, we're done
                    break;
                }
            } else {
                // Not qw, break out to handle other argument types
                break;
            }
        }

        // Handle unary plus forcing hash syntax: use constant +{ FOO => 42 }
        if self.peek_kind() == Some(TokenKind::Plus) {
            let plus = self.consume_token()?;
            args.push(plus.text.clone());
            // Next should be a hash
            if self.peek_kind() == Some(TokenKind::LeftBrace) {
                // Consume the hash expression
                let mut depth = 0;
                while !self.tokens.is_eof() {
                    match self.peek_kind() {
                        Some(TokenKind::LeftBrace) => {
                            depth += 1;
                            args.push(self.consume_token()?.text.clone());
                        }
                        Some(TokenKind::RightBrace) => {
                            args.push(self.consume_token()?.text.clone());
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {
                            args.push(self.consume_token()?.text.clone());
                        }
                    }
                }
            }
        }
        // Handle hash syntax for pragmas like: use constant { FOO => 42, BAR => 43 }
        else if self.peek_kind() == Some(TokenKind::LeftBrace) {
            loop {
                // consume one { ... } block (track depth)
                let mut depth = 0;
                self.consume_token()?; // '{'
                depth += 1;
                args.push("{".into());
                while !self.tokens.is_eof() && depth > 0 {
                    match self.peek_kind() {
                        Some(TokenKind::LeftBrace) => {
                            depth += 1;
                            args.push(self.consume_token()?.text.clone());
                        }
                        Some(TokenKind::RightBrace) => {
                            args.push(self.consume_token()?.text.clone());
                            depth -= 1;
                        }
                        _ => {
                            args.push(self.consume_token()?.text.clone());
                        }
                    }
                }
                // optional: => "ignored"
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    self.consume_token()?; // =>
                    if let Some(TokenKind::String | TokenKind::Number | TokenKind::Identifier) =
                        self.peek_kind()
                    {
                        args.push(self.consume_token()?.text.clone());
                    } else {
                        while !Self::is_statement_terminator(self.peek_kind())
                            && self.peek_kind() != Some(TokenKind::Comma)
                        {
                            args.push(self.consume_token()?.text.clone());
                        }
                    }
                }
                // another block after comma?
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // ','
                    if self.peek_kind() == Some(TokenKind::LeftBrace) {
                        continue; // loop for the next { ... }
                    }
                }
                break;
            }
        }
        // Handle bare arguments (no parentheses)
        else if matches!(self.peek_kind(), Some(k) if matches!(k, TokenKind::String | TokenKind::Identifier | TokenKind::Minus | TokenKind::QuoteWords))
            && !Self::is_statement_terminator(self.peek_kind())
        {
            // Parse bare arguments like: use warnings 'void' or use constant FOO => 42
            // Also handle -strict flag and comma forms
            loop {
                // Check for qw BEFORE the match to avoid it being consumed as a generic identifier
                if let Ok(tok) = self.tokens.peek() {
                    if tok.text == "qw" {
                        self.consume_token()?; // consume 'qw'
                        let list = self.parse_qw_words()?;
                        // Format as "qw(FOO BAR BAZ)" so DeclarationProvider can recognize it
                        // We use parentheses regardless of original delimiter for consistency
                        let qw_str = format!("qw({})", list.join(" "));
                        args.push(qw_str);
                        // optional: qw(...) => <value>
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.consume_token()?; // =>
                            if let Some(
                                TokenKind::String | TokenKind::Number | TokenKind::Identifier,
                            ) = self.peek_kind()
                            {
                                args.push(self.consume_token()?.text.clone());
                            } else {
                                // best-effort: slurp tokens until ',' or ';'
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && self.peek_kind() != Some(TokenKind::Comma)
                                {
                                    args.push(self.consume_token()?.text.clone());
                                }
                            }
                        }
                        continue; // Don't fall through to the match below
                    }
                }

                match self.peek_kind() {
                    Some(TokenKind::String) => {
                        args.push(self.consume_token()?.text.clone());
                    }
                    Some(TokenKind::QuoteWords) => {
                        // Handle qw(...) in use statements
                        // Format it as "qw(FOO BAR)" for consistency with DeclarationProvider
                        let qw_token = self.consume_token()?;
                        let text: &str = qw_token.text.as_ref();
                        if let Some(content) = text.strip_prefix("qw").and_then(|s| {
                            // Extract content between delimiters
                            if s.starts_with('(') && s.ends_with(')') {
                                Some(&s[1..s.len() - 1])
                            } else if s.starts_with('[') && s.ends_with(']') {
                                Some(&s[1..s.len() - 1])
                            } else if s.starts_with('{') && s.ends_with('}') {
                                Some(&s[1..s.len() - 1])
                            } else if s.starts_with('<') && s.ends_with('>') {
                                Some(&s[1..s.len() - 1])
                            } else {
                                None
                            }
                        }) {
                            // Reformat as "qw(FOO BAR)" for consistency
                            let words: Vec<&str> = content.split_whitespace().collect();
                            let qw_str = format!("qw({})", words.join(" "));
                            args.push(qw_str);
                        } else {
                            // Fallback: just add the whole token as string
                            args.push(qw_token.text.to_string());
                        }
                    }
                    Some(TokenKind::Minus) => {
                        // Handle -strict and other flags
                        let minus = self.consume_token()?;
                        if self.peek_kind() == Some(TokenKind::Identifier) {
                            let flag = self.consume_token()?;
                            // Combine minus and identifier as a single flag
                            args.push(format!("-{}", flag.text));
                        } else {
                            // Just a minus sign (shouldn't happen in use statements)
                            args.push(minus.text.clone());
                        }
                    }
                    Some(TokenKind::Identifier) => {
                        // Check if this might be a constant declaration
                        let ident = self.consume_token()?;
                        args.push(ident.text.clone());

                        // Check for comma or fat arrow
                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?; // consume comma
                                // Continue to parse next argument
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?; // consume =>
                                // Parse the value as a simple expression
                                match self.peek_kind() {
                                    Some(TokenKind::Number | TokenKind::String) => {
                                        args.push(self.consume_token()?.text.clone());
                                    }
                                    Some(TokenKind::Identifier) => {
                                        args.push(self.consume_token()?.text.clone());
                                    }
                                    _ => {
                                        // For more complex expressions, just consume tokens until semicolon
                                        while !Self::is_statement_terminator(self.peek_kind())
                                            && self.peek_kind() != Some(TokenKind::Comma)
                                        {
                                            args.push(self.consume_token()?.text.clone());
                                        }
                                    }
                                }
                            }
                            _ => {
                                // No separator, just continue
                            }
                        }
                    }
                    Some(TokenKind::Comma) => {
                        // Skip standalone commas (already handled after identifiers)
                        self.consume_token()?;
                    }
                    _ => break,
                }

                // Check if we should continue parsing arguments
                if Self::is_statement_terminator(self.peek_kind()) {
                    break;
                }
            }
        } else if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            // Parse import list
            while self.peek_kind() != Some(TokenKind::RightParen) {
                if self.peek_kind() == Some(TokenKind::String) {
                    args.push(self.consume_token()?.text.clone());
                } else if self.peek_kind() == Some(TokenKind::Identifier) {
                    args.push(self.consume_token()?.text.clone());
                } else {
                    return Err(ParseError::syntax(
                        "Expected string or identifier in import list",
                        self.current_position(),
                    ));
                }

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis",
                        self.current_position(),
                    ));
                }
            }

            self.expect(TokenKind::RightParen)?;
        }

        // Don't consume semicolon here - let parse_statement handle it uniformly

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Use { module, args }, SourceLocation { start, end }))
    }

    /// Parse special block (AUTOLOAD, DESTROY, etc.)
    fn parse_special_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let name_token = self.consume_token()?;
        let name = name_token.text.clone();

        // Capture name_span from token for precise LSP navigation
        let name_span = Some(SourceLocation { start: name_token.start, end: name_token.end });

        let block = self.parse_block()?;
        let end = block.location.end;

        // Treat as a special subroutine
        Ok(Node::new(
            NodeKind::Subroutine {
                name: Some(name),
                name_span,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse phase block (BEGIN, END, CHECK, INIT, UNITCHECK)
    fn parse_phase_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let phase_token = self.consume_token()?;
        let phase = phase_token.text.clone();

        // Capture phase_span from token for precise LSP navigation
        let phase_span = Some(SourceLocation { start: phase_token.start, end: phase_token.end });

        // Phase blocks must be followed by a block
        if self.peek_kind() != Some(TokenKind::LeftBrace) {
            return Err(ParseError::syntax(
                format!("{} must be followed by a block", phase),
                self.current_position(),
            ));
        }

        let block = self.parse_block()?;
        let end = block.location.end;

        // Create a special node for phase blocks
        Ok(Node::new(
            NodeKind::PhaseBlock { phase, phase_span, block: Box::new(block) },
            SourceLocation { start, end },
        ))
    }

    /// Parse data section (__DATA__ or __END__)
    fn parse_data_section(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Consume the data marker token
        let marker_token = self.consume_token()?;
        let marker = marker_token.text.clone();

        // Check if there's a data body token
        let body = if self.peek_kind() == Some(TokenKind::DataBody) {
            let body_token = self.consume_token()?;
            Some(body_token.text.clone())
        } else {
            None
        };

        let end = self.previous_position();

        // Create a data section node
        Ok(Node::new(NodeKind::DataSection { marker, body }, SourceLocation { start, end }))
    }

    /// Parse no statement (similar to use but disables pragmas/modules)
    fn parse_no(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'no'

        // Parse module name (can include ::)
        let mut module = self.expect(TokenKind::Identifier)?.text.clone();

        // Handle :: in module names
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.consume_token()?; // consume ::
                module.push_str("::");
            } else {
                // Handle two separate Colon tokens as ::
                self.consume_token()?; // consume first :
                self.consume_token()?; // consume second :
                module.push_str("::");
            }
            // In Perl, trailing :: is valid (e.g., Foo::Bar::)
            // Only consume identifier if there is one
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let next_part = self.consume_token()?;
                module.push_str(&next_part.text);
            }
            // No error for trailing :: - it's valid in Perl
        }

        // Parse optional version number
        if self.peek_kind() == Some(TokenKind::Number) {
            module.push(' ');
            module.push_str(&self.consume_token()?.text);
        }

        // Parse optional arguments list
        let mut args = Vec::new();

        // Handle bare arguments (no parentheses)
        if matches!(self.peek_kind(), Some(TokenKind::String) | Some(TokenKind::Identifier))
            && !matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None)
        {
            // Parse bare arguments like: no warnings 'void'
            loop {
                // Check for qw BEFORE the match to avoid it being consumed as a generic identifier
                if let Ok(tok) = self.tokens.peek() {
                    if tok.text == "qw" {
                        self.consume_token()?; // consume 'qw'
                        let list = self.parse_qw_words()?;
                        // Format as "qw(FOO BAR BAZ)" so DeclarationProvider can recognize it
                        // We use parentheses regardless of original delimiter for consistency
                        let qw_str = format!("qw({})", list.join(" "));
                        args.push(qw_str);
                        // optional: qw(...) => <value>
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.consume_token()?; // =>
                            if let Some(
                                TokenKind::String | TokenKind::Number | TokenKind::Identifier,
                            ) = self.peek_kind()
                            {
                                args.push(self.consume_token()?.text.clone());
                            } else {
                                // best-effort: slurp tokens until ',' or ';'
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && self.peek_kind() != Some(TokenKind::Comma)
                                {
                                    args.push(self.consume_token()?.text.clone());
                                }
                            }
                        }
                        continue; // Don't fall through to the match below
                    }
                }

                match self.peek_kind() {
                    Some(TokenKind::String) => {
                        args.push(self.consume_token()?.text.clone());
                    }
                    Some(TokenKind::Identifier) => {
                        args.push(self.consume_token()?.text.clone());
                    }
                    _ => break,
                }

                // Check if we should continue parsing arguments
                if Self::is_statement_terminator(self.peek_kind()) {
                    break;
                }
            }
        } else if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            // Parse argument list
            while self.peek_kind() != Some(TokenKind::RightParen) {
                if self.peek_kind() == Some(TokenKind::String) {
                    args.push(self.consume_token()?.text.clone());
                } else if self.peek_kind() == Some(TokenKind::Identifier) {
                    args.push(self.consume_token()?.text.clone());
                } else {
                    return Err(ParseError::syntax(
                        "Expected string or identifier in argument list",
                        self.current_position(),
                    ));
                }

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis",
                        self.current_position(),
                    ));
                }
            }

            self.expect(TokenKind::RightParen)?;
        }

        // Don't consume semicolon here - let parse_statement handle it uniformly

        let end = self.previous_position();
        Ok(Node::new(NodeKind::No { module, args }, SourceLocation { start, end }))
    }

    /// Parse format declaration
    /// Parse return statement
    fn parse_return(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'return'

        // Check if we have a value to return - only stop at clear ends or statement modifiers
        let value = if Self::is_statement_terminator(self.peek_kind())
            || matches!(self.peek_kind(), Some(TokenKind::RightBrace))
            || matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k))
        {
            None
        } else {
            // Parse the return value
            Some(Box::new(self.parse_expression()?))
        };

        let end = value.as_ref().map(|v| v.location.end).unwrap_or(self.previous_position());
        Ok(Node::new(NodeKind::Return { value }, SourceLocation { start, end }))
    }

    /// Parse eval expression/block
    fn parse_eval(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'eval'

        // Eval can take either a block or a string expression
        if self.peek_kind() == Some(TokenKind::LeftBrace) {
            // eval { ... }
            let block = self.parse_block()?;
            let end = block.location.end;
            Ok(Node::new(NodeKind::Eval { block: Box::new(block) }, SourceLocation { start, end }))
        } else {
            // eval "string" or eval $expr
            let expr = self.parse_expression()?;
            let end = expr.location.end;
            Ok(Node::new(NodeKind::Eval { block: Box::new(expr) }, SourceLocation { start, end }))
        }
    }

    /// Parse try/catch/finally block
    fn parse_try(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'try'

        // Parse the try body
        let body = self.parse_block()?;

        let mut catch_blocks = Vec::new();
        let mut finally_block = None;

        // Parse catch blocks
        while self.peek_kind() == Some(TokenKind::Catch) {
            self.consume_token()?; // consume 'catch'

            // Check for optional variable
            let var = if self.peek_kind() == Some(TokenKind::LeftParen) {
                self.consume_token()?; // consume '('
                let var_name = if self.peek_kind() == Some(TokenKind::ScalarSigil)
                    || self.tokens.peek()?.text.starts_with('$')
                {
                    let var = self.parse_variable()?;
                    match &var.kind {
                        NodeKind::Variable { sigil, name } => Some(format!("{}{}", sigil, name)),
                        _ => None,
                    }
                } else {
                    None
                };
                self.expect(TokenKind::RightParen)?;
                var_name
            } else {
                None
            };

            let block = self.parse_block()?;
            catch_blocks.push((var, block));
        }

        // Parse optional finally block
        if self.peek_kind() == Some(TokenKind::Finally) {
            self.consume_token()?; // consume 'finally'
            finally_block = Some(Box::new(self.parse_block()?));
        }

        let end = finally_block
            .as_ref()
            .map(|b| b.location.end)
            .or_else(|| catch_blocks.last().map(|(_, b)| b.location.end))
            .unwrap_or(body.location.end);

        Ok(Node::new(
            NodeKind::Try {
                body: Box::new(body),
                catch_blocks: catch_blocks.into_iter().map(|(v, b)| (v, Box::new(b))).collect(),
                finally_block,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse do expression/block
    fn parse_do(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'do'

        // Do can take either a block or a string (filename)
        if self.peek_kind() == Some(TokenKind::LeftBrace) {
            // do { ... }
            let block = self.parse_block()?;
            let end = block.location.end;
            Ok(Node::new(NodeKind::Do { block: Box::new(block) }, SourceLocation { start, end }))
        } else {
            // do "filename" or do $expr
            let expr = self.parse_expression()?;
            let end = expr.location.end;
            Ok(Node::new(NodeKind::Do { block: Box::new(expr) }, SourceLocation { start, end }))
        }
    }

    /// Parse given statement
    fn parse_given_statement(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'given'

        // Parse the expression in parentheses
        self.expect(TokenKind::LeftParen)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        // Parse the body block
        let body = self.parse_given_block()?;
        let end = body.location.end;

        Ok(Node::new(
            NodeKind::Given { expr: Box::new(expr), body: Box::new(body) },
            SourceLocation { start, end },
        ))
    }

    /// Parse given block (which contains when/default statements)
    fn parse_given_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.expect(TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
            match self.peek_kind() {
                Some(TokenKind::When) => {
                    statements.push(self.parse_when_statement()?);
                }
                Some(TokenKind::Default) => {
                    statements.push(self.parse_default_statement()?);
                }
                _ => {
                    return Err(ParseError::syntax(
                        "Expected 'when' or 'default' in given block",
                        self.current_position(),
                    ));
                }
            }
        }

        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();

        Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
    }

    /// Parse when statement
    fn parse_when_statement(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'when'

        // Parse the condition in parentheses
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        // Parse the body block
        let body = self.parse_block()?;
        let end = body.location.end;

        Ok(Node::new(
            NodeKind::When { condition: Box::new(condition), body: Box::new(body) },
            SourceLocation { start, end },
        ))
    }

    /// Parse default statement
    fn parse_default_statement(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'default'

        // Parse the body block
        let body = self.parse_block()?;
        let end = body.location.end;

        Ok(Node::new(NodeKind::Default { body: Box::new(body) }, SourceLocation { start, end }))
    }

    /// Parse expression statement
    fn parse_expression_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Check for special blocks like AUTOLOAD and DESTROY
        if let Ok(token) = self.tokens.peek() {
            if matches!(token.text.as_str(), "AUTOLOAD" | "DESTROY" | "CLONE" | "CLONE_SKIP") {
                // Check if next token is a block
                if let Ok(second) = self.tokens.peek_second() {
                    if second.kind == TokenKind::LeftBrace {
                        return self.parse_special_block();
                    }
                }
            }
        }

        // First, try to parse the initial part as a simple statement
        let mut expr = self.parse_simple_statement()?;

        // Check for word operators (or, and, xor) which have very low precedence
        expr = self.parse_word_or_expr(expr)?;

        // Statement modifiers are handled at the statement level in parse_statement()

        let end = self.previous_position();

        // Wrap the expression in an ExpressionStatement node
        Ok(Node::new(
            NodeKind::ExpressionStatement { expression: Box::new(expr) },
            SourceLocation { start, end },
        ))
    }

    /// Parse simple statement (print, die, next, last, etc. with their arguments)
    fn parse_simple_statement(&mut self) -> ParseResult<Node> {
        // Check if it's a builtin that can take arguments without parens
        if let Ok(token) = self.tokens.peek() {
            match token.text.as_ref() {
                "print" | "say" | "die" | "warn" | "return" | "next" | "last" | "redo" | "open"
                | "tie" | "printf" | "close" | "pipe" | "sysopen" | "sysread" | "syswrite"
                | "truncate" | "fcntl" | "ioctl" | "flock" | "seek" | "tell" | "select"
                | "binmode" | "exec" | "system" | "bless" | "ref" | "defined" | "undef"
                | "keys" | "values" | "each" | "delete" | "exists" | "push" | "pop" | "shift"
                | "unshift" | "sort" | "map" | "grep" | "chomp" | "chop" | "split" | "join" => {
                    let start = token.start;
                    let func_name = token.text.clone();

                    // Check for indirect object syntax before consuming the token
                    if self.is_indirect_call_pattern(&func_name) {
                        return self.parse_indirect_call();
                    }

                    // Consume the function name token
                    self.consume_token()?;

                    // We're consuming the function name, no longer at statement start
                    self.mark_not_stmt_start();

                    // Check if there are arguments (not followed by semicolon or modifier)
                    match self.peek_kind() {
                        Some(TokenKind::Semicolon)
                        | Some(TokenKind::If)
                        | Some(TokenKind::Unless)
                        | Some(TokenKind::While)
                        | Some(TokenKind::Until)
                        | Some(TokenKind::For)
                        | Some(TokenKind::Foreach)
                        | Some(TokenKind::RightBrace)
                        | Some(TokenKind::Eof)
                        | None => {
                            // No arguments - return as function call with empty args
                            let end = self.previous_position();
                            Ok(Node::new(
                                NodeKind::FunctionCall { name: func_name, args: vec![] },
                                SourceLocation { start, end },
                            ))
                        }
                        _ => {
                            // Has arguments - parse them as a comma-separated list
                            let mut args = vec![];

                            // Parse first argument
                            // Special handling for open/pipe/socket/tie which can take my $var as first arg
                            if (func_name == "open"
                                || func_name == "pipe"
                                || func_name == "socket"
                                || func_name == "tie")
                                && self.peek_kind() == Some(TokenKind::My)
                            {
                                args.push(self.parse_variable_declaration()?);
                            } else if matches!(func_name.as_str(), "map" | "grep" | "sort")
                                && self.peek_kind() == Some(TokenKind::LeftBrace)
                            {
                                // Special handling for map/grep/sort with block first argument
                                args.push(self.parse_builtin_block()?);
                            } else {
                                // For builtins, don't parse word operators as part of arguments
                                // Word operators should be handled at statement level
                                args.push(self.parse_assignment()?);
                            }

                            // Parse remaining arguments
                            // For map/grep/sort, parse list arguments without requiring commas
                            if matches!(func_name.as_str(), "map" | "grep" | "sort") {
                                // Parse list arguments until statement boundary
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && !self.is_statement_modifier_keyword()
                                {
                                    // Skip optional comma
                                    if self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?;
                                    }
                                    args.push(self.parse_assignment()?);
                                }
                            } else {
                                // For other functions, require commas between arguments
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma

                                    // Check if we hit a statement modifier
                                    match self.peek_kind() {
                                        Some(TokenKind::If)
                                        | Some(TokenKind::Unless)
                                        | Some(TokenKind::While)
                                        | Some(TokenKind::Until)
                                        | Some(TokenKind::For)
                                        | Some(TokenKind::Foreach) => break,
                                        _ => args.push(self.parse_assignment()?),
                                    }
                                }
                            }

                            let end = args.last().map(|a| a.location.end).unwrap_or(start);

                            Ok(Node::new(
                                NodeKind::FunctionCall { name: func_name, args },
                                SourceLocation { start, end },
                            ))
                        }
                    }
                }
                "new" => {
                    // Check for indirect constructor syntax
                    let _start = token.start;
                    let func_name = token.text.clone();

                    if self.is_indirect_call_pattern(&func_name) {
                        return self.parse_indirect_call();
                    }

                    // Otherwise parse as regular expression
                    self.parse_expression()
                }
                _ => {
                    // Regular expression
                    self.parse_expression()
                }
            }
        } else {
            // Regular expression
            self.parse_expression()
        }
    }

    /// Parse statement modifier (if, unless, while, until, for)
    fn parse_statement_modifier(&mut self, statement: Node) -> ParseResult<Node> {
        let modifier_token = self.consume_token()?;
        let modifier = modifier_token.text.clone();

        // For 'for' and 'foreach', we parse a list expression
        let condition = if matches!(modifier_token.kind, TokenKind::For | TokenKind::Foreach) {
            self.parse_expression()?
        } else {
            // For other modifiers, parse a regular expression
            self.parse_expression()?
        };

        let start = statement.location.start;
        let end = condition.location.end;

        Ok(Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(statement),
                modifier,
                condition: Box::new(condition),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse a block statement
    fn parse_block(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let start = self.current_position();
        self.expect(TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
            let stmt = self.parse_statement()?;
            // Don't add empty blocks (from lone semicolons) to the statement list
            if !matches!(stmt.kind, NodeKind::Block { ref statements } if statements.is_empty()) {
                statements.push(stmt);
            }

            // parse_statement already invalidates peek, so we don't need to do it again

            // Swallow any stray semicolons before checking for the next statement or closing brace
            while self.peek_kind() == Some(TokenKind::Semicolon) {
                self.consume_token()?;
                self.tokens.invalidate_peek();
            }
        }

        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();

        let result = Node::new(NodeKind::Block { statements }, SourceLocation { start, end });
        self.exit_recursion();
        Ok(result)
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let result = self.parse_comma();
        self.exit_recursion();
        result
    }

    /// Parse comma operator (lowest precedence except for word operators)
    fn parse_comma(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_assignment()?;

        // In scalar context, comma creates a list
        // For now, we'll just parse it as sequential expressions
        // Also handle fat arrow (=>) which acts like comma
        if self.peek_kind() == Some(TokenKind::Comma)
            || self.peek_kind() == Some(TokenKind::FatArrow)
        {
            let mut expressions = vec![expr];
            let mut saw_fat_comma = false;

            // Handle initial fat arrow
            if self.peek_kind() == Some(TokenKind::FatArrow) {
                saw_fat_comma = true;
                self.tokens.next()?; // consume =>
                expressions.push(self.parse_assignment()?);
            }

            while self.peek_kind() == Some(TokenKind::Comma)
                || self.peek_kind() == Some(TokenKind::FatArrow)
            {
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                }

                // Check for end of expression
                match self.peek_kind() {
                    Some(TokenKind::Semicolon)
                    | Some(TokenKind::RightParen)
                    | Some(TokenKind::RightBrace)
                    | Some(TokenKind::RightBracket) => break,
                    _ => {}
                }

                let elem = self.parse_assignment()?;

                // Check for fat arrow after element
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    saw_fat_comma = true;
                    self.tokens.next()?; // consume =>
                    expressions.push(elem);

                    // Check again for end of expression
                    match self.peek_kind() {
                        Some(TokenKind::Semicolon)
                        | Some(TokenKind::RightParen)
                        | Some(TokenKind::RightBrace)
                        | Some(TokenKind::RightBracket) => break,
                        _ => expressions.push(self.parse_assignment()?),
                    }
                } else {
                    expressions.push(elem);
                }
            }

            // Convert to hash literal if we saw fat comma and have even number of elements
            let start = expressions[0].location.start;
            let end = expressions
                .last()
                .ok_or_else(|| ParseError::syntax("Empty expression list", start))?
                .location
                .end;
            expr = Self::build_list_or_hash(expressions, saw_fat_comma, start, end);
        }

        // Now handle word operators (or, xor, and, not) which have the lowest precedence
        expr = self.parse_word_or_expr(expr)?;

        Ok(expr)
    }

    /// Parse word or expression (or, xor) - takes an existing expr and applies word operators
    fn parse_word_or_expr(&mut self, mut expr: Node) -> ParseResult<Node> {
        // First handle 'and' which has higher precedence than 'or'/'xor'
        expr = self.parse_word_and_expr_with(expr)?;

        // Then handle 'or' and 'xor' which have lowest precedence
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::WordOr | TokenKind::WordXor => {
                    let op_token = self.tokens.next()?;
                    // Parse the right side as a full expression starting with assignment
                    let right = self.parse_assignment()?;
                    // Apply any 'and' operators to the right side
                    let right = self.parse_word_and_expr_with(right)?;

                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse word and expression with existing left side
    fn parse_word_and_expr_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::WordAnd) {
            let op_token = self.tokens.next()?;
            // Parse right side as a 'not' expression or assignment
            let right = self.parse_word_not_expr()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse word not expression - handles 'not' operator
    fn parse_word_not_expr(&mut self) -> ParseResult<Node> {
        if self.peek_kind() == Some(TokenKind::WordNot) {
            let op_token = self.tokens.next()?;
            let start = op_token.start;
            let operand = self.parse_word_not_expr()?;
            let end = operand.location.end;

            return Ok(Node::new(
                NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(operand) },
                SourceLocation { start, end },
            ));
        }

        // The right side of a word operator should be a full expression
        self.parse_assignment()
    }

    /// Parse assignment expression
    fn parse_assignment(&mut self) -> ParseResult<Node> {
        // Check if we have a 'not' operator first
        if self.peek_kind() == Some(TokenKind::WordNot) {
            return self.parse_word_not_expr();
        }

        // Handle 'return' as an expression in expression context
        // This allows patterns like: open $fh, $file or return;
        if self.peek_kind() == Some(TokenKind::Return) {
            return self.parse_return();
        }

        let mut expr = self.parse_ternary()?;

        // Check for assignment operators
        if let Some(kind) = self.peek_kind() {
            let op = match kind {
                TokenKind::Assign => Some("="),
                TokenKind::PlusAssign => Some("+="),
                TokenKind::MinusAssign => Some("-="),
                TokenKind::StarAssign => Some("*="),
                TokenKind::SlashAssign => Some("/="),
                TokenKind::PercentAssign => Some("%="),
                TokenKind::DotAssign => Some(".="),
                TokenKind::AndAssign => Some("&="),
                TokenKind::OrAssign => Some("|="),
                TokenKind::XorAssign => Some("^="),
                TokenKind::PowerAssign => Some("**="),
                TokenKind::LeftShiftAssign => Some("<<="),
                TokenKind::RightShiftAssign => Some(">>="),
                TokenKind::LogicalAndAssign => Some("&&="),
                TokenKind::LogicalOrAssign => Some("||="),
                TokenKind::DefinedOrAssign => Some("//="),
                _ => None,
            };

            if let Some(op) = op {
                self.tokens.next()?; // consume operator
                // The RHS can be a 'not' expression
                let rhs = if self.peek_kind() == Some(TokenKind::WordNot) {
                    self.parse_word_not_expr()?
                } else {
                    self.parse_assignment()?
                };
                let start = expr.location.start;
                let end = rhs.location.end;

                expr = Node::new(
                    NodeKind::Assignment {
                        lhs: Box::new(expr),
                        rhs: Box::new(rhs),
                        op: op.to_string(),
                    },
                    SourceLocation { start, end },
                );
            }
        }

        Ok(expr)
    }

    /// Parse ternary conditional expression
    /// Right-associative: `$a ? $b ? $c : $d : $e` parses as `$a ? ($b ? $c : $d) : $e`
    fn parse_ternary(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_or()?;

        if self.peek_kind() == Some(TokenKind::Question) {
            self.tokens.next()?; // consume ?
            // Allow nested ternary in then-branch for right associativity
            let then_expr = self.parse_ternary()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;

            let start = expr.location.start;
            let end = else_expr.location.end;

            expr = Node::new(
                NodeKind::Ternary {
                    condition: Box::new(expr),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse logical OR expression
    fn parse_or(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_and()?;

        while Self::is_logical_or(self.peek_kind()) {
            let op_token = self.tokens.next()?;
            let right = self.parse_and()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse logical AND expression
    fn parse_and(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_bitwise_or()?;

        while self.peek_kind() == Some(TokenKind::And) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_or()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse bitwise OR expression
    fn parse_bitwise_or(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_bitwise_xor()?;

        while self.peek_kind() == Some(TokenKind::BitwiseOr) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_xor()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse bitwise XOR expression
    fn parse_bitwise_xor(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_bitwise_and()?;

        while self.peek_kind() == Some(TokenKind::BitwiseXor) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_and()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse range expression
    fn parse_range(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_equality()?;

        while self.peek_kind() == Some(TokenKind::Range) {
            let op_token = self.tokens.next()?;
            let right = self.parse_equality()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse bitwise AND expression
    fn parse_bitwise_and(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_range()?;

        while self.peek_kind() == Some(TokenKind::BitwiseAnd) {
            let op_token = self.tokens.next()?;
            let right = self.parse_range()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse equality expression
    fn parse_equality(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_relational()?;

        while let Some(kind) = self.peek_kind() {
            match kind {
                // Handle word comparison operators (eq, ne, lt, le, gt, ge, cmp)
                TokenKind::Identifier => {
                    // Check if this is a word comparison operator
                    let next_text = self.tokens.peek()?.text.as_ref();
                    if matches!(next_text, "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "cmp") {
                        let op_token = self.tokens.next()?;
                        let right = self.parse_relational()?;
                        let start = expr.location.start;
                        let end = right.location.end;

                        expr = Node::new(
                            NodeKind::Binary {
                                op: op_token.text.clone(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end },
                        );
                    } else {
                        break;
                    }
                }
                TokenKind::Equal
                | TokenKind::NotEqual
                | TokenKind::Match
                | TokenKind::NotMatch
                | TokenKind::SmartMatch => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_relational()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    // Special handling for match operators with substitution/transliteration
                    if matches!(op_token.kind, TokenKind::Match | TokenKind::NotMatch) {
                        // Check if right side is already a substitution or transliteration
                        if let NodeKind::Substitution { pattern, replacement, modifiers, .. } =
                            &right.kind
                        {
                            // Update the expression in the substitution
                            expr = Node::new(
                                NodeKind::Substitution {
                                    expr: Box::new(expr),
                                    pattern: pattern.clone(),
                                    replacement: replacement.clone(),
                                    modifiers: modifiers.clone(),
                                },
                                SourceLocation { start, end },
                            );
                        } else if let NodeKind::Transliteration {
                            search, replace, modifiers, ..
                        } = &right.kind
                        {
                            // Update the expression in the transliteration
                            expr = Node::new(
                                NodeKind::Transliteration {
                                    expr: Box::new(expr),
                                    search: search.clone(),
                                    replace: replace.clone(),
                                    modifiers: modifiers.clone(),
                                },
                                SourceLocation { start, end },
                            );
                        } else if let NodeKind::Regex { pattern, replacement, modifiers } =
                            &right.kind
                        {
                            if let Some(replacement) = replacement {
                                let pat = if pattern.len() >= 2 {
                                    pattern[1..pattern.len() - 1].to_string()
                                } else {
                                    pattern.clone()
                                };
                                expr = Node::new(
                                    NodeKind::Substitution {
                                        expr: Box::new(expr),
                                        pattern: pat,
                                        replacement: replacement.clone(),
                                        modifiers: modifiers.clone(),
                                    },
                                    SourceLocation { start, end },
                                );
                            } else {
                                expr = Node::new(
                                    NodeKind::Match {
                                        expr: Box::new(expr),
                                        pattern: pattern.clone(),
                                        modifiers: modifiers.clone(),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        } else {
                            // Normal binary operation
                            expr = Node::new(
                                NodeKind::Binary {
                                    op: op_token.text.clone(),
                                    left: Box::new(expr),
                                    right: Box::new(right),
                                },
                                SourceLocation { start, end },
                            );
                        }
                    } else {
                        // Normal binary operation for == and !=
                        expr = Node::new(
                            NodeKind::Binary {
                                op: op_token.text.clone(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end },
                        );
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse relational expression
    fn parse_relational(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_shift()?;

        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Less
                | TokenKind::Greater
                | TokenKind::LessEqual
                | TokenKind::GreaterEqual
                | TokenKind::Spaceship
                | TokenKind::StringCompare => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_shift()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                TokenKind::Identifier => {
                    // Check if it's ISA operator
                    if self.tokens.peek()?.text == "ISA" {
                        let _op_token = self.tokens.next()?;
                        let right = self.parse_shift()?;
                        let start = expr.location.start;
                        let end = right.location.end;

                        expr = Node::new(
                            NodeKind::Binary {
                                op: "ISA".to_string(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end },
                        );
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse shift expression
    fn parse_shift(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_additive()?;

        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::LeftShift | TokenKind::RightShift => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_additive()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse additive expression
    fn parse_additive(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_multiplicative()?;

        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Plus | TokenKind::Minus | TokenKind::Dot => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_multiplicative()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse multiplicative expression
    fn parse_multiplicative(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_power()?;

        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_unary()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse power expression
    fn parse_power(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_unary()?;

        while self.peek_kind() == Some(TokenKind::Power) {
            let op_token = self.tokens.next()?;
            let right = self.parse_unary()?; // Right associative
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse unary expression
    fn parse_unary(&mut self) -> ParseResult<Node> {
        if let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Minus => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Check for file test operators (-e, -f, -d, etc.)
                    if let Some(TokenKind::Identifier) = self.peek_kind() {
                        let next_token = self.tokens.peek()?;
                        if next_token.text.len() == 1 {
                            // It's a file test operator
                            let test_token = self.tokens.next()?;
                            let file_test = format!("-{}", test_token.text);

                            // File test can be used without operand (tests $_)
                            let operand = if self.is_at_statement_end() {
                                // No operand, test $_
                                Node::new(
                                    NodeKind::Variable {
                                        sigil: "$".to_string(),
                                        name: "_".to_string(),
                                    },
                                    SourceLocation { start: test_token.end, end: test_token.end },
                                )
                            } else {
                                self.parse_unary()?
                            };

                            let end = operand.location.end;
                            return Ok(Node::new(
                                NodeKind::Unary { op: file_test, operand: Box::new(operand) },
                                SourceLocation { start, end },
                            ));
                        }
                    }

                    // Regular unary minus
                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Plus => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Special case: +{ ... } forces a hash constructor (not a block)
                    if self.peek_kind() == Some(TokenKind::LeftBrace) {
                        // Parse as hash literal
                        let hash = self.parse_hash_or_block()?;
                        let end = hash.location.end;

                        // Wrap the hash in a unary plus to preserve the explicit disambiguation
                        return Ok(Node::new(
                            NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(hash) },
                            SourceLocation { start, end },
                        ));
                    }

                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.clone(),
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end },
                                )),
                            },
                            SourceLocation { start, end },
                        ));
                    }

                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Not | TokenKind::Backslash | TokenKind::BitwiseNot | TokenKind::Star => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.clone(),
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end },
                                )),
                            },
                            SourceLocation { start, end },
                        ));
                    }

                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Increment | TokenKind::Decrement => {
                    // Pre-increment and pre-decrement
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;
                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::SmartMatch => {
                    // Smart match can be used as a unary operator
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.clone(),
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end },
                                )),
                            },
                            SourceLocation { start, end },
                        ));
                    }

                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                _ => {}
            }
        }

        self.parse_postfix()
    }

    /// Parse postfix expression
    fn parse_postfix(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek_kind() {
                Some(k) if Self::is_postfix_op(Some(k)) => {
                    let op_token = self.consume_token()?;
                    let start = expr.location.start;
                    let end = op_token.end;

                    expr = Node::new(
                        NodeKind::Unary { op: op_token.text.clone(), operand: Box::new(expr) },
                        SourceLocation { start, end },
                    );
                }

                Some(TokenKind::Arrow) => {
                    self.tokens.next()?; // consume ->

                    // Check for postfix dereference operators
                    match self.peek_kind() {
                        Some(TokenKind::ArraySigil) => {
                            // ->@* or ->@[...]
                            self.tokens.next()?; // consume @

                            if self.peek_kind() == Some(TokenKind::Star) {
                                // ->@*
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();

                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->@*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            } else if self.peek_kind() == Some(TokenKind::LeftBracket) {
                                // ->@[...] array slice
                                self.tokens.next()?; // consume [
                                let index = self.parse_expression()?;
                                self.expect(TokenKind::RightBracket)?;

                                let start = expr.location.start;
                                let end = self.previous_position();

                                // Represent as a special binary operation for array slice dereference
                                expr = Node::new(
                                    NodeKind::Binary {
                                        op: "->@[]".to_string(),
                                        left: Box::new(expr),
                                        right: Box::new(index),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(TokenKind::HashSigil) => {
                            // ->%* or ->%{...}
                            self.tokens.next()?; // consume %

                            if self.peek_kind() == Some(TokenKind::Star) {
                                // ->%*
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();

                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->%*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            } else if self.peek_kind() == Some(TokenKind::LeftBrace) {
                                // ->%{...} hash slice
                                self.tokens.next()?; // consume {
                                let key = self.parse_expression()?;
                                self.expect(TokenKind::RightBrace)?;

                                let start = expr.location.start;
                                let end = self.previous_position();

                                // Represent as a special binary operation for hash slice dereference
                                expr = Node::new(
                                    NodeKind::Binary {
                                        op: "->%{}".to_string(),
                                        left: Box::new(expr),
                                        right: Box::new(key),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(TokenKind::ScalarSigil) => {
                            // ->$*
                            self.tokens.next()?; // consume $

                            if self.peek_kind() == Some(TokenKind::Star) {
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();

                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->$*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(TokenKind::SubSigil | TokenKind::BitwiseAnd) => {
                            // ->&* (code dereference)
                            self.tokens.next()?; // consume &

                            if self.peek_kind() == Some(TokenKind::Star) {
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();

                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->&*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(TokenKind::Star) => {
                            // ->** (glob dereference)
                            self.tokens.next()?; // consume first *

                            if self.peek_kind() == Some(TokenKind::Star) {
                                self.tokens.next()?; // consume second *
                                let start = expr.location.start;
                                let end = self.previous_position();

                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->**".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(TokenKind::Identifier | TokenKind::Method) => {
                            // Method call
                            let method = self.tokens.next()?.text.clone();

                            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                                self.parse_args()?
                            } else {
                                Vec::new()
                            };

                            let start = expr.location.start;
                            let end = self.previous_position();

                            expr = Node::new(
                                NodeKind::MethodCall { object: Box::new(expr), method, args },
                                SourceLocation { start, end },
                            );
                        }

                        _ => {
                            // Just the arrow by itself - could be an error or incomplete
                            // For now, we'll leave expr unchanged
                        }
                    }
                }

                Some(TokenKind::LeftBracket) => {
                    // Array indexing - can be a single index or slice with multiple indices
                    self.tokens.next()?; // consume [

                    // Check if this might be a slice (multiple indices)
                    let mut indices = vec![self.parse_expression()?];

                    // Look for comma-separated indices
                    while self.peek_kind() == Some(TokenKind::Comma) {
                        self.consume_token()?; // consume comma
                        indices.push(self.parse_expression()?);
                    }

                    self.expect(TokenKind::RightBracket)?;

                    // Create the index node - either single index or array of indices
                    let index = if indices.len() == 1 {
                        indices.into_iter().next().ok_or_else(|| {
                            ParseError::syntax("Empty indices vector", expr.location.start)
                        })?
                    } else {
                        // Multiple indices - create an array literal node
                        let start = indices
                            .first()
                            .ok_or_else(|| {
                                ParseError::syntax("Empty indices vector", expr.location.start)
                            })?
                            .location
                            .start;
                        let end = indices
                            .last()
                            .ok_or_else(|| {
                                ParseError::syntax("Empty indices vector", expr.location.start)
                            })?
                            .location
                            .end;
                        Node::new(
                            NodeKind::ArrayLiteral { elements: indices },
                            SourceLocation { start, end },
                        )
                    };

                    let start = expr.location.start;
                    let end = self.previous_position();

                    // Represent as binary subscript operation
                    expr = Node::new(
                        NodeKind::Binary {
                            op: "[]".to_string(),
                            left: Box::new(expr),
                            right: Box::new(index),
                        },
                        SourceLocation { start, end },
                    );
                }

                Some(TokenKind::LeftBrace) => {
                    // Check if this is a builtin function that needs special handling
                    if let NodeKind::Identifier { name } = &expr.kind {
                        if Self::is_builtin_function(name) {
                            // This is a builtin function with {} as argument
                            // Parse arguments without parentheses
                            let mut args = Vec::new();

                            // Special handling for bless {} - parse it as a hash
                            if name == "bless" {
                                args.push(self.parse_hash_or_block()?);

                                // Parse remaining arguments separated by commas
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma
                                    if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_comma()?);
                                }
                            } else if matches!(name.as_str(), "sort" | "map" | "grep") {
                                // Parse block expression as first argument
                                let block_start = self.current_position();
                                self.expect(TokenKind::LeftBrace)?;

                                // Parse the expression inside the block (if any)
                                let mut statements = Vec::new();
                                if self.peek_kind() != Some(TokenKind::RightBrace) {
                                    statements.push(self.parse_expression()?);
                                }

                                self.expect(TokenKind::RightBrace)?;
                                let block_end = self.previous_position();

                                // Wrap the expression in a block node
                                let block = Node::new(
                                    NodeKind::Block { statements },
                                    SourceLocation { start: block_start, end: block_end },
                                );

                                args.push(block);

                                // Parse remaining arguments
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma
                                    if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_comma()?);
                                }
                            } else if matches!(name.as_str(), "sort" | "map" | "grep") {
                                // These builtins should parse {} as blocks, not hashes
                                args.push(self.parse_builtin_block()?);
                            } else {
                                // Other builtins - parse {} as first argument
                                args.push(self.parse_hash_or_block()?);

                                // Parse remaining arguments separated by commas
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma
                                    if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_comma()?);
                                }
                            }

                            let start = expr.location.start;

                            let end = args
                                .last()
                                .ok_or_else(|| ParseError::syntax("Empty arguments list", start))?
                                .location
                                .end;

                            expr = Node::new(
                                NodeKind::FunctionCall { name: name.clone(), args },
                                SourceLocation { start, end },
                            );
                            continue; // Continue the loop
                        }
                    }

                    // Hash element access
                    self.tokens.next()?; // consume {
                    let key = self.parse_expression()?;
                    self.expect(TokenKind::RightBrace)?;

                    let start = expr.location.start;
                    let end = self.previous_position();

                    // Represent as binary subscript operation
                    expr = Node::new(
                        NodeKind::Binary {
                            op: "{}".to_string(),
                            left: Box::new(expr),
                            right: Box::new(key),
                        },
                        SourceLocation { start, end },
                    );
                }

                Some(TokenKind::LeftParen) if matches!(&expr.kind, NodeKind::Identifier { .. }) => {
                    // Function call
                    if let NodeKind::Identifier { name } = &expr.kind {
                        let name = name.clone();

                        // Special handling for qw()
                        if name == "qw" {
                            let words = self.parse_qw_list()?;
                            let start = expr.location.start;
                            let end = self.previous_position();

                            expr = Node::new(
                                NodeKind::ArrayLiteral { elements: words },
                                SourceLocation { start, end },
                            );
                        } else {
                            let args = self.parse_args()?;
                            let start = expr.location.start;
                            let end = self.previous_position();

                            expr = Node::new(
                                NodeKind::FunctionCall { name, args },
                                SourceLocation { start, end },
                            );
                        }
                    }
                }

                _ => {
                    // Check if this is a builtin function that can take bare arguments
                    if let NodeKind::Identifier { name } = &expr.kind {
                        // Check for quote operators first
                        if matches!(name.as_str(), "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s") {
                            // This was already parsed as a quote operator in parse_primary
                            // Don't try to parse arguments
                        } else if Self::is_builtin_function(name) {
                            // Builtins always become function calls, even with no arguments
                            // This ensures they work correctly in expressions like "return $x or die"
                            if self.is_at_statement_end() {
                                // Bare builtin with no arguments
                                expr = Node::new(
                                    NodeKind::FunctionCall { name: name.clone(), args: vec![] },
                                    expr.location,
                                );
                            } else {
                                // Parse arguments without parentheses
                                let mut args = Vec::new();

                                // Special handling for sort, map, grep with block first argument
                                if matches!(name.as_str(), "sort" | "map" | "grep")
                                    && self.peek_kind() == Some(TokenKind::LeftBrace)
                                {
                                    // Parse block expression as first argument
                                    let block_start = self.current_position();
                                    self.expect(TokenKind::LeftBrace)?;

                                    // Parse the expression inside the block (if any)
                                    let mut statements = Vec::new();
                                    if self.peek_kind() != Some(TokenKind::RightBrace) {
                                        statements.push(self.parse_expression()?);
                                    }

                                    self.expect(TokenKind::RightBrace)?;
                                    let block_end = self.previous_position();

                                    // Wrap the expression in a block node
                                    let block = Node::new(
                                        NodeKind::Block { statements },
                                        SourceLocation { start: block_start, end: block_end },
                                    );

                                    args.push(block);

                                    // Parse remaining arguments for map/grep/sort without requiring commas
                                    // But respect statement boundaries including ] and )
                                    while !self.is_at_statement_end() {
                                        // Skip comma if present
                                        if self.peek_kind() == Some(TokenKind::Comma) {
                                            self.consume_token()?;
                                        }
                                        // Check again after potential comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else if name == "bless"
                                    && self.peek_kind() == Some(TokenKind::LeftBrace)
                                {
                                    // Special handling for bless {} - parse it as a hash
                                    args.push(self.parse_hash_or_block()?);

                                    // Parse remaining arguments separated by commas
                                    while self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?; // consume comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_assignment()?);
                                    }
                                } else {
                                    // Parse the first argument
                                    args.push(self.parse_ternary()?);

                                    // Parse remaining arguments separated by commas
                                    while self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?; // consume comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                }

                                let start = expr.location.start;

                                let end = args
                                    .last()
                                    .ok_or_else(|| {
                                        ParseError::syntax("Empty arguments list", start)
                                    })?
                                    .location
                                    .end;

                                expr = Node::new(
                                    NodeKind::FunctionCall { name: name.clone(), args },
                                    SourceLocation { start, end },
                                );
                            }
                        }
                    }
                    break;
                }
            }
        }

        Ok(expr)
    }

    /// Check if we're at a statement boundary
    fn is_at_statement_end(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Semicolon)
                | Some(TokenKind::RightBrace)
                | Some(TokenKind::RightParen)
                | Some(TokenKind::RightBracket)
                | Some(TokenKind::If)
                | Some(TokenKind::Unless)
                | Some(TokenKind::While)
                | Some(TokenKind::Until)
                | Some(TokenKind::For)
                | Some(TokenKind::Foreach)
                | Some(TokenKind::Eof)
                | None
        )
    }

    /// Parse quote operator (q, qq, qw, qr, qx)
    fn parse_quote_operator(&mut self) -> ParseResult<Node> {
        let op_token = self.consume_token()?; // consume q/qq/qw/qr/qx
        let start = op_token.start;
        let op = op_token.text.as_ref();

        // Get the delimiter - it might be a bracket token or other punctuation
        let delim_token = self.consume_token()?;
        let delim_char = match delim_token.kind {
            TokenKind::LeftBrace => '{',
            TokenKind::LeftBracket => '[',
            TokenKind::LeftParen => '(',
            TokenKind::Less => '<',
            _ => delim_token.text.chars().next().ok_or_else(|| {
                ParseError::syntax("Expected delimiter after quote operator", delim_token.start)
            })?,
        };

        // Determine closing delimiter
        let close_delim = match delim_char {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delim_char, // For other delimiters like / or |, use the same char
        };

        // Store delimiters for later use
        let opening_delim = delim_char;
        let closing_delim = close_delim;

        // Collect content until closing delimiter
        let mut content = String::new();
        let mut depth = 1;

        // For regex operators (m, s), we need to preserve the exact pattern
        let preserve_exact_content = matches!(op, "m" | "s" | "qr");

        while depth > 0 && !self.tokens.is_eof() {
            // Check token kind first
            let token_kind = self.peek_kind();

            // Check for matching delimiter tokens
            if matches!(delim_char, '{' | '[' | '(' | '<') {
                // Handle bracket-based delimiters
                match (delim_char, token_kind) {
                    ('{', Some(TokenKind::LeftBrace)) => {
                        self.consume_token()?;
                        content.push('{');
                        depth += 1;
                    }
                    ('{', Some(TokenKind::RightBrace)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push('}');
                        }
                    }
                    ('[', Some(TokenKind::LeftBracket)) => {
                        self.consume_token()?;
                        content.push('[');
                        depth += 1;
                    }
                    ('[', Some(TokenKind::RightBracket)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push(']');
                        }
                    }
                    ('(', Some(TokenKind::LeftParen)) => {
                        self.consume_token()?;
                        content.push('(');
                        depth += 1;
                    }
                    ('(', Some(TokenKind::RightParen)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push(')');
                        }
                    }
                    ('<', Some(TokenKind::Less)) => {
                        self.consume_token()?;
                        content.push('<');
                        depth += 1;
                    }
                    ('<', Some(TokenKind::Greater)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push('>');
                        }
                    }
                    _ => {
                        // Regular token, add to content
                        let token = self.consume_token()?;
                        content.push_str(&token.text);
                        if !preserve_exact_content && !self.tokens.is_eof() && !content.is_empty() {
                            content.push(' ');
                        }
                    }
                }
            } else {
                // For non-bracket delimiters, just look for the closing delimiter
                let token = self.consume_token()?;
                if token.text.contains(close_delim) {
                    let pos = token.text.find(close_delim).ok_or_else(|| {
                        ParseError::syntax("Closing delimiter not found in token", token.start)
                    })?;
                    content.push_str(&token.text[..pos]);
                    break;
                } else {
                    content.push_str(&token.text);
                    if !preserve_exact_content && !self.tokens.is_eof() {
                        content.push(' ');
                    }
                }
            }
        }

        // Parse modifiers for regex operators
        let mut modifiers = String::new();
        if matches!(op, "m" | "qr") {
            // Check for modifiers (letters after closing delimiter)
            while let Ok(token) = self.tokens.peek() {
                if token.kind == TokenKind::Identifier && token.text.len() == 1 {
                    // Single letter identifier could be a modifier
                    let ch =
                        token.text.chars().next().ok_or_else(|| {
                            ParseError::syntax("Empty identifier token", token.start)
                        })?;
                    if ch.is_ascii_alphabetic() {
                        modifiers.push(ch);
                        self.tokens.next()?;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        let mut end = self.previous_position();

        // Create appropriate node based on operator
        match op {
            "qq" => {
                // Double-quoted string with interpolation
                Ok(Node::new(
                    NodeKind::String { value: format!("\"{}\"", content), interpolated: true },
                    SourceLocation { start, end },
                ))
            }
            "q" => {
                // Single-quoted string without interpolation
                Ok(Node::new(
                    NodeKind::String { value: format!("'{}'", content), interpolated: false },
                    SourceLocation { start, end },
                ))
            }
            "qw" => {
                // Word list - split on whitespace
                let words: Vec<Node> = content
                    .split_whitespace()
                    .map(|word| {
                        Node::new(
                            NodeKind::String { value: format!("'{}'", word), interpolated: false },
                            SourceLocation { start, end },
                        )
                    })
                    .collect();

                Ok(Node::new(
                    NodeKind::ArrayLiteral { elements: words },
                    SourceLocation { start, end },
                ))
            }
            "qr" => {
                // Regular expression
                let mut modifiers = String::new();
                while let Ok(token) = self.tokens.peek() {
                    if token.kind == TokenKind::Identifier && token.text.len() == 1 {
                        let ch = token.text.chars().next().ok_or_else(|| {
                            ParseError::syntax("Empty identifier token", token.start)
                        })?;
                        if ch.is_ascii_alphabetic() {
                            modifiers.push(ch);
                            self.tokens.next()?;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                end = self.previous_position();
                Ok(Node::new(
                    NodeKind::Regex {
                        pattern: format!("{}{}{}", opening_delim, content, closing_delim),
                        replacement: None,
                        modifiers,
                    },
                    SourceLocation { start, end },
                ))
            }
            "qx" => {
                // Backticks/command execution
                Ok(Node::new(
                    NodeKind::String { value: format!("`{}`", content), interpolated: true },
                    SourceLocation { start, end },
                ))
            }
            "m" => {
                // Match operator with pattern
                let mut modifiers = String::new();
                while let Ok(token) = self.tokens.peek() {
                    if token.kind == TokenKind::Identifier && token.text.len() == 1 {
                        let ch = token.text.chars().next().ok_or_else(|| {
                            ParseError::syntax("Empty identifier token", token.start)
                        })?;
                        if ch.is_ascii_alphabetic() {
                            modifiers.push(ch);
                            self.tokens.next()?;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                end = self.previous_position();
                Ok(Node::new(
                    NodeKind::Regex {
                        pattern: format!("{}{}{}", opening_delim, content, closing_delim),
                        replacement: None,
                        modifiers,
                    },
                    SourceLocation { start, end },
                ))
            }
            "s" => {
                // Substitution operator shouldn't reach here - handled by TokenKind::Substitution
                // This is kept for defensive programming
                Err(ParseError::syntax(
                    "Substitution operator should be handled by TokenKind::Substitution",
                    start,
                ))
            }
            _ => Err(ParseError::syntax(format!("Unknown quote operator: {}", op), start)),
        }
    }

    /// Parse qualified identifier (may contain ::)
    fn parse_qualified_identifier(&mut self) -> ParseResult<Node> {
        let start_token = self.consume_token()?;
        let start = start_token.start;
        let mut name = if start_token.kind == TokenKind::DoubleColon {
            // Handle absolute path like ::Foo::Bar
            "::".to_string()
        } else {
            start_token.text.clone()
        };

        // Keep consuming :: and identifiers
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.consume_token()?; // consume ::
                name.push_str("::");
            } else if self.peek_kind() == Some(TokenKind::Colon) {
                // Handle two separate Colon tokens as ::
                self.consume_token()?; // consume first :
                self.consume_token()?; // consume second :
                name.push_str("::");
            }

            // In Perl, trailing :: is valid (e.g., Foo::Bar::)
            // Only consume identifier if there is one
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let next_part = self.consume_token()?;
                name.push_str(&next_part.text);
            }
            // No error for trailing :: - it's valid in Perl
        }

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Identifier { name }, SourceLocation { start, end }))
    }

    /// Check if an identifier is a builtin function that can take arguments without parens
    fn is_builtin_function(name: &str) -> bool {
        matches!(
            name,
            "print"
                | "say"
                | "die"
                | "warn"
                | "return"
                | "defined"
                | "undef"
                | "ref"
                | "chomp"
                | "chop"
                | "split"
                | "join"
                | "push"
                | "pop"
                | "shift"
                | "unshift"
                | "sort"
                | "map"
                | "grep"
                | "keys"
                | "values"
                | "each"
                | "delete"
                | "exists"
                | "open"
                | "close"
                | "read"
                | "write"
                | "printf"
                | "sprintf"
                | "exit"
                | "next"
                | "last"
                | "redo"
                | "goto"
                | "dump"
                | "caller"
                | "import"
                | "unimport"
                | "require"
                | "bless"
                | "tie"
                | "tied"
                | "untie"
                | "scalar"
                | "wantarray"
                // Math functions
                | "abs"
                | "atan2"
                | "cos"
                | "sin"
                | "exp"
                | "log"
                | "sqrt"
                | "rand"
                | "srand"
                | "int"
        )
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> ParseResult<Node> {
        let token = self.tokens.peek()?;
        let token_kind = token.kind;

        match token_kind {
            TokenKind::Number => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Number { value: token.text.clone() },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::String => {
                let token = self.tokens.next()?;
                // Check if it's a double-quoted string (interpolated)
                let interpolated = token.text.starts_with('"');
                Ok(Node::new(
                    NodeKind::String { value: token.text.clone(), interpolated },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Regex => {
                let token = self.tokens.next()?;
                let (pattern, modifiers) = quote_parser::extract_regex_parts(&token.text);
                Ok(Node::new(
                    NodeKind::Regex { pattern, replacement: None, modifiers },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::QuoteSingle | TokenKind::QuoteDouble => {
                let token = self.tokens.next()?;
                // Quote operators produce strings
                let interpolated = matches!(token.kind, TokenKind::QuoteDouble);
                Ok(Node::new(
                    NodeKind::String { value: token.text.clone(), interpolated },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::QuoteWords => {
                let token = self.tokens.next()?;
                let start = token.start;
                let text = token.text.as_str();

                // Parse qw(...) to extract words
                if let Some(content) = text.strip_prefix("qw") {
                    // Find the delimiter and extract content
                    let (content_str, _delimiter) = if let Some(rest) = content.strip_prefix('(') {
                        (rest.strip_suffix(')').unwrap_or(rest), '(')
                    } else if let Some(rest) = content.strip_prefix('[') {
                        (rest.strip_suffix(']').unwrap_or(rest), '[')
                    } else if let Some(rest) = content.strip_prefix('{') {
                        (rest.strip_suffix('}').unwrap_or(rest), '{')
                    } else if let Some(rest) = content.strip_prefix('<') {
                        (rest.strip_suffix('>').unwrap_or(rest), '<')
                    } else {
                        // Other delimiter - find matching pair
                        let delim = content.chars().next().unwrap_or(' ');
                        let inner = &content[delim.len_utf8()..];
                        let trimmed = inner.trim_end_matches(delim);
                        (trimmed, delim)
                    };

                    // Split into words
                    let words: Vec<Node> = content_str
                        .split_whitespace()
                        .map(|word| {
                            Node::new(
                                NodeKind::String { value: word.to_string(), interpolated: false },
                                SourceLocation { start, end: token.end },
                            )
                        })
                        .collect();

                    Ok(Node::new(
                        NodeKind::ArrayLiteral { elements: words },
                        SourceLocation { start, end: token.end },
                    ))
                } else {
                    // Fallback - shouldn't happen with proper lexer
                    Ok(Node::new(
                        NodeKind::String { value: token.text.clone(), interpolated: false },
                        SourceLocation { start, end: token.end },
                    ))
                }
            }

            TokenKind::QuoteCommand => {
                let token = self.tokens.next()?;
                // qx/backticks - for now treat as a string
                Ok(Node::new(
                    NodeKind::String { value: token.text.clone(), interpolated: true },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Substitution => {
                let token = self.tokens.next()?;
                // Use strict validation that rejects invalid modifiers
                let (pattern, replacement, modifiers) =
                    quote_parser::extract_substitution_parts_strict(&token.text).map_err(
                        |e| {
                            let message = match e {
                                quote_parser::SubstitutionError::InvalidModifier(c) => {
                                    format!(
                                        "Invalid substitution modifier '{}'. Valid modifiers are: g, i, m, s, x, o, e, r",
                                        c
                                    )
                                }
                                quote_parser::SubstitutionError::MissingDelimiter => {
                                    "Missing delimiter after 's'".to_string()
                                }
                                quote_parser::SubstitutionError::MissingPattern => {
                                    "Missing pattern in substitution".to_string()
                                }
                                quote_parser::SubstitutionError::MissingReplacement => {
                                    "Missing replacement in substitution".to_string()
                                }
                                quote_parser::SubstitutionError::MissingClosingDelimiter => {
                                    "Missing closing delimiter in substitution".to_string()
                                }
                            };
                            ParseError::SyntaxError {
                                message,
                                location: token.start,
                            }
                        },
                    )?;

                // Substitution as a standalone expression (will be used with =~ later)
                Ok(Node::new(
                    NodeKind::Substitution {
                        expr: Box::new(Node::new(
                            NodeKind::Identifier { name: String::from("$_") },
                            SourceLocation { start: token.start, end: token.start },
                        )),
                        pattern,
                        replacement,
                        modifiers,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Transliteration => {
                let token = self.tokens.next()?;
                let (search, replace, modifiers) =
                    quote_parser::extract_transliteration_parts(&token.text);

                // Transliteration as a standalone expression (will be used with =~ later)
                Ok(Node::new(
                    NodeKind::Transliteration {
                        expr: Box::new(Node::new(
                            NodeKind::Identifier { name: String::from("$_") },
                            SourceLocation { start: token.start, end: token.start },
                        )),
                        search,
                        replace,
                        modifiers,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::HeredocStart => {
                let start_token = self.tokens.next()?;
                let text = &start_token.text;
                let start = start_token.start;
                let end = start_token.end;

                // Parse heredoc delimiter from the token text
                let (delimiter, interpolated, indented) = parse_heredoc_delimiter(text);

                // Map interpolation to QuoteKind (check original text for quote style)
                let quote = map_heredoc_quote_kind(text, interpolated);

                // Enqueue for later content collection (Sprint A Day 4)
                self.push_heredoc_decl(delimiter.to_string(), indented, quote, start, end);
                self.byte_cursor = end;

                // Return declaration node (content will be attached in Day 5)
                Ok(Node::new(
                    NodeKind::Heredoc {
                        delimiter: delimiter.to_string(),
                        content: String::new(), // Placeholder until drain_pending_heredocs
                        interpolated,
                        indented,
                    },
                    SourceLocation { start, end },
                ))
            }

            TokenKind::Eval => self.parse_eval(),

            TokenKind::Do => self.parse_do(),

            // Note: TokenKind::Sub is handled in the keyword-as-identifier case below
            // This allows 'sub' to be used as a hash key or identifier in expressions
            TokenKind::Try => self.parse_try(),

            TokenKind::Less => {
                // Could be diamond operator <> or <FILEHANDLE>
                let start = self.consume_token()?.start; // consume <

                if self.peek_kind() == Some(TokenKind::Greater) {
                    // Diamond operator <>
                    self.consume_token()?; // consume >
                    let end = self.previous_position();
                    Ok(Node::new(NodeKind::Diamond, SourceLocation { start, end }))
                } else {
                    // Try to parse content until >
                    let mut pattern = String::new();
                    let mut has_glob_chars = false;

                    while self.peek_kind() != Some(TokenKind::Greater) && !self.tokens.is_eof() {
                        let token = self.consume_token()?;

                        // Check if this looks like a glob pattern
                        if token.text.contains('*')
                            || token.text.contains('?')
                            || token.text.contains('[')
                            || token.text.contains('.')
                        {
                            has_glob_chars = true;
                        }

                        pattern.push_str(&token.text);
                    }

                    if self.peek_kind() == Some(TokenKind::Greater) {
                        self.consume_token()?; // consume >
                        let end = self.previous_position();

                        if pattern.is_empty() {
                            // Empty <> is diamond operator
                            Ok(Node::new(NodeKind::Diamond, SourceLocation { start, end }))
                        } else if has_glob_chars || pattern.contains('/') {
                            // Looks like a glob pattern
                            Ok(Node::new(NodeKind::Glob { pattern }, SourceLocation { start, end }))
                        } else if pattern.chars().all(|c| c.is_uppercase() || c == '_') {
                            // Looks like a filehandle
                            Ok(Node::new(
                                NodeKind::Readline { filehandle: Some(pattern) },
                                SourceLocation { start, end },
                            ))
                        } else {
                            // Default to glob
                            Ok(Node::new(NodeKind::Glob { pattern }, SourceLocation { start, end }))
                        }
                    } else {
                        Err(ParseError::syntax(
                            "Expected '>' to close angle bracket construct",
                            self.current_position(),
                        ))
                    }
                }
            }

            TokenKind::Identifier => {
                // Check if it's a variable (starts with sigil)
                let token = self.tokens.peek()?;
                if token.text.starts_with('$')
                    || token.text.starts_with('@')
                    || token.text.starts_with('%')
                    || token.text.starts_with('&')
                {
                    self.parse_variable()
                } else if token.text.starts_with('*') && token.text.len() > 1 {
                    // Only treat * as a glob sigil if followed by identifier
                    self.parse_variable()
                } else {
                    // Check if it's a quote operator (q, qq, qw, qr, qx, m, s)
                    match token.text.as_ref() {
                        "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s" => self.parse_quote_operator(),
                        _ => {
                            // Regular identifier (possibly qualified with ::)
                            self.parse_qualified_identifier()
                        }
                    }
                }
            }

            // Handle sigil tokens (for when lexer sends them separately)
            TokenKind::ScalarSigil
            | TokenKind::ArraySigil
            | TokenKind::HashSigil
            | TokenKind::SubSigil
            | TokenKind::GlobSigil => self.parse_variable_from_sigil(),

            TokenKind::LeftParen => {
                let start_token = self.tokens.next()?; // consume (
                let start = start_token.start;

                // Check for empty list
                if self.peek_kind() == Some(TokenKind::RightParen) {
                    let end_token = self.tokens.next()?;
                    return Ok(Node::new(
                        NodeKind::ArrayLiteral { elements: vec![] },
                        SourceLocation { start, end: end_token.end },
                    ));
                }

                // Check if we might have a simple parenthesized expression
                // If there's no comma or fat arrow after the first element, parse the full expression
                // to handle operators like 'or', 'and' etc.
                let first = if self.peek_kind() == Some(TokenKind::RightParen) {
                    // Simple case - just one element
                    self.parse_assignment()?
                } else {
                    // Peek ahead to see if this is a list or a complex expression
                    let expr = self.parse_assignment()?;

                    // Check what comes after
                    match self.peek_kind() {
                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow) => {
                            // It's a list, continue with list parsing
                            expr
                        }
                        Some(TokenKind::RightParen) => {
                            // End of simple expression
                            expr
                        }
                        _ => {
                            // Could be an operator like 'or', 'and', etc.
                            // We need to continue parsing the expression
                            self.parse_word_or_expr(expr)?
                        }
                    }
                };

                if self.peek_kind() == Some(TokenKind::Comma)
                    || self.peek_kind() == Some(TokenKind::FatArrow)
                {
                    // It's a list
                    let mut elements = vec![first];
                    let mut saw_fat_comma = false;

                    // Handle fat arrow after first element
                    if self.peek_kind() == Some(TokenKind::FatArrow) {
                        saw_fat_comma = true;
                        self.tokens.next()?; // consume =>
                        elements.push(self.parse_assignment()?);
                    }

                    while self.peek_kind() == Some(TokenKind::Comma)
                        || self.peek_kind() == Some(TokenKind::FatArrow)
                    {
                        if self.peek_kind() == Some(TokenKind::Comma) {
                            self.consume_token()?; // consume comma
                        }

                        if self.peek_kind() == Some(TokenKind::RightParen) {
                            break;
                        }

                        let elem = self.parse_assignment()?;

                        // Check for fat arrow after element
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            saw_fat_comma = true;
                            self.consume_token()?; // consume =>
                            elements.push(elem);
                            if self.peek_kind() != Some(TokenKind::RightParen) {
                                elements.push(self.parse_assignment()?);
                            }
                        } else {
                            elements.push(elem);
                        }
                    }

                    self.expect(TokenKind::RightParen)?;
                    let end = self.previous_position();

                    // Only convert to hash if we saw a fat comma
                    Ok(Self::build_list_or_hash(elements, saw_fat_comma, start, end))
                } else {
                    // It's a parenthesized expression
                    self.expect(TokenKind::RightParen)?;
                    Ok(first)
                }
            }

            TokenKind::LeftBracket => {
                // Array literal
                let start_token = self.tokens.next()?; // consume [
                let start = start_token.start;

                let mut elements = Vec::new();

                while self.peek_kind() != Some(TokenKind::RightBracket) && !self.tokens.is_eof() {
                    elements.push(self.parse_expression()?);

                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.tokens.next()?;
                    } else {
                        break;
                    }
                }

                self.expect(TokenKind::RightBracket)?;
                let end = self.previous_position();

                Ok(Node::new(NodeKind::ArrayLiteral { elements }, SourceLocation { start, end }))
            }

            // Handle & as sigil when at primary position
            TokenKind::BitwiseAnd => {
                // This is a subroutine call or code dereference
                // Convert to SubSigil behavior
                self.parse_variable_from_sigil()
            }

            TokenKind::LeftBrace => {
                // Could be hash literal or block
                // Try to parse as hash literal first
                self.parse_hash_or_block()
            }

            TokenKind::Ellipsis => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Ellipsis,
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Undef => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Undef,
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            // Handle 'sub' specially - it might be an anonymous subroutine
            TokenKind::Sub => {
                // Check if this is an anonymous subroutine
                let next = self.peek_kind();
                if matches!(next, Some(k) if matches!(k, TokenKind::LeftBrace | TokenKind::LeftParen))
                {
                    // It's an anonymous subroutine
                    self.parse_subroutine()
                } else {
                    // It's used as an identifier
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end },
                    ))
                }
            }

            // Handle keywords that can be used as identifiers in certain contexts
            // Note: Statement-level keywords (if, unless, while, return, etc.) should NOT be here
            TokenKind::My
            | TokenKind::Our
            | TokenKind::Local
            | TokenKind::State
            | TokenKind::Package
            | TokenKind::Use
            | TokenKind::No
            | TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck
            | TokenKind::Given
            | TokenKind::When
            | TokenKind::Default
            | TokenKind::Catch
            | TokenKind::Finally
            | TokenKind::Continue
            | TokenKind::Class
            | TokenKind::Method
            | TokenKind::Format => {
                // In expression context, some keywords can be used as barewords/identifiers
                // This happens in hash keys, method names, etc.
                // But NOT for statement modifiers like if, unless, while, etc.
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Identifier { name: token.text.to_string() },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::DoubleColon => {
                // Absolute package path like ::Foo::Bar
                self.parse_qualified_identifier()
            }

            _ => {
                // Get position before consuming
                let pos = self.current_position();
                Err(ParseError::unexpected("expression", format!("{:?}", token_kind), pos))
            }
        }
    }

    /// Parse function arguments
    fn parse_args(&mut self) -> ParseResult<Vec<Node>> {
        self.expect(TokenKind::LeftParen)?;
        let mut args = Vec::new();

        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            // Use parse_assignment instead of parse_expression to avoid comma operator handling
            args.push(self.parse_assignment()?);

            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?;
            } else {
                break;
            }
        }

        self.expect(TokenKind::RightParen)?;
        Ok(args)
    }

    // Helper methods

    /// Peek at the next token's kind
    fn peek_kind(&mut self) -> Option<TokenKind> {
        self.tokens.peek().ok().map(|t| t.kind)
    }

    /// Peek at the next token without consuming it
    #[allow(dead_code)]
    fn peek_token(&mut self) -> ParseResult<&Token> {
        self.tokens.peek()
    }

    /// Check if we're at the start of a labeled statement (LABEL: ...)
    fn is_label_start(&mut self) -> bool {
        // We need an identifier followed by a colon
        if self.peek_kind() != Some(TokenKind::Identifier) {
            return false;
        }

        // Check if the second token is a colon
        if let Ok(second_token) = self.tokens.peek_second() {
            if second_token.kind == TokenKind::Colon {
                // To avoid conflict with qualified identifiers (Package::name),
                // we need to be more careful. A true label should be:
                // IDENTIFIER : STATEMENT
                // where STATEMENT doesn't start with another colon.
                //
                // For now, let's be conservative and disable label detection
                // when we see patterns that could be qualified identifiers.
                // This is a simple heuristic: if we see IDENTIFIER : and the
                // identifier looks like it could be a package name (starts with
                // uppercase), we'll let the expression parser handle it.
                if let Ok(first_token) = self.tokens.peek() {
                    let name = &first_token.text;
                    // If identifier starts with uppercase, it might be a package name
                    // so avoid treating it as a label
                    if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }

    /// Parse a labeled statement (LABEL: statement)
    fn parse_labeled_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Parse the label
        let label_token = self.expect(TokenKind::Identifier)?;
        let label = label_token.text.clone();

        // Consume the colon
        self.expect(TokenKind::Colon)?;

        // Parse the statement after the label
        let statement = Box::new(self.parse_statement()?);

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::LabeledStatement { label, statement },
            SourceLocation { start, end },
        ))
    }

    /// Check if the next token starts a variable
    fn is_variable_start(&mut self) -> bool {
        Self::is_variable_sigil(self.peek_kind())
    }

    /// Expect a specific token kind
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        let token = self.tokens.next()?;
        if token.kind != kind {
            return Err(ParseError::unexpected(
                format!("{:?}", kind),
                format!("{:?}", token.kind),
                token.start,
            ));
        }
        self.last_end_position = token.end;
        Ok(token)
    }

    /// Get current position
    fn current_position(&mut self) -> usize {
        self.tokens.peek().map(|t| t.start).unwrap_or(0)
    }

    /// Get previous position
    fn previous_position(&self) -> usize {
        self.last_end_position
    }

    /// Consume next token and track position
    fn consume_token(&mut self) -> ParseResult<Token> {
        let token = self.tokens.next()?;
        self.last_end_position = token.end;
        Ok(token)
    }

    /// Get closing delimiter for a given opening delimiter
    #[inline]
    fn closing_delim_for(open_txt: &str) -> Option<String> {
        // prefer textual comparison so we don't need to enumerate TokenKind variants
        match open_txt {
            "(" => Some(")".to_string()),
            "[" => Some("]".to_string()),
            "{" => Some("}".to_string()),
            "<" => Some(">".to_string()),
            // symmetric delimiters (| ! # ~ / etc.) close with themselves
            s if s.len() == 1 => Some(open_txt.to_string()),
            _ => None,
        }
    }

    /// After having consumed the `qw` identifier, parse `qw<delim>...<close>`
    fn parse_qw_words(&mut self) -> ParseResult<Vec<String>> {
        // Grab the opening delimiter as a single *token* (whatever it is).
        // This could be (, [, {, <, or any single character like |, !, #, etc.
        let open = self.tokens.next()?; // e.g., '(', '{', '|', '#', '!'
        let open_txt = open.text.as_str();

        // Special case for # - it causes lexer issues as it starts comments
        // When we see qw#, we need to consume carefully
        if open_txt == "#" {
            let mut words = Vec::<String>::new();

            // The lexer will treat the closing # as starting a comment,
            // so we won't see it as a token. We need to consume words
            // until we hit something that indicates the qw list is done.
            // We'll stop when we see a keyword that starts a new statement.
            while !self.tokens.is_eof() {
                let peek = self.tokens.peek()?;

                // Stop if we see a keyword that starts a new statement
                if matches!(
                    peek.kind,
                    TokenKind::Use
                        | TokenKind::My
                        | TokenKind::Our
                        | TokenKind::Sub
                        | TokenKind::Package
                        | TokenKind::If
                        | TokenKind::While
                        | TokenKind::For
                        | TokenKind::Return
                ) {
                    break;
                }

                // Also stop on semicolon (though we likely won't see it after #)
                if matches!(peek.kind, TokenKind::Semicolon) {
                    break;
                }

                match peek.kind {
                    TokenKind::Identifier | TokenKind::Number => {
                        // Check if this is a keyword that likely isn't part of the qw list
                        if matches!(peek.text.as_str(), "use" | "constant" | "my" | "our" | "sub") {
                            // Don't consume it, just stop here
                            break;
                        }
                        let t = self.tokens.next()?;
                        words.push(t.text.clone());
                    }
                    _ => {
                        // Skip other tokens
                        self.tokens.next()?;
                    }
                }
            }
            return Ok(words);
        }

        let close_txt = if let Some(ct) = Self::closing_delim_for(open_txt) {
            ct
        } else {
            // If we can't determine closing delimiter, use the same as opening for symmetric
            open_txt.to_string()
        };

        let mut words = Vec::<String>::new();

        // naive word split: treat IDENT/STRING/NUMBER as word atoms; anything else
        // (including newlines and whitespace that your lexer doesn't surface) just
        // acts as a separator or gets skipped.
        while !self.tokens.is_eof() {
            let peek = self.tokens.peek()?;
            if peek.text == close_txt.as_str() {
                self.tokens.next()?; // consume closer
                break;
            }

            match self.peek_kind() {
                Some(TokenKind::Identifier) | Some(TokenKind::Number) => {
                    let t = self.tokens.next()?;
                    words.push(t.text.clone());
                }
                Some(TokenKind::String) => {
                    let t = self.tokens.next()?;
                    // normalize quotes → word (qw() is non-interpolating as list of words)
                    let w = t.text.trim_matches(|c| c == '"' || c == '\'').to_string();
                    if !w.is_empty() {
                        words.push(w);
                    }
                }
                // Skip whitespace, newlines, and any other tokens
                _ => {
                    self.tokens.next()?;
                }
            }
        }
        Ok(words)
    }

    /// Parse qw() word list
    fn parse_qw_list(&mut self) -> ParseResult<Vec<Node>> {
        // Handle different delimiters for qw
        let delimiter_token = self.tokens.peek()?.clone();
        let close_delim = match delimiter_token.kind {
            TokenKind::LeftParen => {
                self.consume_token()?;
                TokenKind::RightParen
            }
            TokenKind::LeftBracket => {
                self.consume_token()?;
                TokenKind::RightBracket
            }
            TokenKind::LeftBrace => {
                self.consume_token()?;
                TokenKind::RightBrace
            }
            TokenKind::Less => {
                self.consume_token()?;
                TokenKind::Greater
            }
            // For other delimiters like |, !, #, ~, etc.
            _ => {
                // Try to consume whatever delimiter is there
                // For now, default to parentheses if we don't recognize it
                self.expect(TokenKind::LeftParen)?;
                TokenKind::RightParen
            }
        };

        let mut words = Vec::new();

        // Parse space-separated words until closing delimiter
        while self.peek_kind() != Some(close_delim) && !self.tokens.is_eof() {
            if let Some(TokenKind::Identifier) = self.peek_kind() {
                let token = self.tokens.next()?;
                words.push(Node::new(
                    NodeKind::String {
                        value: format!("'{}'", token.text), // qw produces single-quoted strings
                        interpolated: false,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ));
            } else if self.peek_kind() == Some(TokenKind::String) {
                // Also allow string tokens in qw lists
                let token = self.tokens.next()?;
                words.push(Node::new(
                    NodeKind::String {
                        value: format!("'{}'", token.text.trim_matches(|c| c == '"' || c == '\'')),
                        interpolated: false,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ));
            } else {
                // Skip other tokens (might be separators or special chars)
                self.tokens.next()?;
            }
        }

        self.expect(close_delim)?;
        Ok(words)
    }

    /// Parse block specifically for builtin functions (map, grep, sort)
    /// These always parse {} as blocks, never as hashes
    fn parse_builtin_block(&mut self) -> ParseResult<Node> {
        let start_token = self.tokens.next()?; // consume {
        let start = start_token.start;

        // Parse the expression inside the block (if any)
        let mut statements = Vec::new();
        if self.peek_kind() != Some(TokenKind::RightBrace) {
            statements.push(self.parse_expression()?);
        }

        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();

        // Always return a block node for builtin functions
        Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
    }

    /// Parse hash literal or block
    fn parse_hash_or_block(&mut self) -> ParseResult<Node> {
        self.parse_hash_or_block_with_context(false)
    }

    /// Parse hash literal or block with context about whether blocks are expected
    fn parse_hash_or_block_with_context(&mut self, _expect_block: bool) -> ParseResult<Node> {
        let start_token = self.tokens.next()?; // consume {
        let start = start_token.start;

        // Peek ahead to determine if it's a hash or block
        // For empty {}, decide based on context
        if self.peek_kind() == Some(TokenKind::RightBrace) {
            self.tokens.next()?; // consume }
            let end = self.previous_position();

            // For empty braces, default to hash (correct for most functions)
            // Functions like sort/map/grep have special handling that creates blocks
            return Ok(Node::new(
                NodeKind::HashLiteral { pairs: Vec::new() },
                SourceLocation { start, end },
            ));
        }

        // For non-empty braces, we need to check if it contains hash-like content
        // Save position to potentially backtrack
        let _saved_pos = self.current_position();

        // Try to parse as expression (which might be hash contents)
        let first_expr = match self.parse_expression() {
            Ok(expr) => expr,
            Err(_) => {
                // If we can't parse an expression, parse as block statements
                let mut statements = Vec::new();
                while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
                    statements.push(self.parse_statement()?);
                }

                self.expect(TokenKind::RightBrace)?;
                let end = self.previous_position();

                return Ok(Node::new(
                    NodeKind::Block { statements },
                    SourceLocation { start, end },
                ));
            }
        };

        // Check if we should close the brace now
        if self.peek_kind() == Some(TokenKind::RightBrace) {
            self.tokens.next()?; // consume }
            let end = self.previous_position();

            // Check if the expression is an array literal that should be a hash
            // This happens when parse_comma creates an array from key => value pairs
            if let NodeKind::ArrayLiteral { elements } = &first_expr.kind {
                // Check if this looks like hash pairs (even number of elements)
                if elements.len() % 2 == 0 && !elements.is_empty() {
                    // Convert array elements to hash pairs
                    let mut pairs = Vec::new();
                    for i in (0..elements.len()).step_by(2) {
                        pairs.push((elements[i].clone(), elements[i + 1].clone()));
                    }

                    return Ok(Node::new(
                        NodeKind::HashLiteral { pairs },
                        SourceLocation { start, end },
                    ));
                }
            }

            // If the expression is already a HashLiteral, return it directly
            // This happens when parse_comma creates a HashLiteral from key => value pairs
            if matches!(first_expr.kind, NodeKind::HashLiteral { .. }) {
                return Ok(first_expr);
            }

            // Otherwise it's a block with a single expression
            return Ok(Node::new(
                NodeKind::Block { statements: vec![first_expr] },
                SourceLocation { start, end },
            ));
        }

        // If there's more content, we need to determine if it's hash pairs or block statements
        let mut pairs = Vec::new();
        let mut _is_hash = false;

        // Check if next token is => or ,
        let next_kind = self.peek_kind();

        // Parse as hash if we see => or comma-separated pairs
        if matches!(next_kind, Some(k) if matches!(k, TokenKind::FatArrow | TokenKind::Comma)) {
            // Parse as hash
            _is_hash = true;

            if self.peek_kind() != /* ~ changed by cargo-mutants ~ */ Some(TokenKind::FatArrow) {
                // key => value pattern
                self.tokens.next()?; // consume =>
                let value = self.parse_expression()?;
                pairs.push((first_expr, value));
            } else if self.peek_kind() == Some(TokenKind::Comma) {
                // comma-separated pattern: key, value, key2, value2
                self.tokens.next()?; // consume comma

                if self.peek_kind() != Some(TokenKind::RightBrace) {
                    let second = self.parse_expression()?;
                    pairs.push((first_expr, second));
                } else {
                    // Trailing comma - treat as single element hash with undef value
                    let undef = Node::new(
                        NodeKind::Identifier { name: "undef".to_string() },
                        SourceLocation {
                            start: self.current_position(),
                            end: self.current_position(),
                        },
                    );
                    pairs.push((first_expr, undef));
                }
            }

            // Parse remaining pairs
            while self.peek_kind() == Some(TokenKind::Comma)
                || self.peek_kind() == Some(TokenKind::FatArrow)
            {
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                }

                if self.peek_kind() == Some(TokenKind::RightBrace) {
                    break;
                }

                let key = self.parse_expression()?;

                // Check for => or comma after key
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    self.tokens.next()?; // consume =>
                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                } else if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma

                    if self.peek_kind() == Some(TokenKind::RightBrace) {
                        // Odd number of elements - last one becomes undef value
                        let undef = Node::new(
                            NodeKind::Identifier { name: "undef".to_string() },
                            SourceLocation {
                                start: self.current_position(),
                                end: self.current_position(),
                            },
                        );
                        pairs.push((key, undef));
                        break;
                    }

                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                } else if self.peek_kind() == Some(TokenKind::RightBrace) {
                    // Key without value at end - add undef
                    let undef = Node::new(
                        NodeKind::Identifier { name: "undef".to_string() },
                        SourceLocation {
                            start: self.current_position(),
                            end: self.current_position(),
                        },
                    );
                    pairs.push((key, undef));
                    break;
                } else {
                    // No comma or => after key - might be missing
                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                }
            }

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            Ok(Node::new(NodeKind::HashLiteral { pairs }, SourceLocation { start, end }))
        } else {
            // Not a hash - parse as block
            if self.peek_kind() == Some(TokenKind::RightBrace) {
                // Single expression block
                self.tokens.next()?; // consume }
                let end = self.previous_position();

                return Ok(Node::new(
                    NodeKind::Block { statements: vec![first_expr] },
                    SourceLocation { start, end },
                ));
            }

            // Multiple statement block
            let mut statements = vec![first_expr];

            // Might need a semicolon
            if self.peek_kind() == Some(TokenKind::Semicolon) {
                self.tokens.next()?;
            }

            while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
                statements.push(self.parse_statement()?);
            }

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
        }
    }

    /// Check if the parenthesized content after sub name is a prototype (not a signature)
    #[allow(dead_code)]
    fn is_prototype(&mut self) -> bool {
        // Peek at the next token after (
        match self.tokens.peek_second() {
            Ok(token) => {
                // Check if it starts with prototype characters or looks like a prototype
                matches!(token.kind,
                    TokenKind::ScalarSigil | TokenKind::ArraySigil |
                    TokenKind::HashSigil | TokenKind::SubSigil |
                    TokenKind::Star | TokenKind::Semicolon |
                    TokenKind::Backslash) ||
                // Check for special vars that look like prototypes ($$, $#, etc)
                (token.kind == TokenKind::Identifier &&
                 token.text.chars().all(|c| matches!(c, '$' | '@' | '%' | '*' | '&' | ';' | '\\')))
            }
            Err(_) => false,
        }
    }

    /// Check if the parentheses likely contain a prototype rather than a signature
    fn is_likely_prototype(&mut self) -> ParseResult<bool> {
        // We need to peek past the opening paren without consuming
        // First, ensure we're at a left paren
        if self.tokens.peek()?.kind != TokenKind::LeftParen {
            return Ok(false);
        }

        // Use peek_second to look at the token after the paren
        match self.tokens.peek_second() {
            Ok(token) => {
                Ok(match token.kind {
                    // These are definitely prototype sigils
                    TokenKind::ScalarSigil
                    | TokenKind::ArraySigil
                    | TokenKind::HashSigil
                    | TokenKind::Star
                    | TokenKind::Backslash
                    | TokenKind::Semicolon
                    | TokenKind::BitwiseAnd
                    | TokenKind::GlobSigil => true,
                    // Empty prototype
                    TokenKind::RightParen => true,
                    // Colon indicates named parameter (:$foo), so it's a signature
                    TokenKind::Colon => false,
                    // Identifiers usually mean signature, but could be a special case
                    TokenKind::Identifier => {
                        // Check if it's a sigil-only identifier like "$" or "@"
                        // or the special underscore prototype
                        token.text == "_"
                            || token.text.chars().all(|c| matches!(c, '$' | '@' | '%' | '*' | '&'))
                    }
                    // Anything else suggests a signature
                    _ => false,
                })
            }
            Err(_) => Ok(false),
        }
    }

    /// Parse old-style prototype
    fn parse_prototype(&mut self) -> ParseResult<String> {
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut prototype = String::new();

        while !self.tokens.is_eof() {
            let token = self.tokens.next()?;

            match token.kind {
                TokenKind::RightParen => {
                    // End of prototype
                    break;
                }
                TokenKind::ScalarSigil => prototype.push('$'),
                TokenKind::ArraySigil => prototype.push('@'),
                TokenKind::HashSigil => prototype.push('%'),
                TokenKind::GlobSigil | TokenKind::Star => prototype.push('*'),
                TokenKind::SubSigil | TokenKind::BitwiseAnd => prototype.push('&'),
                TokenKind::Semicolon => prototype.push(';'),
                TokenKind::Backslash => prototype.push('\\'),
                _ => {
                    // For any other token, just add its text
                    // This handles cases where sigils might be parsed differently
                    prototype.push_str(&token.text);
                }
            }
        }

        Ok(prototype)
    }

    /// Utility to build either a HashLiteral or ArrayLiteral based on whether
    /// fat arrow (=>) was seen and we have an even number of elements
    fn build_list_or_hash(
        elements: Vec<Node>,
        saw_fat_arrow: bool,
        start: usize,
        end: usize,
    ) -> Node {
        if saw_fat_arrow && elements.len().is_multiple_of(2) {
            // Convert to HashLiteral
            let mut pairs = Vec::with_capacity(elements.len() / 2);
            for chunk in elements.chunks(2) {
                pairs.push((chunk[0].clone(), chunk[1].clone()));
            }
            Node::new(NodeKind::HashLiteral { pairs }, SourceLocation { start, end })
        } else {
            Node::new(NodeKind::ArrayLiteral { elements }, SourceLocation { start, end })
        }
    }
}

/// Parse heredoc delimiter from a string like "<<EOF", "<<'EOF'", "<<~EOF"
fn parse_heredoc_delimiter(s: &str) -> (&str, bool, bool) {
    let mut chars = s.chars();

    // Skip <<
    chars.next();
    chars.next();

    // Check for indented heredoc
    let indented = if chars.as_str().starts_with('~') {
        chars.next();
        true
    } else {
        false
    };

    let rest = chars.as_str().trim();

    // Check quoting to determine interpolation
    let (delimiter, interpolated) =
        if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
            // Double-quoted: interpolated
            (&rest[1..rest.len() - 1], true)
        } else if rest.starts_with('\'') && rest.ends_with('\'') && rest.len() >= 2 {
            // Single-quoted: not interpolated
            (&rest[1..rest.len() - 1], false)
        } else {
            // Bare word: interpolated
            (rest, true)
        };

    (delimiter, interpolated, indented)
}

/// Map heredoc delimiter text to collector QuoteKind (Sprint A Day 4)
fn map_heredoc_quote_kind(text: &str, _interpolated: bool) -> heredoc_collector::QuoteKind {
    // Skip << and optional ~
    let rest = text.trim_start_matches('<').trim_start_matches('~').trim();

    if rest.starts_with('\'') && rest.ends_with('\'') {
        heredoc_collector::QuoteKind::Single
    } else if rest.starts_with('"') && rest.ends_with('"') {
        heredoc_collector::QuoteKind::Double
    } else {
        // Bare word (unquoted)
        heredoc_collector::QuoteKind::Unquoted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable() {
        let mut parser = Parser::new("my $x = 42;");
        let result = parser.parse();
        assert!(result.is_ok());

        let ast = result.unwrap();
        println!("AST: {}", ast.to_sexp());
    }

    #[test]
    fn test_if_statement() {
        let mut parser = Parser::new("if ($x > 10) { print $x; }");
        let result = parser.parse();
        assert!(result.is_ok());

        let ast = result.unwrap();
        println!("AST: {}", ast.to_sexp());
    }

    #[test]
    fn test_function_definition() {
        let mut parser = Parser::new("sub greet { print \"Hello\"; }");
        let result = parser.parse();
        assert!(result.is_ok());

        let ast = result.unwrap();
        println!("AST: {}", ast.to_sexp());
    }

    #[test]
    fn test_list_declarations() {
        // Test simple list declaration
        let mut parser = Parser::new("my ($x, $y);");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("List declaration AST: {}", ast.to_sexp());

        // Test list declaration with initialization
        let mut parser = Parser::new("state ($a, $b) = (1, 2);");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("List declaration with init AST: {}", ast.to_sexp());

        // Test mixed sigils
        let mut parser = Parser::new("our ($scalar, @array, %hash);");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("Mixed sigils AST: {}", ast.to_sexp());

        // Test empty list
        let mut parser = Parser::new("my ();");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("Empty list AST: {}", ast.to_sexp());
    }

    #[test]
    fn test_qw_delimiters() {
        // Test qw with parentheses
        let mut parser = Parser::new("qw(one two three)");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(
            ast.to_sexp(),
            r#"(source_file (array (string "one") (string "two") (string "three")))"#
        );

        // Test qw with brackets
        let mut parser = Parser::new("qw[foo bar]");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.to_sexp(), r#"(source_file (array (string "foo") (string "bar")))"#);

        // Test qw with non-paired delimiters
        let mut parser = Parser::new("qw/alpha beta/");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.to_sexp(), r#"(source_file (array (string "alpha") (string "beta")))"#);

        // Test qw with exclamation marks
        let mut parser = Parser::new("qw!hello world!");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.to_sexp(), r#"(source_file (array (string "hello") (string "world")))"#);
    }

    #[test]
    fn test_block_vs_hash_context() {
        // Statement context: block containing hash
        let mut parser = Parser::new("{ key => 'value' }");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        // Statement context: block with hash inside
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(block (expression_statement (hash"),
            "Statement context should have block containing hash, got: {}",
            sexp
        );

        // Expression context: direct hash literal in assignment
        let mut parser = Parser::new("my $x = { key => 'value' }");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        // In expression context, should have hash
        let sexp = ast.to_sexp();
        assert!(sexp.contains("(hash"), "Expression context should have hash, got: {}", sexp);
        assert!(sexp.contains("my"), "Should have my declaration, got: {}", sexp);

        // Hash reference with parentheses
        let mut parser = Parser::new("$ref = ( a => 1, b => 2 )");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        // Parentheses with fat arrow should create hash
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(hash") || sexp.contains("(array"),
            "Should have hash or array, got: {}",
            sexp
        );
    }

    #[test]
    fn test_qualified_function_call() {
        let mut parser = Parser::new("return Data::Dumper::Dumper($param);");
        let result = parser.parse();
        match result {
            Ok(ast) => {
                println!("✅ Successfully parsed qualified function call: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Failed to parse qualified function call: {}", e);
                panic!("Parsing failed: {}", e);
            }
        }
    }
}
