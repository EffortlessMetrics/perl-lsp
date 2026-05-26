use std::{ops::Range, sync::Arc};

use crate::TokenKind;

/// Byte span carried by a [`Token`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan {
    /// Starting byte position.
    pub start: usize,
    /// Ending byte position.
    pub end: usize,
}

impl TokenSpan {
    /// Create a span from raw byte positions.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a span, returning an error when `end < start`.
    pub fn try_new(start: usize, end: usize) -> Result<Self, TokenSpanError> {
        if end < start {
            return Err(TokenSpanError::EndBeforeStart { start, end });
        }

        Ok(Self { start, end })
    }

    /// Span length in bytes.
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span length is zero bytes.
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Convert this span to a standard `Range`.
    pub const fn range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Return whether `offset` is inside this half-open span.
    ///
    /// The start is inclusive and the end is exclusive, matching Rust
    /// [`Range`] semantics. Empty spans contain no offsets.
    pub const fn contains(self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Return whether `offset` touches this span, including the end boundary.
    ///
    /// This is useful for cursor-oriented callers that need positions at token
    /// boundaries to resolve to the adjacent token. Empty spans touch exactly
    /// their single boundary offset.
    pub const fn touches(self, offset: usize) -> bool {
        self.start <= offset && offset <= self.end
    }

    /// Return whether this span overlaps `other`.
    ///
    /// Spans are treated as half-open byte ranges, so adjacent spans such as
    /// `0..2` and `2..4` do not overlap. Empty spans never overlap.
    pub const fn overlaps(self, other: Self) -> bool {
        !self.is_empty() && !other.is_empty() && self.start < other.end && other.start < self.end
    }

    /// Return the smallest span covering both spans.
    pub const fn cover(self, other: Self) -> Self {
        Self { start: min_usize(self.start, other.start), end: max_usize(self.end, other.end) }
    }
}

const fn min_usize(left: usize, right: usize) -> usize {
    if left <= right { left } else { right }
}

const fn max_usize(left: usize, right: usize) -> usize {
    if left >= right { left } else { right }
}

/// Error type for checked token/span constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSpanError {
    /// End offset is before start offset.
    EndBeforeStart {
        /// Start byte offset that was supplied.
        start: usize,
        /// End byte offset that violated `end >= start`.
        end: usize,
    },
    /// Empty span is only valid for EOF or explicit synthetic tokens.
    EmptySpanNotAllowed {
        /// Token kind that disallows an empty span.
        kind: TokenKind,
        /// Byte offset where the empty span was constructed.
        at: usize,
    },
}

impl std::fmt::Display for TokenSpanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndBeforeStart { start, end } => {
                write!(f, "token span invariant violated: end ({end}) < start ({start})")
            }
            Self::EmptySpanNotAllowed { kind, at } => {
                write!(f, "empty span not allowed for token kind {kind:?} at byte {at}")
            }
        }
    }
}

impl std::error::Error for TokenSpanError {}

#[inline]
const fn allows_empty_span(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Eof | TokenKind::Unknown)
}

#[inline]
fn validate_non_empty_span(
    kind: TokenKind,
    start: usize,
    is_empty: bool,
) -> Result<(), TokenSpanError> {
    if is_empty && !allows_empty_span(kind) {
        return Err(TokenSpanError::EmptySpanNotAllowed { kind, at: start });
    }

    Ok(())
}

/// Borrowed view over token data for allocation-sensitive paths.
///
/// Unlike [`Token`], this type borrows source text and does not allocate.
/// Convert to [`Token`] explicitly with [`TokenRef::to_owned_token`] or `From`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRef<'src> {
    /// Token classification for parser decision making
    pub kind: TokenKind,
    /// Borrowed source text slice
    pub text: &'src str,
    /// Starting byte position for error reporting and location tracking
    pub start: usize,
    /// Ending byte position for span calculation and navigation
    pub end: usize,
}

impl<'src> TokenRef<'src> {
    /// Create a borrowed token view with the given kind, source text, and byte span.
    pub fn new(kind: TokenKind, text: &'src str, start: usize, end: usize) -> Self {
        Self { kind, text, start, end }
    }

    /// Create a borrowed token view with checked span ordering.
    ///
    /// Unlike [`TokenRef::new`], this rejects spans where `end < start`.
    pub fn try_new(
        kind: TokenKind,
        text: &'src str,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let span = TokenSpan::try_new(start, end)?;
        Ok(Self { kind, text, start: span.start, end: span.end })
    }

    /// Create a borrowed token view while enforcing span invariants.
    ///
    /// Rules:
    /// - `start <= end`
    /// - zero-length spans are accepted for EOF and explicit synthetic unknown tokens
    pub fn new_checked(
        kind: TokenKind,
        text: &'src str,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let token = Self::try_new(kind, text, start, end)?;
        validate_non_empty_span(token.kind, token.start, token.is_empty())?;

        Ok(token)
    }

