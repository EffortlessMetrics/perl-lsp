//! Stack trace handling for Perl DAP
//!
//! This crate provides types and utilities for parsing and managing stack traces
//! in the Debug Adapter Protocol (DAP) format for Perl debugging.
//!
//! # Overview
//!
//! The crate provides:
//!
//! - [`StackFrame`] - Represents a single stack frame
//! - [`StackTraceProvider`] - Trait for stack trace retrieval
//! - [`PerlStackParser`] - Parser for Perl debugger stack output
//! - [`FrameClassifier`] - Classifies frames as user code vs library code
//!
//! # Example
//!
//! ```rust
//! use perl_dap_stack::{StackFrame, Source, PerlStackParser};
//!
//! let mut parser = PerlStackParser::new();
//! let output = "  #0  main::foo at /path/script.pl line 42";
//!
//! if let Some(frame) = parser.parse_frame(output, 0) {
//!     assert_eq!(frame.name, "main::foo");
//!     assert_eq!(frame.line, 42);
//! }
//! ```

mod classifier;
mod parser;

pub use classifier::{FrameCategory, FrameClassifier, PerlFrameClassifier};
pub use parser::{PerlStackParser, StackParseError};

use serde::{Deserialize, Serialize};

/// Represents a stack frame in the call stack.
///
/// This struct follows the DAP specification for stack frames and includes
/// all necessary information for debugger navigation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    /// Unique identifier for this frame within the debug session
    pub id: i64,

    /// The name of the frame (typically the function name)
    pub name: String,

    /// The source file associated with this frame
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,

    /// The 1-based line number in the source file
    pub line: i64,

    /// The 1-based column number (defaults to 1)
    pub column: i64,

    /// The optional end line (for multi-line frames)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i64>,

    /// The optional end column
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i64>,

    /// Whether the frame can be restarted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_restart: Option<bool>,

    /// Presentation hint for UI rendering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<StackFramePresentationHint>,

    /// Module information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
}

impl StackFrame {
    /// Creates a new stack frame with the given ID, name, and location.
    #[must_use]
    pub fn new(id: i64, name: impl Into<String>, source: Option<Source>, line: i64) -> Self {
        Self {
            id,
            name: name.into(),
            source,
            line,
            column: 1,
            end_line: None,
            end_column: None,
            can_restart: None,
            presentation_hint: None,
            module_id: None,
        }
    }

    /// Creates a stack frame for a Perl subroutine.
    #[must_use]
    pub fn for_subroutine(id: i64, package: &str, sub_name: &str, file: &str, line: i64) -> Self {
        let name = if package.is_empty() || package == "main" {
            sub_name.to_string()
        } else {
            format!("{}::{}", package, sub_name)
        };

        Self::new(id, name, Some(Source::new(file)), line)
    }

    /// Sets the column for this frame.
    #[must_use]
    pub fn with_column(mut self, column: i64) -> Self {
        self.column = column;
        self
    }

    /// Sets the end position for this frame.
    #[must_use]
    pub fn with_end(mut self, end_line: i64, end_column: i64) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }

    /// Sets the presentation hint for this frame.
    #[must_use]
    pub fn with_presentation_hint(mut self, hint: StackFramePresentationHint) -> Self {
        self.presentation_hint = Some(hint);
        self
    }

    /// Sets the module ID for this frame.
    #[must_use]
    pub fn with_module(mut self, module_id: impl Into<String>) -> Self {
        self.module_id = Some(module_id.into());
        self
    }

    /// Returns the full qualified name of this frame.
    #[must_use]
    pub fn qualified_name(&self) -> &str {
        &self.name
    }

    /// Returns the file path if available.
    #[must_use]
    pub fn file_path(&self) -> Option<&str> {
        self.source.as_ref().and_then(|s| s.path.as_deref())
    }

    /// Returns true if this frame represents user code (not library/core).
    #[must_use]
    pub fn is_user_code(&self) -> bool {
        self.presentation_hint.as_ref() != Some(&StackFramePresentationHint::Subtle)
    }
}

impl Default for StackFrame {
    fn default() -> Self {
        Self::new(0, "<unknown>", None, 0)
    }
}

/// Presentation hints for stack frame display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StackFramePresentationHint {
    /// Normal frame (user code)
    Normal,
    /// Label frame (e.g., exception handler)
    Label,
    /// Subtle frame (library code, typically collapsed)
    Subtle,
}

/// Represents a source file in the debugging context.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// The short name of the source file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The full path to the source file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// A reference ID for retrieving source content dynamically
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i64>,

    /// The origin of the source (e.g., "eval", "require")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,

    /// Presentation hint for the source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<SourcePresentationHint>,
}

impl Source {
    /// Creates a new source from a file path.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        let path_str = path.into();
        let name =
            std::path::Path::new(&path_str).file_name().and_then(|n| n.to_str()).map(String::from);

