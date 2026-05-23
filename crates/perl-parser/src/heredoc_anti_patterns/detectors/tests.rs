use super::AntiPatternDetector;
use crate::heredoc_anti_patterns::model::AntiPattern;

#[test]
fn test_format_heredoc_detection() {
    let detector = AntiPatternDetector::new();
    let code = r#"
format REPORT =
<<'END'
Name: @<<<<<<<<<<<<
$name
END
.
"#;

    let diagnostics = detector.detect_all(code);
    // Note: DynamicDelimiterDetector might also flag the << inside the format body as a false positive.
    // But FormatHeredoc should appear first because it starts at 'format'.
    // So diagnostics[0] should be FormatHeredoc.
    assert!(!diagnostics.is_empty());
    assert!(matches!(diagnostics[0].pattern, AntiPattern::FormatHeredoc { .. }));

    if let AntiPattern::FormatHeredoc { heredoc_delimiter, .. } = &diagnostics[0].pattern {
        assert_eq!(heredoc_delimiter, "END");
    }
}

#[test]
fn test_begin_heredoc_detection() {
    let detector = AntiPatternDetector::new();
    let code = r###"
BEGIN {
    $config = <<'END';
    server = localhost
END
}
"###;

    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(diagnostics[0].pattern, AntiPattern::BeginTimeHeredoc { .. }));
}

#[test]
fn test_begin_heredoc_detection_with_nested_braces() {
    let detector = AntiPatternDetector::new();
    let code = r###"
BEGIN {
    if ($ENV{DEV}) {
        $config = <<'END';
        server = localhost
END
    }
}
"###;

    let diagnostics = detector.detect_all(code);
    let begin_count = diagnostics
        .iter()
        .filter(|diag| matches!(diag.pattern, AntiPattern::BeginTimeHeredoc { .. }))
        .count();
    assert_eq!(begin_count, 1);
}

#[test]
fn test_begin_heredoc_ignores_braces_in_comments() {
    let detector = AntiPatternDetector::new();
    let code = r###"
BEGIN {
    # comment with } brace
    $config = <<'END';
    server = localhost
END
}
"###;

    let diagnostics = detector.detect_all(code);
    let begin_count = diagnostics
        .iter()
        .filter(|diag| matches!(diag.pattern, AntiPattern::BeginTimeHeredoc { .. }))
        .count();
    assert_eq!(begin_count, 1);
}

#[test]
fn test_dynamic_delimiter_detection() {
    let detector = AntiPatternDetector::new();
    let code = r###"
my $delimiter = "EOF";
my $content = <<$delimiter;
This is dynamic
EOF
"###;

    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(diagnostics[0].pattern, AntiPattern::DynamicHeredocDelimiter { .. }));
}

#[test]
fn test_source_filter_detection() {
    let detector = AntiPatternDetector::new();
    let code = r###"
use Filter::Simple;
print <<EOF;
Filtered content
EOF
"###;
    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(diagnostics[0].pattern, AntiPattern::SourceFilterHeredoc { .. }));
}

#[test]
fn test_regex_heredoc_detection() {
    let detector = AntiPatternDetector::new();
    let code = r###"
m/pattern(?{
    print <<'MATCH';
    Match text
MATCH
})/
"###;
    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(diagnostics[0].pattern, AntiPattern::RegexCodeBlockHeredoc { .. }));
}

#[test]
fn test_eval_heredoc_detection() {
    let detector = AntiPatternDetector::new();
    let code = r###"
eval 'print <<"EVAL";
Eval content
EVAL';
"###;
    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(diagnostics[0].pattern, AntiPattern::EvalStringHeredoc { .. }));
}

#[test]
fn test_tied_handle_detection() {
    let detector = AntiPatternDetector::new();
    let code = r###"
tie *FH, 'Tie::Handle';
print FH <<'DATA';
Tied output
DATA
"###;
    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(diagnostics[0].pattern, AntiPattern::TiedHandleHeredoc { .. }));
}

#[test]
fn test_tied_scalar_handle_detection() {
    let detector = AntiPatternDetector::new();
    let code = r###"
tie $fh, 'Tie::Handle';
print $fh <<'DATA';
Tied output
DATA
"###;
    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(diagnostics[0].pattern, AntiPattern::TiedHandleHeredoc { .. }));
}

#[test]
fn test_tied_handle_reports_multiple_writes() {
    let detector = AntiPatternDetector::new();
    let code = r###"
tie *FH, 'Tie::Handle';
print FH <<'FIRST';
One
FIRST
print FH <<'SECOND';
Two
SECOND
"###;

    let diagnostics = detector.detect_all(code);
    let tied_handle_count = diagnostics
        .iter()
        .filter(|diag| matches!(diag.pattern, AntiPattern::TiedHandleHeredoc { .. }))
        .count();
    assert_eq!(tied_handle_count, 2);
}

