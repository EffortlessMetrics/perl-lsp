use color_eyre::eyre::{Result, bail, eyre};
use perl_lsp_rs_core::providers::inline_completion::{
    InlineCompletionEnvironment, InlineCompletionItem, InlineCompletionList,
    InlineCompletionProvider,
};
use perl_parser_core::{
    Parser, RecoverySalvageProfile,
    position::{offset_to_utf16_line_col, utf16_line_col_to_offset},
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

const CURSOR: &str = "<<CURSOR>>";
const SUPPRESSION_HARD_ZONE: &str = "hard_zone";
const SUPPRESSION_NO_VISIBLE_CONTEXT: &str = "no_visible_context";

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct InlineCompletionQualityReceipt {
    schema_version: &'static str,
    provider: &'static str,
    provider_action: &'static str,
    claim_boundary: &'static str,
    fixtures_total: usize,
    fixtures_passed: usize,
    latency_p95_ms: u128,
    checks: InlineCompletionQualityChecks,
    sources: BTreeMap<String, SourceQualityReceipt>,
    scenarios: Vec<ScenarioQualityReceipt>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
struct InlineCompletionQualityChecks {
    expected_text: CountReceipt,
    silence: CountReceipt,
    replacement_range: CountReceipt,
    edit_application: CountReceipt,
    hard_zone_rejected: usize,
    suppression_reasons: BTreeMap<String, usize>,
    parse_regressions: usize,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
struct SourceQualityReceipt {
    expected: usize,
    passed: usize,
    failed: usize,
    returned_items: usize,
    edit_application: CountReceipt,
    parse_regressions: usize,
    suppression_reasons: BTreeMap<String, usize>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
struct CountReceipt {
    total: usize,
    passed: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ScenarioQualityReceipt {
    name: &'static str,
    source: &'static str,
    outcome: &'static str,
    item_count: usize,
    latency_ms: u128,
    notes: Vec<String>,
}

struct Scenario {
    name: &'static str,
    source_name: &'static str,
    source: &'static str,
    available_modules: &'static [&'static str],
    hard_zone: bool,
    assertion: ScenarioAssertion,
}

enum ScenarioAssertion {
    Suggestion {
        first: Option<&'static str>,
        expected: &'static [&'static str],
        not_expected: &'static [&'static str],
    },
    Silent,
    ReplacementRange {
        insert_text: &'static str,
        replaces: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditApplicationOutcome {
    NotApplicable,
    Passed,
    Failed,
}

#[derive(Debug, Clone, Copy)]
struct SourceScenarioOutcome {
    passed: bool,
    item_count: usize,
    parse_regressions: usize,
    edit_application: EditApplicationOutcome,
    suppression_reason: Option<&'static str>,
}

struct InlineCompletionScenario {
    text: String,
    line: u32,
    character: u32,
}

pub fn run(receipt: PathBuf) -> Result<()> {
    run_with_scenarios(receipt, scenarios())
}

fn run_with_scenarios(receipt: PathBuf, scenario_set: &[Scenario]) -> Result<()> {
    let provider = InlineCompletionProvider::new();
    let mut receipt_data = InlineCompletionQualityReceipt {
        schema_version: "inline-completion-quality.v1",
        provider: "inline_completion",
        provider_action: "deterministic_fixture_quality",
        claim_boundary: "local deterministic fixture receipt only; no telemetry upload and no AI/provider behavior change",
        fixtures_total: scenario_set.len(),
        fixtures_passed: 0,
        latency_p95_ms: 0,
        checks: InlineCompletionQualityChecks::default(),
        sources: BTreeMap::new(),
        scenarios: Vec::new(),
    };
    let mut latencies = Vec::new();
    let mut failures = Vec::new();

    for scenario in scenario_set {
        let started = Instant::now();
        let result = run_scenario(&provider, scenario);
        let latency_ms = started.elapsed().as_millis();
        latencies.push(latency_ms);

        match result {
            Ok((item_count, mut notes, parse_regressions, edit_application)) => {
                receipt_data.checks.parse_regressions += parse_regressions;
                record_edit_application(&mut receipt_data.checks, edit_application);
                let passed = parse_regressions == 0
                    && !matches!(edit_application, EditApplicationOutcome::Failed);
                update_check_counts(&mut receipt_data.checks, scenario, passed);
                let suppression_reason = measured_suppression_reason(scenario, item_count, passed);
                record_source_result(
                    &mut receipt_data.sources,
                    scenario.source_name,
                    SourceScenarioOutcome {
                        passed,
                        item_count,
                        parse_regressions,
                        edit_application,
                        suppression_reason,
                    },
                );

                if let Some(reason) = suppression_reason {
                    record_suppression_reason(&mut receipt_data.checks, reason);
                    notes.push(format!("suppression_reason={reason}"));
                    if reason == SUPPRESSION_HARD_ZONE {
                        receipt_data.checks.hard_zone_rejected += 1;
                        notes.push("hard zone stayed silent".to_string());
                    }
                }
                if parse_regressions == 0 {
                    if matches!(edit_application, EditApplicationOutcome::Failed) {
                        failures.push(format!(
                            "{}: top inline completion edit application was not parse-stable",
                            scenario.name
                        ));
                    } else {
                        receipt_data.fixtures_passed += 1;
                    }
                } else {
                    failures.push(format!(
                        "{}: {parse_regressions} returned item(s) worsened parse damage",
                        scenario.name
                    ));
                    notes.push(format!("parse_regressions={parse_regressions}"));
                }
                receipt_data.scenarios.push(ScenarioQualityReceipt {
                    name: scenario.name,
                    source: scenario.source_name,
                    outcome: if passed { "pass" } else { "fail" },
                    item_count,
                    latency_ms,
                    notes,
                });
            }
            Err(error) => record_scenario_error(
                &mut receipt_data,
                &mut failures,
                scenario,
                latency_ms,
                error.to_string(),
            ),
        }
    }

    receipt_data.latency_p95_ms = percentile95(&mut latencies);
    write_receipt(&receipt, &receipt_data)?;

    if failures.is_empty() {
        println!(
            "inline-completion quality receipt OK: {} fixtures, p95={}ms, {}",
            receipt_data.fixtures_passed,
            receipt_data.latency_p95_ms,
            receipt.display()
        );
        return Ok(());
    }

    bail!(
        "inline-completion quality receipt failed {} fixture(s): {}",
        failures.len(),
        failures.join("; ")
    )
}

fn record_source_result(
    sources: &mut BTreeMap<String, SourceQualityReceipt>,
    source_name: &str,
    outcome: SourceScenarioOutcome,
) {
    let source_entry = sources.entry(source_name.to_string()).or_default();
    source_entry.expected += 1;
    source_entry.returned_items += outcome.item_count;
    source_entry.parse_regressions += outcome.parse_regressions;
    record_edit_application_count(&mut source_entry.edit_application, outcome.edit_application);
    if let Some(reason) = outcome.suppression_reason {
        *source_entry.suppression_reasons.entry(reason.to_string()).or_default() += 1;
    }
    if outcome.passed {
        source_entry.passed += 1;
    } else {
        source_entry.failed += 1;
    }
}

fn record_scenario_error(
    receipt_data: &mut InlineCompletionQualityReceipt,
    failures: &mut Vec<String>,
    scenario: &Scenario,
    latency_ms: u128,
    message: String,
) {
    record_source_result(
        &mut receipt_data.sources,
        scenario.source_name,
        SourceScenarioOutcome {
            passed: false,
            item_count: 0,
            parse_regressions: 0,
            edit_application: EditApplicationOutcome::NotApplicable,
            suppression_reason: None,
        },
    );
    update_check_counts(&mut receipt_data.checks, scenario, false);
    failures.push(format!("{}: {message}", scenario.name));
    receipt_data.scenarios.push(ScenarioQualityReceipt {
        name: scenario.name,
        source: scenario.source_name,
        outcome: "fail",
        item_count: 0,
        latency_ms,
        notes: vec![message],
    });
}

fn run_scenario(
    provider: &InlineCompletionProvider,
    scenario: &Scenario,
) -> Result<(usize, Vec<String>, usize, EditApplicationOutcome)> {
    let fixture = InlineCompletionScenario::from_fixture(scenario.source)?;
    let environment = InlineCompletionEnvironment {
        available_modules: scenario
            .available_modules
            .iter()
            .map(|module| module.to_string())
            .collect(),
    };
    let completions = provider.get_inline_completions_with_environment(
        fixture.text.as_str(),
        fixture.line,
        fixture.character,
        &environment,
    );
    let item_count = completions.items.len();
    let mut notes = match scenario.assertion {
        ScenarioAssertion::Suggestion { first, expected, not_expected } => {
            assert_suggestion(scenario.name, &completions, first, expected, not_expected)?
        }
        ScenarioAssertion::Silent => {
            assert_silent(scenario.name, &completions)?;
            vec!["no items returned".to_string()]
        }
        ScenarioAssertion::ReplacementRange { insert_text, replaces } => assert_replacement_range(
            scenario.name,
            &fixture.text,
            &completions,
            insert_text,
            replaces,
        )?,
    };
    let edit_application =
        check_top_edit_application(scenario.name, scenario, &fixture, &completions, &mut notes)?;
    let parse_regressions =
        count_parse_regressions(scenario.name, &fixture, &completions, &mut notes)?;
    Ok((item_count, notes, parse_regressions, edit_application))
}

fn measured_suppression_reason(
    scenario: &Scenario,
    item_count: usize,
    passed: bool,
) -> Option<&'static str> {
    if !passed || item_count != 0 {
        return None;
    }

    match scenario.assertion {
        ScenarioAssertion::Silent if scenario.hard_zone => Some(SUPPRESSION_HARD_ZONE),
        ScenarioAssertion::Silent => Some(SUPPRESSION_NO_VISIBLE_CONTEXT),
        ScenarioAssertion::Suggestion { .. } | ScenarioAssertion::ReplacementRange { .. } => None,
    }
}

fn record_suppression_reason(checks: &mut InlineCompletionQualityChecks, reason: &str) {
    *checks.suppression_reasons.entry(reason.to_string()).or_default() += 1;
}

fn record_edit_application(
    checks: &mut InlineCompletionQualityChecks,
    outcome: EditApplicationOutcome,
) {
    record_edit_application_count(&mut checks.edit_application, outcome);
}

fn record_edit_application_count(counter: &mut CountReceipt, outcome: EditApplicationOutcome) {
    match outcome {
        EditApplicationOutcome::NotApplicable => {}
        EditApplicationOutcome::Passed => {
            counter.total += 1;
            counter.passed += 1;
        }
        EditApplicationOutcome::Failed => {
            counter.total += 1;
            counter.failed += 1;
        }
    }
}

fn update_check_counts(
    checks: &mut InlineCompletionQualityChecks,
    scenario: &Scenario,
    passed: bool,
) {
    let counter = match scenario.assertion {
        ScenarioAssertion::Suggestion { .. } => &mut checks.expected_text,
        ScenarioAssertion::Silent => &mut checks.silence,
        ScenarioAssertion::ReplacementRange { .. } => &mut checks.replacement_range,
    };
    counter.total += 1;
    if passed {
        counter.passed += 1;
    } else {
        counter.failed += 1;
    }
}

fn assert_suggestion(
    name: &str,
    completions: &InlineCompletionList,
    first: Option<&str>,
    expected: &[&str],
    not_expected: &[&str],
) -> Result<Vec<String>> {
    if let Some(first) = first {
        let actual = completions
            .items
            .first()
            .map(|item| item.insert_text.as_str())
            .ok_or_else(|| eyre!("{name}: expected first completion {first}, got none"))?;
        if actual != first {
            bail!("{name}: expected first completion {first}, got {actual}");
        }
    }

    for expected in expected {
        if !completions.items.iter().any(|item| item.insert_text == *expected) {
            bail!(
                "{name}: expected completion {expected}, got {:?}",
                completion_texts(completions)
            );
        }
    }

    for unexpected in not_expected {
        if completions.items.iter().any(|item| item.insert_text.contains(*unexpected)) {
            bail!(
                "{name}: unexpected completion containing {unexpected}, got {:?}",
                completion_texts(completions)
            );
        }
    }

    Ok(vec![format!("items={:?}", completion_texts(completions))])
}

fn assert_silent(name: &str, completions: &InlineCompletionList) -> Result<()> {
    if completions.items.is_empty() {
        return Ok(());
    }

    bail!("{name}: expected no inline completions, got {:?}", completion_texts(completions))
}

fn assert_replacement_range(
    name: &str,
    text: &str,
    completions: &InlineCompletionList,
    insert_text: &str,
    replaces: &str,
) -> Result<Vec<String>> {
    let item =
        completions.items.iter().find(|item| item.insert_text == insert_text).ok_or_else(|| {
            eyre!(
                "{name}: expected completion {insert_text}, got {:?}",
                completion_texts(completions)
            )
        })?;
    let range = item.range.as_ref().ok_or_else(|| eyre!("{name}: expected replacement range"))?;
    let start = utf16_line_col_to_offset(text, range.start.line, range.start.character);
    let end = utf16_line_col_to_offset(text, range.end.line, range.end.character);
    let replaced = text
        .get(start..end)
        .ok_or_else(|| eyre!("invalid UTF-8 boundaries for range {start}..{end}"))?;

    if replaced != replaces {
        bail!("{name}: expected replacement range to cover {replaces:?}, got {replaced:?}");
    }

    Ok(vec![format!("replaces={replaced:?}")])
}

fn check_top_edit_application(
    name: &str,
    scenario: &Scenario,
    fixture: &InlineCompletionScenario,
    completions: &InlineCompletionList,
    notes: &mut Vec<String>,
) -> Result<EditApplicationOutcome> {
    if matches!(scenario.assertion, ScenarioAssertion::Silent) {
        return Ok(EditApplicationOutcome::NotApplicable);
    }

    let Some(item) = completions.items.first() else {
        notes.push("edit_application=missing_top_item".to_string());
        return Ok(EditApplicationOutcome::Failed);
    };

    let current_line = fixture.current_line()?;
    let Some(probe) = parse_probe_after_item(current_line, item, fixture.line, fixture.character)?
    else {
        notes.push(format!("edit_application=unsupported_range for {:?}", item.insert_text));
        return Ok(EditApplicationOutcome::Failed);
    };

    let baseline = parse_damage_for_probe(current_line);
    let candidate = parse_damage_for_probe(probe.as_str());
    if candidate.worse_than(&baseline) {
        notes.push(format!(
            "{name}: edit_application=failed for {:?}; baseline={baseline:?}; candidate={candidate:?}",
            item.insert_text
        ));
        return Ok(EditApplicationOutcome::Failed);
    }

    notes.push(format!("edit_application=passed for {:?}", item.insert_text));
    Ok(EditApplicationOutcome::Passed)
}

fn count_parse_regressions(
    name: &str,
    fixture: &InlineCompletionScenario,
    completions: &InlineCompletionList,
    notes: &mut Vec<String>,
) -> Result<usize> {
    let current_line = fixture.current_line()?;
    let baseline = parse_damage_for_probe(current_line);
    let mut regressions = 0;
    let mut checked = 0;

    for item in &completions.items {
        let Some(probe) =
            parse_probe_after_item(current_line, item, fixture.line, fixture.character)?
        else {
            notes.push(format!("parse probe skipped for {:?}", item.insert_text));
            continue;
        };
        checked += 1;
        let candidate = parse_damage_for_probe(probe.as_str());
        if candidate.worse_than(&baseline) {
            regressions += 1;
            notes.push(format!(
                "parse regression for {:?}: baseline={baseline:?}; candidate={candidate:?}",
                item.insert_text
            ));
        }
    }

    notes.push(format!("parse_checked={checked}; parse_regressions={regressions}"));
    if regressions > 0 {
        notes.push(format!("{name}: returned completions must not worsen parse damage"));
    }
    Ok(regressions)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParseDamage {
    terminated_early: bool,
    error_node_count: usize,
    diagnostics_count: usize,
    recovered_count: usize,
}

impl ParseDamage {
    fn worse_than(&self, baseline: &Self) -> bool {
        (self.terminated_early && !baseline.terminated_early)
            || self.error_node_count > baseline.error_node_count
            || self.diagnostics_count > baseline.diagnostics_count
            || self.recovered_count > baseline.recovered_count
    }
}

fn parse_damage_for_probe(source: &str) -> ParseDamage {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let salvage = RecoverySalvageProfile::from_parse(&output.ast, &output.diagnostics, false);

    ParseDamage {
        terminated_early: output.terminated_early,
        error_node_count: salvage.error_node_count,
        diagnostics_count: output.error_count(),
        recovered_count: output.recovered_count,
    }
}

fn parse_probe_after_item(
    current_line: &str,
    item: &InlineCompletionItem,
    line: u32,
    character: u32,
) -> Result<Option<String>> {
    let Some((start_character, end_character)) = item
        .range
        .as_ref()
        .map(|range| {
            if range.start.line != line || range.end.line != line {
                return None;
            }
            Some((range.start.character, range.end.character))
        })
        .unwrap_or(Some((character, character)))
    else {
        return Ok(None);
    };

    let start = utf16_line_col_to_offset(current_line, 0, start_character);
    let end = utf16_line_col_to_offset(current_line, 0, end_character);
    if start > end {
        return Ok(None);
    }
    let before =
        current_line.get(..start).ok_or_else(|| eyre!("invalid UTF-8 range start {start}"))?;
    let after = current_line.get(end..).ok_or_else(|| eyre!("invalid UTF-8 range end {end}"))?;

    let mut probe = String::with_capacity(current_line.len() + item.insert_text.len());
    probe.push_str(before);
    probe.push_str(item.insert_text.as_str());
    probe.push_str(after);
    Ok(Some(probe))
}

fn completion_texts(completions: &InlineCompletionList) -> Vec<&str> {
    completions.items.iter().map(|item| item.insert_text.as_str()).collect()
}

fn percentile95(latencies: &mut [u128]) -> u128 {
    if latencies.is_empty() {
        return 0;
    }
    latencies.sort_unstable();
    let index = ((latencies.len() * 95).div_ceil(100)).saturating_sub(1);
    latencies[index]
}

fn write_receipt(path: &PathBuf, receipt: &InlineCompletionQualityReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(receipt)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

impl InlineCompletionScenario {
    fn from_fixture(fixture: &str) -> Result<Self> {
        let byte =
            fixture.find(CURSOR).ok_or_else(|| eyre!("fixture must include {CURSOR} marker"))?;
        let text = fixture.replacen(CURSOR, "", 1);
        let (line, character) = offset_to_utf16_line_col(&text, byte);

        Ok(Self { text, line, character })
    }

    fn current_line(&self) -> Result<&str> {
        let line_index = usize::try_from(self.line)?;
        self.text
            .split('\n')
            .nth(line_index)
            .ok_or_else(|| eyre!("fixture line {} is out of range", self.line))
    }
}

fn scenarios() -> &'static [Scenario] {
    &[
        Scenario {
            name: "use_pragmas",
            source_name: "syntax",
            source: "use <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("strict;"),
                expected: &["strict;", "warnings;"],
                not_expected: &["done_testing();"],
            },
        },
        Scenario {
            name: "use_namespace_prefers_available_project_module",
            source_name: "module",
            source: "use My::<<CURSOR>>",
            available_modules: &["My::App", "My::App::Config", "Other::Tool"],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("My::App;"),
                expected: &["My::App;", "My::App::Config;"],
                not_expected: &["Other::Tool;", "strict;"],
            },
        },
        Scenario {
            name: "test_more_assertion_prefers_visible_actual_expected",
            source_name: "test",
            source: "use Test::More;\n\nmy $got = compute();\nmy $expected = 42;\n\n<<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("is($got, $expected, 'test description');"),
                expected: &["is($got, $expected, 'test description');"],
                not_expected: &["done_testing();"],
            },
        },
        Scenario {
            name: "test2_assertion_uses_visible_result",
            source_name: "test",
            source: "use Test2::V0;\n\nmy $result = compute();\n\n<<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("ok($result, 'test description');"),
                expected: &["ok($result, 'test description');"],
                not_expected: &["done_testing();"],
            },
        },
        Scenario {
            name: "blank_line_in_sub_uses_visible_lexical",
            source_name: "contextual_fallback",
            source: "sub compute {\n    my $result = build();\n    <<CURSOR>>\n}\n",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("return $result;"),
                expected: &["return $result;"],
                not_expected: &["return $ghost;"],
            },
        },
        Scenario {
            name: "for_loop_uses_visible_array_binding",
            source_name: "syntax",
            source: "my @users = fetch_users();\nfor <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("my $user (@users) {\n    \n}"),
                expected: &["my $user (@users) {\n    \n}"],
                not_expected: &["my $item (@items)"],
            },
        },
        Scenario {
            name: "for_loop_does_not_trim_singular_status_name",
            source_name: "syntax",
            source: "my @status = fetch_status();\nfor <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("my $item (@status) {\n    \n}"),
                expected: &["my $item (@status) {\n    \n}"],
                not_expected: &["$statu"],
            },
        },
        Scenario {
            name: "for_loop_ignores_closed_block_array",
            source_name: "scope",
            source: "{\n    my @users = fetch_users();\n}\nfor <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "for_loop_uses_visible_hash_keys_when_no_array_is_available",
            source_name: "syntax",
            source: "my %users_by_id = load_users();\nfor <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("my $id (keys %users_by_id) {\n    \n}"),
                expected: &["my $id (keys %users_by_id) {\n    \n}"],
                not_expected: &["my $item (@items)"],
            },
        },
        Scenario {
            name: "guard_condition_prefers_boolean_named_visible_scalar",
            source_name: "syntax",
            source: "my $result = compute();\nmy $is_valid = validate($result);\nreturn unless <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("$is_valid;"),
                expected: &["$is_valid;"],
                not_expected: &["$result;"],
            },
        },
        Scenario {
            name: "guard_condition_prefers_skip_flag_over_receiver",
            source_name: "syntax",
            source: "for my $user (@users) {\n    my $should_skip = should_skip_user($user);\n    next if <<CURSOR>>\n}",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("$should_skip;"),
                expected: &["$should_skip;"],
                not_expected: &["$user;"],
            },
        },
        Scenario {
            name: "next_unless_uses_visible_guard_variable",
            source_name: "syntax",
            source: "for my $user (@users) {\n    my $should_skip = should_skip_user($user);\n    next unless <<CURSOR>>\n}",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("$should_skip;"),
                expected: &["$should_skip;"],
                not_expected: &["$user;", "$result;"],
            },
        },
        Scenario {
            name: "lexical_assignment_uses_visible_scalar",
            source_name: "syntax",
            source: "sub copy {\n    my $result = compute();\n    my $copy = <<CURSOR>>\n}",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("$result;"),
                expected: &["$result;"],
                not_expected: &["$copy;"],
            },
        },
        Scenario {
            name: "array_assignment_uses_visible_array",
            source_name: "syntax",
            source: "sub copy {\n    my @users = fetch_users();\n    my @copy = <<CURSOR>>\n}",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("@users;"),
                expected: &["@users;"],
                not_expected: &["@copy;", "$users;"],
            },
        },
        Scenario {
            name: "hash_assignment_uses_visible_hash",
            source_name: "syntax",
            source: "sub copy {\n    my %users_by_id = load_users();\n    my %copy = <<CURSOR>>\n}",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("%users_by_id;"),
                expected: &["%users_by_id;"],
                not_expected: &["%copy;", "$users_by_id;"],
            },
        },
        Scenario {
            name: "self_receiver_prefers_current_package_methods",
            source_name: "receiver",
            source: "package Other;\nsub external {}\n\npackage Demo;\nsub save {}\nsub display_name {}\nsub caller {\n    my $self = shift;\n    $self-><<CURSOR>>\n}\n",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("save()"),
                expected: &["save()", "display_name()"],
                not_expected: &["external()", "new()"],
            },
        },
        Scenario {
            name: "dbi_database_handle_prefers_dbi_methods",
            source_name: "receiver",
            source: "use DBI;\nmy $dbh = DBI->connect($dsn);\n$dbh-><<CURSOR>>\n",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("prepare()"),
                expected: &["prepare()", "do()", "disconnect()"],
                not_expected: &["new()"],
            },
        },
        Scenario {
            name: "constructor_completion_keeps_signature_style",
            source_name: "syntax",
            source: "sub helper ($self, %args) {\n}\n\nsub new<<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some(
                    " ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}",
                ),
                expected: &[
                    " ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}",
                ],
                not_expected: &["my $class = shift;"],
            },
        },
        Scenario {
            name: "line_comment_stays_silent",
            source_name: "hard_zone",
            source: "# use <<CURSOR>>",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "string_literal_stays_silent",
            source_name: "hard_zone",
            source: "my $text = \"use <<CURSOR>>\";",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "unterminated_string_stays_silent",
            source_name: "hard_zone",
            source: "my $text = \"use <<CURSOR>>",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "single_quote_operator_stays_silent",
            source_name: "hard_zone",
            source: "my $text = q{use <<CURSOR>>};",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "double_quote_operator_stays_silent",
            source_name: "hard_zone",
            source: "my $text = qq{use <<CURSOR>>};",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "word_quote_operator_stays_silent",
            source_name: "hard_zone",
            source: "my @words = qw(use <<CURSOR>>);",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "command_quote_operator_stays_silent",
            source_name: "hard_zone",
            source: "my $output = qx(use <<CURSOR>>);",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "heredoc_body_stays_silent",
            source_name: "hard_zone",
            source: "print <<'EOF';\nuse <<CURSOR>>\nEOF\n",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "format_body_stays_silent",
            source_name: "hard_zone",
            source: "format STDOUT =\nuse <<CURSOR>>\n.\nwrite STDOUT;\n",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "data_body_stays_silent",
            source_name: "hard_zone",
            source: "__DATA__\nuse <<CURSOR>>\n",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "pod_body_stays_silent",
            source_name: "hard_zone",
            source: "=pod\nuse <<CURSOR>>\n=cut\n",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "substitution_body_stays_silent",
            source_name: "hard_zone",
            source: "$name =~ s/use <<CURSOR>>/strict/;",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "transliteration_body_stays_silent",
            source_name: "hard_zone",
            source: "$name =~ tr/use <<CURSOR>>/abc/;",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "regex_literal_stays_silent",
            source_name: "hard_zone",
            source: "if ($name =~ /use <<CURSOR>>/) {}",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "use_partial_token_replacement_range",
            source_name: "replacement_range",
            source: "use str<<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::ReplacementRange {
                insert_text: "strict;",
                replaces: "str",
            },
        },
        Scenario {
            name: "method_arrow_partial_token_replacement_range",
            source_name: "replacement_range",
            source: "$obj->n<<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::ReplacementRange { insert_text: "new()", replaces: "n" },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use tempfile::TempDir;

    #[test]
    fn inline_completion_quality_guard_condition_scenarios_are_registered() {
        let names: Vec<&str> = scenarios().iter().map(|scenario| scenario.name).collect();

        assert!(names.contains(&"guard_condition_prefers_boolean_named_visible_scalar"));
        assert!(names.contains(&"guard_condition_prefers_skip_flag_over_receiver"));
    }

    #[test]
    fn inline_completion_quality_guard_condition_scenarios_pass() -> Result<()> {
        let provider = InlineCompletionProvider::new();

        for name in [
            "guard_condition_prefers_boolean_named_visible_scalar",
            "guard_condition_prefers_skip_flag_over_receiver",
        ] {
            let scenario = scenarios()
                .iter()
                .find(|scenario| scenario.name == name)
                .ok_or_else(|| eyre!("missing inline completion quality scenario {name}"))?;
            let (_item_count, _notes, parse_regressions, edit_application) =
                run_scenario(&provider, scenario)?;
            assert_eq!(parse_regressions, 0);
            assert_eq!(edit_application, EditApplicationOutcome::Passed);
        }

        Ok(())
    }

    #[test]
    fn edit_application_outcome_records_failed_counts() {
        let mut checks = InlineCompletionQualityChecks::default();

        record_edit_application(&mut checks, EditApplicationOutcome::NotApplicable);
        record_edit_application(&mut checks, EditApplicationOutcome::Passed);
        record_edit_application(&mut checks, EditApplicationOutcome::Failed);

        assert_eq!(checks.edit_application.total, 2);
        assert_eq!(checks.edit_application.passed, 1);
        assert_eq!(checks.edit_application.failed, 1);
    }

    #[test]
    fn check_top_edit_application_skips_silent_scenarios() -> Result<()> {
        let scenario = Scenario {
            name: "silent",
            source_name: "unit",
            source: "use <<CURSOR>>",
            available_modules: &[],
            hard_zone: true,
            assertion: ScenarioAssertion::Silent,
        };
        let fixture = InlineCompletionScenario::from_fixture(scenario.source)?;
        let completions = InlineCompletionList { items: Vec::new() };
        let mut notes = Vec::new();

        let outcome = check_top_edit_application(
            scenario.name,
            &scenario,
            &fixture,
            &completions,
            &mut notes,
        )?;

        assert_eq!(outcome, EditApplicationOutcome::NotApplicable);
        assert!(notes.is_empty());
        Ok(())
    }

    #[test]
    fn check_top_edit_application_reports_missing_top_item() -> Result<()> {
        let scenario = Scenario {
            name: "missing_top_item",
            source_name: "unit",
            source: "use <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("strict;"),
                expected: &["strict;"],
                not_expected: &[],
            },
        };
        let fixture = InlineCompletionScenario::from_fixture(scenario.source)?;
        let completions = InlineCompletionList { items: Vec::new() };
        let mut notes = Vec::new();

        let outcome = check_top_edit_application(
            scenario.name,
            &scenario,
            &fixture,
            &completions,
            &mut notes,
        )?;

        assert_eq!(outcome, EditApplicationOutcome::Failed);
        assert_eq!(notes, vec!["edit_application=missing_top_item"]);
        Ok(())
    }

    #[test]
    fn check_top_edit_application_reports_unsupported_ranges() -> Result<()> {
        let scenario = Scenario {
            name: "unsupported_range",
            source_name: "unit",
            source: "use <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("strict;"),
                expected: &["strict;"],
                not_expected: &[],
            },
        };
        let fixture = InlineCompletionScenario::from_fixture(scenario.source)?;
        let item = serde_json::from_value(json!({
            "insertText": "strict;",
            "range": {
                "start": { "line": 1, "character": 0 },
                "end": { "line": 1, "character": 0 }
            }
        }))?;
        let completions = InlineCompletionList { items: vec![item] };
        let mut notes = Vec::new();

        let outcome = check_top_edit_application(
            scenario.name,
            &scenario,
            &fixture,
            &completions,
            &mut notes,
        )?;

        assert_eq!(outcome, EditApplicationOutcome::Failed);
        assert_eq!(notes, vec!["edit_application=unsupported_range for \"strict;\""]);
        Ok(())
    }

    #[test]
    fn check_top_edit_application_reports_parse_worsening_edits() -> Result<()> {
        let scenario = Scenario {
            name: "parse_worse",
            source_name: "unit",
            source: "my $value = 1;<<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some(")"),
                expected: &[")"],
                not_expected: &[],
            },
        };
        let fixture = InlineCompletionScenario::from_fixture(scenario.source)?;
        let completions = InlineCompletionList {
            items: vec![InlineCompletionItem {
                insert_text: ")".to_string(),
                filter_text: None,
                range: None,
                command: None,
            }],
        };
        let mut notes = Vec::new();

        let outcome = check_top_edit_application(
            scenario.name,
            &scenario,
            &fixture,
            &completions,
            &mut notes,
        )?;

        assert_eq!(outcome, EditApplicationOutcome::Failed);
        assert!(notes.iter().any(|note| note.contains("edit_application=failed")));
        Ok(())
    }

    #[test]
    fn measured_suppression_reason_ignores_non_silent_scenarios() {
        let suggestion = Scenario {
            name: "suggestion",
            source_name: "unit",
            source: "use <<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("strict;"),
                expected: &["strict;"],
                not_expected: &[],
            },
        };
        let replacement = Scenario {
            name: "replacement",
            source_name: "unit",
            source: "use str<<CURSOR>>",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::ReplacementRange {
                insert_text: "strict;",
                replaces: "str",
            },
        };

        assert!(measured_suppression_reason(&suggestion, 0, true).is_none());
        assert!(measured_suppression_reason(&replacement, 0, true).is_none());
    }

    #[test]
    fn inline_completion_quality_receipt_records_suppression_reasons() -> Result<()> {
        let temp = TempDir::new()?;
        let receipt_path = temp.path().join("inline-completion-quality.json");

        run(receipt_path.clone())?;

        let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        let expected_hard_zones = scenarios()
            .iter()
            .filter(|scenario| {
                scenario.hard_zone && matches!(scenario.assertion, ScenarioAssertion::Silent)
            })
            .count() as u64;
        let expected_no_visible_context = scenarios()
            .iter()
            .filter(|scenario| {
                !scenario.hard_zone && matches!(scenario.assertion, ScenarioAssertion::Silent)
            })
            .count() as u64;

        assert_eq!(
            receipt.pointer("/checks/suppression_reasons/hard_zone").and_then(Value::as_u64),
            Some(expected_hard_zones)
        );
        assert_eq!(
            receipt
                .pointer("/checks/suppression_reasons/no_visible_context")
                .and_then(Value::as_u64),
            Some(expected_no_visible_context)
        );
        assert_eq!(
            receipt.pointer("/checks/hard_zone_rejected").and_then(Value::as_u64),
            Some(expected_hard_zones)
        );

        Ok(())
    }

    #[test]
    fn inline_completion_quality_receipt_records_edit_application_checks() -> Result<()> {
        let temp = TempDir::new()?;
        let receipt_path = temp.path().join("inline-completion-quality.json");

        run(receipt_path.clone())?;

        let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        let expected_applicable = scenarios()
            .iter()
            .filter(|scenario| !matches!(scenario.assertion, ScenarioAssertion::Silent))
            .count() as u64;

        assert_eq!(
            receipt.pointer("/checks/edit_application/total").and_then(Value::as_u64),
            Some(expected_applicable)
        );
        assert_eq!(
            receipt.pointer("/checks/edit_application/passed").and_then(Value::as_u64),
            Some(expected_applicable)
        );
        assert_eq!(
            receipt.pointer("/checks/edit_application/failed").and_then(Value::as_u64),
            Some(0)
        );

        Ok(())
    }

    #[test]
    fn inline_completion_quality_receipt_breaks_down_source_outcomes() -> Result<()> {
        let temp = TempDir::new()?;
        let receipt_path = temp.path().join("inline-completion-quality.json");

        run(receipt_path.clone())?;

        let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        let expected_hard_zone_scenarios =
            scenarios().iter().filter(|scenario| scenario.source_name == "hard_zone").count()
                as u64;
        let expected_replacement_range_scenarios = scenarios()
            .iter()
            .filter(|scenario| scenario.source_name == "replacement_range")
            .count() as u64;

        assert_eq!(
            receipt.pointer("/sources/hard_zone/expected").and_then(Value::as_u64),
            Some(expected_hard_zone_scenarios)
        );
        assert_eq!(
            receipt.pointer("/sources/hard_zone/returned_items").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            receipt
                .pointer("/sources/hard_zone/suppression_reasons/hard_zone")
                .and_then(Value::as_u64),
            Some(expected_hard_zone_scenarios)
        );
        assert_eq!(
            receipt.pointer("/sources/hard_zone/edit_application/total").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            receipt
                .pointer("/sources/replacement_range/edit_application/total")
                .and_then(Value::as_u64),
            Some(expected_replacement_range_scenarios)
        );
        assert_eq!(
            receipt
                .pointer("/sources/replacement_range/edit_application/passed")
                .and_then(Value::as_u64),
            Some(expected_replacement_range_scenarios)
        );
        let module_returned_items = receipt
            .pointer("/sources/module/returned_items")
            .and_then(Value::as_u64)
            .ok_or_else(|| eyre!("missing module returned_items source receipt"))?;
        assert!(module_returned_items > 0);

        Ok(())
    }

    #[test]
    fn inline_completion_quality_receipt_records_scenario_errors() -> Result<()> {
        let temp = TempDir::new()?;
        let receipt_path = temp.path().join("inline-completion-quality.json");
        let scenario_set = [Scenario {
            name: "bad_fixture",
            source_name: "unit_source",
            source: "missing cursor",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("strict;"),
                expected: &["strict;"],
                not_expected: &[],
            },
        }];

        let error = run_with_scenarios(receipt_path.clone(), &scenario_set)
            .expect_err("invalid fixture should fail the quality receipt");

        assert!(
            error.to_string().contains("inline-completion quality receipt failed 1 fixture(s)")
        );
        let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        assert_eq!(receipt.pointer("/fixtures_passed").and_then(Value::as_u64), Some(0));
        assert_eq!(
            receipt.pointer("/sources/unit_source/expected").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(receipt.pointer("/sources/unit_source/failed").and_then(Value::as_u64), Some(1));
        assert_eq!(
            receipt.pointer("/sources/unit_source/returned_items").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            receipt.pointer("/scenarios/0/notes/0").and_then(Value::as_str),
            Some("fixture must include <<CURSOR>> marker")
        );

        Ok(())
    }

    #[test]
    fn source_breakdown_records_failed_scenario_accounting() -> Result<()> {
        let mut receipt = InlineCompletionQualityReceipt {
            schema_version: "inline-completion-quality.v1",
            provider: "inline_completion",
            provider_action: "deterministic_fixture_quality",
            claim_boundary: "unit test receipt",
            fixtures_total: 1,
            fixtures_passed: 0,
            latency_p95_ms: 0,
            checks: InlineCompletionQualityChecks::default(),
            sources: BTreeMap::new(),
            scenarios: Vec::new(),
        };
        let mut failures = Vec::new();
        let scenario = Scenario {
            name: "bad_fixture",
            source_name: "unit_source",
            source: "missing cursor",
            available_modules: &[],
            hard_zone: false,
            assertion: ScenarioAssertion::Suggestion {
                first: Some("strict;"),
                expected: &["strict;"],
                not_expected: &[],
            },
        };

        record_scenario_error(
            &mut receipt,
            &mut failures,
            &scenario,
            7,
            "missing cursor marker".to_string(),
        );
        record_source_result(
            &mut receipt.sources,
            "unit_source",
            SourceScenarioOutcome {
                passed: false,
                item_count: 3,
                parse_regressions: 2,
                edit_application: EditApplicationOutcome::Failed,
                suppression_reason: Some(SUPPRESSION_NO_VISIBLE_CONTEXT),
            },
        );

        let source = receipt
            .sources
            .get("unit_source")
            .ok_or_else(|| eyre!("missing unit source receipt"))?;
        assert_eq!(source.expected, 2);
        assert_eq!(source.passed, 0);
        assert_eq!(source.failed, 2);
        assert_eq!(source.returned_items, 3);
        assert_eq!(source.parse_regressions, 2);
        assert_eq!(source.edit_application.total, 1);
        assert_eq!(source.edit_application.failed, 1);
        assert_eq!(
            source.suppression_reasons.get(SUPPRESSION_NO_VISIBLE_CONTEXT).copied(),
            Some(1)
        );
        assert_eq!(receipt.checks.expected_text.total, 1);
        assert_eq!(receipt.checks.expected_text.failed, 1);
        assert_eq!(receipt.scenarios.len(), 1);
        assert_eq!(receipt.scenarios[0].outcome, "fail");
        assert_eq!(receipt.scenarios[0].latency_ms, 7);
        assert_eq!(receipt.scenarios[0].notes, vec!["missing cursor marker"]);
        assert_eq!(failures, vec!["bad_fixture: missing cursor marker"]);

        Ok(())
    }
}
