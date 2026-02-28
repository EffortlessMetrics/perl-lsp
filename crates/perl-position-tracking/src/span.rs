//! Byte-based span types for source location tracking.
//!
//! This module provides foundational span types used throughout the Perl LSP
//! ecosystem for tracking source locations. These types use byte offsets,
//! which are efficient for the parser but must be converted to line/character
//! positions for LSP communication.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

/// A byte-based span representing a range in source text.
///
/// `ByteSpan` uses byte offsets (not character or line positions) for precise
/// and efficient source location tracking. For LSP communication, use
/// [`WireRange`](crate::WireRange) or convert via [`LineStartsCache`](crate::LineStartsCache).
///
/// # Invariants
///
/// - `start <= end` (enforced by constructors, but not at type level for Copy)
/// - Both `start` and `end` are valid byte offsets in the source text
/// - Spans are half-open intervals: `[start, end)`
///
/// # Example
///
/// ```
/// use perl_position_tracking::ByteSpan;
///
/// let span = ByteSpan::new(0, 10);
/// assert_eq!(span.len(), 10);
/// assert!(!span.is_empty());
///
/// // Extract the spanned text
/// let source = "hello world";
/// let text = span.slice(source);
/// assert_eq!(text, "hello worl");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ByteSpan {
    /// Starting byte offset in the source text (inclusive)
    pub start: usize,
    /// Ending byte offset in the source text (exclusive)
    pub end: usize,
}

impl ByteSpan {
    /// Creates a new `ByteSpan` with the given start and end offsets.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `start > end`.
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "ByteSpan: start ({}) > end ({})", start, end);
        Self { start, end }
    }

    /// Creates an empty span at the given position.
    #[inline]
    pub const fn empty(pos: usize) -> Self {
        Self { start: pos, end: pos }
    }

    /// Creates a span covering the entire source text.
    #[inline]
    pub fn whole(source: &str) -> Self {
        Self { start: 0, end: source.len() }
    }

    /// Returns the length of this span in bytes.
    #[inline]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns true if this span is empty (start == end).
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns true if this span contains the given byte offset.
    #[inline]
    pub const fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Returns true if this span contains the given span entirely.
    #[inline]
    pub const fn contains_span(&self, other: ByteSpan) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Returns true if this span overlaps with the given span.
    #[inline]
    pub const fn overlaps(&self, other: ByteSpan) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Returns the intersection of two spans, or None if they don't overlap.
    pub fn intersection(&self, other: ByteSpan) -> Option<ByteSpan> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end { Some(ByteSpan { start, end }) } else { None }
    }

    /// Returns a new span that covers both this span and the given span.
    #[inline]
    pub fn union(&self, other: ByteSpan) -> ByteSpan {
        ByteSpan { start: self.start.min(other.start), end: self.end.max(other.end) }
    }

    /// Extracts the slice of source text covered by this span.
    ///
    /// # Panics
    ///
    /// Panics if the span is out of bounds for the source text.
    #[inline]
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    /// Safely extracts the slice of source text, returning None if out of bounds.
    #[inline]
    pub fn try_slice<'a>(&self, source: &'a str) -> Option<&'a str> {
        source.get(self.start..self.end)
    }

    /// Converts to a standard Range.
    #[inline]
    pub const fn to_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

impl fmt::Display for ByteSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<Range<usize>> for ByteSpan {
    #[inline]
    fn from(range: Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl From<ByteSpan> for Range<usize> {
    #[inline]
    fn from(span: ByteSpan) -> Self {
        span.start..span.end
    }
}

impl From<(usize, usize)> for ByteSpan {
    #[inline]
    fn from((start, end): (usize, usize)) -> Self {
        Self::new(start, end)
    }
}

impl From<ByteSpan> for (usize, usize) {
    #[inline]
    fn from(span: ByteSpan) -> Self {
        (span.start, span.end)
    }
}

/// Type alias for backward compatibility with `SourceLocation`.
///
/// New code should use [`ByteSpan`] directly.
pub type SourceLocation = ByteSpan;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_span_basics() {
        let span = ByteSpan::new(5, 10);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_empty_span() {
        let span = ByteSpan::empty(5);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 5);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }

    #[test]
    fn test_contains() {
        let span = ByteSpan::new(5, 10);
        assert!(!span.contains(4));
        assert!(span.contains(5));
        assert!(span.contains(9));
        assert!(!span.contains(10)); // end is exclusive
    }

    #[test]
    fn test_contains_span() {
        let outer = ByteSpan::new(0, 20);
        let inner = ByteSpan::new(5, 15);
        let partial = ByteSpan::new(15, 25);

        assert!(outer.contains_span(inner));
        assert!(!inner.contains_span(outer));
        assert!(!outer.contains_span(partial));
    }

    #[test]
    fn test_overlaps() {
        let a = ByteSpan::new(0, 10);
        let b = ByteSpan::new(5, 15);
        let c = ByteSpan::new(10, 20);
        let d = ByteSpan::new(15, 25);

        assert!(a.overlaps(b)); // partial overlap
        assert!(!a.overlaps(c)); // adjacent (no overlap)
        assert!(!a.overlaps(d)); // disjoint
    }

    #[test]
    fn test_intersection() {
        let a = ByteSpan::new(0, 10);
        let b = ByteSpan::new(5, 15);

        assert_eq!(a.intersection(b), Some(ByteSpan::new(5, 10)));
        assert_eq!(a.intersection(ByteSpan::new(10, 20)), None);
    }

    #[test]
    fn test_union() {
        let a = ByteSpan::new(0, 10);
        let b = ByteSpan::new(5, 15);

        assert_eq!(a.union(b), ByteSpan::new(0, 15));
    }

    #[test]
    fn test_slice() {
        let source = "hello world";
        let span = ByteSpan::new(0, 5);
        assert_eq!(span.slice(source), "hello");
    }

    #[test]
    fn test_conversions() {
        let span = ByteSpan::new(5, 10);

        // To/from Range
        let range: Range<usize> = span.into();
        assert_eq!(range, 5..10);
        let span2: ByteSpan = (5..10).into();
        assert_eq!(span, span2);

        // To/from tuple
        let tuple: (usize, usize) = span.into();
        assert_eq!(tuple, (5, 10));
        let span3: ByteSpan = (5, 10).into();
        assert_eq!(span, span3);
    }

    #[test]
    fn test_display() {
        let span = ByteSpan::new(5, 10);
        assert_eq!(format!("{}", span), "5..10");
    }
}
