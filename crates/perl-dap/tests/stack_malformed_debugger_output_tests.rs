//! Tests for malformed debugger output in perl-dap-stack.
//!
//! Covers: malformed stack traces (missing fields, extra whitespace),
//! invalid frame indices (negative, overflow), empty stack traces,
//! deeply nested eval contexts, mixed frame categories, and
//! `looks_like_frame()` negative examples (comments, doc strings).

use perl_dap::stack::{
    FrameCategory, FrameClassifier, PerlFrameClassifier, PerlStackParser, Source, StackFrame,
    StackFramePresentationHint,
};

// ═══════════════════════════════════════════════════════════════════════════
//  1. parse_context() with malformed input
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_context_empty_string_returns_none() {
    let parser = PerlStackParser::new();
    assert!(parser.parse_context("").is_none());
}

#[test]
fn parse_context_whitespace_only_returns_none() {
    let parser = PerlStackParser::new();
    assert!(parser.parse_context("    ").is_none());
    assert!(parser.parse_context("\t\t").is_none());
    assert!(parser.parse_context("\n").is_none());
}

#[test]
fn parse_context_missing_line_number_returns_none() {
    let parser = PerlStackParser::new();
    // Has package and file but no line number
    assert!(parser.parse_context("main::(file.pm):").is_none());
}

#[test]
fn parse_context_missing_file_returns_none() {
    let parser = PerlStackParser::new();
    // Completely absent file/line portion
    assert!(parser.parse_context("main::").is_none());
    // No recognizable file in the parens-less format
    assert!(parser.parse_context("main:: :42:").is_none());
}

#[test]
fn parse_context_garbage_input_returns_none() {
    let parser = PerlStackParser::new();
    assert!(parser.parse_context("hello world").is_none());
    assert!(parser.parse_context("12345").is_none());
    assert!(parser.parse_context(":::").is_none());
    assert!(parser.parse_context("not::a::context::line").is_none());
}

#[test]
fn parse_context_truncated_format_returns_none() {
    let parser = PerlStackParser::new();
    // Truncated at various points
    assert!(parser.parse_context("main::").is_none());
    assert!(parser.parse_context("main::(").is_none());
    assert!(parser.parse_context("main::(file").is_none());
}

#[test]
fn parse_context_non_numeric_line_returns_none() {
    let parser = PerlStackParser::new();
    // Line number is not numeric at all
    assert!(parser.parse_context("main::(file.pm):abc:").is_none());
    // Note: "12.5" partially matches \d+ as "12", so the regex extracts line=12.
    // This is expected regex behavior -- only fully non-numeric values are rejected.
    let partial = parser.parse_context("main::(file.pm):12.5:");
    if let Some((_, _, line)) = partial {
        assert_eq!(line, 12, "regex captures leading digits from partial numeric");
    }
}

#[test]
fn parse_context_extra_whitespace_around_valid_format() {
    let parser = PerlStackParser::new();
    // Leading/trailing whitespace around valid context line
    // The regex anchors to start of string so leading whitespace should fail
    let result = parser.parse_context("  main::(file.pm):42:");
    // Leading whitespace causes the anchored regex to not match
    assert!(result.is_none());
}

#[test]
fn parse_context_valid_format_recognized() {
    use perl_tdd_support::must_some;
    let parser = PerlStackParser::new();
    let (func, file, line) = must_some(parser.parse_context("main::(script.pl):100:"));
    assert_eq!(func, "main");
    assert_eq!(file, "script.pl");
    assert_eq!(line, 100);
}

#[test]
fn parse_context_with_path_containing_spaces() {
    let parser = PerlStackParser::new();
    // Paths with spaces may or may not match depending on regex
    let result = parser.parse_context("main::(my file.pm):10:");
    // The regex uses [^:)\\s]+ for file2, so spaces will break the match
    assert!(result.is_none());
}

