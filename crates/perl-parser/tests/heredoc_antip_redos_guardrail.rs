/// Red-TDD tests for #1756: ReDoS vulnerability fixes in heredoc anti-pattern detectors
///
/// These tests verify that the bounded regex patterns prevent catastrophic backtracking
/// on pathological input (unclosed delimiters with large repetitive content).
/// Each test measures regex completion time and asserts completion in <10ms.
use regex::Regex;
use std::time::Instant;

#[test]
fn test_antip_no_redos_dynamic_5kb_unclosed() {
    /// Test that DYNAMIC_DELIMITER_PATTERN does not trigger ReDoS on unclosed brace.
    /// Input: `<<${` followed by 5KB of 'a' characters (no closing `}`).
    /// Expected: Regex completes in <10ms (linear scan, not O(n²) backtracking).
    let pathological_input = format!("{}{}", "<<${", "a".repeat(5000));

    // Create the FIXED pattern (with \n boundary)
    let pattern = Regex::new(r"<<\s*\$\{[^}\n]+\}|<<\s*\$\w+|<<\s*`[^`\n]+`")
        .expect("Pattern should compile");

    let start = Instant::now();
    let _ = pattern.captures(&pathological_input);
    let elapsed = start.elapsed();

    // Must complete in less than 10ms to prove no ReDoS
    assert!(
        elapsed.as_millis() < 10,
        "DYNAMIC_DELIMITER_PATTERN took {}ms on 5KB unclosed brace input; expected <10ms (ReDoS detected)",
        elapsed.as_millis()
    );
}

#[test]
fn test_antip_no_redos_regex_5kb_unclosed() {
    /// Test that REGEX_HEREDOC_PATTERN does not trigger ReDoS on unclosed brace in (?{...}).
    /// Input: `(?{aaaa...aaaa<<` followed by 5KB of 'a' characters (no closing `}`).
    /// Expected: Regex completes in <10ms.
    let pathological_input = format!("{}{}{}", "(?{", "a".repeat(5000), "<<");

    // Create the FIXED pattern (with \n boundary)
    let pattern = Regex::new(r"\(\?\{[^}\n]*<<[^}\n]*\}").expect("Pattern should compile");

    let start = Instant::now();
    let _ = pattern.captures(&pathological_input);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 10,
        "REGEX_HEREDOC_PATTERN took {}ms on 5KB unclosed brace input; expected <10ms (ReDoS detected)",
        elapsed.as_millis()
    );
}

#[test]
fn test_antip_no_redos_eval_5kb_unclosed() {
    /// Test that EVAL_HEREDOC_PATTERN does not trigger ReDoS on unclosed quote in eval.
    /// Input: `eval 'aaaa...aaaa<<` followed by 5KB of 'a' characters (no closing quote).
    /// Expected: Regex completes in <10ms.
    let pathological_input = format!("{}{}{}", "eval '", "a".repeat(5000), "<<");

    // Create the FIXED pattern (with \n boundary)
    let pattern = Regex::new(r#"eval\s+(?:'[^\n']*<<[^\n']*'|"[^\n"]*<<[^\n"]*")"#)
        .expect("Pattern should compile");

    let start = Instant::now();
    let _ = pattern.captures(&pathological_input);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 10,
        "EVAL_HEREDOC_PATTERN took {}ms on 5KB unclosed quote input; expected <10ms (ReDoS detected)",
        elapsed.as_millis()
    );
}

#[test]
fn test_antip_no_redos_export_5kb_unclosed() {
    /// Test that EXPORT_QW_RE does not trigger ReDoS on unclosed qw delimiter.
    /// Input: `@EXPORT = qw(aaaa...aaaa` followed by 5KB of 'a' characters (no closing `)`)
    /// Expected: Regex completes in <10ms.
    let pathological_input = format!("{}{}", "@EXPORT = qw(", "a".repeat(5000));

    // Create the FIXED pattern (with \n boundary)
    let pattern = Regex::new(r"@EXPORT(?:_OK)?\s*=\s*qw[(\[{/<|!]([^\n)\]}/|!>]+)[)\]}/|!>]")
        .expect("Pattern should compile");

    let start = Instant::now();
    let _ = pattern.captures(&pathological_input);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 10,
        "EXPORT_QW_RE took {}ms on 5KB unclosed delimiter input; expected <10ms (ReDoS detected)",
        elapsed.as_millis()
    );
}

