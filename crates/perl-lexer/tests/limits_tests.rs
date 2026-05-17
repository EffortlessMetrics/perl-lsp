//! Tests for `crates/perl-lexer/src/limits.rs` — parse-budget constants.
//!
//! # Coverage strategy
//!
//! `MAX_REGEX_PARSE_STEPS` is the only constant with `pub` visibility, so it
//! is the only one verifiable directly from an integration test.  The remaining
//! budgets (`MAX_REGEX_BYTES`, `MAX_HEREDOC_BYTES`, `MAX_DELIM_NEST`,
//! `MAX_HEREDOC_DEPTH`, `HEREDOC_TIMEOUT_MS`) are `pub(crate)` and are tested
//! indirectly through their observable lexer behaviour.
//!
//! Existing test suites (`lexer_catastrophic_regex_test`, `heredoc_security_tests`,
//! `lexer_slash_timeout_tests`, `hang_risk_regex_literal_tests`) already cover
//! many behavioural paths.  This suite focuses on:
//!
//! 1. Sanity / invariant checks on `MAX_REGEX_PARSE_STEPS`.
//! 2. The ordering invariant: the step budget fires before the byte budget.
//! 3. Quote-like operator deep-nesting (not covered by other suites).
//! 4. Wall-clock termination assertion for specific pathological inputs
//!    (hang-detector, 2-second ceiling — generous, not a latency benchmark).

use perl_lexer::{MAX_REGEX_PARSE_STEPS, PerlLexer, TokenType};
use std::time::Instant;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// 1. Sanity checks on the `MAX_REGEX_PARSE_STEPS` constant
// ---------------------------------------------------------------------------

/// The step budget must be strictly positive.
#[test]
fn limits_max_regex_parse_steps_is_positive() {
    const { assert!(MAX_REGEX_PARSE_STEPS > 0, "MAX_REGEX_PARSE_STEPS must be > 0") };
}

/// The constant is documented as 32 * 1024; pin it so regressions are caught.
#[test]
fn limits_max_regex_parse_steps_pinned_value() {
    assert_eq!(
        MAX_REGEX_PARSE_STEPS,
        32 * 1024,
        "MAX_REGEX_PARSE_STEPS changed unexpectedly; update the doc comment and this pin"
    );
}

/// The step budget must be smaller than the byte budget (64 KB = 65_536).
/// This invariant is stated in the doc comment of `MAX_REGEX_PARSE_STEPS`.
#[test]
fn limits_step_budget_below_byte_budget() {
    // MAX_REGEX_BYTES is 64 * 1024 = 65_536 (pub(crate), verified via the hard-coded value).
    const MAX_REGEX_BYTES: usize = 64 * 1024;
    const {
        assert!(
            MAX_REGEX_PARSE_STEPS < MAX_REGEX_BYTES,
            "MAX_REGEX_PARSE_STEPS must be < MAX_REGEX_BYTES",
        )
    };
}

// ---------------------------------------------------------------------------
// 2. Ordering invariant: step budget fires before byte budget
// ---------------------------------------------------------------------------

/// A pattern whose byte length is less than 64 KB but whose step count exceeds
/// `MAX_REGEX_PARSE_STEPS` must emit `UnknownRest` — the step guard is
/// reachable before the byte guard on this input.
#[test]
fn limits_step_budget_reachable_before_byte_budget() -> TestResult {
    // 33 * 1024 bytes of 'a' — above the 32 K step limit, below the 64 KB byte limit.
    let pattern = "a".repeat(MAX_REGEX_PARSE_STEPS + 1_024);
    let input = format!("/{pattern}/");

    assert!(
        input.len() < 64 * 1024,
        "Test input ({} bytes) must stay below the byte budget for the step guard to be the first to fire",
        input.len()
    );

    let mut lexer = PerlLexer::new(&input);
    let tokens = lexer.collect_tokens();

    let found_unknown_rest = tokens.iter().any(|t| matches!(t.token_type, TokenType::UnknownRest));
    assert!(
        found_unknown_rest,
        "Pattern exceeding MAX_REGEX_PARSE_STEPS but under MAX_REGEX_BYTES should emit UnknownRest"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Quote-like operator deep-nesting
// ---------------------------------------------------------------------------

/// `q{}` with 200 levels of `{` nesting (exceeds MAX_DELIM_NEST = 128).
/// The lexer must terminate and emit either a valid token or `UnknownRest`.
#[test]
fn limits_q_operator_excessive_nesting_terminates() -> TestResult {
    let depth = 200_usize;
    let mut input = String::from("q{");
    for _ in 0..depth {
        input.push('{');
    }
    input.push_str("hello");
    for _ in 0..depth {
        input.push('}');
    }
    input.push('}');

    let start = Instant::now();
    let mut lexer = PerlLexer::new(&input);
    let tokens = lexer.collect_tokens();
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs() < 2, "Lexer hung on deeply-nested q{{}} (elapsed {:?})", elapsed);
    assert!(!tokens.is_empty(), "Lexer must emit at least one token for deeply-nested q{{}}");
    Ok(())
}

/// `qq{}` with 200 levels of nesting (double-quoted version).
#[test]
fn limits_qq_operator_excessive_nesting_terminates() -> TestResult {
    let depth = 200_usize;
    let mut input = String::from("qq{");
    for _ in 0..depth {
        input.push('{');
    }
    input.push_str("text");
    for _ in 0..depth {
        input.push('}');
    }
    input.push('}');

    let start = Instant::now();
    let mut lexer = PerlLexer::new(&input);
    let tokens = lexer.collect_tokens();
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs() < 2, "Lexer hung on deeply-nested qq{{}} (elapsed {:?})", elapsed);
    assert!(!tokens.is_empty(), "Lexer must emit at least one token for deeply-nested qq{{}}");
    Ok(())
}

