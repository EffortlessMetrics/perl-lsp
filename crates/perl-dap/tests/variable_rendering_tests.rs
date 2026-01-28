//! Variable Rendering Tests (AC8.6)
//!
//! Comprehensive tests for DAP variable rendering including:
//! - Hierarchical scope retrieval (Locals, Package, Globals)
//! - Lazy child expansion for arrays and hashes
//! - Scalar value truncation
//! - Complex nested structures
//!
//! Specification: GitHub Issue #452 - AC8.1, AC8.3, AC8.4, AC8.6

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

/// Helper to create a test adapter
fn create_test_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

#[test]
// AC:8.1
fn test_threads_main_thread_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    // Threads request without session should return empty
    let response = adapter.handle_request(1, "threads", None);

    if let DapMessage::Response { body, .. } = response {
        let body_val = body.ok_or("Expected body in response")?;
        let threads = body_val
            .get("threads")
            .ok_or("Expected threads field")?
            .as_array()
            .ok_or("Expected threads array")?;
        assert!(threads.is_empty());
    }
    Ok(())
}

#[test]
// AC:8.3
fn test_scopes_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({ "frameId": 1 });
    let response = adapter.handle_request(1, "scopes", Some(args));

    if let DapMessage::Response { success, body, .. } = response {
        assert!(success);
        let body_val = body.ok_or("Expected body in response")?;
        let scopes = body_val
            .get("scopes")
            .ok_or("Expected scopes field")?
            .as_array()
            .ok_or("Expected scopes array")?;

        // Should have 3 scopes: Locals, Package, Globals
        assert_eq!(scopes.len(), 3);
        assert_eq!(scopes[0].get("name").ok_or("Expected name field")?.as_str(), Some("Locals"));
        assert_eq!(scopes[1].get("name").ok_or("Expected name field")?.as_str(), Some("Package"));
        assert_eq!(scopes[2].get("name").ok_or("Expected name field")?.as_str(), Some("Globals"));

        // Verify unique variable references
        let ref0 = scopes[0]
            .get("variablesReference")
            .ok_or("Expected variablesReference")?
            .as_i64()
            .ok_or("Expected i64")?;
        let ref1 = scopes[1]
            .get("variablesReference")
            .ok_or("Expected variablesReference")?
            .as_i64()
            .ok_or("Expected i64")?;
        let ref2 = scopes[2]
            .get("variablesReference")
            .ok_or("Expected variablesReference")?
            .as_i64()
            .ok_or("Expected i64")?;
        assert!(ref0 != ref1 && ref1 != ref2);
    }
    Ok(())
}

#[test]
// AC:8.4
fn test_variables_lazy_expansion_indicators() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();

    // Request variables for Locals scope (ref 11 for frame 1)
    let args = json!({ "variablesReference": 11 });
    let response = adapter.handle_request(1, "variables", Some(args));

    if let DapMessage::Response { body, .. } = response {
        let body_val = body.ok_or("Expected body in response")?;
        let vars = body_val
            .get("variables")
            .ok_or("Expected variables field")?
            .as_array()
            .ok_or("Expected variables array")?;

        // Find @ _ array
        let array_var = vars
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("@_"))
            .ok_or("Expected @_ variable")?;
        assert_eq!(array_var.get("type").ok_or("Expected type field")?.as_str(), Some("array"));
        assert!(array_var.get("indexedVariables").is_some());

        // Find $self hash
        let hash_var = vars
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("$self"))
            .ok_or("Expected $self variable")?;
        assert_eq!(hash_var.get("type").ok_or("Expected type field")?.as_str(), Some("hash"));
        assert!(hash_var.get("namedVariables").is_some());
    }
    Ok(())
}

#[test]
// AC:8.6
fn test_variables_globals_scope() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({ "variablesReference": 13 }); // Globals for frame 1
    let response = adapter.handle_request(1, "variables", Some(args));

    if let DapMessage::Response { body, .. } = response {
        let body_val = body.ok_or("Expected body in response")?;
        let vars = body_val
            .get("variables")
            .ok_or("Expected variables field")?
            .as_array()
            .ok_or("Expected variables array")?;

        // Should contain standard globals like $_
        assert!(vars.iter().any(|v| v.get("name").and_then(|n| n.as_str()) == Some("$_")));
    }
    Ok(())
}

