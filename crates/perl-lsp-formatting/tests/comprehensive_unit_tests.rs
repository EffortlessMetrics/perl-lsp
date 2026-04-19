//! Comprehensive unit tests for perl-lsp-formatting crate.
//!
//! Tests cover:
//! - FormattingProvider construction and configuration
//! - format_document with various inputs and mock responses
//! - format_range with various ranges and edge cases
//! - FormattingError variants
//! - Edge cases: empty documents, unchanged formatting, error paths

use std::sync::Arc;

use perl_lsp_formatting::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingError,
    FormattingOptions, FormattingProvider,
};
use perl_lsp_tooling::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::{must, must_err};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_options() -> FormattingOptions {
    FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    }
}

fn tab_options() -> FormattingOptions {
    FormattingOptions {
        tab_size: 8,
        insert_spaces: false,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(false),
    }
}

fn make_provider(runtime: MockSubprocessRuntime) -> FormattingProvider<MockSubprocessRuntime> {
    FormattingProvider::new(runtime)
}

/// Shared wrapper around `MockSubprocessRuntime` so the test can inspect
/// invocations *after* the provider has consumed input.
struct SharedMock(Arc<MockSubprocessRuntime>);

impl SharedMock {
    fn new() -> (Self, Arc<MockSubprocessRuntime>) {
        let inner = Arc::new(MockSubprocessRuntime::new());
        (Self(Arc::clone(&inner)), inner)
    }
}

impl perl_lsp_tooling::SubprocessRuntime for SharedMock {
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<perl_lsp_tooling::SubprocessOutput, perl_lsp_tooling::SubprocessError> {
        self.0.run_command(program, args, stdin)
    }
}

// ---------------------------------------------------------------------------
// FormattingProvider construction
// ---------------------------------------------------------------------------

#[test]
fn test_new_provider_default_perltidy() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document("original", &default_options()));
    // Should have called "perltidy" (default command)
    assert!(!doc.text.is_empty());
    Ok(())
}

#[test]
fn test_with_perltidy_path_overrides_default() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let provider =
        FormattingProvider::new(runtime).with_perltidy_path("/usr/local/bin/perltidy".to_string());

    let _doc = must(provider.format_document("code", &default_options()));
    Ok(())
}

// ---------------------------------------------------------------------------
// format_document — successful formatting
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_returns_edits_when_changed() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"my $x = 1;\n".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document("my $x=1;\n", &default_options()));
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "my $x = 1;\n");
    Ok(())
}

#[test]
fn test_format_document_no_edits_when_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(source.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert!(doc.edits.is_empty());
    assert_eq!(doc.text, source);
    Ok(())
}

#[test]
fn test_format_document_formatted_text_matches_output() -> Result<(), Box<dyn std::error::Error>> {
    let expected = "use strict;\nuse warnings;\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(expected.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document("use strict;use warnings;", &default_options()));
    assert_eq!(doc.text, expected);
    Ok(())
}

#[test]
fn test_format_document_edit_range_covers_whole_document() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "line1\nline2\nline3";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    let edit = &doc.edits[0];
    assert_eq!(edit.range.start.line, 0);
    assert_eq!(edit.range.start.character, 0);
    assert_eq!(edit.range.end.line, 2);
    assert_eq!(edit.range.end.character, 5); // "line3".len()
    Ok(())
}

// ---------------------------------------------------------------------------
// format_document — empty document
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document("", &default_options()));
    assert!(doc.edits.is_empty());
    assert_eq!(doc.text, "");
    Ok(())
}

#[test]
fn test_format_document_empty_produces_content() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"\n".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document("", &default_options()));
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// format_document — formatting options pass-through
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_tab_options() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"tabbed".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document("untabbed", &tab_options()));
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "tabbed");
    Ok(())
}

// ---------------------------------------------------------------------------
// format_document — perltidy invocation verification
// ---------------------------------------------------------------------------

