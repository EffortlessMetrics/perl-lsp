use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

const FIXTURE_MATRIX: &str = "crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json";
const UX_TESTS_DIR: &str = "crates/perl-lsp-ux-tests/tests";

/// Valid `component` values matching the `UxComponent` enum in `taxonomy.rs`.
const VALID_COMPONENTS: &[&str] = &[
    "completion",
    "diagnostics",
    "module_resolution",
    "workspace_symbols",
    "rename",
    "safe_delete",
    "hover",
    "goto_definition",
    "signature_help",
    "code_lens",
    "folding_range",
    "semantic_tokens",
    "infra",
    "ai_completion",
];

/// Required keys inside each workflow's `instrumentation` object.
const INSTRUMENTATION_KEYS: &[&str] = &["run_receipt", "first_useful_result", "protocol_goldens"];
const SEMANTIC_INLINE_RECEIPT_IDS: &[&str] = &[
    "mojolicious_inline_completion_quality",
    "test_inline_completion_quality",
    "constructor_inline_completion_quality",
    "self_receiver_inline_completion_quality",
    "dbi_receiver_inline_completion_quality",
    "lexical_return_inline_completion_quality",
    "loop_binding_inline_completion_quality",
    "guard_condition_inline_completion_quality",
    "real_workspace_module_import_inline_completion_quality",
    "package_boundary_receiver_inline_completion_quality",
    "project_test_assertion_inline_completion_quality",
    "project_control_flow_inline_completion_quality",
    "project_constructor_inline_completion_quality",
];

fn workspace_root() -> &'static Path {
    match Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent) {
        Some(p) => p,
        None => unreachable!("CARGO_MANIFEST_DIR always has two parent directories"),
    }
}

fn load_fixture_matrix() -> Result<Value> {
    let matrix_path = workspace_root().join(FIXTURE_MATRIX);
    let matrix_text = fs::read_to_string(&matrix_path)
        .with_context(|| format!("reading {}", matrix_path.display()))?;
    serde_json::from_str(&matrix_text).with_context(|| format!("parsing {}", matrix_path.display()))
}

