//! BDD-style UX workflows for completion behavior.
//!
//! Focus: a real editor lifecycle (open -> complete -> edit -> complete) should
//! keep suggestions fresh and never leak stale symbols.

mod support;

use serial_test::serial;
use support::lsp_harness::LspHarness;
use support::ux_bdd::{UxScenario, completion_contains_label, completion_labels};

fn completion_fixture_v1() -> &'static str {
    r#"my $count = 1;
my $counter = 2;

$cou
"#
}

fn completion_fixture_v2() -> &'static str {
    r#"my $total = 1;
my $topic = 2;

$to
"#
}

#[test]
#[serial]
fn bdd_completion_refreshes_after_full_document_change() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = UxScenario::new("Completion refreshes after edits");
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///ux-completion-refresh.pl";

    scenario.given("an opened document with `$count` and `$counter` variables");
    harness.open(uri, completion_fixture_v1())?;
    harness.barrier();

    scenario.when("requesting completion at `$cou`");
    let initial = harness.completion_at(uri, 3, 4)?;

    scenario.then("the completion list contains `$count` and `$counter`");
    assert!(completion_contains_label(&initial, "$count"));
    assert!(completion_contains_label(&initial, "$counter"));

    scenario.when("the user replaces the document with `$total` and `$topic`");
    harness.change_full(uri, 2, completion_fixture_v2())?;
    harness.barrier();

    scenario.then("completion at `$to` shows only the new variable family");
    let refreshed = harness.completion_at(uri, 3, 3)?;
    assert!(completion_contains_label(&refreshed, "$total"));
    assert!(completion_contains_label(&refreshed, "$topic"));
    assert!(
        !completion_contains_label(&refreshed, "$count"),
        "stale labels after didChange: {:?}",
        completion_labels(&refreshed)
    );

    Ok(())
}

#[test]
#[serial]
fn bdd_completion_round_trip_edit_preserves_expected_symbols()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = UxScenario::new("Completion round-trip editing");
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///ux-completion-roundtrip.pl";

    scenario.given("the first completion fixture is opened");
    harness.open(uri, completion_fixture_v1())?;
    harness.barrier();

    scenario.when("the user edits to a different completion prefix and then undoes");
    harness.change_full(uri, 2, completion_fixture_v2())?;
    harness.change_full(uri, 3, completion_fixture_v1())?;
    harness.barrier();

    scenario.then("completion for `$cou` returns the original symbol set again");
    let round_trip = harness.completion_at(uri, 3, 4)?;

    assert!(completion_contains_label(&round_trip, "$count"));
    assert!(completion_contains_label(&round_trip, "$counter"));
    assert!(
        !completion_contains_label(&round_trip, "$total"),
        "unexpected carry-over labels after undo-like edit: {:?}",
        completion_labels(&round_trip)
    );

    Ok(())
}
