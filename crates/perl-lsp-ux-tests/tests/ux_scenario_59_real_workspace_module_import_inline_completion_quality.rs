//! Scenario 59 - real-workspace module import inline-completion quality proof.
//!
//! This receipt exercises inline module-import ghost text over a small
//! CPAN-shaped workspace. It verifies that project modules are suggested only
//! when they are reachable from the file's effective `@INC` context, and that
//! the workspace root itself does not turn into a wildcard module root.

// UX receipt tests intentionally write structured receipts to stderr for --nocapture logs.
#![allow(clippy::print_stderr)]

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    LspEvent, ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available,
    missing_binary_skip, run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str =
    "ux_scenario_59_real_workspace_module_import_inline_completion_quality.rs";

const APP_PATH: &str = "lib/My/App.pm";
const CONFIG_PATH: &str = "lib/My/App/Config.pm";
const LOCAL_THING_PATH: &str = "local/lib/perl5/Local/Thing.pm";
const ROOT_ONLY_PATH: &str = "My/RootOnly.pm";
const REACHABLE_PROBE_PATH: &str = "script/reachable-import.pl";
const CANCELLED_LIB_PROBE_PATH: &str = "script/cancelled-lib-import.pl";
const LOCAL_PROBE_PATH: &str = "script/local-lib-import.pl";
const CANCELLED_LOCAL_PROBE_PATH: &str = "script/cancelled-local-import.pl";

const APP_PM: &str = "package My::App;\nuse strict;\nuse warnings;\n1;\n";
const CONFIG_PM: &str = "package My::App::Config;\nuse strict;\nuse warnings;\n1;\n";
const LOCAL_THING_PM: &str = "package Local::Thing;\nuse strict;\nuse warnings;\n1;\n";
const ROOT_ONLY_PM: &str = "package My::RootOnly;\nuse strict;\nuse warnings;\n1;\n";

const REACHABLE_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use My::"#;

const CANCELLED_LIB_PROBE: &str = r#"use strict;
use warnings;
no lib 'lib';
use My::"#;

const LOCAL_PROBE: &str = r#"use strict;
use warnings;
use lib 'local/lib/perl5';
use Local::"#;

const CANCELLED_LOCAL_PROBE: &str = r#"use strict;
use warnings;
no lib 'local/lib/perl5';
use Local::"#;

#[derive(Debug, Serialize)]
struct ModuleImportProbeReport {
    name: &'static str,
    file: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_texts: Vec<&'static str>,
    missing_expected_insert_texts: Vec<&'static str>,
    forbidden_insert_texts: Vec<&'static str>,
}

#[derive(Debug)]
struct ModuleImportProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    marker: &'static str,
    expected: &'static [&'static str],
    forbidden: &'static [&'static str],
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(APP_PATH, APP_PM)
        .with_file(CONFIG_PATH, CONFIG_PM)
        .with_file(LOCAL_THING_PATH, LOCAL_THING_PM)
        .with_file(ROOT_ONLY_PATH, ROOT_ONLY_PM)
        .with_file(REACHABLE_PROBE_PATH, REACHABLE_PROBE)
        .with_file(CANCELLED_LIB_PROBE_PATH, CANCELLED_LIB_PROBE)
        .with_file(LOCAL_PROBE_PATH, LOCAL_PROBE)
        .with_file(CANCELLED_LOCAL_PROBE_PATH, CANCELLED_LOCAL_PROBE);
    config.client_capability_overrides = json!({
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    });

    UxHarness::new(config)
}

fn module_import_probes() -> Vec<ModuleImportProbe> {
    vec![
        ModuleImportProbe {
            name: "reachable_lib_modules",
            file: REACHABLE_PROBE_PATH,
            source: REACHABLE_PROBE,
            marker: "use My::",
            expected: &["My::App;", "My::App::Config;"],
            forbidden: &["My::RootOnly;", "strict;", "warnings;", "feature ':5.36';"],
        },
        ModuleImportProbe {
            name: "cancelled_lib_suppresses_project_modules",
            file: CANCELLED_LIB_PROBE_PATH,
            source: CANCELLED_LIB_PROBE,
            marker: "use My::",
            expected: &[],
            forbidden: &["My::App;", "My::App::Config;", "My::RootOnly;"],
        },
        ModuleImportProbe {
            name: "reachable_local_lib_modules",
            file: LOCAL_PROBE_PATH,
            source: LOCAL_PROBE,
            marker: "use Local::",
            expected: &["Local::Thing;"],
            forbidden: &["My::App;", "My::RootOnly;"],
        },
        ModuleImportProbe {
            name: "cancelled_local_lib_suppresses_local_modules",
            file: CANCELLED_LOCAL_PROBE_PATH,
            source: CANCELLED_LOCAL_PROBE,
            marker: "use Local::",
            expected: &[],
            forbidden: &["Local::Thing;"],
        },
    ]
}

fn position_after(source: &str, needle: &str) -> Result<(u32, u32)> {
    let byte_offset =
        source.find(needle).with_context(|| format!("missing `{needle}`"))? + needle.len();
    position_from_byte_offset(source, byte_offset)
}