#[test]
fn test_perltidy_receives_spaces_args() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"x".to_vec()));

    let provider = FormattingProvider::new(shared);
    let _ = must(provider.format_document("y", &default_options()));

    let invocations = inner.invocations();
    assert_eq!(invocations.len(), 1);
    let inv = &invocations[0];
    assert_eq!(inv.program, "perltidy");
    assert!(inv.args.contains(&"-st".to_string()));
    assert!(inv.args.contains(&"-se".to_string()));
    assert!(inv.args.contains(&"-et=4".to_string()));
    assert!(inv.args.contains(&"-i=4".to_string()));
    Ok(())
}

#[test]
fn test_perltidy_receives_tab_args() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"x".to_vec()));

    let provider = FormattingProvider::new(shared);
    let _ = must(provider.format_document("y", &tab_options()));

    let invocations = inner.invocations();
    assert_eq!(invocations.len(), 1);
    let inv = &invocations[0];
    assert!(inv.args.contains(&"-dt".to_string()));
    assert!(inv.args.contains(&"-i=8".to_string()));
    assert!(!inv.args.iter().any(|a| a.starts_with("-et=")));
    Ok(())
}

#[test]
fn test_perltidy_receives_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"formatted".to_vec()));

    let provider = FormattingProvider::new(shared);
    let _ = must(provider.format_document("my $code;", &default_options()));

    let invocations = inner.invocations();
    assert_eq!(invocations.len(), 1);
    let stdin = invocations[0].stdin.as_deref();
    assert_eq!(stdin, Some(b"my $code;".as_slice()));
    Ok(())
}

#[test]
fn test_custom_perltidy_path_used_in_invocation() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"ok".to_vec()));

    let provider =
        FormattingProvider::new(shared).with_perltidy_path("/opt/bin/perltidy".to_string());
    let _ = must(provider.format_document("code", &default_options()));

    let invocations = inner.invocations();
    assert_eq!(invocations[0].program, "/opt/bin/perltidy");
    Ok(())
}

// ---------------------------------------------------------------------------
// format_document — error paths
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_perltidy_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let provider = FormattingProvider::new(ErrorRuntime);
    let result = provider.format_document("code", &default_options());

    let err = must_err(result);
    let msg = format!("{err}");
    assert!(msg.contains("perltidy not found"));
    Ok(())
}

#[test]
fn test_format_document_perltidy_execution_error() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::failure(
        b"syntax error near line 5".to_vec(),
        1,
    ));
    let provider = make_provider(runtime);

    let result = provider.format_document("bad code", &default_options());
    let err = must_err(result);
    let msg = format!("{err}");
    assert!(msg.contains("perltidy error"));
    assert!(msg.contains("syntax error near line 5"));
    Ok(())
}

#[test]
fn test_format_document_perltidy_nonzero_exit() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::failure(b"error output".to_vec(), 2));
    let provider = make_provider(runtime);

    let result = provider.format_document("code", &default_options());
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// format_range — basic range formatting
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_formats_selected_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nline2\nline3\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"FORMATTED1\nFORMATTED2".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(2, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "FORMATTED1\nFORMATTED2");
    assert_eq!(doc.edits[0].range.start.line, 1);
    assert_eq!(doc.edits[0].range.end.line, 2);
    Ok(())
}

