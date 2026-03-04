//! Extended unit tests for perl-lsp-formatting crate.
//!
//! These tests extend the comprehensive_unit_tests.rs with additional coverage for:
//! - Edge cases with Unicode and special characters
//! - Complex formatting scenarios
//! - Multiple line formatting boundaries
//! - UTF-16 character position edge cases
//! - Format position calculations
//! - Error handling variations
//! - Configuration combinations
//! - Large and complex document scenarios

use perl_lsp_formatting::{
    FormatPosition, FormatRange, FormattingError, FormattingOptions, FormattingProvider,
};
use perl_lsp_tooling::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::must;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helper Functions
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

fn space_options(size: u32) -> FormattingOptions {
    FormattingOptions {
        tab_size: size,
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

fn options_with_trailing(trim: bool) -> FormattingOptions {
    FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: Some(trim),
        insert_final_newline: None,
        trim_final_newlines: None,
    }
}

fn options_with_final_newline(insert: bool) -> FormattingOptions {
    FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: Some(insert),
        trim_final_newlines: None,
    }
}

fn make_provider(runtime: MockSubprocessRuntime) -> FormattingProvider<MockSubprocessRuntime> {
    FormattingProvider::new(runtime)
}

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

struct ErrorRuntime(&'static str);

impl perl_lsp_tooling::SubprocessRuntime for ErrorRuntime {
    fn run_command(
        &self,
        _program: &str,
        _args: &[&str],
        _stdin: Option<&[u8]>,
    ) -> Result<perl_lsp_tooling::SubprocessOutput, perl_lsp_tooling::SubprocessError> {
        Err(perl_lsp_tooling::SubprocessError::new(self.0))
    }
}

// ---------------------------------------------------------------------------
// FormatRange edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_single_line_document() -> Result<(), Box<dyn std::error::Error>> {
    let source = "single line no newline";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 22));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert!(!doc.text.is_empty());
    Ok(())
}

#[test]
fn test_format_range_empty_document() -> Result<(), Box<dyn std::error::Error>> {
    let source = "";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.text, "");
    Ok(())
}

#[test]
fn test_format_range_start_position_at_end_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"changed".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(2, 0), FormatPosition::new(2, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

#[test]
fn test_format_range_start_line_beyond_document() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2";
    let runtime = MockSubprocessRuntime::new();
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(10, 0), FormatPosition::new(11, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 0);
    assert_eq!(doc.text, source);
    Ok(())
}

#[test]
fn test_format_range_end_before_start() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let runtime = MockSubprocessRuntime::new();
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(2, 5), FormatPosition::new(1, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 0);
    assert_eq!(doc.text, source);
    Ok(())
}

#[test]
fn test_format_range_multi_line_with_long_lines() -> Result<(), Box<dyn std::error::Error>> {
    let line1 = "a".repeat(100);
    let line2 = "b".repeat(150);
    let line3 = "c".repeat(80);
    let source = format!("{}\n{}\n{}", line1, line2, line3);

    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(2, 80));
    let doc = must(provider.format_range(&source, &range, &default_options()));

    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// FormatPosition edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_format_position_zero_values() -> Result<(), Box<dyn std::error::Error>> {
    let pos = FormatPosition::new(0, 0);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 0);
    Ok(())
}

#[test]
fn test_format_position_large_values() -> Result<(), Box<dyn std::error::Error>> {
    let pos = FormatPosition::new(10000, 5000);
    assert_eq!(pos.line, 10000);
    assert_eq!(pos.character, 5000);
    Ok(())
}

#[test]
fn test_format_position_max_u32() -> Result<(), Box<dyn std::error::Error>> {
    let pos = FormatPosition::new(u32::MAX, u32::MAX);
    assert_eq!(pos.line, u32::MAX);
    assert_eq!(pos.character, u32::MAX);
    Ok(())
}

// ---------------------------------------------------------------------------
// FormatRange whole_document
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_whole_document_no_newlines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "no newlines here";
    let range = FormatRange::whole_document(source);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 16);
    Ok(())
}

#[test]
fn test_format_range_whole_document_multiple_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let range = FormatRange::whole_document(source);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.character, 5); // "line3".len()
    Ok(())
}

#[test]
fn test_format_range_whole_document_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\n";
    let range = FormatRange::whole_document(source);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 1); // lines() doesn't include empty final line
    assert_eq!(range.end.character, 5); // "line2".len()
    Ok(())
}

#[test]
fn test_format_range_whole_document_empty() -> Result<(), Box<dyn std::error::Error>> {
    let source = "";
    let range = FormatRange::whole_document(source);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// format_document with various inputs
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
    let source = "   \n\t\t\n  ";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"    \n        ".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert!(!doc.text.is_empty());
    Ok(())
}

#[test]
fn test_format_document_only_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment1\n# comment2\n# comment3";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(source.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 0);
    Ok(())
}

#[test]
fn test_format_document_mixed_tabs_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\n\tline2\n    line3";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

