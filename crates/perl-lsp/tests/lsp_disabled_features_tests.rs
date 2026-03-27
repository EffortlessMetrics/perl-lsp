//! Integration tests for per-feature disable via `initializationOptions.disabledFeatures`.
//!
//! Issue #2170: devex: feature flags — no per-feature user disable mechanism.
//! Phase 1: Static disable at initialize time via `initializationOptions`.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Disabling `lsp.semantic_tokens` must remove the semanticTokensProvider from the
/// server capabilities response.
#[test]
fn test_disabled_features_removes_semantic_tokens_from_caps() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let result = harness.initialize_with_init_options(
        Some(json!({})),
        json!({ "disabledFeatures": ["lsp.semantic_tokens"] }),
    )?;
    let caps = &result["capabilities"];
    assert!(
        caps.get("semanticTokensProvider").is_none() || caps["semanticTokensProvider"].is_null(),
        "semanticTokensProvider must be absent when lsp.semantic_tokens is disabled, got: {:?}",
        caps.get("semanticTokensProvider")
    );
    Ok(())
}

/// Unknown feature IDs must be silently ignored - the server must still initialize
/// and must not accidentally disable valid features.
#[test]
fn test_disabled_features_unknown_id_is_tolerated() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let result = harness.initialize_with_init_options(
        Some(json!({})),
        json!({ "disabledFeatures": ["lsp.does_not_exist", "semanticTokens"] }),
    )?;
    let caps = &result["capabilities"];
    // "semanticTokens" (without lsp. prefix) is not a valid ID - semantic tokens must remain.
    assert!(
        caps.get("semanticTokensProvider").is_some(),
        "semanticTokensProvider must still be present when only unknown IDs are given"
    );
    assert!(
        result["capabilities"].is_object(),
        "Server must initialize successfully when unknown feature IDs are given"
    );
    Ok(())
}

/// Non-string elements in `disabledFeatures` must be silently skipped.
#[test]
fn test_disabled_features_non_string_elements_skipped() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let result = harness.initialize_with_init_options(
        Some(json!({})),
        json!({ "disabledFeatures": [null, 42, true, {}, "lsp.does_not_exist"] }),
    )?;
    let caps = &result["capabilities"];
    // None of the non-string elements should disable anything; semantic tokens must remain.
    assert!(
        caps.get("semanticTokensProvider").is_some(),
        "semanticTokensProvider must be present when disabledFeatures contains only non-string/unknown items"
    );
    Ok(())
}

/// `disabledFeatures` set to a non-array value must be gracefully ignored.
#[test]
fn test_disabled_features_non_array_value_ignored() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let result = harness.initialize_with_init_options(
        Some(json!({})),
        json!({ "disabledFeatures": "lsp.semantic_tokens" }),
    )?;
    let caps = &result["capabilities"];
    // A bare string (not an array) must be ignored; semantic tokens must remain.
    assert!(
        caps.get("semanticTokensProvider").is_some(),
        "semanticTokensProvider must be present when disabledFeatures is a string, not an array"
    );
    Ok(())
}

/// Passing an empty `disabledFeatures` array must not change any capabilities.
/// Semantic tokens should remain present (it is in the production profile default).
#[test]
fn test_disabled_features_empty_array_is_noop() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let result =
        harness.initialize_with_init_options(Some(json!({})), json!({ "disabledFeatures": [] }))?;
    let caps = &result["capabilities"];
    assert!(
        caps.get("semanticTokensProvider").is_some(),
        "semanticTokensProvider must be present when disabledFeatures is empty, got caps: {:?}",
        caps
    );
    Ok(())
}

/// Disabling `lsp.declaration` must suppress the unconditional `declarationProvider: true`
/// override that the server applies to the capabilities JSON.
#[test]
fn test_disabled_features_declaration_suppresses_json_override() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let result = harness.initialize_with_init_options(
        Some(json!({})),
        json!({ "disabledFeatures": ["lsp.declaration"] }),
    )?;
    let caps = &result["capabilities"];
    assert!(
        caps.get("declarationProvider").is_none()
            || !caps["declarationProvider"].as_bool().unwrap_or(false),
        "declarationProvider must not be true when lsp.declaration is disabled, got: {:?}",
        caps.get("declarationProvider")
    );
    Ok(())
}

/// When `initializationOptions` is absent entirely, capabilities must be identical to
/// a normal initialization (no regression for non-VSCode clients).
#[test]
fn test_absent_initialization_options_is_noop() -> TestResult {
    let mut harness_with = LspHarness::new_raw();
    let with_empty = harness_with.initialize_with_init_options(Some(json!({})), json!({}))?;

    let mut harness_without = LspHarness::new_raw();
    let without = harness_without.initialize(Some(json!({})))?;

    // Both should have semanticTokensProvider
    let present_with = with_empty["capabilities"].get("semanticTokensProvider").is_some();
    let present_without = without["capabilities"].get("semanticTokensProvider").is_some();
    assert_eq!(
        present_with, present_without,
        "semanticTokensProvider presence must match: with_empty={}, without={}",
        present_with, present_without
    );
    Ok(())
}

/// Disabling multiple valid features simultaneously must suppress all of them.
/// This exercises the loop path: each ID in the array must independently zero
/// its BuildFlags field, and all zeroed fields must be absent from the response.
#[test]
fn test_disabled_features_multiple_features_all_suppressed() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let result = harness.initialize_with_init_options(
        Some(json!({})),
        json!({ "disabledFeatures": ["lsp.semantic_tokens", "lsp.hover", "lsp.completion"] }),
    )?;
    let caps = &result["capabilities"];

    assert!(
        caps.get("semanticTokensProvider").is_none() || caps["semanticTokensProvider"].is_null(),
        "semanticTokensProvider must be absent when lsp.semantic_tokens is disabled"
    );
    assert!(
        caps.get("hoverProvider").is_none() || caps["hoverProvider"].is_null(),
        "hoverProvider must be absent when lsp.hover is disabled"
    );
    assert!(
        caps.get("completionProvider").is_none() || caps["completionProvider"].is_null(),
        "completionProvider must be absent when lsp.completion is disabled"
    );
    Ok(())
}