/// `m{}` with 200 levels of `{` nesting (regex with brace delimiters).
/// This specifically exercises the delimiter-nesting budget path for the
/// match operator, complementing the `/…/` style covered elsewhere.
#[test]
fn limits_m_brace_operator_excessive_nesting_terminates() -> TestResult {
    let depth = 200_usize;
    let mut input = String::from("m{");
    for _ in 0..depth {
        input.push('{');
    }
    input.push('x');
    for _ in 0..depth {
        input.push('}');
    }
    input.push('}');

    let start = Instant::now();
    let mut lexer = PerlLexer::new(&input);
    let tokens = lexer.collect_tokens();
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs() < 2, "Lexer hung on deeply-nested m{{}} (elapsed {:?})", elapsed);
    assert!(!tokens.is_empty(), "Lexer must emit at least one token for deeply-nested m{{}}");
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Wall-clock termination for pathological heredoc input
// ---------------------------------------------------------------------------

/// A heredoc body that exceeds MAX_HEREDOC_BYTES (256 KB).
/// The lexer must terminate — truncation or error is acceptable.
#[test]
fn limits_heredoc_oversized_body_terminates() -> TestResult {
    // 270 KB body — larger than MAX_HEREDOC_BYTES (256 KB).
    let line = "some heredoc content line\n";
    let repeat_count = (270 * 1024) / line.len() + 1;
    let mut input = String::from("my $x = <<END;\n");
    for _ in 0..repeat_count {
        input.push_str(line);
    }
    // Deliberately omit the terminator to maximise stress on the budget guard.

    let start = Instant::now();
    let mut lexer = PerlLexer::new(&input);
    let tokens = lexer.collect_tokens();
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs() < 2, "Lexer hung on oversized heredoc body (elapsed {:?})", elapsed);
    assert!(!tokens.is_empty(), "Lexer must emit at least one token even for an oversized heredoc");
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Boundary: pattern just at MAX_REGEX_PARSE_STEPS (last valid step)
// ---------------------------------------------------------------------------

/// A pattern of exactly `MAX_REGEX_PARSE_STEPS - 1` characters must parse
/// successfully (not emit `UnknownRest`).
#[test]
fn limits_regex_at_step_boundary_parses_successfully() -> TestResult {
    let pattern = "a".repeat(MAX_REGEX_PARSE_STEPS - 1);
    let input = format!("/{pattern}/");

    assert!(
        input.len() < 64 * 1024,
        "Boundary test must remain below the byte budget ({} bytes)",
        input.len()
    );

    let mut lexer = PerlLexer::new(&input);
    let tokens = lexer.collect_tokens();

    let has_unknown_rest = tokens.iter().any(|t| matches!(t.token_type, TokenType::UnknownRest));
    assert!(
        !has_unknown_rest,
        "Pattern of exactly MAX_REGEX_PARSE_STEPS-1 chars should not trigger the step budget"
    );

    let has_regex = tokens.iter().any(|t| matches!(t.token_type, TokenType::RegexMatch));
    assert!(
        has_regex,
        "Pattern just under MAX_REGEX_PARSE_STEPS should produce a RegexMatch token"
    );
    Ok(())
}
