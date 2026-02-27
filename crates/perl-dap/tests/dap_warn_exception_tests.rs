use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn initialize_adapter() -> DebugAdapter {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);
    adapter
}

#[test]
fn test_initialize_includes_warn_filter() -> TestResult {
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(1, "initialize", None);

    let body = match response {
        DapMessage::Response { success: true, body: Some(body), .. } => body,
        _ => return Err("Expected successful initialize".into()),
    };

    let filters = body
        .get("exceptionBreakpointFilters")
        .and_then(|v| v.as_array())
        .ok_or("Missing exceptionBreakpointFilters")?;

    let has_warn = filters.iter().any(|f| f.get("filter").and_then(|v| v.as_str()) == Some("warn"));

    if perl_dap::feature_catalog::has_feature("dap.exceptions.warn") {
        assert!(has_warn, "warn filter should be advertised when feature is enabled");

        // Verify warn filter properties
        let warn_filter = filters
            .iter()
            .find(|f| f.get("filter").and_then(|v| v.as_str()) == Some("warn"))
            .ok_or("warn filter not found")?;

        assert_eq!(
            warn_filter.get("label").and_then(|v| v.as_str()),
            Some("Perl warn() and Carp warnings")
        );
        assert_eq!(warn_filter.get("default").and_then(|v| v.as_bool()), Some(false));
    } else {
        assert!(!has_warn, "warn filter should not be advertised when feature is disabled");
    }

    Ok(())
}

#[test]
fn test_set_exception_breakpoints_with_warn_filter() -> TestResult {
    let mut adapter = initialize_adapter();

    let response =
        adapter.handle_request(2, "setExceptionBreakpoints", Some(json!({ "filters": ["warn"] })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected response".into()),
    }

    Ok(())
}

#[test]
fn test_set_exception_breakpoints_warn_and_die() -> TestResult {
    let mut adapter = initialize_adapter();

    let response = adapter.handle_request(
        2,
        "setExceptionBreakpoints",
        Some(json!({ "filters": ["die", "warn"] })),
    );

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected response".into()),
    }

    Ok(())
}

#[test]
fn test_set_exception_breakpoints_all_includes_warn() -> TestResult {
    let mut adapter = initialize_adapter();

    // "all" should activate both die and warn
    let response =
        adapter.handle_request(2, "setExceptionBreakpoints", Some(json!({ "filters": ["all"] })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected response".into()),
    }

    Ok(())
}

#[test]
fn test_set_exception_breakpoints_warn_via_filter_options() -> TestResult {
    let mut adapter = initialize_adapter();

    let response = adapter.handle_request(
        2,
        "setExceptionBreakpoints",
        Some(json!({
            "filters": [],
            "filterOptions": [{ "filterId": "warn" }]
        })),
    );

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected response".into()),
    }

    Ok(())
}

#[test]
fn test_set_exception_breakpoints_clear_warn() -> TestResult {
    let mut adapter = initialize_adapter();

    // Enable warn
    adapter.handle_request(2, "setExceptionBreakpoints", Some(json!({ "filters": ["warn"] })));

    // Clear all filters
    let response =
        adapter.handle_request(3, "setExceptionBreakpoints", Some(json!({ "filters": [] })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected response".into()),
    }

    Ok(())
}

#[test]
fn test_set_exception_breakpoints_warn_case_insensitive() -> TestResult {
    let mut adapter = initialize_adapter();

    let response =
        adapter.handle_request(2, "setExceptionBreakpoints", Some(json!({ "filters": ["WARN"] })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected response".into()),
    }

    Ok(())
}

#[test]
fn test_warn_filter_in_feature_catalog() {
    // The warn feature should be registered in the catalog
    let has_feature = perl_dap::feature_catalog::has_feature("dap.exceptions.warn");
    assert!(has_feature, "dap.exceptions.warn should be a registered feature");
}
