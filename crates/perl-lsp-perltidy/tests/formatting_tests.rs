use perl_lsp_perltidy::{PerlTidyConfig, PerlTidyFormatter};
use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::must;
use std::path::Path;
use std::sync::Arc;

// --- Basic formatting request tests ---

#[test]
fn formatter_with_mock_runtime_formats_code() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"my $x = 1;\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

    let result = formatter.format("my $x=1;");
    assert_eq!(must(result), "my $x = 1;\n");

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].program, "perltidy");
    assert!(invocations[0].args.contains(&"-st".to_string()));
}

#[test]
fn formatter_caches_repeat_requests() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"formatted\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

    let first = formatter.format("original");
    let second = formatter.format("original");
    assert_eq!(must(first), must(second));
    assert_eq!(runtime.invocations().len(), 1);
}

#[test]
fn formatter_surfaces_runtime_failures() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::failure(b"syntax error".to_vec(), 1));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format("invalid code");
    assert!(result.is_err());
    assert!(result.err().is_some_and(|msg| msg.contains("syntax error")));
}

#[test]
fn format_file_uses_argument_separator() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(Vec::new()));
    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

    let result = formatter.format_file(Path::new("test.pl"));
    assert!(result.is_ok());

    let invocations = runtime.invocations();
    let sep_pos =
        perl_tdd_support::must_some(invocations[0].args.iter().position(|arg| arg == "--"));
    let file_pos =
        perl_tdd_support::must_some(invocations[0].args.iter().position(|arg| arg == "test.pl"));
    assert!(sep_pos < file_pos);
}

#[test]
fn format_passes_code_via_stdin() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"use strict;\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

    let _result = must(formatter.format("use strict;"));

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].stdin, Some(b"use strict;".to_vec()));
}

#[test]
fn format_appends_stdout_flag() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"output\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

    let _ = must(formatter.format("input"));

    let invocations = runtime.invocations();
    let last_config_arg = invocations[0].args.last().map(String::as_str);
    assert_eq!(last_config_arg, Some("-st"));
}

#[test]
fn format_returns_perltidy_output_verbatim() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let expected = "my $x = 1;\nmy $y = 2;\n";
    runtime.add_response(MockResponse::success(expected.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = must(formatter.format("my $x=1;\nmy $y=2;"));
    assert_eq!(result, expected);
}

#[test]
fn format_handles_empty_input() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = must(formatter.format(""));
    assert_eq!(result, "");
}

// --- get_suggestions tests ---

#[test]
fn get_suggestions_returns_empty_when_code_unchanged() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let code = "my $x = 1;\n";
    runtime.add_response(MockResponse::success(code.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let suggestions = must(formatter.get_suggestions(code));
    assert!(suggestions.is_empty());
}

#[test]
fn get_suggestions_returns_changed_lines() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let original = "my $x=1;\nmy $y = 2;\n";
    let formatted = "my $x = 1;\nmy $y = 2;\n";
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let suggestions = must(formatter.get_suggestions(original));
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].line, 0);
    assert_eq!(suggestions[0].original, "my $x=1;");
    assert_eq!(suggestions[0].formatted, "my $x = 1;");
}

#[test]
fn get_suggestions_reports_multiple_changes() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let original = "my $a=1;\nmy $b=2;\nmy $c=3;\n";
    let formatted = "my $a = 1;\nmy $b=2;\nmy $c = 3;\n";
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let suggestions = must(formatter.get_suggestions(original));
    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0].line, 0);
    assert_eq!(suggestions[1].line, 2);
}

// --- clear_cache tests ---

#[test]
fn clear_cache_forces_re_invocation() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"result1\n".to_vec()));
    runtime.add_response(MockResponse::success(b"result2\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

    let first = must(formatter.format("code"));
    assert_eq!(first, "result1\n");

    formatter.clear_cache();

    let second = must(formatter.format("code"));
    assert_eq!(second, "result2\n");

    // Two invocations: cache was cleared between them
    assert_eq!(runtime.invocations().len(), 2);
}

// --- Invalid UTF-8 output handling ---

#[test]
fn format_returns_error_on_invalid_utf8() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    // Invalid UTF-8 bytes
    runtime.add_response(MockResponse::success(vec![0xFF, 0xFE, 0x00]));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format("code");
    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Invalid UTF-8"));
}