        Self {
            name,
            path: Some(path_str),
            source_reference: None,
            origin: None,
            presentation_hint: None,
        }
    }

    /// Creates a source with a dynamic reference (no file path).
    #[must_use]
    pub fn from_reference(reference: i64, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            path: None,
            source_reference: Some(reference),
            origin: None,
            presentation_hint: None,
        }
    }

    /// Sets the origin for this source.
    #[must_use]
    pub fn with_origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }

    /// Sets the presentation hint.
    #[must_use]
    pub fn with_presentation_hint(mut self, hint: SourcePresentationHint) -> Self {
        self.presentation_hint = Some(hint);
        self
    }

    /// Returns true if this source is from an eval.
    #[must_use]
    pub fn is_eval(&self) -> bool {
        self.origin.as_deref() == Some("eval")
            || self.path.as_ref().is_some_and(|p| p.contains("(eval"))
    }

    /// Returns true if this source has a file on disk.
    #[must_use]
    pub fn has_file(&self) -> bool {
        self.path.is_some() && !self.is_eval()
    }
}

/// Presentation hints for source display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourcePresentationHint {
    /// Normal source file
    Normal,
    /// Emphasize this source (e.g., current file)
    Emphasize,
    /// Deemphasize this source (e.g., library code)
    Deemphasize,
}

/// Trait for providing stack traces.
///
/// Implementations of this trait retrieve stack trace information from
/// a debugging session.
pub trait StackTraceProvider {
    /// The error type for stack trace retrieval.
    type Error;

    /// Gets the current stack trace.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - The thread to get the stack trace for
    /// * `start_frame` - The starting frame index (0-based)
    /// * `levels` - Maximum number of frames to return (None = all)
    ///
    /// # Returns
    ///
    /// A vector of stack frames, ordered from innermost (current) to outermost.
    fn get_stack_trace(
        &self,
        thread_id: i64,
        start_frame: usize,
        levels: Option<usize>,
    ) -> Result<Vec<StackFrame>, Self::Error>;

    /// Gets the total number of frames in the stack.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - The thread to query
    fn total_frames(&self, thread_id: i64) -> Result<usize, Self::Error>;

    /// Gets a single frame by ID.
    ///
    /// # Arguments
    ///
    /// * `frame_id` - The frame identifier
    fn get_frame(&self, frame_id: i64) -> Result<Option<StackFrame>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_frame_new() {
        let frame = StackFrame::new(1, "main::foo", Some(Source::new("/path/to/file.pl")), 42);

        assert_eq!(frame.id, 1);
        assert_eq!(frame.name, "main::foo");
        assert_eq!(frame.line, 42);
        assert_eq!(frame.column, 1);
        assert!(frame.source.is_some());
    }

    #[test]
    fn test_stack_frame_for_subroutine() {
        let frame =
            StackFrame::for_subroutine(1, "My::Package", "do_stuff", "/lib/My/Package.pm", 100);

        assert_eq!(frame.name, "My::Package::do_stuff");
        assert_eq!(frame.line, 100);
        assert_eq!(frame.file_path(), Some("/lib/My/Package.pm"));
    }

    #[test]
    fn test_stack_frame_for_main() {
        let frame = StackFrame::for_subroutine(1, "main", "run", "/script.pl", 10);

        assert_eq!(frame.name, "run");
    }

    #[test]
    fn test_stack_frame_with_presentation_hint() {
        let frame = StackFrame::new(1, "foo", None, 1)
            .with_presentation_hint(StackFramePresentationHint::Subtle);

        assert_eq!(frame.presentation_hint, Some(StackFramePresentationHint::Subtle));
        assert!(!frame.is_user_code());
    }

    #[test]
    fn test_source_new() {
        let source = Source::new("/path/to/file.pm");

        assert_eq!(source.path, Some("/path/to/file.pm".to_string()));
        assert_eq!(source.name, Some("file.pm".to_string()));
    }

    #[test]
    fn test_source_is_eval() {
        let eval_source = Source::new("(eval 42)");
        assert!(eval_source.is_eval());

        let file_source = Source::new("/path/to/file.pl");
        assert!(!file_source.is_eval());

        let origin_eval = Source::new("/path/to/file.pl").with_origin("eval");
        assert!(origin_eval.is_eval());
    }

    #[test]
    fn test_source_has_file() {
        let file_source = Source::new("/path/to/file.pl");
        assert!(file_source.has_file());

        let eval_source = Source::new("(eval 42)");
        assert!(!eval_source.has_file());

        let ref_source = Source::from_reference(1, "dynamic");
        assert!(!ref_source.has_file());
    }

    #[test]
    fn test_source_from_reference() {
        let source = Source::from_reference(42, "eval code");

        assert_eq!(source.source_reference, Some(42));
        assert_eq!(source.name, Some("eval code".to_string()));
        assert!(source.path.is_none());
    }
}
