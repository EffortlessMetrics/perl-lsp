//! Tests for the TAP reader.

use super::*;

#[test]
fn parses_simple_pass() {
    let report = parse_tap("1..2\nok 1 - first\nok 2 - second\n");
    assert_eq!(report.plan, Some(TapPlan { count: 2, skip_all: None }));
    assert_eq!(report.tests.len(), 2);
    assert!(report.passed());
    assert_eq!(report.summary.passed, 2);
    assert_eq!(report.summary.failed, 0);
    assert!(report.failures().is_empty());
    assert!(report.plan_mismatch().is_none());
}

#[test]
fn parses_failure_with_source_location() {
    let output = "1..2\n\
        ok 1 - addition works\n\
        not ok 2 - email matches\n\
        #   Failed test 'email matches'\n\
        #   at t/user.t line 12.\n\
        #          got: 'wrong@example.com'\n\
        #     expected: 'a@example.com'\n";
    let report = parse_tap(output);
    assert!(!report.passed());
    let failures = report.failures();
    assert_eq!(failures.len(), 1);
    let fail = failures[0];
    assert_eq!(fail.number, Some(2));
    assert_eq!(fail.description, "email matches");
    assert_eq!(fail.file.as_deref(), Some("t/user.t"));
    assert_eq!(fail.line, Some(12));
    assert_eq!(fail.got.as_deref(), Some("'wrong@example.com'"));
    assert_eq!(fail.expected.as_deref(), Some("'a@example.com'"));
}

#[test]
fn todo_failure_is_not_a_hard_failure() {
    let report = parse_tap("1..1\nnot ok 1 - not yet # TODO implement later\n");
    assert!(report.passed(), "a TODO failure must not fail the run");
    assert_eq!(report.summary.failed, 0);
    assert_eq!(report.summary.todo, 1);
    let test = &report.tests[0];
    assert!(!test.is_failure());
    assert!(test.is_todo());
    assert_eq!(test.directive, Some(TapDirective::Todo("implement later".to_string())));
    assert_eq!(test.description, "not yet");
}

#[test]
fn skip_is_not_a_failure() {
    let report = parse_tap("1..1\nok 1 - platform specific # SKIP not on windows\n");
    assert!(report.passed());
    assert_eq!(report.summary.skipped, 1);
    assert_eq!(report.summary.passed, 0, "a SKIP is not counted as a plain pass");
    let test = &report.tests[0];
    assert!(test.is_skipped());
    assert_eq!(test.directive, Some(TapDirective::Skip("not on windows".to_string())));
}

#[test]
fn skip_all_plan() {
    let report = parse_tap("1..0 # SKIP no database configured\n");
    let plan = report.plan.clone().expect("plan present");
    assert_eq!(plan.count, 0);
    assert_eq!(plan.skip_all.as_deref(), Some("no database configured"));
    assert!(report.plan_mismatch().is_none(), "skip-all is not a plan mismatch");
}

#[test]
fn bail_out_is_captured() {
    let report = parse_tap("1..5\nok 1\nBail out! Database connection lost\n");
    assert_eq!(report.bailed_out.as_deref(), Some("Database connection lost"));
    assert!(!report.passed(), "a bail-out is not a pass");
}

#[test]
fn nested_subtest_indentation_tracks_depth() {
    // Explicit spaces — a `\`-continuation would strip the TAP indentation.
    let output = "# Subtest: user lookup\n    ok 1 - found\n    not ok 2 - email\nnot ok 1 - user lookup\n1..1\n";
    let report = parse_tap(output);
    // The two indented lines are depth 1; the summary line is depth 0.
    let nested: Vec<_> = report.tests.iter().filter(|t| t.depth == 1).collect();
    assert_eq!(nested.len(), 2);
    let top: Vec<_> = report.tests.iter().filter(|t| t.depth == 0).collect();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].description, "user lookup");
    // One nested failure + one top-level summary failure.
    assert_eq!(report.summary.failed, 2);
}

#[test]
fn plan_mismatch_detected() {
    let report = parse_tap("1..3\nok 1\nok 2\n");
    assert_eq!(report.plan_mismatch(), Some((2, 3)));
    // The run still has no hard failures — mismatch is reported separately.
    assert!(report.passed());
}

#[test]
fn test_line_without_number_or_description() {
    let report = parse_tap("ok\nnot ok\n");
    assert_eq!(report.tests.len(), 2);
    assert!(report.tests[0].ok);
    assert!(!report.tests[1].ok);
    assert!(report.tests[1].is_failure());
}

#[test]
fn tap_version_header_ignored() {
    let report = parse_tap("TAP version 13\n1..1\nok 1 - ok\n");
    assert_eq!(report.tests.len(), 1);
    assert!(report.passed());
}

#[test]
fn diagnostics_before_any_test_are_ignored() {
    let report = parse_tap("# a preamble comment\n1..1\nok 1\n");
    assert_eq!(report.tests.len(), 1);
    assert!(report.tests[0].diagnostics.is_empty());
}

#[test]
fn focus_subtest_matches_summary_line_and_inner_failures() {
    // Buffered subtest: two nested lines then a depth-0 summary line.
    let output =
        "    ok 1 - found\n    not ok 2 - email\nnot ok 1 - user lookup\nok 2 - other\n1..2\n";
    let report = parse_tap(output);

    let focus = focus_subtest(&report, "user lookup").expect("subtest present");
    assert!(focus.found);
    assert!(!focus.passed, "the subtest summary line is `not ok`");
    assert_eq!(focus.inner_failed, 1, "one nested `not ok` belongs to the subtest");

    // A passing subtest.
    let focus_other = focus_subtest(&report, "other").expect("subtest present");
    assert!(focus_other.passed);
    assert_eq!(focus_other.inner_failed, 0);
}

#[test]
fn focus_subtest_absent_returns_none() {
    let report = parse_tap("1..1\nok 1 - something\n");
    assert!(focus_subtest(&report, "no such subtest").is_none());
}

#[test]
fn todo_that_unexpectedly_passes_is_not_a_failure() {
    let report = parse_tap("1..1\nok 1 - surprise # TODO should fail\n");
    assert!(report.passed());
    assert_eq!(report.summary.todo, 1);
    assert_eq!(report.summary.failed, 0);
}
