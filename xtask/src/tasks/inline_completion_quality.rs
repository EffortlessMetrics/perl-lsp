use color_eyre::eyre::{Result, bail, eyre};
use perl_lsp_rs_core::providers::inline_completion::{
    InlineCompletionEnvironment, InlineCompletionList, InlineCompletionProvider,
};
use perl_parser_core::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

const CURSOR: &str = "<<CURSOR>>";

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
    hard_zone_rejected: usize,
    parse_regressions: usize,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
struct SourceQualityReceipt {
    expected: usize,
    passed: usize,
    failed: usize,
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

struct InlineCompletionScenario {
    text: String,
    line: u32,
    character: u32,
}

pub fn run(receipt: PathBuf) -> Result<()> {
    let provider = InlineCompletionProvider::new();
    let mut receipt_data = InlineCompletionQualityReceipt {
        schema_version: "inline-completion-quality.v1",
        provider: "inline_completion",
        provider_action: "deterministic_fixture_quality",
        claim_boundary: "local deterministic fixture receipt only; no telemetry upload and no AI/provider behavior change",
        fixtures_total: scenarios().len(),
        fixtures_passed: 0,
        latency_p95_ms: 0,
        checks: InlineCompletionQualityChecks::default(),
        sources: BTreeMap::new(),
        scenarios: Vec::new(),
    };
    let mut latencies = Vec::new();
    let mut failures = Vec::new();

    for scenario in scenarios() {
        let started = Instant::now();
        let result = run_scenario(&provider, scenario);
        let latency_ms = started.elapsed().as_millis();
        latencies.push(latency_ms);

        let passed = result.is_ok();
        record_source_result(&mut receipt_data.sources, scenario.source_name, passed);
        update_check_counts(&mut receipt_data.checks, scenario, passed);

        match result {
            Ok((item_count, mut notes)) => {
                receipt_data.fixtures_passed += 1;
                if matches!(scenario.assertion, ScenarioAssertion::Silent) {
                    receipt_data.checks.hard_zone_rejected += 1;
                    notes.push("hard zone stayed silent".to_string());
                }
                receipt_data.scenarios.push(ScenarioQualityReceipt {
                    name: scenario.name,
                    source: scenario.source_name,
                    outcome: "pass",
                    item_count,
                    latency_ms,
                    notes,
                });
            }
            Err(error) => {
                let message = error.to_string();
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
    passed: bool,
) {
    let source_entry = sources.entry(source_name.to_string()).or_default();
    source_entry.expected += 1;
    if passed {
        source_entry.passed += 1;
    } else {
        source_entry.failed += 1;
    }
}

fn run_scenario(
    provider: &InlineCompletionProvider,
    scenario: &Scenario,
) -> Result<(usize, Vec<String>)> {
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
    let notes = match scenario.assertion {
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
    Ok((item_count, notes))
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
        if completions.items.iter().any(|item| item.insert_text == *unexpected) {
            bail!(
                "{name}: unexpected completion {unexpected}, got {:?}",
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
}

fn scenarios() -> &'static [Scenario] {
    &[
        Scenario {
            name: "use_pragmas",
            source_name: "syntax",
            source: "use <<CURSOR>>",
            available_modules: &[],
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
            assertion: ScenarioAssertion::Suggestion {
                first: Some("return $result;"),
                expected: &["return $result;"],
                not_expected: &["return $ghost;"],
            },
        },
        Scenario {
            name: "self_receiver_prefers_current_package_methods",
            source_name: "receiver",
            source: "package Other;\nsub external {}\n\npackage Demo;\nsub save {}\nsub display_name {}\nsub caller {\n    my $self = shift;\n    $self-><<CURSOR>>\n}\n",
            available_modules: &[],
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
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "string_literal_stays_silent",
            source_name: "hard_zone",
            source: "my $text = \"use <<CURSOR>>\";",
            available_modules: &[],
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "heredoc_body_stays_silent",
            source_name: "hard_zone",
            source: "print <<'EOF';\nuse <<CURSOR>>\nEOF\n",
            available_modules: &[],
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "pod_body_stays_silent",
            source_name: "hard_zone",
            source: "=pod\nuse <<CURSOR>>\n=cut\n",
            available_modules: &[],
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "regex_literal_stays_silent",
            source_name: "hard_zone",
            source: "if ($name =~ /use <<CURSOR>>/) {}",
            available_modules: &[],
            assertion: ScenarioAssertion::Silent,
        },
        Scenario {
            name: "use_partial_token_replacement_range",
            source_name: "replacement_range",
            source: "use str<<CURSOR>>",
            available_modules: &[],
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
            assertion: ScenarioAssertion::ReplacementRange { insert_text: "new()", replaces: "n" },
        },
    ]
}