#[test]
fn test_format_range_no_edits_when_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nline2";
    let runtime = MockSubprocessRuntime::new();
    // Return the exact extracted text (lines 1-2 joined with \n)
    runtime.add_response(MockResponse::success(b"line1\nline2".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(2, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert!(doc.edits.is_empty());
    assert_eq!(doc.text, source);
    Ok(())
}

#[test]
fn test_format_range_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nline2";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(1, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].range.start.line, 1);
    assert_eq!(doc.edits[0].range.end.line, 1);
    assert_eq!(doc.edits[0].range.end.character, 5);
    Ok(())
}

// ---------------------------------------------------------------------------
// format_range — edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_reversed_lines_returns_no_edits() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1
line2
line3";
    let runtime = MockSubprocessRuntime::new();
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(2, 0), FormatPosition::new(1, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert!(doc.edits.is_empty());
    assert_eq!(doc.text, source);
    Ok(())
}

#[test]
fn test_format_range_start_beyond_document() -> Result<(), Box<dyn std::error::Error>> {
    let source = "only one line";
    let runtime = MockSubprocessRuntime::new();
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(100, 0), FormatPosition::new(200, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert!(doc.edits.is_empty());
    assert_eq!(doc.text, source);
    Ok(())
}

#[test]
fn test_format_range_end_clamped_to_last_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED0\nCHANGED1".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(999, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    // end_line should be clamped to 1 (last valid line index)
    assert_eq!(doc.edits[0].range.end.line, 1);
    Ok(())
}

#[test]
fn test_format_range_empty_document() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 0));
    let doc = must(provider.format_range("", &range, &default_options()));

    // Empty document has 0 lines, so start_line >= lines.len() triggers early return
    assert!(doc.edits.is_empty());
    Ok(())
}

#[test]
fn test_format_range_first_line_only() -> Result<(), Box<dyn std::error::Error>> {
    let source = "first\nsecond\nthird";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"FIRST".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "FIRST");
    assert_eq!(doc.edits[0].range.start.line, 0);
    assert_eq!(doc.edits[0].range.end.line, 0);
    Ok(())
}

#[test]
fn test_format_range_last_line_only() -> Result<(), Box<dyn std::error::Error>> {
    let source = "first\nsecond\nthird";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"THIRD".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(2, 0), FormatPosition::new(2, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "THIRD");
    assert_eq!(doc.edits[0].range.start.line, 2);
    assert_eq!(doc.edits[0].range.end.line, 2);
    Ok(())
}

#[test]
fn test_format_range_preserves_full_document_text() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nline2";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(1, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    // format_range returns the original document text (not the formatted range)
    assert_eq!(doc.text, source);
    Ok(())
}

// ---------------------------------------------------------------------------
// format_range — error propagation
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_perltidy_error() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::failure(b"range error".to_vec(), 1));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 10));
    let result = provider.format_range("some code", &range, &default_options());
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_format_range_perltidy_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let provider = FormattingProvider::new(ErrorRuntime);
    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 5));
    let result = provider.format_range("code", &range, &default_options());

    let err = must_err(result);
    let msg = format!("{err}");
    assert!(msg.contains("perltidy not found"));
    Ok(())
}

// ---------------------------------------------------------------------------
// FormattingError display
// ---------------------------------------------------------------------------

#[test]
fn test_error_perltidy_not_found_display() {
    let err = FormattingError::PerltidyNotFound("command not found".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("perltidy not found"));
    assert!(msg.contains("command not found"));
    assert!(msg.contains("cpan Perl::Tidy"));
}

#[test]
fn test_error_perltidy_error_display() {
    let err = FormattingError::PerltidyError("bad syntax".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("perltidy error"));
    assert!(msg.contains("bad syntax"));
}

#[test]
fn test_error_io_error_display() {
    let err = FormattingError::IoError("disk full".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("IO error"));
    assert!(msg.contains("disk full"));
}

#[test]
fn test_error_is_debug() {
    let err = FormattingError::PerltidyError("test".to_string());
    let debug = format!("{err:?}");
    assert!(debug.contains("PerltidyError"));
}

// ---------------------------------------------------------------------------
// FormattedDocument and types
// ---------------------------------------------------------------------------

#[test]
fn test_formatted_document_empty_edits() {
    let doc = FormattedDocument {
        text: "hello".to_string(),
        edits: vec![],
    };
    assert_eq!(doc.text, "hello");
    assert!(doc.edits.is_empty());
}