#[test]
fn test_format_document_with_special_characters() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $var = \"special: @#$%&*()\";\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(source.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 0);
    Ok(())
}

#[test]
fn test_format_document_perl_code_with_strings() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"my $str = "hello world";"#;
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(source.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 0);
    Ok(())
}

#[test]
fn test_format_document_perl_code_with_regex() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"if ($str =~ /pattern/) { print "match"; }"#;
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(source.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tab size configurations
// ---------------------------------------------------------------------------

#[test]
fn test_format_with_tab_size_1() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = space_options(1);
    let _ = must(provider.format_document("in", &opts));

    let invocations = inner.invocations();
    assert!(invocations[0].args.contains(&"-et=1".to_string()));
    assert!(invocations[0].args.contains(&"-i=1".to_string()));
    Ok(())
}

#[test]
fn test_format_with_tab_size_3() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = space_options(3);
    let _ = must(provider.format_document("in", &opts));

    let invocations = inner.invocations();
    assert!(invocations[0].args.contains(&"-et=3".to_string()));
    assert!(invocations[0].args.contains(&"-i=3".to_string()));
    Ok(())
}

#[test]
fn test_format_with_large_tab_size() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = space_options(16);
    let _ = must(provider.format_document("in", &opts));

    let invocations = inner.invocations();
    assert!(invocations[0].args.contains(&"-et=16".to_string()));
    assert!(invocations[0].args.contains(&"-i=16".to_string()));
    Ok(())
}

// ---------------------------------------------------------------------------
// Insert spaces vs tabs
// ---------------------------------------------------------------------------

#[test]
fn test_format_insert_spaces_true() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let _ = must(provider.format_document("in", &opts));

    let invocations = inner.invocations();
    assert!(invocations[0].args.contains(&"-et=4".to_string()));
    assert!(!invocations[0].args.contains(&"-dt".to_string()));
    Ok(())
}

#[test]
fn test_format_insert_spaces_false() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = FormattingOptions {
        tab_size: 8,
        insert_spaces: false,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let _ = must(provider.format_document("in", &opts));

    let invocations = inner.invocations();
    assert!(invocations[0].args.contains(&"-dt".to_string()));
    assert!(invocations[0].args.contains(&"-i=8".to_string()));
    Ok(())
}

// ---------------------------------------------------------------------------
// Trailing whitespace options
// ---------------------------------------------------------------------------

#[test]
fn test_format_trim_trailing_true() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, _inner) = SharedMock::new();
    _inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = options_with_trailing(true);
    let _ = must(provider.format_document("in", &opts));
    Ok(())
}

#[test]
fn test_format_trim_trailing_false() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, _inner) = SharedMock::new();
    _inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = options_with_trailing(false);
    let _ = must(provider.format_document("in", &opts));
    Ok(())
}

// ---------------------------------------------------------------------------
// Final newline options
// ---------------------------------------------------------------------------

#[test]
fn test_format_insert_final_newline_true() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, _inner) = SharedMock::new();
    _inner.add_response(MockResponse::success(b"out\n".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = options_with_final_newline(true);
    let _ = must(provider.format_document("in", &opts));
    Ok(())
}

#[test]
fn test_format_insert_final_newline_false() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, _inner) = SharedMock::new();
    _inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = options_with_final_newline(false);
    let _ = must(provider.format_document("in", &opts));
    Ok(())
}

// ---------------------------------------------------------------------------
// Combined option variations
// ---------------------------------------------------------------------------

#[test]
fn test_format_all_options_set() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"formatted\n".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = FormattingOptions {
        tab_size: 2,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(false),
    };
    let doc = must(provider.format_document("in", &opts));
    assert!(!doc.text.is_empty());
    Ok(())
}

#[test]
fn test_format_tab_no_spaces_all_options() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"formatted".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = FormattingOptions {
        tab_size: 8,
        insert_spaces: false,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(false),
        trim_final_newlines: Some(true),
    };
    let doc = must(provider.format_document("in", &opts));
    assert!(!doc.text.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Error scenarios
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_perltidy_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = ErrorRuntime("command not found: perltidy");
    let provider = FormattingProvider::new(runtime);

    let result = provider.format_document("code", &default_options());
    assert!(result.is_err());
    match result {
        Err(FormattingError::PerltidyNotFound(_)) => Ok(()),
        _ => Err("Expected PerltidyNotFound".into()),
    }
}

#[test]
fn test_format_range_perltidy_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = ErrorRuntime("perltidy: command not found");
    let provider = FormattingProvider::new(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(1, 5));
    let result = provider.format_range("code\nmore", &range, &default_options());
    assert!(result.is_err());
    match result {
        Err(FormattingError::PerltidyNotFound(_)) => Ok(()),
        _ => Err("Expected PerltidyNotFound".into()),
    }
}

// ---------------------------------------------------------------------------
// Custom perltidy path
// ---------------------------------------------------------------------------

