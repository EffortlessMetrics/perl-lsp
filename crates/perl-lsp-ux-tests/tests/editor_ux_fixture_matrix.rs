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
    let confidence_signals =
        matrix.get("ux_confidence_signals").context("ux_confidence_signals missing")?;
    validate_confidence_signals(confidence_signals)?;
    let allowed_metrics =
        top_line_metrics.union(&component_metrics).cloned().collect::<BTreeSet<_>>();

    let workflows =
        matrix.get("workflows").and_then(Value::as_array).context("workflows missing")?;

    let mut workflow_ids = BTreeSet::new();
    let mut scenarios_in_matrix = BTreeSet::new();
    let mut component_metrics_exercised = BTreeSet::new();
    for workflow in workflows {
        let workflow_id =
            workflow.get("id").and_then(Value::as_str).context("workflow missing id")?;
        assert!(
            workflow_ids.insert(workflow_id.to_string()),
            "workflow id `{workflow_id}` is duplicated"
        );
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

        let expected_outcomes = workflow
            .get("expected_outcomes")
            .and_then(Value::as_array)
            .context("workflow missing expected_outcomes")?;
        assert!(
            !expected_outcomes.is_empty(),
            "workflow `{scenario_file}` must define expected outcomes"
        );

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

fn validate_confidence_signals(value: &Value) -> Result<()> {
    let signals = value.as_object().context("ux_confidence_signals must be an object")?;
    for signal_name in ["manual_editor_smoke", "first_5_minutes_harness", "open_issue_burndown"] {
        assert!(signals.contains_key(signal_name), "missing confidence signal `{signal_name}`");
    }

    let runbook = signals["manual_editor_smoke"]
        .get("runbook")
        .and_then(Value::as_str)
        .context("manual_editor_smoke.runbook missing")?;
    assert_eq!(runbook, "docs/reference/UX_TESTING.md");

    let tracking_issues = signals["open_issue_burndown"]
        .get("tracking_issues")
        .and_then(Value::as_array)
        .context("open_issue_burndown.tracking_issues missing")?;
    assert!(!tracking_issues.is_empty(), "tracking_issues must not be empty");
    for issue in tracking_issues {
        let issue_number = issue.as_u64().context("tracking issue IDs must be integers")?;
        assert!(issue_number > 0, "tracking issue IDs must be positive");
    }

    Ok(())
}