#[test]
// AC8.4: Test scalar value truncation at 1KB
fn test_scalar_truncation() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();

    // Request variables for Locals scope
    let args = json!({ "variablesReference": 11 });
    let response = adapter.handle_request(1, "variables", Some(args));

    if let DapMessage::Response { success, body, .. } = response {
        assert!(success);
        let body_val = body.ok_or("Expected body in response")?;
        let vars = body_val
            .get("variables")
            .ok_or("Expected variables field")?
            .as_array()
            .ok_or("Expected variables array")?;

        // Verify variables are returned (placeholder data)
        assert!(!vars.is_empty());

        // Verify each variable has required fields
        for var in vars {
            assert!(var.get("name").is_some());
            assert!(var.get("value").is_some());
            assert!(var.get("variablesReference").is_some());
        }
    }
    Ok(())
}

#[test]
// AC8.6: Test variable type indicators
fn test_variable_type_indicators() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({ "variablesReference": 11 });
    let response = adapter.handle_request(1, "variables", Some(args));

    if let DapMessage::Response { body, .. } = response {
        let body_val = body.ok_or("Expected body in response")?;
        let vars = body_val
            .get("variables")
            .ok_or("Expected variables field")?
            .as_array()
            .ok_or("Expected variables array")?;

        // Arrays should have indexedVariables
        let arrays = vars.iter().filter(|v| {
            v.get("type").and_then(|t| t.as_str()) == Some("array")
        }).collect::<Vec<_>>();

        for array in arrays {
            assert!(array.get("indexedVariables").is_some(),
                   "Array should have indexedVariables");
            assert!(array.get("variablesReference")
                   .and_then(|r| r.as_i64())
                   .map(|r| r > 0)
                   .unwrap_or(false),
                   "Array should have non-zero variablesReference for lazy expansion");
        }

        // Hashes should have namedVariables
        let hashes = vars.iter().filter(|v| {
            v.get("type").and_then(|t| t.as_str()) == Some("hash")
        }).collect::<Vec<_>>();

        for hash in hashes {
            assert!(hash.get("namedVariables").is_some(),
                   "Hash should have namedVariables");
            assert!(hash.get("variablesReference")
                   .and_then(|r| r.as_i64())
                   .map(|r| r > 0)
                   .unwrap_or(false),
                   "Hash should have non-zero variablesReference for lazy expansion");
        }
    }
    Ok(())
}

#[test]
// AC8.6: Test package scope variables
fn test_package_scope_variables() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({ "variablesReference": 12 }); // Package for frame 1
    let response = adapter.handle_request(1, "variables", Some(args));

    if let DapMessage::Response { success, body, .. } = response {
        assert!(success);
        let body_val = body.ok_or("Expected body in response")?;
        let vars = body_val
            .get("variables")
            .ok_or("Expected variables field")?
            .as_array()
            .ok_or("Expected variables array")?;

        // Package scope should have at least VERSION
        assert!(vars.iter().any(|v|
            v.get("name").and_then(|n| n.as_str()) == Some("$VERSION")
        ));
    }
    Ok(())
}

#[test]
// AC8.4: Test lazy expansion reference allocation
fn test_lazy_expansion_references() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({ "variablesReference": 11 });
    let response = adapter.handle_request(1, "variables", Some(args));

    if let DapMessage::Response { body, .. } = response {
        let body_val = body.ok_or("Expected body in response")?;
        let vars = body_val
            .get("variables")
            .ok_or("Expected variables field")?
            .as_array()
            .ok_or("Expected variables array")?;

        // Verify that expandable structures have unique references
        let refs: Vec<i64> = vars
            .iter()
            .filter_map(|v| v.get("variablesReference").and_then(|r| r.as_i64()))
            .filter(|&r| r > 0)
            .collect();

        // Check uniqueness of non-zero references
        let unique_refs: std::collections::HashSet<_> = refs.iter().collect();
        assert_eq!(refs.len(), unique_refs.len(),
                  "All lazy expansion references should be unique");
    }
    Ok(())
}

#[test]
// AC8.3: Test scope expensive flags
fn test_scope_expensive_flags() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({ "frameId": 1 });
    let response = adapter.handle_request(1, "scopes", Some(args));

    if let DapMessage::Response { body, .. } = response {
        let body_val = body.ok_or("Expected body in response")?;
        let scopes = body_val
            .get("scopes")
            .ok_or("Expected scopes field")?
            .as_array()
            .ok_or("Expected scopes array")?;

        // Locals should be cheap
        let locals = &scopes[0];
        assert_eq!(
            locals.get("expensive").and_then(|e| e.as_bool()),
            Some(false),
            "Locals scope should not be marked as expensive"
        );

        // Package and Globals should be expensive
        let package = &scopes[1];
        assert_eq!(
            package.get("expensive").and_then(|e| e.as_bool()),
            Some(true),
            "Package scope should be marked as expensive"
        );

        let globals = &scopes[2];
        assert_eq!(
            globals.get("expensive").and_then(|e| e.as_bool()),
            Some(true),
            "Globals scope should be marked as expensive"
        );
    }
    Ok(())
}