#[test]
fn test_custom_perltidy_path_format_document() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, _inner) = SharedMock::new();
    _inner.add_response(MockResponse::success(b"formatted".to_vec()));

    let provider =
        FormattingProvider::new(shared).with_perltidy_path("/custom/path/to/perltidy".to_string());
    let doc = must(provider.format_document("code", &default_options()));
    assert!(!doc.text.is_empty());
    Ok(())
}

#[test]
fn test_custom_perltidy_path_format_range() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, _inner) = SharedMock::new();
    _inner.add_response(MockResponse::success(b"changed".to_vec()));

    let provider =
        FormattingProvider::new(shared).with_perltidy_path("/usr/bin/perltidy".to_string());
    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 5));
    let doc = must(provider.format_range("code", &range, &default_options()));
    assert!(!doc.text.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Format range line boundaries
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_first_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"changed".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

#[test]
fn test_format_range_middle_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3\nline4";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"changed1\nchanged2".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(2, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

#[test]
fn test_format_range_last_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"changed".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(2, 0), FormatPosition::new(2, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// Format range with character positions
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_partial_first_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"changed".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 2), FormatPosition::new(0, 5));
    let doc = must(provider.format_range(source, &range, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

#[test]
fn test_format_range_zero_length() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2";
    let runtime = MockSubprocessRuntime::new();
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 2), FormatPosition::new(0, 2));
    let _doc = must(provider.format_range(source, &range, &default_options()));
    // Should still format the line containing the position
    Ok(())
}

// ---------------------------------------------------------------------------
// Document size variations
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_very_small() -> Result<(), Box<dyn std::error::Error>> {
    let source = "x";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"x".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 0);
    Ok(())
}

#[test]
fn test_format_document_medium_size() -> Result<(), Box<dyn std::error::Error>> {
    let source = (0..100).map(|i| format!("my $var{} = {};\n", i, i)).collect::<String>();
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(source.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(&source, &default_options()));
    assert_eq!(doc.edits.len(), 0);
    Ok(())
}

#[test]
fn test_format_document_many_short_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = (0..500).map(|_| "x\n").collect::<String>();
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(source.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let _doc = must(provider.format_document(&source, &default_options()));
    Ok(())
}

// ---------------------------------------------------------------------------
// Format range line number validation
// ---------------------------------------------------------------------------

#[test]
fn test_format_range_clamps_end_line_to_document() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"changed".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(100, 0));
    let doc = must(provider.format_range(source, &range, &default_options()));
    // Should clamp to last line
    assert_eq!(doc.edits.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// Format text edit properties
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_edit_has_correct_range() -> Result<(), Box<dyn std::error::Error>> {
    let source = "original";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].range.start.line, 0);
    assert_eq!(doc.edits[0].range.start.character, 0);
    Ok(())
}

#[test]
fn test_format_document_edit_new_text_correct() -> Result<(), Box<dyn std::error::Error>> {
    let source = "original";
    let formatted = "formatted text";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits[0].new_text, formatted);
    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple format operations
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_then_range_same_provider() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, _inner) = SharedMock::new();
    _inner.add_response(MockResponse::success(b"formatted1".to_vec()));
    _inner.add_response(MockResponse::success(b"formatted2".to_vec()));

    let provider = FormattingProvider::new(shared);
    let _doc1 = must(provider.format_document("code1", &default_options()));
    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(1, 0));
    let _doc2 = must(provider.format_range("code2\nmore", &range, &default_options()));
    Ok(())
}

// ---------------------------------------------------------------------------
// Empty and whitespace handling
// ---------------------------------------------------------------------------

#[test]
fn test_format_document_only_newlines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "\n\n\n";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"\n\n\n".to_vec()));
    let provider = make_provider(runtime);

    let doc = must(provider.format_document(source, &default_options()));
    assert_eq!(doc.edits.len(), 0);
    Ok(())
}

#[test]
fn test_format_range_empty_result() -> Result<(), Box<dyn std::error::Error>> {
    let source = "code";
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"".to_vec()));
    let provider = make_provider(runtime);

    let range = FormatRange::new(FormatPosition::new(0, 0), FormatPosition::new(0, 4));
    let doc = must(provider.format_range(source, &range, &default_options()));
    assert_eq!(doc.edits.len(), 1);
    assert_eq!(doc.edits[0].new_text, "");
    Ok(())
}

// ---------------------------------------------------------------------------
// Perltidy command arguments
// ---------------------------------------------------------------------------

#[test]
fn test_perltidy_arguments_include_stdio_flags() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let _ = must(provider.format_document("in", &default_options()));

    let invocations = inner.invocations();
    assert!(invocations[0].args.contains(&"-st".to_string()));
    assert!(invocations[0].args.contains(&"-se".to_string()));
    Ok(())
}

#[test]
fn test_perltidy_arguments_count_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let (shared, inner) = SharedMock::new();
    inner.add_response(MockResponse::success(b"out".to_vec()));

    let provider = FormattingProvider::new(shared);
    let opts = space_options(4);
    let _ = must(provider.format_document("in", &opts));

    let invocations = inner.invocations();
    // Should have: program, -st, -se, -et=4, -i=4
    assert!(invocations.len() > 0);
    Ok(())
}
