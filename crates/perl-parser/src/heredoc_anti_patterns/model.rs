//! Shared anti-pattern data model types.

/// Source location of a detected anti-pattern.
///
/// All three coordinates are provided so callers can serve both LSP (line/column)
/// and byte-level (offset) consumers without re-computing positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    /// Zero-based line number within the scanned source fragment.
    pub line: usize,
    /// Zero-based column (byte offset from the start of the line).
    pub column: usize,
    /// Absolute byte offset from the start of the scanned source fragment.
    pub offset: usize,
}

/// Diagnostic severity level for a detected anti-pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    /// The construct will likely cause a runtime or parse failure.
    Error,
    /// The construct works but is fragile or difficult to analyze statically.
    Warning,
    /// The construct is valid but could be improved for readability or tooling support.
    Info,
}

/// A specific category of heredoc-related anti-pattern found in Perl source.
///
/// Each variant captures the [`Location`] of the offending construct plus any
/// context needed to produce a useful diagnostic message.
#[derive(Debug, Clone, PartialEq)]
pub enum AntiPattern {
    /// A heredoc declared inside a `format` body.
    FormatHeredoc { location: Location, format_name: String, heredoc_delimiter: String },
    /// A heredoc declared inside a `BEGIN { ... }` block, evaluated at compile time.
    BeginTimeHeredoc { location: Location, heredoc_content: String, side_effects: Vec<String> },
    /// A heredoc whose terminator is determined by a variable or expression at runtime.
    DynamicHeredocDelimiter { location: Location, expression: String },
    /// A `use Filter::*` statement that may rewrite source before static analysis runs.
    SourceFilterHeredoc { location: Location, module: String },
    /// A heredoc embedded inside a `(?{ ... })` regex code block.
    RegexCodeBlockHeredoc { location: Location },
    /// A heredoc embedded inside a string argument to `eval`.
    EvalStringHeredoc { location: Location },
    /// A heredoc written to a filehandle that has been `tie`d to a custom class.
    TiedHandleHeredoc { location: Location, handle_name: String },
}

/// A fully-formed diagnostic produced by the anti-pattern detector.
///
/// Contains everything needed to display a problem in an IDE or report:
/// the severity, the matched pattern (with location), a human-readable message,
/// a longer explanation, an optional suggested fix, and `perldoc` references.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// How serious the problem is.
    pub severity: Severity,
    /// The specific anti-pattern that triggered this diagnostic.
    pub pattern: AntiPattern,
    /// Short one-line summary suitable for an IDE problem marker.
    pub message: String,
    /// Longer explanation of why the construct is problematic.
    pub explanation: String,
    /// Optional concrete suggestion for fixing the problem.
    pub suggested_fix: Option<String>,
    /// Relevant `perldoc` pages or documentation references.
    pub references: Vec<String>,
}