fn position_from_byte_offset(source: &str, byte_offset: usize) -> Result<(u32, u32)> {
    let prefix = source
        .get(..byte_offset)
        .with_context(|| format!("byte offset {byte_offset} is not a UTF-8 boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().map(str::chars).map(Iterator::count).unwrap_or(0);
    Ok((u32::try_from(line)?, u32::try_from(character)?))
}

fn inline_registration_seen(events: &[LspEvent]) -> bool {
    events.iter().any(|event| {
        let LspEvent::Other { method, params } = event else {
            return false;
        };
        method == "client/registerCapability"
            && params.get("registrations").and_then(Value::as_array).into_iter().flatten().any(
                |registration| {
                    registration.get("method").and_then(Value::as_str)
                        == Some("textDocument/inlineCompletion")
                        && registration.get("id").and_then(Value::as_str)
                            == Some("perl-inlineCompletion")
                },
            )
    })
}

fn wait_for_inline_registration(harness: &UxHarness) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if inline_registration_seen(&harness.client.peek_events()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

fn probe_inline_import(
    harness: &UxHarness,
    probe: &ModuleImportProbe,
) -> Result<ModuleImportProbeReport> {
    let (line, character) = position_after(probe.source, probe.marker)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let items = loop {
        let items = harness.inline_completion_with_trigger_kind(probe.file, line, character, 1)?;
        for item in &items {
            anyhow::ensure!(
                item.get("insertText").and_then(Value::as_str).is_some(),
                "inline item must include insertText: {item:?}"
            );
        }
        let insert_texts = insert_texts_for(&items);
        if probe
            .expected
            .iter()
            .all(|expected| insert_texts.iter().any(|actual| actual == expected))
            || Instant::now() >= deadline
        {
            break items;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let insert_texts = insert_texts_for(&items);
    let missing_expected_insert_texts = probe
        .expected
        .iter()
        .copied()
        .filter(|expected| !insert_texts.iter().any(|actual| actual == expected))
        .collect::<Vec<_>>();
    let forbidden_insert_texts = probe
        .forbidden
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect::<Vec<_>>();

    Ok(ModuleImportProbeReport {
        name: probe.name,
        file: probe.file,
        trigger_kind: 1,
        candidate_count: items.len(),
        insert_texts,
        expected_insert_texts: probe.expected.to_vec(),
        missing_expected_insert_texts,
        forbidden_insert_texts,
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.get("insertText").and_then(Value::as_str).map(str::to_string))
        .collect()
}

#[test]
fn scenario_59_real_workspace_module_import_inline_completion_quality_receipt() {
    run_ux_scenario(
        "real_workspace_module_import_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_59_real_workspace_module_import_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            for (path, source) in [
                (APP_PATH, APP_PM),
                (CONFIG_PATH, CONFIG_PM),
                (LOCAL_THING_PATH, LOCAL_THING_PM),
                (ROOT_ONLY_PATH, ROOT_ONLY_PM),
                (REACHABLE_PROBE_PATH, REACHABLE_PROBE),
                (CANCELLED_LIB_PROBE_PATH, CANCELLED_LIB_PROBE),
                (LOCAL_PROBE_PATH, LOCAL_PROBE),
                (CANCELLED_LOCAL_PROBE_PATH, CANCELLED_LOCAL_PROBE),
            ] {
                harness.open_file(path, source)?;
            }
            std::thread::sleep(Duration::from_millis(500));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            let probes = module_import_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_inline_import(&harness, probe)?;
                if report.missing_expected_insert_texts.is_empty()
                    && report.forbidden_insert_texts.is_empty()
                {
                    recorder.mark_first_useful_result(probe.name);
                }
                reports.push(report);
            }

            let missing_expected = reports
                .iter()
                .filter(|report| !report.missing_expected_insert_texts.is_empty())
                .map(|report| report.name)
                .collect::<Vec<_>>();
            let forbidden_hits = reports
                .iter()
                .filter(|report| !report.forbidden_insert_texts.is_empty())
                .map(|report| report.name)
                .collect::<Vec<_>>();

            let receipt = json!({
                "schema_version": 1,
                "receipt": "real_workspace_module_import_inline_completion_quality",
                "workspace_fixture": "CPAN-shaped inline module import workspace with lib, local/lib/perl5, and root-only module probes",
                "claim_boundary": "real-workspace inline module-import quality receipt only; no provider behavior change, support-tier promotion, source mirror, release action, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "missing_expected": missing_expected,
                "forbidden_hits": forbidden_hits,
                "reports": reports,
            });
            eprintln!(
                "real_workspace_module_import_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "reachable lib module imports were suggested",
                reports.iter().any(|report| {
                    report.name == "reachable_lib_modules"
                        && report.missing_expected_insert_texts.is_empty()
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "no lib cancellation suppressed lib module imports",
                reports.iter().any(|report| {
                    report.name == "cancelled_lib_suppresses_project_modules"
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "reachable local lib module imports were suggested",
                reports.iter().any(|report| {
                    report.name == "reachable_local_lib_modules"
                        && report.missing_expected_insert_texts.is_empty()
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "no lib cancellation suppressed local lib module imports",
                reports.iter().any(|report| {
                    report.name == "cancelled_local_lib_suppresses_local_modules"
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "workspace root did not leak root-only My:: modules into import ghost text",
                reports
                    .iter()
                    .all(|report| !report.forbidden_insert_texts.contains(&"My::RootOnly;")),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