// --- Range formatting tests ---

#[test]
fn format_range_formats_selected_lines() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    // The range formatter will extract line 1 ("my $y=2;") and format it
    runtime.add_response(MockResponse::success(b"my $y = 2;".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "my $x = 1;\nmy $y=2;\nmy $z = 3;";
    let result = must(formatter.format_range(code, 1, 1));

    // Line 0 and line 2 preserved, line 1 formatted
    assert!(result.contains("my $x = 1;"));
    assert!(result.contains("my $y = 2;"));
    assert!(result.contains("my $z = 3;"));
}

#[test]
fn format_range_preserves_lines_before_range() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "line0\nline1\nline2\nline3";
    let result = must(formatter.format_range(code, 2, 2));

    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "line0");
    assert_eq!(lines[1], "line1");
}

#[test]
fn format_range_preserves_lines_after_range() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "line0\nline1\nline2\nline3";
    let result = must(formatter.format_range(code, 1, 1));

    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(perl_tdd_support::must_some(lines.last().copied()), "line3");
}

#[test]
fn format_range_multiline_range() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"a\nb".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "header\nline1\nline2\nfooter";
    let result = must(formatter.format_range(code, 1, 2));

    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "header");
    assert_eq!(lines[1], "a");
    assert_eq!(lines[2], "b");
    assert_eq!(lines[3], "footer");
}

#[test]
fn format_range_start_line_out_of_bounds() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "one\ntwo\nthree";
    let result = formatter.format_range(code, 10, 12);

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("out of bounds"));
}

#[test]
fn format_range_end_line_out_of_bounds() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "one\ntwo\nthree";
    let result = formatter.format_range(code, 0, 10);

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("out of bounds"));
}

#[test]
fn format_range_first_line_only() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"formatted_first".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "line0\nline1\nline2";
    let result = must(formatter.format_range(code, 0, 0));

    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "formatted_first");
    assert_eq!(lines[1], "line1");
}

#[test]
fn format_range_last_line_only() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"formatted_last".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "line0\nline1\nline2";
    let result = must(formatter.format_range(code, 2, 2));

    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "line0");
    assert_eq!(lines[1], "line1");
    assert_eq!(lines[2], "formatted_last");
}

// --- format_file failure handling ---

#[test]
fn format_file_returns_error_on_perltidy_failure() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::failure(b"can't open file".to_vec(), 2));
    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format_file(Path::new("/nonexistent/file.pl"));
    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("can't open file"));
}

#[test]
fn get_suggestions_includes_added_lines() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let original = "my $x=1;\n";
    let formatted = "my $x = 1;\nmy $y = 2;\n";
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let suggestions = must(formatter.get_suggestions(original));
    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0].line, 0);
    assert_eq!(suggestions[0].description, "Line formatting change");
    assert_eq!(suggestions[1].line, 1);
    assert_eq!(suggestions[1].original, "");
    assert_eq!(suggestions[1].formatted, "my $y = 2;");
    assert_eq!(suggestions[1].description, "Line added by formatting");
}

#[test]
fn get_suggestions_includes_removed_lines() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let original = "my $x=1;\nmy $y = 2;\n";
    let formatted = "my $x = 1;\n";
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let suggestions = must(formatter.get_suggestions(original));
    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0].line, 0);
    assert_eq!(suggestions[0].description, "Line formatting change");
    assert_eq!(suggestions[1].line, 1);
    assert_eq!(suggestions[1].original, "my $y = 2;");
    assert_eq!(suggestions[1].formatted, "");
    assert_eq!(suggestions[1].description, "Line removed by formatting");
}