#[test]
fn test_formatted_document_with_edits() {
    let doc = FormattedDocument {
        text: "formatted".to_string(),
        edits: vec![FormatTextEdit {
            range: FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 5)),
            new_text: "formatted".to_string(),
        }],
    };
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "formatted");
}

#[test]
fn test_format_position_new() {
    let pos = FormatPosition::new(42, 17);
    assert_eq!(pos.line, 42);
    assert_eq!(pos.character, 17);
}

#[test]
fn test_format_range_new() {
    let range = FormatRange::new(FormatPosition::new(1, 2), FormatPosition::new(3, 4));
    assert_eq!(range.start.line, 1);
    assert_eq!(range.start.character, 2);
    assert_eq!(range.end.line, 3);
    assert_eq!(range.end.character, 4);
}

#[test]
fn test_format_range_whole_document_single_line() {
    let range = FormatRange::whole_document("hello");
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 5);
}

#[test]
fn test_format_range_whole_document_multi_line() {
    let range = FormatRange::whole_document("line1\nline2\nline3");
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.character, 5);
}

#[test]
fn test_format_range_whole_document_empty() {
    let range = FormatRange::whole_document("");
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 0);
}

// ---------------------------------------------------------------------------
// FormattingOptions variations
// ---------------------------------------------------------------------------

#[test]
fn test_options_with_all_optional_fields() {
    let opts = FormattingOptions {
        tab_size: 2,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(false),
        trim_final_newlines: Some(true),
    };
    assert_eq!(opts.tab_size, 2);
    assert!(opts.insert_spaces);
    assert_eq!(opts.trim_trailing_whitespace, Some(true));
    assert_eq!(opts.insert_final_newline, Some(false));
    assert_eq!(opts.trim_final_newlines, Some(true));
}

#[test]
fn test_options_with_no_optional_fields() {
    let opts = FormattingOptions {
        tab_size: 4,
        insert_spaces: false,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    assert_eq!(opts.tab_size, 4);
    assert!(!opts.insert_spaces);
    assert!(opts.trim_trailing_whitespace.is_none());
}

// ---------------------------------------------------------------------------
// Multi-line and complex content scenarios
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_multiline_perl() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo{\nmy $x=1;\nreturn $x;\n}\n";
    let formatted = "sub foo {\n    my $x = 1;\n    return $x;\n}\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.text, formatted);
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

#[test]
fn test_format_document_unicode_content() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $greeting = \"こんにちは\";\n";
    let formatted = "my $greeting = \"こんにちは\";\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert!(doc.edits.is_empty()); // unchanged
    Ok(())
}

#[test]
fn test_format_document_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    let source = "code\n\n\n";
    let formatted = "code\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "code\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// Perltidy invocation with various tab sizes
// ---------------------------------------------------------------------------

#[test]
fn test_format_with_tab_size_2() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = FormattingOptions {
        tab_size: 2,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let _ = must(provider.format_document("in", &opts));

    let invocations = inner.invocations();
    assert!(invocations[0].args.contains(&"-et=2".to_string()));
    assert!(invocations[0].args.contains(&"-i=2".to_string()));
    Ok(())
}

// ---------------------------------------------------------------------------
// Custom error-producing runtime for testing PerltidyNotFound
// ---------------------------------------------------------------------------

struct ErrorRuntime;

impl perl_lsp_tooling::SubprocessRuntime for ErrorRuntime {
    fn run_command(
        &self,
        _program: &str,
        _args: &[&str],
        _stdin: Option<&[u8]>,
    ) -> Result<perl_lsp_tooling::SubprocessOutput, perl_lsp_tooling::SubprocessError> {
        Err(perl_lsp_tooling::SubprocessError::new(
            "command not found: perltidy",
        ))
    }
}

