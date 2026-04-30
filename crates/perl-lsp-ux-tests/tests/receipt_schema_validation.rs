//! Unit tests validating that serialized `UxScenarioRunReceipt` instances
//! conform to the JSON schema at `.ci/schemas/ux-scenario-run.schema.json`.
//!
//! Uses structural validation with `serde_json` — no external jsonschema crate.
//! Receipts are built via `UxRunRecorder` (the public API) since the structs
//! are `#[non_exhaustive]`.

use perl_lsp_ux_tests::{UxCiTier, UxComponent, UxFailureClass, UxRunRecorder, UxScenarioSkip};

// ── Schema constants (derived from .ci/schemas/ux-scenario-run.schema.json) ──

const REQUIRED_TOP_LEVEL: &[&str] = &[
    "kind",
    "schema_version",
    "measured_at",
    "run_identity",
    "workflow_id",
    "scenario_file",
    "test_name",
    "ci_tier",
    "result",
    "duration_ms",
    "assertions",
    "canonical_repro",
    "friendly_repro",
];

const ALLOWED_TOP_LEVEL: &[&str] = &[
    "kind",
    "schema_version",
    "measured_at",
    "run_identity",
    "workflow_id",
    "scenario_file",
    "test_name",
    "component",
    "ci_tier",
    "result",
    "duration_ms",
    "time_to_first_useful_result_ms",
    "operation_timings",
    "assertions",
    "failure_class",
    "route",
    "skip_reason",
    "canonical_repro",
    "friendly_repro",
];

const ALLOWED_RUN_IDENTITY: &[&str] = &["sha", "branch", "run_id", "attempt", "platform"];

const VALID_COMPONENTS: &[&str] = &[
    "completion",
    "diagnostics",
    "module_resolution",
    "workspace_symbols",
    "rename",
    "hover",
    "goto_definition",
    "infra",
    "ai_completion",
];

const VALID_CI_TIERS: &[&str] = &["pr", "nightly", "release"];

const VALID_RESULTS: &[&str] = &["pass", "fail", "quarantined", "skipped"];

const VALID_FAILURE_CLASSES: &[&str] = &[
    "provider_regression",
    "server_crash",
    "timeout",
    "test_race",
    "infra",
    "matrix_drift",
    "baseline_drift",
    "new_test_bug",
    "unknown",
];

const VALID_ROUTES: &[&str] = &[
    "ci_investigation",
    "fixture_update",
    "test_fix",
    "provider_fix",
    "triage",
    "baseline_update",
    "crash_fix",
    "timeout_triage",
];

const VALID_ASSERTION_BASES: &[&str] = &["instrumented", "not_yet_instrumented"];

const REQUIRED_ASSERTION_FIELDS: &[&str] = &["passed", "failed", "basis"];

const ALLOWED_ASSERTION_FIELDS: &[&str] = &["passed", "failed", "basis", "failed_check_names"];

const REQUIRED_OPERATION_TIMING_FIELDS: &[&str] = &["operation"];

const ALLOWED_OPERATION_TIMING_FIELDS: &[&str] =
    &["operation", "time_to_first_useful_result_ms", "timing_status"];

// ── Structural validation helpers ────────────────────────────────────────

/// Validate that a JSON object contains all required keys.
fn assert_has_required_keys(
    obj: &serde_json::Map<String, serde_json::Value>,
    required: &[&str],
    context: &str,
) {
    for key in required {
        assert!(obj.contains_key(*key), "{context}: missing required key \"{key}\"");
    }
}

/// Validate that a JSON object contains only allowed keys (additionalProperties: false).
fn assert_no_extra_keys(
    obj: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    context: &str,
) {
    for key in obj.keys() {
        assert!(
            allowed.contains(&key.as_str()),
            "{context}: unexpected key \"{key}\" (allowed: {allowed:?})"
        );
    }
}

/// Validate that a string value is one of the allowed enum values.
fn assert_enum_value(value: &serde_json::Value, allowed: &[&str], context: &str) {
    assert!(value.is_string(), "{context}: expected string, got {value}");
    let s = value.as_str().unwrap_or_default();
    assert!(allowed.contains(&s), "{context}: \"{s}\" not in {allowed:?}");
}

/// Validate a non-empty string field.
fn assert_non_empty_string(value: &serde_json::Value, context: &str) {
    assert!(value.is_string(), "{context}: expected string, got {value}");
    let s = value.as_str().unwrap_or_default();
    assert!(!s.is_empty(), "{context}: string must not be empty");
}

/// Validate a non-negative number field.
fn assert_non_negative_number(value: &serde_json::Value, context: &str) {
    assert!(value.is_number(), "{context}: expected number, got {value}");
    let n = value.as_f64().unwrap_or(-1.0);
    assert!(n >= 0.0, "{context}: expected non-negative, got {n}");
}

