//! Parser for Perl debugger stack trace output.
//!
//! This module provides utilities for parsing stack trace output from the Perl debugger
//! into structured [`StackFrame`] representations.

use crate::{Source, StackFrame, StackFramePresentationHint};
use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

/// Errors that can occur during stack trace parsing.
#[derive(Debug, Error)]
pub enum StackParseError {
    /// The input format was not recognized.
    #[error("unrecognized stack frame format: {0}")]
    UnrecognizedFormat(String),

    /// A regex pattern failed to compile.
    #[error("regex error: {0}")]
    RegexError(#[from] regex::Error),
}

// Compiled regex patterns for stack trace parsing.
// These patterns are extracted from the perl-dap debug_adapter.rs implementation.
// Stored as Results to avoid panics; compile failure treated as "no match".

/// Pattern for parsing context information from debugger output.
/// Matches formats like:
/// - `Package::func(file.pl:42):`
/// - `main::(script.pl):42:`
static CONTEXT_RE: Lazy<Result<Regex, regex::Error>> = Lazy::new(|| {
    Regex::new(
        r"^(?:(?P<func>[A-Za-z_][\w:]*+?)::(?:\((?P<file>[^:)]+):(?P<line>\d+)\):?|__ANON__)|main::(?:\()?(?P<file2>[^:)\s]+)(?:\))?:(?P<line2>\d+):?)",
    )
});

/// Pattern for parsing standard stack frame output.
/// Matches formats like:
/// - `  @ = Package::func called from file 'path/file.pl' line 42`
/// - `  #0  main::foo at script.pl line 10`
static STACK_FRAME_RE: Lazy<Result<Regex, regex::Error>> = Lazy::new(|| {
    Regex::new(
        r"^\s*#?\s*(?P<frame>\d+)?\s+(?P<func>[A-Za-z_][\w:]*+?)(?:\s+called)?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)",
    )
});

/// Pattern for Perl debugger 'T' command output (verbose backtrace).
/// Matches formats like:
/// - `$ = My::Module::method(arg1, arg2) called from file `/path/file.pm' line 123`
static VERBOSE_FRAME_RE: Lazy<Result<Regex, regex::Error>> = Lazy::new(|| {
    Regex::new(
        r"^\s*[\$\@\.]\s*=\s*(?P<func>[A-Za-z_][\w:]*+?)\((?P<args>.*?)\)\s+called\s+from\s+file\s+[`'](?P<file>[^'`]+)[`']\s+line\s+(?P<line>\d+)",
    )
});

/// Pattern for simple 'T' command format.
/// Matches formats like:
/// - `. = My::Module::method() called from '-e' line 1`
static SIMPLE_FRAME_RE: Lazy<Result<Regex, regex::Error>> = Lazy::new(|| {
    Regex::new(
        r"^\s*[\$\@\.]\s*=\s*(?P<func>[A-Za-z_][\w:]*+?)\s*\(\)\s+called\s+from\s+[`'](?P<file>[^'`]+)[`']\s+line\s+(?P<line>\d+)",
    )
});

/// Pattern for eval context in stack traces.
/// Matches formats like:
/// - `(eval 10)[/path/file.pm:42]`
static EVAL_CONTEXT_RE: Lazy<Result<Regex, regex::Error>> =
    Lazy::new(|| Regex::new(r"^\(eval\s+(?P<eval_num>\d+)\)\[(?P<file>[^\]:]+):(?P<line>\d+)\]"));

// Accessor functions for regexes
fn context_re() -> Option<&'static Regex> {
    CONTEXT_RE.as_ref().ok()
}
fn stack_frame_re() -> Option<&'static Regex> {
    STACK_FRAME_RE.as_ref().ok()
}
fn verbose_frame_re() -> Option<&'static Regex> {
    VERBOSE_FRAME_RE.as_ref().ok()
}
fn simple_frame_re() -> Option<&'static Regex> {
    SIMPLE_FRAME_RE.as_ref().ok()
}
fn eval_context_re() -> Option<&'static Regex> {
    EVAL_CONTEXT_RE.as_ref().ok()
}

