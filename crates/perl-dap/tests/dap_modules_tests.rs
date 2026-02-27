use perl_dap::{DapMessage, DebugAdapter};
use perl_tdd_support::must;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper: send a modules request and return the response body.
fn modules_request(
    adapter: &mut DebugAdapter,
    arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let response = adapter.handle_request(2, "modules", arguments);
    match response {
        DapMessage::Response { success: true, body: Some(body), .. } => Ok(body),
        DapMessage::Response { success: false, message, .. } => {
            Err(message.unwrap_or_else(|| "unknown error".to_string()))
        }
        _ => Err("unexpected response type".to_string()),
    }
}

#[test]
fn test_modules_no_session() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = modules_request(&mut adapter, Some(json!({ "startModule": 0 })))
        .map_err(|e| e.to_string())?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules array")?;
    let total =
        body.get("totalModules").and_then(|v| v.as_i64()).ok_or("missing totalModules field")?;

    assert!(modules.is_empty(), "No session should mean empty modules");
    assert_eq!(total, 0, "No session should mean total 0");

    Ok(())
}

#[test]
fn test_modules_with_default_arguments() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = modules_request(&mut adapter, Some(json!({ "startModule": 0 })))
        .map_err(|e| e.to_string())?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules")?;
    assert!(modules.is_empty(), "Expected empty modules without active session");

    Ok(())
}

#[test]
fn test_modules_pagination_count_zero() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = modules_request(&mut adapter, Some(json!({ "startModule": 0, "moduleCount": 0 })))
        .map_err(|e| e.to_string())?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules")?;
    assert!(modules.is_empty());

    Ok(())
}

#[test]
fn test_modules_negative_start_clamped() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    // Negative start should be clamped to 0
    let body = modules_request(&mut adapter, Some(json!({ "startModule": -1 })))
        .map_err(|e| e.to_string())?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules")?;
    // No session, so still empty — but the important thing is no crash
    assert!(modules.is_empty());

    Ok(())
}

#[test]
fn test_modules_negative_count_clamped() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = modules_request(&mut adapter, Some(json!({ "startModule": 0, "moduleCount": -1 })))
        .map_err(|e| e.to_string())?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules")?;
    assert!(modules.is_empty());

    Ok(())
}

#[test]
fn test_modules_missing_arguments() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    // None arguments should still succeed (optional args pattern)
    let body = modules_request(&mut adapter, None).map_err(|e| e.to_string())?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules")?;
    assert!(modules.is_empty());

    Ok(())
}

#[test]
fn test_modules_response_structure() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = modules_request(&mut adapter, Some(json!({ "startModule": 0 })))
        .map_err(|e| e.to_string())?;

    // Verify required response fields exist
    assert!(body.get("modules").is_some(), "Response must have 'modules' field");
    assert!(body.get("modules").and_then(|v| v.as_array()).is_some(), "modules must be an array");
    assert!(body.get("totalModules").is_some(), "Response must have 'totalModules' field");
    assert!(
        body.get("totalModules").and_then(|v| v.as_i64()).is_some(),
        "totalModules must be a number"
    );

    Ok(())
}

#[test]
fn test_modules_response_is_success() {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let response = adapter.handle_request(2, "modules", Some(json!({ "startModule": 0 })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "modules");
        }
        _ => must(Err::<(), _>("Expected response message")),
    }
}

#[test]
fn test_modules_large_start_offset() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    // Start beyond any possible module count should return empty
    let body = modules_request(&mut adapter, Some(json!({ "startModule": 999999 })))
        .map_err(|e| e.to_string())?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules")?;
    assert!(modules.is_empty());

    Ok(())
}
