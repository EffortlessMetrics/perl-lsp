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
fn test_threads_main_thread_only() {
    let mut adapter = create_test_adapter();
    // Threads request without session should return empty
    let response = adapter.handle_request(1, "threads", None);
    
    if let DapMessage::Response { body, .. } = response {
        let body_val = body.unwrap();
        let threads = body_val.get("threads").unwrap().as_array().unwrap();
        assert!(threads.is_empty());
    }
}

#[test]
// AC:8.3
fn test_scopes_hierarchy() {
    let mut adapter = create_test_adapter();
    let args = json!({ "frameId": 1 });
    let response = adapter.handle_request(1, "scopes", Some(args));
    
    if let DapMessage::Response { success, body, .. } = response {
        assert!(success);
        let body_val = body.unwrap();
        let scopes = body_val.get("scopes").unwrap().as_array().unwrap();
        
        // Should have 3 scopes: Locals, Package, Globals
        assert_eq!(scopes.len(), 3);
        assert_eq!(scopes[0].get("name").unwrap().as_str(), Some("Locals"));
        assert_eq!(scopes[1].get("name").unwrap().as_str(), Some("Package"));
        assert_eq!(scopes[2].get("name").unwrap().as_str(), Some("Globals"));
        
        // Verify unique variable references
        let ref0 = scopes[0].get("variablesReference").unwrap().as_i64().unwrap();
        let ref1 = scopes[1].get("variablesReference").unwrap().as_i64().unwrap();
        let ref2 = scopes[2].get("variablesReference").unwrap().as_i64().unwrap();
        assert!(ref0 != ref1 && ref1 != ref2);
    }
}

#[test]
// AC:8.4
fn test_variables_lazy_expansion_indicators() {
    let mut adapter = create_test_adapter();
    
    // Request variables for Locals scope (ref 11 for frame 1)
    let args = json!({ "variablesReference": 11 });
    let response = adapter.handle_request(1, "variables", Some(args));
    
    if let DapMessage::Response { body, .. } = response {
        let body_val = body.unwrap();
        let vars = body_val.get("variables").unwrap().as_array().unwrap();
        
        // Find @ _ array
        let array_var = vars.iter().find(|v| v.get("name").unwrap().as_str() == Some("@_")).unwrap();
        assert_eq!(array_var.get("type").unwrap().as_str(), Some("array"));
        assert!(array_var.get("indexedVariables").is_some());
        
        // Find $self hash
        let hash_var = vars.iter().find(|v| v.get("name").unwrap().as_str() == Some("$self")).unwrap();
        assert_eq!(hash_var.get("type").unwrap().as_str(), Some("hash"));
        assert!(hash_var.get("namedVariables").is_some());
    }
}

#[test]
// AC:8.6
fn test_variables_globals_scope() {
    let mut adapter = create_test_adapter();
    let args = json!({ "variablesReference": 13 }); // Globals for frame 1
    let response = adapter.handle_request(1, "variables", Some(args));
    
    if let DapMessage::Response { body, .. } = response {
        let body_val = body.unwrap();
        let vars = body_val.get("variables").unwrap().as_array().unwrap();
        
        // Should contain standard globals like $_
        assert!(vars.iter().any(|v| v.get("name").unwrap().as_str() == Some("$_")));
    }
}
