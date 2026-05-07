use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use serde_json::Value;

type TestResult = Result<()>;

const TOP_LEVEL_REQUIRED: &[&str] = &[
    "schema_version",
    "subsystem",
    "generated_at",
    "commit",
    "cadence",
    "denominator",
    "families",
    "metrics",
    "failure_packets",
    "gold_drift",
    "metric_runtime",
];

const DENOMINATOR_REQUIRED: &[&str] = &[
    "fixture_count",
    "fixture_family_count",
    "scored_line_count",
    "scored_symbol_count",
    "fully_labeled_region_count",
    "partial_labeled_region_count",
    "unknown_region_count",
    "negative_region_count",
    "dynamic_boundary_case_count",
    "unsupported_construct_case_count",
    "real_project_file_count",
    "generated_fixture_count",
    "hand_labeled_fixture_count",
];

#[test]
fn parser_accuracy_schema_declares_required_contract() -> TestResult {
    let schema = read_json(&repo_root()?.join(".ci/schemas/parser-accuracy.schema.json"))?;
    assert_eq!(schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert_eq!(schema["properties"]["subsystem"]["const"], "parser_accuracy");

    let required = string_array(&schema["required"], "top-level required fields")?;
    for field in TOP_LEVEL_REQUIRED {
        assert!(
            required.iter().any(|item| item == field),
            "schema is missing required top-level field {field}"
        );
    }

    let metric_row = &schema["$defs"]["metricRow"];
    let one_of = metric_row["oneOf"].as_array().ok_or_else(|| {
        eyre!("metricRow must define measured and insufficient-data alternatives")
    })?;
    assert_eq!(one_of.len(), 2, "metricRow must have exactly two states");

    Ok(())
}

#[test]
fn parser_accuracy_example_artifact_matches_schema_contract() -> TestResult {
    let root = repo_root()?;
    let artifact =
        read_json(&root.join("xtask/tests/fixtures/parser-accuracy/example-artifact.json"))?;

    for field in TOP_LEVEL_REQUIRED {
        assert!(artifact.get(*field).is_some(), "example artifact is missing {field}");
    }
    assert_eq!(artifact["schema_version"], 1);
    assert_eq!(artifact["subsystem"], "parser_accuracy");
    assert!(
        ["pr", "merge_gate", "nightly", "release"].contains(
            &artifact["cadence"]
                .as_str()
                .ok_or_else(|| eyre!("cadence must be a string in example artifact"))?,
        ),
        "cadence must match the schema enum"
    );

    validate_denominator(&artifact["denominator"])?;
    validate_families(&artifact)?;
    validate_metric_rows(&artifact)?;
    validate_gold_drift(&artifact["gold_drift"])?;
    validate_metric_runtime(&artifact["metric_runtime"])?;

    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .ok_or_else(|| eyre!("xtask manifest should have a workspace parent"))?;
    Ok(root.to_path_buf())
}

fn read_json(path: &Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn string_array(value: &Value, label: &str) -> Result<Vec<String>> {
    let items = value.as_array().ok_or_else(|| eyre!("{label} must be an array"))?;
    let mut output = Vec::with_capacity(items.len());
    for item in items {
        let text = item.as_str().ok_or_else(|| eyre!("{label} must contain strings"))?;
        output.push(text.to_string());
    }
    Ok(output)
}

fn validate_denominator(value: &Value) -> TestResult {
    let object = value.as_object().ok_or_else(|| eyre!("denominator must be an object"))?;
    for field in DENOMINATOR_REQUIRED {
        object
            .get(*field)
            .and_then(Value::as_u64)
            .ok_or_else(|| eyre!("denominator field {field} must be a non-negative integer"))?;
    }
    Ok(())
}

fn validate_families(artifact: &Value) -> TestResult {
    let families =
        artifact["families"].as_array().ok_or_else(|| eyre!("families must be an array"))?;
    assert!(!families.is_empty(), "example artifact should include at least one family");
    for family in families {
        assert!(
            family["family"].as_str().is_some_and(|value| !value.is_empty()),
            "family rows require a non-empty family name"
        );
        assert!(family["fixture_count"].as_u64().is_some(), "family rows require fixture_count");
        let label_modes = family["label_modes"]
            .as_array()
            .ok_or_else(|| eyre!("label_modes must be an array"))?;
        assert!(!label_modes.is_empty(), "family rows require label_modes");
    }
    Ok(())
}

fn validate_metric_rows(artifact: &Value) -> TestResult {
    let metrics =
        artifact["metrics"].as_array().ok_or_else(|| eyre!("metrics must be an array"))?;
    assert!(!metrics.is_empty(), "example artifact should include metric rows");

    let mut saw_measured = false;
    let mut saw_measured_line = false;
    let mut saw_measured_ast = false;
    let mut saw_measured_symbol = false;

    for metric in metrics {
        let state =
            metric["state"].as_str().ok_or_else(|| eyre!("metric state must be a string"))?;
        let sample_count = metric["sample_count"]
            .as_u64()
            .ok_or_else(|| eyre!("metric sample_count must be a non-negative integer"))?;
        match state {
            "measured" => {
                saw_measured = true;
                assert!(sample_count > 0, "measured rows require a positive sample_count");
                assert!(metric.get("value").and_then(Value::as_f64).is_some());
                assert!(metric.get("direction").and_then(Value::as_str).is_some());
                assert!(metric.get("confidence").and_then(Value::as_str).is_some());
                if metric["metric"].as_str() == Some("line_construct_f1") {
                    saw_measured_line = true;
                }
                if metric["metric"].as_str() == Some("ast_node_kind_f1") {
                    saw_measured_ast = true;
                }
                if metric["metric"].as_str() == Some("symbol_decl_f1") {
                    saw_measured_symbol = true;
                }
            }
            "insufficient_data" => {
                assert!(metric.get("reason").and_then(Value::as_str).is_some());
            }
            other => return Err(eyre!("unexpected metric state {other}")),
        }
    }

    assert!(saw_measured, "example should include at least one measured denominator row");
    assert!(saw_measured_line, "example should include measured line F1 row");
    assert!(saw_measured_ast, "example should include measured AST F1 row");
    assert!(saw_measured_symbol, "example should include measured symbol F1 row");
    Ok(())
}

fn validate_gold_drift(value: &Value) -> TestResult {
    let object = value.as_object().ok_or_else(|| eyre!("gold_drift must be an object"))?;
    for (key, count) in object {
        assert!(count.as_u64().is_some(), "gold_drift field {key} must be a non-negative integer");
    }
    Ok(())
}

fn validate_metric_runtime(value: &Value) -> TestResult {
    assert!(
        value["runtime_ms"].as_f64().is_some_and(|runtime| runtime >= 0.0),
        "metric_runtime.runtime_ms must be non-negative"
    );
    for field in ["timeout_count", "flake_count", "artifact_size_bytes"] {
        assert!(
            value[field].as_u64().is_some(),
            "metric_runtime.{field} must be a non-negative integer"
        );
    }
    if !value["allocated_bytes"].is_null() {
        assert!(
            value["allocated_bytes"].as_u64().is_some(),
            "metric_runtime.allocated_bytes must be a non-negative integer when present"
        );
    }
    if !value["allocation_count"].is_null() {
        assert!(
            value["allocation_count"].as_u64().is_some(),
            "metric_runtime.allocation_count must be a non-negative integer when present"
        );
    }
    Ok(())
}