#[test]
fn test_tied_handle_does_not_report_other_handles() {
    // Regression: PRINT_HEREDOC_PATTERN must only flag handles in the tied set.
    // Writing a heredoc to an *untied* handle (OTHER) must not produce a diagnostic.
    let detector = AntiPatternDetector::new();
    let code = r###"
tie *FH, 'Tie::Handle';
print OTHER <<'DATA';
Not tied
DATA
"###;

    let diagnostics = detector.detect_all(code);
    let tied_handle_count = diagnostics
        .iter()
        .filter(|diag| matches!(diag.pattern, AntiPattern::TiedHandleHeredoc { .. }))
        .count();
    assert_eq!(tied_handle_count, 0);
}

#[test]
fn test_location_column_is_zero_based_for_new_line_matches() {
    let detector = AntiPatternDetector::new();
    let code = "my $x = 1;\nuse Filter::Simple;\n";

    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);

    assert!(
        matches!(diagnostics[0].pattern, AntiPattern::SourceFilterHeredoc { .. }),
        "expected SourceFilterHeredoc pattern, got: {:?}",
        diagnostics[0].pattern
    );
    let AntiPattern::SourceFilterHeredoc { location, .. } = &diagnostics[0].pattern else {
        return;
    };

    assert_eq!(location.line, 1);
    assert_eq!(location.column, 0);
    assert_eq!(location.offset, 11);
}

#[test]
fn test_location_first_byte_is_line_zero_column_zero() {
    // A match at byte offset 0 must report line=0, column=0.
    let detector = AntiPatternDetector::new();
    let code = "use Filter::Simple;\n";

    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    let AntiPattern::SourceFilterHeredoc { location, .. } = &diagnostics[0].pattern else {
        unreachable!("expected SourceFilterHeredoc");
    };
    assert_eq!(location.line, 0, "first-byte match must be on line 0");
    assert_eq!(location.column, 0, "first-byte match must be at column 0");
    assert_eq!(location.offset, 0);
}

#[test]
fn test_location_third_line_accurate() {
    // Three-line file — match on line 2, column 0.
    let detector = AntiPatternDetector::new();
    // Line 0: "my $a = 1;\n"  (11 bytes, \n at index 10)
    // Line 1: "my $b = 2;\n"  (11 bytes, \n at index 21)
    // Line 2: "use Filter::Simple;\n"
    let code = "my $a = 1;\nmy $b = 2;\nuse Filter::Simple;\n";

    let diagnostics = detector.detect_all(code);
    assert_eq!(diagnostics.len(), 1);
    let AntiPattern::SourceFilterHeredoc { location, .. } = &diagnostics[0].pattern else {
        unreachable!("expected SourceFilterHeredoc");
    };
    assert_eq!(location.line, 2, "match on third line must report line 2");
    assert_eq!(location.column, 0, "match at start of line must report column 0");
    assert_eq!(location.offset, 22, "byte offset of third-line start");
}

#[test]
fn test_location_mid_line_column_nonzero() {
    // Match that does not start at column 0 must report the correct column.
    // Line 0: "# comment\n"      (10 bytes, \n at index 9)
    // Line 1: "    use Filter::Simple;\n"  — 4 leading spaces, match at column 4
    let detector = AntiPatternDetector::new();
    let code = "# comment\n    use Filter::Simple;\n";

    let diagnostics = detector.detect_all(code);
    // The comment is masked; only SourceFilterHeredoc on line 1 should fire.
    assert_eq!(diagnostics.len(), 1);
    let AntiPattern::SourceFilterHeredoc { location, .. } = &diagnostics[0].pattern else {
        unreachable!("expected SourceFilterHeredoc");
    };
    assert_eq!(location.line, 1);
    assert_eq!(location.column, 4, "mid-line match must report correct column");
    assert_eq!(location.offset, 14, "byte offset = 10 (first line) + 4 spaces");
}

#[test]
fn test_source_filter_detection_ignores_comments_and_strings() {
    let detector = AntiPatternDetector::new();
    let code = r#"
# use Filter::Simple;
my $s = "use Filter::Simple";
"#;

    let diagnostics = detector.detect_all(code);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_begin_detection_ignores_comments_and_strings() {
    let detector = AntiPatternDetector::new();
    let code = r#"
# BEGIN { my $x = <<'END'; END }
my $s = "BEGIN { my $x = <<'END'; END }";
"#;

    let diagnostics = detector.detect_all(code);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_format_detection_handles_utf8_in_masked_regions() {
    let detector = AntiPatternDetector::new();
    let code = r#"# comment with emoji 😀
format REPORT =
<<'END'
Body
END
.
"#;

    let diagnostics = detector.detect_all(code);
    assert!(
        diagnostics.iter().any(|diag| matches!(diag.pattern, AntiPattern::FormatHeredoc { .. }))
    );
}