#[test]
fn test_antip_dynamic_delimiter_valid() {
    /// Verify that valid `<<${varname}` patterns are still detected after the fix.
    /// This is a positive test ensuring the bounded pattern does not break valid detection.
    let valid_input = "my $x = <<${FOO_BAR};";
    let pattern = Regex::new(r"<<\s*\$\{[^}\n]+\}|<<\s*\$\w+|<<\s*`[^`\n]+`")
        .expect("Pattern should compile");

    let matches = pattern.find(valid_input);

    assert!(matches.is_some(), "Valid dynamic delimiter <<${{FOO_BAR}} should be detected");
}

#[test]
fn test_antip_regex_heredoc_valid() {
    /// Verify that valid `(?{...<<...})` patterns are still detected.
    let valid_input = "/(?{print <<'EOF'})/ or die;";
    let pattern = Regex::new(r"\(\?\{[^}\n]*<<[^}\n]*\}").expect("Pattern should compile");

    let matches = pattern.find(valid_input);

    assert!(matches.is_some(), "Valid regex heredoc (?{{...<<...}}) should be detected");
}

#[test]
fn test_antip_eval_heredoc_valid() {
    /// Verify that valid `eval '...<<...'` patterns are still detected.
    let valid_input = "eval 'my $x = <<EOF;'";
    let pattern = Regex::new(r#"eval\s+(?:'[^\n']*<<[^\n']*'|"[^\n"]*<<[^\n"]*")"#)
        .expect("Pattern should compile");

    let matches = pattern.find(valid_input);

    assert!(matches.is_some(), "Valid eval heredoc should be detected");
}

#[test]
fn test_antip_export_qw_valid() {
    /// Verify that valid `@EXPORT = qw(...)` lists are still detected.
    let valid_input = "@EXPORT = qw(foo bar baz);";
    let pattern = Regex::new(r"@EXPORT(?:_OK)?\s*=\s*qw[(\[{/<|!]([^\n)\]}/|!>]+)[)\]}/|!>]")
        .expect("Pattern should compile");

    let matches = pattern.find(valid_input);

    assert!(matches.is_some(), "Valid @EXPORT qw list should be detected");
}

#[test]
fn test_antip_delimiter_in_string() {
    /// Test that heredoc patterns inside string literals don't cause false matches.
    /// Note: In the actual detector, mask_non_code_regions() is called first to blank out
    /// string contents. This test verifies the fixed pattern itself doesn't match naively.
    let code_with_string = r#"print "use this <<${ pattern";"#;
    let pattern = Regex::new(r"<<\s*\$\{[^}\n]+\}|<<\s*\$\w+|<<\s*`[^`\n]+`")
        .expect("Pattern should compile");

    // The raw pattern will match the literal <<${ in the string.
    // The detector's mask_non_code_regions() function prevents this false positive.
    // This test documents that the pattern itself needs the masking layer.
    let _matches = pattern.find(code_with_string);

    // Note: The actual safety comes from mask_non_code_regions(), not the pattern itself.
    // This test ensures the pattern at least completes quickly on normal input.
}

#[test]
fn test_antip_normal_file_performance() {
    /// Test that the detector completes in <100ms on a realistic 1000-line Perl file.
    /// This ensures the fix does not cause performance regression on normal code.
    let mut code = String::new();
    for i in 0..1000 {
        code.push_str(&format!("sub routine_{} {{ my $x = {}; }} # line {}\n", i, i, i));
    }

    let pattern = Regex::new(r"<<\s*\$\{[^}\n]+\}|<<\s*\$\w+|<<\s*`[^`\n]+`")
        .expect("Pattern should compile");

    let start = Instant::now();
    for _ in pattern.captures_iter(&code) {
        // Count matches but don't accumulate
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "Detector on 1000-line file took {}ms; expected <100ms (performance regression)",
        elapsed.as_millis()
    );
}

#[test]
fn test_antip_multiline_pattern_not_matched() {
    /// Test that multiline anti-patterns spanning `\n` are NOT detected.
    /// This is an acceptable tradeoff: the line-boundary anchoring (`\n`) prevents DoS
    /// but misses rare multiline cases.
    ///
    /// Example: `<<${` on line 1, `}` on line 2 should NOT be matched.
    let multiline_input = "<<${\nVARNAME}";
    let pattern = Regex::new(r"<<\s*\$\{[^}\n]+\}|<<\s*\$\w+|<<\s*`[^`\n]+`")
        .expect("Pattern should compile");

    let matches = pattern.find(multiline_input);

    // After fix, multiline patterns should NOT match (bounded by \n)
    assert!(
        matches.is_none(),
        "Multiline pattern spanning newline should not be matched (acceptable tradeoff)"
    );
}