/// Validate the assertions sub-object against the schema.
fn validate_assertions(assertions: &serde_json::Value, context: &str) {
    assert!(assertions.is_object(), "{context}: assertions must be an object");
    let Some(obj) = assertions.as_object() else {
        return;
    };

    assert_has_required_keys(obj, REQUIRED_ASSERTION_FIELDS, context);
    assert_no_extra_keys(obj, ALLOWED_ASSERTION_FIELDS, context);

    // passed: integer | null, minimum 0
    let passed = &obj["passed"];
    assert!(
        passed.is_null()
            || passed.is_u64()
            || (passed.is_i64() && passed.as_i64().unwrap_or(-1) >= 0),
        "{context}.passed: expected null or non-negative integer, got {passed}"
    );

    // failed: integer | null, minimum 0
    let failed = &obj["failed"];
    assert!(
        failed.is_null()
            || failed.is_u64()
            || (failed.is_i64() && failed.as_i64().unwrap_or(-1) >= 0),
        "{context}.failed: expected null or non-negative integer, got {failed}"
    );

    // basis: enum
    assert_enum_value(&obj["basis"], VALID_ASSERTION_BASES, &format!("{context}.basis"));

    // failed_check_names: optional array of strings
    if let Some(names) = obj.get("failed_check_names") {
        let arr = names.as_array();
        assert!(arr.is_some(), "{context}.failed_check_names: expected array");
        if let Some(items) = arr {
            for (i, item) in items.iter().enumerate() {
                assert!(
                    item.is_string(),
                    "{context}.failed_check_names[{i}]: expected string, got {item}"
                );
            }
        }
    }
}

/// Validate the run_identity sub-object against the schema.
fn validate_run_identity(identity: &serde_json::Value, context: &str) {
    assert!(identity.is_object(), "{context}: run_identity must be an object");
    let Some(obj) = identity.as_object() else {
        return;
    };

    assert_no_extra_keys(obj, ALLOWED_RUN_IDENTITY, context);

    // All run_identity fields are optional strings (minLength: 1) or integer.
    if let Some(sha) = obj.get("sha") {
        assert_non_empty_string(sha, &format!("{context}.sha"));
    }
    if let Some(branch) = obj.get("branch") {
        assert_non_empty_string(branch, &format!("{context}.branch"));
    }
    if let Some(run_id) = obj.get("run_id") {
        assert_non_empty_string(run_id, &format!("{context}.run_id"));
    }
    if let Some(attempt) = obj.get("attempt") {
        assert!(attempt.is_u64(), "{context}.attempt: expected integer, got {attempt}");
        assert!(attempt.as_u64().unwrap_or(0) >= 1, "{context}.attempt: minimum 1, got {attempt}");
    }
    if let Some(platform) = obj.get("platform") {
        assert_non_empty_string(platform, &format!("{context}.platform"));
    }
}

/// Validate an operation_timings entry against the schema.
fn validate_operation_timing(timing: &serde_json::Value, context: &str) {
    assert!(timing.is_object(), "{context}: operation_timing must be an object");
    let Some(obj) = timing.as_object() else {
        return;
    };

    assert_has_required_keys(obj, REQUIRED_OPERATION_TIMING_FIELDS, context);
    assert_no_extra_keys(obj, ALLOWED_OPERATION_TIMING_FIELDS, context);

    assert_non_empty_string(&obj["operation"], &format!("{context}.operation"));

    if let Some(ms) = obj.get("time_to_first_useful_result_ms") {
        if !ms.is_null() {
            assert_non_negative_number(ms, &format!("{context}.time_to_first_useful_result_ms"));
        }
    }

    if let Some(status) = obj.get("timing_status") {
        if !status.is_null() {
            assert_eq!(
                status.as_str(),
                Some("missing_request_start"),
                "{context}.timing_status: must be \"missing_request_start\""
            );
        }
    }
}

