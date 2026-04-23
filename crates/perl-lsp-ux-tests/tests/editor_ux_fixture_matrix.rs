use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

const FIXTURE_MATRIX: &str = "crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json";
const UX_TESTS_DIR: &str = "crates/perl-lsp-ux-tests/tests";

fn workspace_root() -> &'static Path {
    match Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent) {
        Some(p) => p,
        None => unreachable!("CARGO_MANIFEST_DIR always has two parent directories"),
    }
}

#[test]
fn editor_ux_fixture_matrix_covers_all_scenarios() -> Result<()> {
    let matrix_path = workspace_root().join(FIXTURE_MATRIX);
    let matrix_text = fs::read_to_string(&matrix_path)
        .with_context(|| format!("reading {}", matrix_path.display()))?;
    let matrix: Value = serde_json::from_str(&matrix_text)
        .with_context(|| format!("parsing {}", matrix_path.display()))?;

    let schema_version =
        matrix.get("schema_version").and_then(Value::as_u64).context("schema_version missing")?;
    assert_eq!(schema_version, 1, "fixture matrix schema version drifted");

    let subsystem = matrix.get("subsystem").and_then(Value::as_str).context("subsystem missing")?;
    assert_eq!(subsystem, "editor_ux");

    let top_line_metrics = collect_string_set(
        matrix.get("top_line_metrics").context("top_line_metrics missing")?,
        "top_line_metrics",
    )?;
    assert_eq!(
        top_line_metrics,
        BTreeSet::from([
            "workflow_pass_rate".to_string(),
            "workflow_stability_rate".to_string(),
            "p95_time_to_first_useful_result_ms".to_string()
        ])
    );

    let component_metrics = collect_string_set(
        matrix.get("component_metrics").context("component_metrics missing")?,
        "component_metrics",
    )?;
    let confidence_signals = collect_string_set(
        matrix.get("confidence_signals").context("confidence_signals missing")?,
        "confidence_signals",
    )?;
    assert_eq!(
        confidence_signals,
        BTreeSet::from([
            "manual_editor_smoke".to_string(),
            "first_five_minutes_harness".to_string(),
            "issue_burndown_regression_guard".to_string(),
        ])
    );
    let allowed_metrics =
        top_line_metrics.union(&component_metrics).cloned().collect::<BTreeSet<_>>();
    let mut confidence_signals_exercised = BTreeSet::new();

    let workflows =
        matrix.get("workflows").and_then(Value::as_array).context("workflows missing")?;

    let mut scenarios_in_matrix = BTreeSet::new();
    let mut component_metrics_exercised = BTreeSet::new();
    let mut workflows_with_component_metric = 0usize;
    for workflow in workflows {
        let scenario_file = workflow
            .get("scenario_file")
            .and_then(Value::as_str)
            .context("workflow missing scenario_file")?;
        let measures = collect_string_set(
            workflow.get("measures").context("workflow missing measures")?,
            scenario_file,
        )?;
        assert!(
            !measures.is_empty(),
            "workflow `{scenario_file}` must define at least one measure"
        );
        for measure in &measures {
            assert!(
                allowed_metrics.contains(measure),
                "workflow `{scenario_file}` references unknown metric `{measure}`"
            );
            if component_metrics.contains(measure) {
                component_metrics_exercised.insert(measure.clone());
            }
        }
        if measures.iter().any(|measure| component_metrics.contains(measure)) {
            workflows_with_component_metric += 1;
        }

        let expected_outcomes = workflow
            .get("expected_outcomes")
            .and_then(Value::as_array)
            .context("workflow missing expected_outcomes")?;
        assert!(
            !expected_outcomes.is_empty(),
            "workflow `{scenario_file}` must define expected outcomes"
        );
        let workflow_confidence_signals = collect_string_set(
            workflow.get("confidence_signals").context("workflow missing confidence_signals")?,
            &format!("{scenario_file}.confidence_signals"),
        )?;
        assert!(
            !workflow_confidence_signals.is_empty(),
            "workflow `{scenario_file}` must define at least one confidence signal"
        );
        for signal in workflow_confidence_signals {
            assert!(
                confidence_signals.contains(&signal),
                "workflow `{scenario_file}` references unknown confidence signal `{signal}`"
            );
            confidence_signals_exercised.insert(signal);
        }

        let scenario_path = workspace_root().join(UX_TESTS_DIR).join(scenario_file);
        assert!(
            scenario_path.exists(),
            "workflow `{scenario_file}` points at missing scenario file {}",
            scenario_path.display()
        );
        scenarios_in_matrix.insert(scenario_file.to_string());
    }

    let scenarios_on_disk = fs::read_dir(workspace_root().join(UX_TESTS_DIR))
        .context("reading UX tests dir")?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("ux_scenario_") && name.ends_with(".rs"))
        .collect::<BTreeSet<_>>();

    assert_eq!(
        scenarios_in_matrix, scenarios_on_disk,
        "fixture matrix must stay in lockstep with the executable UX scenarios"
    );
    assert_eq!(
        component_metrics_exercised, component_metrics,
        "every declared component metric must be exercised by at least one workflow"
    );
    assert!(
        workflows_with_component_metric * 2 >= workflows.len(),
        "at least half of workflows must exercise a component metric to preserve metric diversity"
    );
    assert_eq!(
        confidence_signals_exercised, confidence_signals,
        "every declared confidence signal must be exercised by at least one workflow"
    );

    Ok(())
}

fn collect_string_set(value: &Value, context_label: &str) -> Result<BTreeSet<String>> {
    let values = value.as_array().with_context(|| format!("{context_label} must be an array"))?;
    let mut out = BTreeSet::new();
    for entry in values {
        let item =
            entry.as_str().with_context(|| format!("{context_label} entries must be strings"))?;
        out.insert(item.to_string());
    }
    Ok(out)
}