#[test]
fn editor_ux_fixture_matrix_covers_all_scenarios() -> Result<()> {
    let matrix = load_fixture_matrix()?;

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
    let baseline_metrics =
        BTreeSet::from(["workflow_pass_rate".to_string(), "workflow_stability_rate".to_string()]);
    let mut metric_usage_counts: HashMap<String, usize> =
        allowed_metrics.iter().cloned().map(|metric| (metric, 0_usize)).collect();
    let mut workflows_with_extended_metrics = 0_usize;

    let valid_components: BTreeSet<&str> = VALID_COMPONENTS.iter().copied().collect();

    let workflows =
        matrix.get("workflows").and_then(Value::as_array).context("workflows missing")?;

    let mut scenarios_in_matrix = BTreeSet::new();
    let mut component_metrics_exercised = BTreeSet::new();
    for workflow in workflows {
        let scenario_file = workflow
            .get("scenario_file")
            .and_then(Value::as_str)
            .context("workflow missing scenario_file")?;

        // ── component field validation ───────────────────────────────
        let component = workflow.get("component").and_then(Value::as_str).with_context(|| {
            format!("workflow `{scenario_file}` missing required string `component`")
        })?;
        assert!(
            valid_components.contains(component),
            "workflow `{scenario_file}` has unknown component `{component}`, \
             expected one of {valid_components:?}"
        );

        // ── instrumentation object validation ────────────────────────
        let instrumentation =
            workflow.get("instrumentation").and_then(Value::as_object).with_context(|| {
                format!("workflow `{scenario_file}` missing required object `instrumentation`")
            })?;
        for &key in INSTRUMENTATION_KEYS {
            let val = instrumentation.get(key).with_context(|| {
                format!("workflow `{scenario_file}` instrumentation missing key `{key}`")
            })?;
            assert!(
                val.is_boolean(),
                "workflow `{scenario_file}` instrumentation.{key} must be a boolean"
            );
        }

        let measures = collect_string_set(
            workflow.get("measures").context("workflow missing measures")?,
            scenario_file,
        )?;
        assert!(
            !measures.is_empty(),
            "workflow `{scenario_file}` must define at least one measure"
        );
        let mut has_extended_metric = false;
        for measure in &measures {
            assert!(
                allowed_metrics.contains(measure),
                "workflow `{scenario_file}` references unknown metric `{measure}`"
            );
            if let Some(count) = metric_usage_counts.get_mut(measure) {
                *count += 1;
            }
            if component_metrics.contains(measure) {
                component_metrics_exercised.insert(measure.clone());
            }
            if !baseline_metrics.contains(measure) {
                has_extended_metric = true;
            }
        }
        if has_extended_metric {
            workflows_with_extended_metrics += 1;
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
    assert_eq!(
        confidence_signals_exercised, confidence_signals,
        "every declared confidence signal must be exercised by at least one workflow"
    );

    let workflow_count = workflows.len();
    let extended_metric_coverage = workflows_with_extended_metrics as f64 / workflow_count as f64;
    assert!(
        extended_metric_coverage >= 0.30,
        "extended metric coverage too low: {workflows_with_extended_metrics}/{workflow_count} workflows (expected >= 30%)"
    );

    for component_metric in &component_metrics {
        let count = *metric_usage_counts.get(component_metric).unwrap_or(&0);
        assert!(
            count >= 2,
            "component metric `{component_metric}` must be exercised by at least two workflows (found {count})"
        );
    }

    Ok(())
}

#[test]
fn semantic_inline_completion_receipt_inventory_is_current() -> Result<()> {
    let matrix = load_fixture_matrix()?;
    let workflows =
        matrix.get("workflows").and_then(Value::as_array).context("workflows missing")?;
    let by_id: HashMap<&str, &Value> = workflows
        .iter()
        .filter_map(|workflow| {
            let id = workflow.get("id")?.as_str()?;
            Some((id, workflow))
        })
        .collect();

    let mut receipt_backed_count = 0_usize;
    let mut direct_stdio_count = 0_usize;

    for id in SEMANTIC_INLINE_RECEIPT_IDS {
        let workflow = by_id.get(id).with_context(|| {
            format!("semantic inline workflow `{id}` missing from fixture matrix")
        })?;
        let scenario_file = workflow
            .get("scenario_file")
            .and_then(Value::as_str)
            .with_context(|| format!("workflow `{id}` missing scenario_file"))?;
        let component = workflow
            .get("component")
            .and_then(Value::as_str)
            .with_context(|| format!("workflow `{id}` missing component"))?;
        assert_eq!(
            component, "completion",
            "semantic inline workflow `{id}` must remain in the completion component"
        );
        assert!(
            scenario_file.contains("inline_completion_quality"),
            "semantic inline workflow `{id}` should point at an inline-completion quality scenario, got `{scenario_file}`"
        );

        let instrumentation = workflow
            .get("instrumentation")
            .and_then(Value::as_object)
            .with_context(|| format!("workflow `{id}` missing instrumentation"))?;
        let run_receipt = instrumentation
            .get("run_receipt")
            .and_then(Value::as_bool)
            .with_context(|| format!("workflow `{id}` missing instrumentation.run_receipt"))?;
        if run_receipt {
            receipt_backed_count += 1;
        } else {
            direct_stdio_count += 1;
        }

        let measures = collect_string_set(
            workflow
                .get("measures")
                .with_context(|| format!("workflow `{id}` missing measures"))?,
            id,
        )?;
        assert!(
            measures.contains("workflow_pass_rate") && measures.contains("workflow_stability_rate"),
            "semantic inline workflow `{id}` must stay visible in pass and stability rollups"
        );

        let confidence_signals = collect_string_set(
            workflow
                .get("confidence_signals")
                .with_context(|| format!("workflow `{id}` missing confidence_signals"))?,
            &format!("{id}.confidence_signals"),
        )?;
        assert!(
            confidence_signals.contains("first_five_minutes_harness")
                && confidence_signals.contains("manual_editor_smoke")
                && confidence_signals.contains("issue_burndown_regression_guard"),
            "semantic inline workflow `{id}` must keep the standard UX confidence signals"
        );

        let expected_outcomes = workflow
            .get("expected_outcomes")
            .and_then(Value::as_array)
            .with_context(|| format!("workflow `{id}` missing expected_outcomes"))?;
        assert!(
            expected_outcomes.iter().filter_map(Value::as_str).any(|outcome| {
                outcome.contains("inline completion") || outcome.contains("inline-completion")
            }),
            "semantic inline workflow `{id}` must state an inline completion outcome"
        );
    }

    assert!(
        receipt_backed_count >= 5,
        "semantic inline inventory should keep at least five receipt-backed workflows, got {receipt_backed_count}"
    );
    assert!(
        direct_stdio_count >= 2,
        "semantic inline inventory should keep direct stdio proof workflows visible, got {direct_stdio_count}"
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