    /// Return the token span length in bytes.
    pub fn len(self) -> usize {
        TokenSpan::new(self.start, self.end).len()
    }

    /// Return whether the token span is empty.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Return the token span as `(start, end)`.
    pub fn span(self) -> (usize, usize) {
        (self.start, self.end)
    }

    /// Return a human-readable display name for this token.
    pub fn display_name(self) -> &'static str {
        self.kind.display_name()
    }

    /// Convert this borrowed token view into an owned [`Token`].
    pub fn to_owned_token(self) -> Token {
        Token::new(self.kind, self.text, self.start, self.end)
    }
}

/// Token produced by the lexer and consumed by the parser.
///
/// Stores the token kind, original source text, and byte span. The text is kept
/// in an `Arc<str>` so buffering and lookahead can clone tokens cheaply.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token classification for parser decision making
    pub kind: TokenKind,
    /// Original source text for precise reconstruction
    pub text: Arc<str>,
    /// Starting byte position for error reporting and location tracking
    pub start: usize,
    /// Ending byte position for span calculation and navigation
    pub end: usize,
}

impl Token {
    /// Create a new token with the given kind, source text, and byte span.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::{Token, TokenKind};
    ///
    /// let tok = Token::new(TokenKind::Sub, "sub", 0, 3);
    /// assert_eq!(tok.kind, TokenKind::Sub);
    /// assert_eq!(&*tok.text, "sub");
    /// ```
    pub fn new(kind: TokenKind, text: impl Into<Arc<str>>, start: usize, end: usize) -> Self {
        Token { kind, text: text.into(), start, end }
    }

    /// Create a token with checked span ordering.
    ///
    /// Unlike [`Token::new`], this rejects spans where `end < start`.
    pub fn try_new(
        kind: TokenKind,
        text: impl Into<Arc<str>>,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let span = TokenSpan::try_new(start, end)?;
        Ok(Self { kind, text: text.into(), start: span.start, end: span.end })
    }

    /// Create a token while enforcing span invariants.
    ///
    /// Rules:
    /// - `start <= end`
    /// - zero-length spans are accepted for EOF and explicit synthetic unknown tokens
    pub fn new_checked(
        kind: TokenKind,
        text: impl Into<Arc<str>>,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let token = Self::try_new(kind, text, start, end)?;
        validate_non_empty_span(token.kind, token.start, token.is_empty())?;

        Ok(token)
    }

    /// Create an EOF token at `pos`.
    pub fn eof_at(pos: usize) -> Self {
        Self::new(TokenKind::Eof, "", pos, pos)
    }

    /// Create an unknown (synthetic) token at `start..end`.
    pub fn unknown_at(text: impl Into<Arc<str>>, start: usize, end: usize) -> Self {
        let bounded_end = end.max(start);
        Self::new(TokenKind::Unknown, text, start, bounded_end)
    }

    /// Return this token's byte span.
    pub fn span(&self) -> TokenSpan {
        TokenSpan::new(self.start, self.end)
    }

    /// Return this token's byte span as `Range<usize>`.
    pub fn range(&self) -> Range<usize> {
        self.span().range()
    }

    /// Clone this token with a new checked span.
    pub fn with_span(&self, start: usize, end: usize) -> Result<Self, TokenSpanError> {
        Self::new_checked(self.kind, self.text.clone(), start, end)
    }

    /// Clone this token with a new token kind.
    pub fn with_kind(&self, kind: TokenKind) -> Self {
        Self::new(kind, self.text.clone(), self.start, self.end)
    }

    /// Return the token span length in bytes.
    ///
    /// This uses saturating subtraction so malformed spans (where `end < start`)
    /// are treated as zero-length instead of underflowing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::{Token, TokenKind};
    ///
    /// let tok = Token::new(TokenKind::Identifier, "foo", 10, 13);
    /// assert_eq!(tok.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.span().len()
    }

    /// Return whether the token span is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::{Token, TokenKind};
    ///
    /// let tok = Token::new(TokenKind::Eof, "", 8, 8);
    /// assert!(tok.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return a human-readable display name for this token.
    pub fn display_name(&self) -> &'static str {
        self.kind.display_name()
    }

    /// Return a borrowed token view over this token.
    pub fn as_ref_token(&self) -> TokenRef<'_> {
        TokenRef { kind: self.kind, text: self.text.as_ref(), start: self.start, end: self.end }
    }
}

impl From<TokenRef<'_>> for Token {
    fn from(value: TokenRef<'_>) -> Self {
        value.to_owned_token()
    }
}
