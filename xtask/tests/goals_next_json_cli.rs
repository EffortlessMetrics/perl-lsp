//! End-to-end CLI regression for #3692 defect 1: `cargo xtask goals next
//! --json` must ALWAYS emit parseable JSON on stdout and exit nonzero on
//! any internal error — never an unstructured `color_eyre` dump on
//! stderr.
//!
//! The in-crate unit tests in `goals/mod.rs` (`json_error_output_is_parseable_and_names_the_failure`,
//! `next_with_json_never_returns_err_to_the_caller`) can only exercise
//! `render_output`/`render_json_error` directly — the json=true branch of
//! `next()` itself calls `std::process::exit(1)`, which cannot be invoked
//! in-process without killing the test runner, so nothing in the unit
//! suite actually proves `next()` is wired to print JSON-then-exit rather
//! than propagate `Err` to its caller (a regression here would previously
//! have surfaced as an unstructured stderr dump with a non-JSON stdout,
//! exactly the #3692 defect 1 symptom). These tests drive the actual
//! compiled binary end-to-end — one on the error path, one on the
//! success path — so real wiring is exercised in both directions.
use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn goals_next_json_prints_parseable_json_and_exits_1_on_internal_error() -> Result<()> {
    // A missing --fixture file is the same cheap, network-free way to
    // force an internal `Err` that the in-crate unit tests use (see
    // `render_output_surfaces_an_err_when_the_fixture_path_does_not_exist`
    // in `goals/mod.rs`) — no `gh auth`/network dependency, and it fails
    // deep inside `build_snapshot` (well past program/manifest
    // resolution), matching the "any internal Err" scope of the fix.
    let assert = cargo_bin_cmd!("xtask")
        .args(["goals", "next", "--json", "--fixture", "definitely/does/not/exist/prs.json"])
        .assert()
        .failure()
        .code(1);

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;

    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|e| {
        anyhow::anyhow!(
            "stdout must be parseable JSON for a --json caller, got parse error {e}\nstdout: {stdout:?}\nstderr: {stderr:?}"
        )
    })?;
    let error_field = parsed
        .get("error")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("expected a string \"error\" field, got {parsed}"))?;
    assert!(!error_field.is_empty(), "expected a non-empty error message, got {parsed}");
    assert!(
        parsed.get("error_chain").is_some_and(|v| v.is_array()),
        "expected an \"error_chain\" array field, got {parsed}"
    );

    // The defining symptom of #3692 defect 1: a --json caller must never
    // see an unstructured color_eyre dump on stderr.
    assert!(
        stderr.trim().is_empty(),
        "a --json caller must never see anything on stderr, got: {stderr:?}"
    );

    Ok(())
}

#[test]
fn goals_next_json_prints_valid_json_and_exits_0_on_success() -> Result<()> {
    // Companion to the error-path test above: proves the OTHER branch of
    // `next()`'s json=true wiring in `xtask/src/tasks/goals/mod.rs` —
    // `Ok(serde_json::to_string_pretty(&output)?)` — actually reaches
    // stdout as well-formed, parseable JSON with a clean exit 0 and empty
    // stderr on the happy path, not just the error path. Before this
    // test, no test anywhere (unit or integration) drove `next()`'s real
    // json=true success branch end to end through the compiled binary; a
    // regression that broke JSON serialization on success (e.g. an
    // unserializable field, or a stray println! before the JSON) would
    // not have been caught.
    //
    // A `--fixture` file with an empty `prs` array avoids any `gh`
    // auth/network dependency, matching the network-free pattern the
    // error-path test above uses; the real repo's `.perl-lsp/goals/`
    // manifests supply the resolvable program/candidate state (this test
    // does not depend on the ledger's exact content — only on `next()`
    // completing without error and producing valid, well-shaped JSON).
    let temp = tempfile::tempdir()?;
    let fixture_path = temp.path().join("empty-prs.json");
    std::fs::write(&fixture_path, r#"{"repository":"ripr-fixture/repo","prs":[]}"#)?;

    let assert = cargo_bin_cmd!("xtask")
        .args(["goals", "next", "--json", "--fixture", &fixture_path.to_string_lossy()])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;

    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|e| {
        anyhow::anyhow!(
            "stdout must be parseable JSON on the success path too, got parse error {e}\nstdout: {stdout:?}\nstderr: {stderr:?}"
        )
    })?;
    assert!(
        parsed.get("repository").and_then(|v| v.as_str()).is_some(),
        "expected a string \"repository\" field on success, got {parsed}"
    );
    assert!(
        matches!(
            parsed.get("decision").and_then(|v| v.as_str()),
            Some("selected") | Some("blocked") | Some("complete")
        ),
        "expected a \"decision\" field naming one of selected/blocked/complete, got {parsed}"
    );
    assert!(
        parsed.get("error").is_none() && parsed.get("error_chain").is_none(),
        "success-path JSON must not carry the error-path's \"error\"/\"error_chain\" fields, got {parsed}"
    );
    assert!(
        stderr.trim().is_empty(),
        "success path must not print anything on stderr, got: {stderr:?}"
    );

    Ok(())
}