// ---------------------------------------------------------------------------
// Verify format_range sends correct subset to perltidy
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_sends_correct_lines_to_perltidy() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nline2\nline3";
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"CHANGED1\nCHANGED2".to_vec()));

    let provider = FormattingProvider::new(shared);
    let range = FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(2, 5));
    let _ = must(provider.format_range(source, &range, &default_options()));

    let invocations = inner.invocations();
    assert_eq!(invocations.len(), 1);
    // Should receive "line1\nline2" as stdin
    let stdin = invocations[0].stdin.as_deref();
    assert_eq!(stdin, Some(b"line1\nline2".as_slice()));
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_end_char_set_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let source = "short\nlonger_line_here\nx";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(1, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    // end_char should be the length of line at end_line (line 1 = "longer_line_here")
    assert_eq!(doc.edits[0].range.end.character, 16);
    Ok(())
}

#[test]
fn test_format_document_large_content() -> Result<(), Box<dyn std::error::Error>> {
    let source = (0..1000)
        .map(|i| format!("my $var{i} = {i};\n"))
        .collect::<String>();
    let formatted = source.replace("my ", "my  "); // slightly different
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(&source, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// UTF-8 / UTF-16 position encoding correctness
// ---------------------------------------------------------------------------

/// LSP positions use UTF-16 code units. A line ending with a multi-byte
/// character like 'é' (U+00E9, 2 UTF-8 bytes, 1 UTF-16 code unit) must
/// produce `end.character == 19` for the line, not 20 (byte length).
#[test]
fn test_format_range_utf8_end_character_is_utf16_count() -> Result<(), Box<dyn std::error::Error>> {
    // Line 1: "my $café = \"hello\";"
    // U+00E9 LATIN SMALL LETTER E WITH ACUTE: 2 UTF-8 bytes, 1 UTF-16 code unit
    // chars().count() = 19, UTF-16 units = 19, UTF-8 bytes = 20
    let source = "use strict;\nmy $caf\u{e9} = \"hello\";";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(1, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    // 19 UTF-16 units (é is BMP, 1 unit) — not 20 (byte length)
    assert_eq!(
        doc.edits[0].range.end.character, 19,
        "end.character must be UTF-16 code unit count (19), not byte length (20)"
    );
    Ok(())
}

/// Characters above U+FFFF (supplementary plane) count as 2 UTF-16 code
/// units (surrogate pair). Verify the character position reflects that count,
/// not the Rust char count (1) or byte length (4).
#[test]
fn test_format_range_supplementary_plane_char_is_two_utf16_units()
-> Result<(), Box<dyn std::error::Error>> {
    // Line 1: "my $x = \"😀\";"
    // U+1F600 GRINNING FACE: 4 UTF-8 bytes, 2 UTF-16 units, 1 Rust char
    // chars().count() = 12, UTF-16 units = 13, UTF-8 bytes = 15
    let source = "use strict;\nmy $x = \"\u{1F600}\";";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(1, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    // 13 UTF-16 units: 11 BMP chars + 2 for the emoji surrogate pair
    assert_eq!(
        doc.edits[0].range.end.character, 13,
        "supplementary plane char must count as 2 UTF-16 units (13 total), not 12 (char count) or 15 (bytes)"
    );
    Ok(())
}

/// `format_document` uses `FormatRange::whole_document` internally. The last
/// line of a document containing multi-byte characters must produce a
/// `end.character` that is a UTF-16 unit count, not a byte length.
#[test]
fn test_format_document_whole_document_range_utf16_end_char()
-> Result<(), Box<dyn std::error::Error>> {
    // Last line: "café" — U+00E9 is 2 UTF-8 bytes but 1 UTF-16 unit
    // chars().count() = 4, UTF-16 units = 4, UTF-8 bytes = 5
    let source = "use strict;\ncaf\u{e9}";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"CHANGED".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    // "café": 4 UTF-16 units — not 5 (byte length)
    assert_eq!(
        doc.edits[0].range.end.character, 4,
        "whole-document end.character must be UTF-16 code unit count (4), not byte length (5)"
    );
    Ok(())
}