/// Full structural validation of a serialized receipt against the schema.
fn validate_receipt_against_schema(json: &serde_json::Value) {
    assert!(json.is_object(), "receipt must be a JSON object");
    let Some(obj) = json.as_object() else {
        return;
    };

    // Required and allowed top-level keys.
    assert_has_required_keys(obj, REQUIRED_TOP_LEVEL, "receipt");
    assert_no_extra_keys(obj, ALLOWED_TOP_LEVEL, "receipt");

    // kind: const "ux_scenario_run"
    assert_eq!(obj["kind"].as_str(), Some("ux_scenario_run"), "kind must be \"ux_scenario_run\"");

    // schema_version: const 1
    assert_eq!(obj["schema_version"].as_u64(), Some(1), "schema_version must be 1");

    // measured_at: string (date-time format — basic check)
    assert_non_empty_string(&obj["measured_at"], "measured_at");
    let ts = obj["measured_at"].as_str().unwrap_or_default();
    assert!(ts.contains('T'), "measured_at should be ISO-8601 (contains 'T'): {ts}");

    // run_identity
    validate_run_identity(&obj["run_identity"], "run_identity");

    // String fields with minLength: 1
    assert_non_empty_string(&obj["workflow_id"], "workflow_id");
    assert_non_empty_string(&obj["scenario_file"], "scenario_file");
    assert_non_empty_string(&obj["test_name"], "test_name");
    assert_non_empty_string(&obj["canonical_repro"], "canonical_repro");
    assert_non_empty_string(&obj["friendly_repro"], "friendly_repro");

    // component: optional enum
    if let Some(component) = obj.get("component") {
        if !component.is_null() {
            assert_enum_value(component, VALID_COMPONENTS, "component");
        }
    }

    // ci_tier: enum
    assert_enum_value(&obj["ci_tier"], VALID_CI_TIERS, "ci_tier");

    // result: enum
    assert_enum_value(&obj["result"], VALID_RESULTS, "result");

    // duration_ms: number >= 0
    assert_non_negative_number(&obj["duration_ms"], "duration_ms");

    // time_to_first_useful_result_ms: optional number >= 0
    if let Some(ms) = obj.get("time_to_first_useful_result_ms") {
        if !ms.is_null() {
            assert_non_negative_number(ms, "time_to_first_useful_result_ms");
        }
    }

    // operation_timings: optional array
    if let Some(timings) = obj.get("operation_timings") {
        let arr = timings.as_array();
        assert!(arr.is_some(), "operation_timings must be an array");
        if let Some(items) = arr {
            for (i, entry) in items.iter().enumerate() {
                validate_operation_timing(entry, &format!("operation_timings[{i}]"));
            }
        }
    }

    // assertions
    validate_assertions(&obj["assertions"], "assertions");

    // failure_class: optional enum
    if let Some(fc) = obj.get("failure_class") {
        if !fc.is_null() {
            assert_enum_value(fc, VALID_FAILURE_CLASSES, "failure_class");
        }
    }

    // route: optional enum
    if let Some(route) = obj.get("route") {
        if !route.is_null() {
            assert_enum_value(route, VALID_ROUTES, "route");
        }
    }

    // skip_reason: required for skipped receipts, omitted otherwise.
    if obj["result"] == "skipped" {
        let reason = obj.get("skip_reason");
        assert!(reason.is_some(), "skipped receipt must include skip_reason");
        if let Some(value) = reason {
            assert_non_empty_string(value, "skip_reason");
        }
    } else {
        assert!(
            obj.get("skip_reason").is_none() || obj["skip_reason"].is_null(),
            "non-skipped receipt should not include skip_reason"
        );
    }
}

// ── Test cases ───────────────────────────────────────────────────────────

/// Full-success receipt: all fields populated, result=pass, assertions
/// instrumented, timing present, component set.
#[test]
fn receipt_full_success_conforms_to_schema() -> Result<(), Box<dyn std::error::Error>> {
    let mut recorder = UxRunRecorder::new(
        "completion_basic",
        "ux_scenario_01.rs",
        "basic_completion_open",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
    );

    // Instrument assertions.
    recorder.check("completion list is non-empty", true)?;
    recorder.check("first item has label", true)?;
    recorder.check("sort order is correct", true)?;

    // Record operation timing.
    recorder.mark_request_start("completion");
    std::thread::sleep(std::time::Duration::from_millis(1));
    recorder.mark_first_useful_result("completion");

    let receipt = recorder.finish_pass();
    let json = serde_json::to_value(&receipt)?;
    validate_receipt_against_schema(&json);

    // Full-success specific checks.
    assert_eq!(json["result"], "pass");
    assert!(json.get("failure_class").is_none());
    assert!(json.get("route").is_none());
    assert_eq!(json["assertions"]["basis"], "instrumented");
    assert_eq!(json["assertions"]["passed"], 3);
    assert_eq!(json["assertions"]["failed"], 0);
    assert!(json["time_to_first_useful_result_ms"].is_number());
    assert_eq!(json["component"], "completion");

    // operation_timings present with one entry.
    let timings = json["operation_timings"].as_array();
    assert!(timings.is_some());
    assert_eq!(timings.map(Vec::len), Some(1));

    Ok(())
}