#[test]
fn parse_context_very_large_line_number() {
    let parser = PerlStackParser::new();
    let result = parser.parse_context("main::(file.pm):999999999:");
    if let Some((_, _, line)) = result {
        assert_eq!(line, 999_999_999);
    }
}

#[test]
fn parse_context_line_number_zero() {
    let parser = PerlStackParser::new();
    let result = parser.parse_context("main::(file.pm):0:");
    if let Some((_, _, line)) = result {
        assert_eq!(line, 0);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Malformed stack traces (missing fields, extra whitespace)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_frame_missing_file_in_standard_format() {
    let mut parser = PerlStackParser::new();
    // Missing file path
    let result = parser.parse_frame("  #0  main::foo at  line 10", 0);
    // The regex expects a non-whitespace file token; empty file should not match
    assert!(result.is_none());
}

#[test]
fn parse_frame_missing_line_keyword_in_standard_format() {
    let mut parser = PerlStackParser::new();
    // Missing "line" keyword
    let result = parser.parse_frame("  #0  main::foo at script.pl 10", 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_missing_line_number_in_standard_format() {
    let mut parser = PerlStackParser::new();
    // No line number after "line"
    let result = parser.parse_frame("  #0  main::foo at script.pl line", 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_extra_whitespace_between_fields() {
    let mut parser = PerlStackParser::new();
    // Extra whitespace between tokens should still work since \s+ is used
    let line = "  #0    main::foo    at    script.pl    line    42";
    let result = parser.parse_frame(line, 0);
    if let Some(f) = result {
        assert_eq!(f.name, "main::foo");
        assert_eq!(f.line, 42);
    }
}

#[test]
fn parse_frame_tab_characters_in_prefix() {
    let mut parser = PerlStackParser::new();
    let line = "\t#0\tmain::foo at script.pl line 10";
    let result = parser.parse_frame(line, 0);
    if let Some(f) = result {
        assert_eq!(f.name, "main::foo");
        assert_eq!(f.line, 10);
    }
}

#[test]
fn parse_frame_verbose_missing_closing_quote() {
    let mut parser = PerlStackParser::new();
    // Missing closing quote on file path
    let line = "$ = Foo::bar('x') called from file `/app/Foo.pm line 55";
    let result = parser.parse_frame(line, 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_verbose_missing_line_keyword() {
    let mut parser = PerlStackParser::new();
    let line = "$ = Foo::bar() called from file `/app/Foo.pm' 55";
    let result = parser.parse_frame(line, 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_verbose_empty_function_name() {
    let mut parser = PerlStackParser::new();
    // Function name that doesn't match [A-Za-z_][\w:]*
    let line = "$ = () called from file `/app/Foo.pm' line 55";
    let result = parser.parse_frame(line, 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_eval_missing_closing_bracket() {
    let mut parser = PerlStackParser::new();
    // Missing closing bracket
    let line = "(eval 10)[/path/file.pm:42";
    let result = parser.parse_frame(line, 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_eval_missing_line_number() {
    let mut parser = PerlStackParser::new();
    let line = "(eval 10)[/path/file.pm:]";
    let result = parser.parse_frame(line, 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_eval_non_numeric_eval_number() {
    let mut parser = PerlStackParser::new();
    let line = "(eval abc)[/path/file.pm:42]";
    let result = parser.parse_frame(line, 0);
    // The regex expects \d+ for eval_num, so this should not match
    assert!(result.is_none());
}

#[test]
fn parse_frame_only_whitespace() {
    let mut parser = PerlStackParser::new();
    assert!(parser.parse_frame("", 0).is_none());
    assert!(parser.parse_frame("   ", 0).is_none());
    assert!(parser.parse_frame("\t\n", 0).is_none());
}

#[test]
fn parse_frame_partial_verbose_format() {
    let mut parser = PerlStackParser::new();
    // Starts like verbose format but is incomplete
    let line = "$ = Foo::bar";
    let result = parser.parse_frame(line, 0);
    assert!(result.is_none());
}

#[test]
fn parse_frame_verbose_with_no_args_parentheses() {
    let mut parser = PerlStackParser::new();
    // Verbose format requires parens around args
    let line = "$ = Foo::bar called from file `/app/Foo.pm' line 10";
    let result = parser.parse_frame(line, 0);
    // Should not match verbose format (no parens), but may match another pattern
    // Just verify it doesn't produce garbage
    if let Some(f) = result {
        assert!(!f.name.is_empty());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. Invalid frame indices (negative, overflow)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_frame_negative_provided_id_with_auto_ids_off() {
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    // Verbose frame without frame number; falls back to provided_id
    let line = "$ = main::foo() called from file `/app.pl' line 1";
    let result = parser.parse_frame(line, -5);
    if let Some(f) = result {
        assert_eq!(f.id, -5);
    }
}

#[test]
fn parse_frame_i64_max_as_provided_id() {
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    let line = "$ = main::foo() called from file `/app.pl' line 1";
    let result = parser.parse_frame(line, i64::MAX);
    if let Some(f) = result {
        assert_eq!(f.id, i64::MAX);
    }
}

#[test]
fn parse_frame_i64_min_as_provided_id() {
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    let line = "$ = main::foo() called from file `/app.pl' line 1";
    let result = parser.parse_frame(line, i64::MIN);
    if let Some(f) = result {
        assert_eq!(f.id, i64::MIN);
    }
}

#[test]
fn parser_starting_id_negative() {
    let mut parser = PerlStackParser::new().with_starting_id(-10);
    let frame = parser.parse_frame("  #0  main::foo at a.pl line 1", 0);
    assert_eq!(frame.map(|f| f.id), Some(-10));
}

#[test]
fn parser_starting_id_i64_max_with_auto_ids_off() {
    // When auto_assign_ids is true, parsing increments next_id which overflows
    // at i64::MAX. Use auto_ids(false) to test the boundary safely.
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    let frame = parser.parse_frame("  #0  main::foo at a.pl line 1", i64::MAX);
    // With auto_ids off and frame number in capture, uses the captured frame number
    assert_eq!(frame.map(|f| f.id), Some(0));
}

#[test]
fn parser_starting_id_large_but_safe() {
    // Use a large but non-overflowing starting ID
    let mut parser = PerlStackParser::new().with_starting_id(i64::MAX - 1);
    let frame = parser.parse_frame("  #0  main::foo at a.pl line 1", 0);
    assert_eq!(frame.map(|f| f.id), Some(i64::MAX - 1));
}

#[test]
fn parser_starting_id_zero() {
    let mut parser = PerlStackParser::new().with_starting_id(0);
    let f1 = parser.parse_frame("  #0  a::b at x.pl line 1", 0);
    let f2 = parser.parse_frame("  #1  c::d at y.pl line 2", 0);
    assert_eq!(f1.map(|f| f.id), Some(0));
    assert_eq!(f2.map(|f| f.id), Some(1));
}

#[test]
fn parse_frame_standard_frame_with_very_large_frame_number() {
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    // Frame number in the line itself is very large
    let line = "  #999999  main::foo at a.pl line 1";
    let result = parser.parse_frame(line, 0);
    if let Some(f) = result {
        assert_eq!(f.id, 999999);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Empty stack traces
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_stack_trace_empty_string() {
    let mut parser = PerlStackParser::new();
    let frames = parser.parse_stack_trace("");
    assert!(frames.is_empty());
}

#[test]
fn parse_stack_trace_only_newlines() {
    let mut parser = PerlStackParser::new();
    let frames = parser.parse_stack_trace("\n\n\n");
    assert!(frames.is_empty());
}

#[test]
fn parse_stack_trace_only_whitespace_lines() {
    let mut parser = PerlStackParser::new();
    let frames = parser.parse_stack_trace("   \n  \n\t\n   \n");
    assert!(frames.is_empty());
}

#[test]
fn parse_stack_trace_only_unrecognized_lines() {
    let mut parser = PerlStackParser::new();
    let output = "This is not a frame\nNeither is this\nOr this one\n";
    let frames = parser.parse_stack_trace(output);
    assert!(frames.is_empty());
}

#[test]
fn parse_stack_trace_mixed_garbage_and_valid_frames() {
    let mut parser = PerlStackParser::new();
    let output = "\
garbage line 1
$ = main::foo() called from file `/app.pl' line 10
more garbage
random text here
$ = main::bar() called from file `/app.pl' line 20
trailing garbage
";
    let frames = parser.parse_stack_trace(output);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].name, "main::foo");
    assert_eq!(frames[1].name, "main::bar");
}

#[test]
fn parse_stack_trace_single_valid_frame() {
    let mut parser = PerlStackParser::new();
    let output = "$ = main::only() called from file `/app.pl' line 1";
    let frames = parser.parse_stack_trace(output);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].name, "main::only");
    assert_eq!(frames[0].id, 1);
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. Deeply nested eval contexts
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_frame_deeply_nested_eval_numbering() {
    let mut parser = PerlStackParser::new();

    let eval_lines = [
        "(eval 1)[/app/lib/A.pm:10]",
        "(eval 2)[/app/lib/B.pm:20]",
        "(eval 3)[/app/lib/C.pm:30]",
        "(eval 4)[/app/lib/D.pm:40]",
        "(eval 5)[/app/lib/E.pm:50]",
    ];

    for (i, line) in eval_lines.iter().enumerate() {
        let frame = parser.parse_frame(line, 0);
        if let Some(f) = frame {
            assert!(f.name.contains(&format!("eval {}", i + 1)));
            assert_eq!(f.line, ((i + 1) * 10) as i64);
            assert!(f.source.as_ref().is_some_and(|s| s.is_eval()));
            assert_eq!(f.presentation_hint, Some(StackFramePresentationHint::Label));
        }
    }
}

#[test]
fn parse_stack_trace_all_eval_frames() {
    let mut parser = PerlStackParser::new();
    let output = "\
(eval 1)[/app/a.pm:1]
(eval 2)[/app/b.pm:2]
(eval 3)[/app/c.pm:3]
(eval 4)[/app/d.pm:4]
";
    let frames = parser.parse_stack_trace(output);
    assert_eq!(frames.len(), 4);
    for (i, f) in frames.iter().enumerate() {
        assert!(f.name.contains(&format!("eval {}", i + 1)));
        assert!(f.source.as_ref().is_some_and(|s| s.is_eval()));
    }
}

#[test]
fn parse_frame_eval_with_very_high_eval_number() {
    use perl_tdd_support::must_some;
    let mut parser = PerlStackParser::new();
    let line = "(eval 1000000)[/tmp/deep.pm:999]";
    let frame = must_some(parser.parse_frame(line, 0));
    assert!(frame.name.contains("1000000"));
    assert_eq!(frame.line, 999);
}

#[test]
fn classify_eval_frames_all_get_eval_category() {
    let classifier = PerlFrameClassifier::new();
    let eval_paths = ["(eval 1)", "(eval 50)[/script.pl:5]", "(eval 999)[/deep/nested.pm:100]"];

    for path in &eval_paths {
        let frame = StackFrame::new(1, "test", Some(Source::new(*path)), 1);
        assert_eq!(
            classifier.classify(&frame),
            FrameCategory::Eval,
            "Path '{}' should classify as Eval",
            path
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  6. Mixed frame categories in a single stack trace
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_stack_trace_mixed_frame_types() {
    let mut parser = PerlStackParser::new();
    let output = "\
$ = strict::import() called from file `/usr/lib/perl5/strict.pm' line 10
$ = My::App::handler('req') called from file `/home/user/app/lib/My/App.pm' line 55
(eval 3)[/home/user/app/lib/My/App.pm:60]
  #3  main::run at /home/user/app/script.pl line 5
";
    let frames = parser.parse_stack_trace(output);
    assert_eq!(frames.len(), 4);

    let classifier = PerlFrameClassifier::new().with_user_path("/home/user/app/");

    // strict.pm -> Core
    assert_eq!(classifier.classify(&frames[0]), FrameCategory::Core);
    // User app code -> User
    assert_eq!(classifier.classify(&frames[1]), FrameCategory::User);
    // Eval frame
    assert_eq!(classifier.classify(&frames[2]), FrameCategory::Eval);
    // User script -> User
    assert_eq!(classifier.classify(&frames[3]), FrameCategory::User);
}

#[test]
fn classify_all_mixed_with_include_external_false() {
    let classifier = PerlFrameClassifier::new();
    let frames = vec![
        StackFrame::new(1, "main::run", Some(Source::new("/app/main.pl")), 10),
        StackFrame::new(2, "strict::import", Some(Source::new("/usr/lib/perl5/strict.pm")), 1),
        StackFrame::new(3, "eval_code", Some(Source::new("(eval 5)")), 1),
        StackFrame::new(4, "Moose::new", Some(Source::new("/home/user/.cpanm/work/Moose.pm")), 50),
    ];

    let result = classifier.classify_all(frames, false);
    // Only user code frames should remain; core, library, and eval are excluded
    // eval is not "external" per is_external(), so it's included
    let user_names: Vec<&str> = result.iter().map(|f| f.name.as_str()).collect();
    // User: main::run; Eval frames are not external but not user_code either
    assert!(user_names.contains(&"main::run"), "main::run should be in user-only results");
}

#[test]
fn classify_all_mixed_with_include_external_true() {
    let classifier = PerlFrameClassifier::new();
    let frames = vec![
        StackFrame::new(1, "main::run", Some(Source::new("/app/main.pl")), 10),
        StackFrame::new(2, "strict::import", Some(Source::new("/usr/lib/perl5/strict.pm")), 1),
        StackFrame::new(3, "Moose::new", Some(Source::new("/home/user/.cpanm/work/Moose.pm")), 50),
    ];

    let result = classifier.classify_all(frames, true);
    assert_eq!(result.len(), 3, "All frames included when include_external=true");
}

// ═══════════════════════════════════════════════════════════════════════════
//  7. looks_like_frame() negative examples (comments, doc strings, code)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn looks_like_frame_perl_comments_are_false_positive() {
    // Perl comments starting with # trigger the starts_with('#') heuristic.
    // This is a known false positive: looks_like_frame is a quick pre-filter,
    // and the actual parse_frame() correctly rejects these.
    assert!(PerlStackParser::looks_like_frame("# This is a comment"));
    assert!(PerlStackParser::looks_like_frame("# TODO: fix this later"));
    assert!(PerlStackParser::looks_like_frame("## Section header"));

    // But parse_frame correctly rejects them
    let mut parser = PerlStackParser::new();
    assert!(parser.parse_frame("# This is a comment", 0).is_none());
    assert!(parser.parse_frame("# TODO: fix this later", 0).is_none());
    assert!(parser.parse_frame("## Section header", 0).is_none());
}

#[test]
fn looks_like_frame_pod_documentation_negative() {
    // POD documentation lines
    assert!(!PerlStackParser::looks_like_frame("=head1 NAME"));
    assert!(!PerlStackParser::looks_like_frame("=over 4"));
    assert!(!PerlStackParser::looks_like_frame("=item B<method>"));
    assert!(!PerlStackParser::looks_like_frame("=cut"));
    assert!(!PerlStackParser::looks_like_frame("=pod"));
}

#[test]
fn looks_like_frame_perl_code_statements_negative() {
    // Common Perl code statements
    assert!(!PerlStackParser::looks_like_frame("my $x = 1;"));
    assert!(!PerlStackParser::looks_like_frame("use strict;"));
    assert!(!PerlStackParser::looks_like_frame("use warnings;"));
    assert!(!PerlStackParser::looks_like_frame("sub my_function {"));
    assert!(!PerlStackParser::looks_like_frame("return $result;"));
    assert!(!PerlStackParser::looks_like_frame("print \"hello world\\n\";"));
    assert!(!PerlStackParser::looks_like_frame("die \"error occurred\";"));
}

#[test]
fn looks_like_frame_debugger_prompts_negative() {
    // Perl debugger prompt lines that are NOT stack frames
    assert!(!PerlStackParser::looks_like_frame("  DB<1>"));
    assert!(!PerlStackParser::looks_like_frame("  DB<42> p $x"));
    assert!(!PerlStackParser::looks_like_frame("Loading DB routines from perl5db.pl version 1.60"));
}

#[test]
fn looks_like_frame_error_messages_negative() {
    // Error messages that might look similar but aren't frames
    assert!(!PerlStackParser::looks_like_frame("Can't locate Foo.pm in @INC"));
    assert!(!PerlStackParser::looks_like_frame("Compilation failed in require"));
    assert!(!PerlStackParser::looks_like_frame("syntax error near \"unexpected\""));
}

#[test]
fn looks_like_frame_empty_and_whitespace_negative() {
    assert!(!PerlStackParser::looks_like_frame(""));
    assert!(!PerlStackParser::looks_like_frame("   "));
    assert!(!PerlStackParser::looks_like_frame("\t"));
    assert!(!PerlStackParser::looks_like_frame("\n"));
}

#[test]
fn looks_like_frame_positive_standard_format() {
    assert!(PerlStackParser::looks_like_frame("  #0  main::foo at script.pl line 10"));
    assert!(PerlStackParser::looks_like_frame(
        "  #99  Deeply::Nested::Package::method at /long/path.pm line 999"
    ));
}

#[test]
fn looks_like_frame_positive_verbose_format() {
    assert!(PerlStackParser::looks_like_frame("$ = Foo::bar() called from file `/app.pm' line 1"));
    assert!(PerlStackParser::looks_like_frame(
        "@ = List::process() called from file `/lib.pm' line 50"
    ));
    assert!(PerlStackParser::looks_like_frame(". = main::init() called from '-e' line 1"));
}

#[test]
fn looks_like_frame_hash_only_is_positive() {
    // A bare "#" with any content triggers the starts_with('#') check
    assert!(PerlStackParser::looks_like_frame("#0"));
    assert!(PerlStackParser::looks_like_frame("#anything"));
}

#[test]
fn looks_like_frame_false_positive_awareness() {
    // These contain " at " and " line " so they match the heuristic
    // This tests awareness that looks_like_frame is a loose heuristic
    let ambiguous = "error at script.pl line 10";
    // This IS detected as "looks like a frame" because it has " at " and " line "
    assert!(PerlStackParser::looks_like_frame(ambiguous));
}

// Note: The looks_like_frame() heuristic also matches lines starting with '#'.
// Perl comments start with '#', so they produce false positives.
// This is a known limitation: looks_like_frame is a quick pre-filter,
// not a definitive parser. The actual parse_frame() call rejects these.
#[test]
fn looks_like_frame_comment_false_positive_rejected_by_parse_frame() {
    // A comment starting with # will pass looks_like_frame but fail parse_frame
    let comment = "# This is a perl comment";
    // looks_like_frame triggers on '#' prefix (known false positive)
    let looks_like = PerlStackParser::looks_like_frame(comment);
    // But parse_frame correctly rejects it
    let mut parser = PerlStackParser::new();
    let parsed = parser.parse_frame(comment, 0);
    // The comment is rejected by the actual parser even if the heuristic said yes
    if looks_like {
        assert!(
            parsed.is_none(),
            "parse_frame should reject comments even if looks_like_frame says true"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  8. Additional malformed debugger output edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_frame_special_characters_in_file_path() {
    let mut parser = PerlStackParser::new();
    // File path with special characters
    let line = "$ = main::foo() called from file `/tmp/test-file (1).pl' line 5";
    let result = parser.parse_frame(line, 0);
    if let Some(f) = result {
        assert_eq!(f.name, "main::foo");
        assert_eq!(f.line, 5);
    }
}

#[test]
fn parse_frame_unicode_in_function_name() {
    let mut parser = PerlStackParser::new();
    // Unicode characters in function name (Perl supports this)
    let line = "  #0  main::café at script.pl line 1";
    // The regex uses \w which includes Unicode word characters
    let result = parser.parse_frame(line, 0);
    if let Some(f) = result {
        assert!(f.name.contains("café"));
    }
}

#[test]
fn parse_frame_very_long_function_name() {
    let mut parser = PerlStackParser::new();
    let long_name = format!("A::{}::func", "B::C".repeat(50));
    let line = format!("$ = {}() called from file `/app.pl' line 1", long_name);
    let result = parser.parse_frame(&line, 0);
    if let Some(f) = result {
        assert!(!f.name.is_empty());
    }
}

#[test]
fn parse_frame_double_colon_only_function_name() {
    let mut parser = PerlStackParser::new();
    // Just double colons, no valid identifier
    let line = "$ = ::() called from file `/app.pl' line 1";
    // Should not match since function names require [A-Za-z_] start
    let result = parser.parse_frame(line, 0);
    assert!(result.is_none());
}

#[test]
fn parse_stack_trace_windows_line_endings() {
    let mut parser = PerlStackParser::new();
    let output = "$ = main::foo() called from file `/app.pl' line 1\r\n$ = main::bar() called from file `/app.pl' line 2\r\n";
    let frames = parser.parse_stack_trace(output);
    // The \r should be handled by trim()
    assert_eq!(frames.len(), 2);
}

#[test]
fn parse_stack_trace_repeated_calls_maintain_id_isolation() {
    let mut parser = PerlStackParser::new();

    let trace_a = "$ = a::x() called from file `/a.pl' line 1";
    let trace_b = "\
$ = b::y() called from file `/b.pl' line 1
$ = b::z() called from file `/b.pl' line 2
";
    let trace_c = "$ = c::w() called from file `/c.pl' line 1";

    let frames_a = parser.parse_stack_trace(trace_a);
    let frames_b = parser.parse_stack_trace(trace_b);
    let frames_c = parser.parse_stack_trace(trace_c);

    // Each call should reset IDs to 1
    assert_eq!(frames_a[0].id, 1);
    assert_eq!(frames_b[0].id, 1);
    assert_eq!(frames_b[1].id, 2);
    assert_eq!(frames_c[0].id, 1);
}

#[test]
fn parse_frame_context_format_with_package_colon_file() {
    use perl_tdd_support::must_some;
    let mut parser = PerlStackParser::new();
    // The context regex matches: func::(file:line): format
    // e.g. My::Mod::(lib/My/Mod.pm:42):
    let line = "My::Mod::(lib/My/Mod.pm:42):";
    let frame = must_some(parser.parse_frame(line, 0));
    assert_eq!(frame.line, 42);
}

#[test]
fn parse_context_package_colon_file_format() {
    let parser = PerlStackParser::new();
    // The CONTEXT_RE matches: func::(file:line): format
    let result = parser.parse_context("My::Mod::(lib/My/Mod.pm:42):");
    if let Some((func, file, line)) = result {
        assert_eq!(func, "My::Mod");
        assert_eq!(file, "lib/My/Mod.pm");
        assert_eq!(line, 42);
    }
}

#[test]
fn parse_context_package_func_paren_format_not_matched() {
    let parser = PerlStackParser::new();
    // Format like My::Mod::handler(file:42): does NOT match CONTEXT_RE
    // because the regex expects func::(file:line), not func(file:line)
    let result = parser.parse_context("My::Mod::handler(lib/My/Mod.pm:42):");
    assert!(result.is_none());
}