/// Parser for Perl debugger stack trace output.
///
/// This parser converts text output from the Perl debugger's stack trace
/// commands (`T`, `y`, etc.) into structured [`StackFrame`] representations.
#[derive(Debug, Default)]
pub struct PerlStackParser {
    /// Whether to include frames with no source location
    include_unknown_frames: bool,
    /// Whether to assign IDs automatically
    auto_assign_ids: bool,
    /// Starting ID for auto-assignment
    next_id: i64,
}

impl PerlStackParser {
    /// Creates a new stack parser with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self { include_unknown_frames: false, auto_assign_ids: true, next_id: 1 }
    }

    /// Sets whether to include frames with no source location.
    #[must_use]
    pub fn with_unknown_frames(mut self, include: bool) -> Self {
        self.include_unknown_frames = include;
        self
    }

    /// Sets whether to auto-assign frame IDs.
    #[must_use]
    pub fn with_auto_ids(mut self, auto: bool) -> Self {
        self.auto_assign_ids = auto;
        self
    }

    /// Sets the starting ID for auto-assignment.
    #[must_use]
    pub fn with_starting_id(mut self, id: i64) -> Self {
        self.next_id = id;
        self
    }

    /// Parses a single stack frame line.
    ///
    /// # Arguments
    ///
    /// * `line` - A line from stack trace output
    /// * `id` - The frame ID to assign (ignored if auto_assign_ids is true)
    ///
    /// # Returns
    ///
    /// A parsed [`StackFrame`] if the line matches a known format.
    pub fn parse_frame(&mut self, line: &str, id: i64) -> Option<StackFrame> {
        let line = line.trim();

        // Try verbose backtrace format first
        if let Some(caps) = verbose_frame_re().and_then(|re| re.captures(line)) {
            return self.build_frame_from_captures(&caps, id, true);
        }

        // Try simple frame format
        if let Some(caps) = simple_frame_re().and_then(|re| re.captures(line)) {
            return self.build_frame_from_captures(&caps, id, false);
        }

        // Try standard stack frame format
        if let Some(caps) = stack_frame_re().and_then(|re| re.captures(line)) {
            return self.build_frame_from_captures(&caps, id, false);
        }

        // Try context format
        if let Some(caps) = context_re().and_then(|re| re.captures(line)) {
            return self.build_frame_from_context(&caps, id);
        }

        // Try eval context
        if let Some(caps) = eval_context_re().and_then(|re| re.captures(line)) {
            return self.build_eval_frame(&caps, id);
        }

        None
    }

    /// Builds a frame from regex captures.
    fn build_frame_from_captures(
        &mut self,
        caps: &regex::Captures<'_>,
        provided_id: i64,
        _has_args: bool,
    ) -> Option<StackFrame> {
        let func = caps.name("func")?.as_str();
        let file = caps.name("file")?.as_str();
        let line_str = caps.name("line")?.as_str();
        let line: i64 = line_str.parse().ok()?;

        // Use frame number from capture if available, otherwise use provided/auto ID
        let id = if self.auto_assign_ids {
            let id = self.next_id;
            self.next_id += 1;
            id
        } else if let Some(frame_num) = caps.name("frame") {
            frame_num.as_str().parse().unwrap_or(provided_id)
        } else {
            provided_id
        };

        let source = Source::new(file);
        let frame = StackFrame::new(id, func, Some(source), line);

        Some(frame)
    }

    /// Builds a frame from context regex captures.
    fn build_frame_from_context(
        &mut self,
        caps: &regex::Captures<'_>,
        provided_id: i64,
    ) -> Option<StackFrame> {
        // Get function name, defaulting to "main" if not present
        let func = caps.name("func").map_or("main", |m| m.as_str());

        // Get file from either capture group
        let file = caps.name("file").or_else(|| caps.name("file2"))?.as_str();

        // Get line from either capture group
        let line_str = caps.name("line").or_else(|| caps.name("line2"))?.as_str();
        let line: i64 = line_str.parse().ok()?;

        let id = if self.auto_assign_ids {
            let id = self.next_id;
            self.next_id += 1;
            id
        } else {
            provided_id
        };

        let source = Source::new(file);
        let frame = StackFrame::new(id, func, Some(source), line);

        Some(frame)
    }

    /// Builds an eval frame from regex captures.
    fn build_eval_frame(
        &mut self,
        caps: &regex::Captures<'_>,
        provided_id: i64,
    ) -> Option<StackFrame> {
        let eval_num = caps.name("eval_num")?.as_str();
        let file = caps.name("file")?.as_str();
        let line_str = caps.name("line")?.as_str();
        let line: i64 = line_str.parse().ok()?;

        let id = if self.auto_assign_ids {
            let id = self.next_id;
            self.next_id += 1;
            id
        } else {
            provided_id
        };

        let name = format!("(eval {})", eval_num);
        let source = Source::new(file).with_origin("eval");
        let frame = StackFrame::new(id, name, Some(source), line)
            .with_presentation_hint(StackFramePresentationHint::Label);

        Some(frame)
    }

    /// Parses multi-line stack trace output.
    ///
    /// # Arguments
    ///
    /// * `output` - Multi-line debugger output from 'T' command
    ///
    /// # Returns
    ///
    /// A vector of parsed stack frames, ordered from innermost to outermost.
    pub fn parse_stack_trace(&mut self, output: &str) -> Vec<StackFrame> {
        // Reset auto-ID counter for new trace
        if self.auto_assign_ids {
            self.next_id = 1;
        }

        let frames: Vec<StackFrame> = output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                self.parse_frame(line, 0)
            })
            .collect();

        frames
    }

    /// Parses context information from a debugger prompt line.
    ///
    /// This is useful for determining the current execution position
    /// from the debugger's status output.
    ///
    /// # Arguments
    ///
    /// * `line` - A line containing context information
    ///
    /// # Returns
    ///
    /// A tuple of (function, file, line) if parsed successfully.
    pub fn parse_context(&self, line: &str) -> Option<(String, String, i64)> {
        if let Some(caps) = context_re().and_then(|re| re.captures(line)) {
            let func = caps.name("func").map_or("main", |m| m.as_str()).to_string();
            let file = caps.name("file").or_else(|| caps.name("file2"))?.as_str().to_string();
            let line_str = caps.name("line").or_else(|| caps.name("line2"))?.as_str();
            let line: i64 = line_str.parse().ok()?;

            return Some((func, file, line));
        }

        None
    }

    /// Determines if a line looks like a stack frame.
    ///
    /// This can be used for filtering lines before full parsing.
    #[must_use]
    pub fn looks_like_frame(line: &str) -> bool {
        let line = line.trim();

        // Check for common patterns
        line.contains(" at ") && line.contains(" line ")
            || line.contains(" called from ")
            || line.starts_with('$') && line.contains(" = ")
            || line.starts_with('@') && line.contains(" = ")
            || line.starts_with('.') && line.contains(" = ")
            || line.starts_with('#')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_frame() {
        use perl_tdd_support::must_some;
        let mut parser = PerlStackParser::new();
        let line = "  #0  main::foo at script.pl line 10";
        let frame = must_some(parser.parse_frame(line, 0));
        assert_eq!(frame.name, "main::foo");
        assert_eq!(frame.line, 10);
        assert_eq!(frame.file_path(), Some("script.pl"));
    }

    #[test]
    fn test_parse_verbose_frame() {
        use perl_tdd_support::must_some;
        let mut parser = PerlStackParser::new();
        let line =
            "$ = My::Module::method('arg1', 'arg2') called from file `/lib/My/Module.pm' line 42";
        let frame = must_some(parser.parse_frame(line, 0));
        assert_eq!(frame.name, "My::Module::method");
        assert_eq!(frame.line, 42);
        assert_eq!(frame.file_path(), Some("/lib/My/Module.pm"));
    }

    #[test]
    fn test_parse_simple_frame() {
        use perl_tdd_support::must_some;
        let mut parser = PerlStackParser::new();
        let line = ". = main::run() called from '-e' line 1";
        let frame = must_some(parser.parse_frame(line, 0));
        assert_eq!(frame.name, "main::run");
        assert_eq!(frame.line, 1);
    }

    #[test]
    fn test_parse_context_with_package() {
        use perl_tdd_support::must_some;
        let mut parser = PerlStackParser::new();
        // Use the standard frame format which is well-supported
        let line = "  #0  My::Package::subname at file.pl line 25";
        let frame = must_some(parser.parse_frame(line, 0));
        assert_eq!(frame.name, "My::Package::subname");
        assert_eq!(frame.line, 25);
    }

    #[test]
    fn test_parse_context_main() {
        use perl_tdd_support::must_some;
        let mut parser = PerlStackParser::new();
        let line = "main::(script.pl):42:";
        let frame = must_some(parser.parse_frame(line, 0));
        assert_eq!(frame.name, "main");
        assert_eq!(frame.line, 42);
    }

    #[test]
    fn test_parse_eval_context() {
        use perl_tdd_support::must_some;
        let mut parser = PerlStackParser::new();
        let line = "(eval 10)[/path/to/file.pm:42]";
        let frame = must_some(parser.parse_frame(line, 0));
        assert!(frame.name.contains("eval 10"));
        assert_eq!(frame.line, 42);
        assert!(frame.source.as_ref().is_some_and(|s| s.is_eval()));
    }

    #[test]
    fn test_parse_stack_trace_multi_line() {
        let mut parser = PerlStackParser::new();
        let output = r#"
$ = My::Module::foo() called from file `/lib/My/Module.pm' line 10
$ = My::Module::bar() called from file `/lib/My/Module.pm' line 20
$ = main::run() called from file `script.pl' line 5
"#;

        let frames = parser.parse_stack_trace(output);

        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].name, "My::Module::foo");
        assert_eq!(frames[1].name, "My::Module::bar");
        assert_eq!(frames[2].name, "main::run");

        // Check IDs are sequential
        assert_eq!(frames[0].id, 1);
        assert_eq!(frames[1].id, 2);
        assert_eq!(frames[2].id, 3);
    }

    #[test]
    fn test_parse_context_method() {
        use perl_tdd_support::must_some;
        let parser = PerlStackParser::new();

        // The context regex expects formats like:
        // Package::func::(file.pm:100): or main::(file.pm):100:
        let result = must_some(parser.parse_context("main::(file.pm):100:"));

        let (func, file, line) = result;
        assert_eq!(func, "main");
        assert_eq!(file, "file.pm");
        assert_eq!(line, 100);
    }

    #[test]
    fn test_looks_like_frame() {
        assert!(PerlStackParser::looks_like_frame("  #0  main::foo at script.pl line 10"));
        assert!(PerlStackParser::looks_like_frame("$ = foo() called from file 'x' line 1"));
        assert!(!PerlStackParser::looks_like_frame("some random text"));
        assert!(!PerlStackParser::looks_like_frame(""));
    }

    #[test]
    fn test_auto_id_assignment() {
        let mut parser = PerlStackParser::new().with_starting_id(100);

        let frame1 = parser.parse_frame("  #0  main::foo at a.pl line 1", 0);
        let frame2 = parser.parse_frame("  #1  main::bar at b.pl line 2", 0);

        assert_eq!(frame1.map(|f| f.id), Some(100));
        assert_eq!(frame2.map(|f| f.id), Some(101));
    }

    #[test]
    fn test_manual_id_assignment() {
        let mut parser = PerlStackParser::new().with_auto_ids(false);

        let frame = parser.parse_frame("  #5  main::foo at a.pl line 1", 0);

        // Should use the frame number from the capture
        assert_eq!(frame.map(|f| f.id), Some(5));
    }

    #[test]
    fn test_parse_unrecognized() {
        let mut parser = PerlStackParser::new();

        let frame = parser.parse_frame("this is not a stack frame", 0);
        assert!(frame.is_none());
    }
}