/// Partial-failure receipt: result=fail, failure_class set, some assertions
/// failed, failed_check_names populated.
#[test]
fn receipt_partial_failure_conforms_to_schema() -> Result<(), Box<dyn std::error::Error>> {
    let mut recorder = UxRunRecorder::new(
        "hover_basic",
        "ux_scenario_02.rs",
        "hover_variable_info",
        UxCiTier::Nightly,
        Some(UxComponent::Hover),
    );

    // Mix of passing and failing checks.
    recorder.check("hover returns response", true)?;
    recorder.check("response has contents", true)?;
    let _ = recorder.check("hover returns type info", false); // fails
    let _ = recorder.check("hover includes documentation", false); // fails
    recorder.check("response format is valid", true)?;

    // Record timing for two operations.
    recorder.mark_request_start("hover");
    std::thread::sleep(std::time::Duration::from_millis(1));
    recorder.mark_first_useful_result("hover");

    recorder.mark_request_start("completion");
    // No mark_first_useful_result for completion — simulates incomplete timing.

    let receipt = recorder.finish_fail(UxFailureClass::ProviderRegression);
    let json = serde_json::to_value(&receipt)?;
    validate_receipt_against_schema(&json);

    // Partial-failure specific checks.
    assert_eq!(json["result"], "fail");
    assert_eq!(json["failure_class"], "provider_regression");
    assert_eq!(json["route"], "provider_fix");
    assert_eq!(json["assertions"]["passed"], 3);
    assert_eq!(json["assertions"]["failed"], 2);
    assert_eq!(json["assertions"]["basis"], "instrumented");

    let names = json["assertions"]["failed_check_names"].as_array();
    assert_eq!(names.map(Vec::len), Some(2));

    // Two operation timings: hover (completed) and completion (started only).
    let timings = json["operation_timings"].as_array();
    assert_eq!(timings.map(Vec::len), Some(2));

    Ok(())
}

/// Uninstrumented receipt: assertions with basis="not_yet_instrumented",
/// passed/failed are null, no timing, no component.
#[test]
fn receipt_uninstrumented_conforms_to_schema() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = UxRunRecorder::new(
        "diagnostics_basic",
        "ux_scenario_03.rs",
        "diagnostics_no_checks",
        UxCiTier::Release,
        Some(UxComponent::Diagnostics),
    );

    // No check() calls — scenario is uninstrumented.
    // No timing marks — no operations recorded.

    let receipt = recorder.finish_pass();
    let json = serde_json::to_value(&receipt)?;
    validate_receipt_against_schema(&json);

    // Uninstrumented specific checks.
    assert_eq!(json["assertions"]["basis"], "not_yet_instrumented");
    assert!(json["assertions"]["passed"].is_null());
    assert!(json["assertions"]["failed"].is_null());
    assert_eq!(json["result"], "pass");

    // No timing recorded.
    assert!(
        json.get("time_to_first_useful_result_ms").is_none()
            || json["time_to_first_useful_result_ms"].is_null()
    );

    // operation_timings should be absent (skip_serializing_if = Vec::is_empty).
    assert!(
        json.get("operation_timings").is_none()
            || json["operation_timings"].as_array().is_none_or(Vec::is_empty)
    );

    // run_identity should be an object (all fields optional, env-dependent).
    assert!(json["run_identity"].is_object());

    Ok(())
}

/// Skipped receipt: result=skipped, minimal fields.
#[test]
fn receipt_skipped_conforms_to_schema() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = UxRunRecorder::new(
        "real_workspace_mojolicious",
        "ux_scenario_real.rs",
        "mojolicious_clone_and_open",
        UxCiTier::Nightly,
        None, // No component for skipped infra scenario.
    );

    let skip = UxScenarioSkip::infra("PERL_LSP_BIN not set and target/debug/perl-lsp not found");
    let receipt = recorder.finish_skipped(&skip);
    let json = serde_json::to_value(&receipt)?;

    validate_receipt_against_schema(&json);

    // Skipped specific checks.
    assert_eq!(json["result"], "skipped");
    assert_eq!(json["failure_class"], "infra");
    assert_eq!(json["route"], "ci_investigation");
    assert_eq!(json["skip_reason"], "PERL_LSP_BIN not set and target/debug/perl-lsp not found");

    // component should be absent (None → skip_serializing_if).
    assert!(
        json.get("component").is_none() || json["component"].is_null(),
        "skipped receipt with no component should omit or null the field"
    );

    // operation_timings should be absent (no operations recorded).
    assert!(
        json.get("operation_timings").is_none()
            || json["operation_timings"].as_array().is_none_or(Vec::is_empty)
    );

    // Assertions should still be valid (uninstrumented since no checks).
    assert_eq!(json["assertions"]["basis"], "not_yet_instrumented");
    assert!(json["assertions"]["passed"].is_null());
    assert!(json["assertions"]["failed"].is_null());

    Ok(())
}
