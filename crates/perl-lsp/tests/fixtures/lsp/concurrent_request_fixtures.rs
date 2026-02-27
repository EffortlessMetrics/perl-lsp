//! LSP concurrent request management test fixtures
//!
//! Provides comprehensive JSON-RPC test data for concurrent request handling,
//! enhanced error handling scenarios, and request correlation testing.
//!
//! Features:
//! - Concurrent request patterns with request correlation
//! - Enhanced error handling with structured responses
//! - Request timeout and cancellation scenarios
//! - Performance metrics collection test data
//! - Thread-safe request management validation

use serde_json::{json, Value};
use std::collections::HashMap;

#[cfg(test)]
pub struct ConcurrentRequestFixture {
    pub name: &'static str,
    pub request_batch: Vec<RequestBatchItem>,
    pub expected_correlation: RequestCorrelation,
    pub expected_responses: Vec<Value>,
    pub total_time_ms: Option<u64>,
    pub thread_safety: bool,
    pub error_scenarios: Vec<ErrorScenario>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct RequestBatchItem {
    pub id: Value,
    pub method: String,
    pub params: Value,
    pub priority: RequestPriority,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum RequestPriority {
    High,    // Interactive requests (completion, hover)
    Medium,  // Navigation requests (definition, references)
    Low,     // Background requests (symbols, diagnostics)
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct RequestCorrelation {
    pub correlation_id: String,
    pub request_count: usize,
    pub expected_order: Vec<Value>,
    pub allow_reordering: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorScenario {
    RequestTimeout,
    InvalidRequest,
    ServerOverload,
    CancellationRequested,
    ResourceUnavailable,
}

/// Concurrent LSP request scenarios for correlation testing
#[cfg(test)]
pub fn load_concurrent_request_fixtures() -> Vec<ConcurrentRequestFixture> {
    vec![
        // Basic concurrent request scenario
        ConcurrentRequestFixture {
            name: "basic_concurrent_requests",
            request_batch: vec![
                RequestBatchItem {
                    id: json!(1),
                    method: "textDocument/documentSymbol".to_string(),
                    params: json!({
                        "textDocument": { "uri": "file:///test/concurrent_1.pl" }
                    }),
                    priority: RequestPriority::Medium,
                },
                RequestBatchItem {
                    id: json!(2),
                    method: "textDocument/hover".to_string(),
                    params: json!({
                        "textDocument": { "uri": "file:///test/concurrent_2.pl" },
                        "position": { "line": 10, "character": 5 }
                    }),
                    priority: RequestPriority::High,
                },
                RequestBatchItem {
                    id: json!(3),
                    method: "textDocument/completion".to_string(),
                    params: json!({
                        "textDocument": { "uri": "file:///test/concurrent_3.pl" },
                        "position": { "line": 15, "character": 8 }
                    }),
                    priority: RequestPriority::High,
                },
            ],
            expected_correlation: RequestCorrelation {
                correlation_id: "batch_001".to_string(),
                request_count: 3,
                expected_order: vec![json!(2), json!(3), json!(1)], // High priority first
                allow_reordering: true,
            },
            expected_responses: vec![
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": []
                }),
                json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "result": {
                        "contents": "Hover information",
                        "range": {
                            "start": { "line": 10, "character": 5 },
                            "end": { "line": 10, "character": 15 }
                        }
                    }
                }),
                json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "result": {
                        "items": [
                            {
                                "label": "$variable",
                                "kind": 6,
                                "insertText": "$variable"
                            }
                        ]
                    }
                }),
            ],
            total_time_ms: Some(450),
            thread_safety: true,
            error_scenarios: vec![],
        },

        // High-load concurrent request scenario
        ConcurrentRequestFixture {
            name: "high_load_concurrent_requests",
            request_batch: (1..=20).map(|i| {
                RequestBatchItem {
                    id: json!(i),
                    method: if i % 4 == 0 {
                        "textDocument/documentSymbol"
                    } else if i % 3 == 0 {
                        "textDocument/references"
                    } else if i % 2 == 0 {
                        "textDocument/hover"
                    } else {
                        "textDocument/completion"
                    }.to_string(),
                    params: json!({
                        "textDocument": { "uri": format!("file:///test/load_test_{}.pl", i) },
                        "position": { "line": i % 50, "character": (i * 3) % 80 }
                    }),
                    priority: match i % 3 {
                        0 => RequestPriority::Low,
                        1 => RequestPriority::Medium,
                        _ => RequestPriority::High,
                    },
                }
            }).collect(),
            expected_correlation: RequestCorrelation {
                correlation_id: "high_load_batch".to_string(),
                request_count: 20,
                expected_order: (1..=20).map(|i| json!(i)).collect(),
                allow_reordering: true,
            },
            expected_responses: (1..=20).map(|i| {
                json!({
                    "jsonrpc": "2.0",
                    "id": i,
                    "result": {}
                })
            }).collect(),
            total_time_ms: Some(2000),
            thread_safety: true,
            error_scenarios: vec![],
        },

        // Error handling scenarios
        ConcurrentRequestFixture {
            name: "error_handling_concurrent_requests",
            request_batch: vec![
                RequestBatchItem {
                    id: json!("error_1"),
                    method: "textDocument/definition".to_string(),
                    params: json!({
                        "textDocument": { "uri": "file:///nonexistent/file.pl" },
                        "position": { "line": 0, "character": 0 }
                    }),
                    priority: RequestPriority::Medium,
                },
                RequestBatchItem {
                    id: json!("error_2"),
                    method: "invalid/method".to_string(),
                    params: json!({}),
                    priority: RequestPriority::High,
                },
                RequestBatchItem {
                    id: json!("success_1"),
                    method: "textDocument/hover".to_string(),
                    params: json!({
                        "textDocument": { "uri": "file:///test/valid.pl" },
                        "position": { "line": 5, "character": 10 }
                    }),
                    priority: RequestPriority::High,
                },
            ],
            expected_correlation: RequestCorrelation {
                correlation_id: "error_batch".to_string(),
                request_count: 3,
                expected_order: vec![json!("error_2"), json!("success_1"), json!("error_1")],
                allow_reordering: true,
            },
            expected_responses: vec![
                json!({
                    "jsonrpc": "2.0",
                    "id": "error_1",
                    "error": {
                        "code": -32002,
                        "message": "Document not found",
                        "data": {
                            "uri": "file:///nonexistent/file.pl"
                        }
                    }
                }),
                json!({
                    "jsonrpc": "2.0",
                    "id": "error_2",
                    "error": {
                        "code": -32601,
                        "message": "Method not found: invalid/method"
                    }
                }),
                json!({
                    "jsonrpc": "2.0",
                    "id": "success_1",
                    "result": {
                        "contents": "Valid hover response"
                    }
                }),
            ],
            total_time_ms: Some(250),
            thread_safety: true,
            error_scenarios: vec![
                ErrorScenario::InvalidRequest,
                ErrorScenario::ResourceUnavailable,
            ],
        },

        // Cancellation scenario
        ConcurrentRequestFixture {
            name: "request_cancellation_scenario",
            request_batch: vec![
                RequestBatchItem {
                    id: json!("long_running"),
                    method: "textDocument/documentSymbol".to_string(),
                    params: json!({
                        "textDocument": { "uri": "file:///test/large_file.pl" }
                    }),
                    priority: RequestPriority::Low,
                },
                RequestBatchItem {
                    id: json!("cancel_request"),
                    method: "$/cancelRequest".to_string(),
                    params: json!({
                        "id": "long_running"
                    }),
                    priority: RequestPriority::High,
                },
                RequestBatchItem {
                    id: json!("after_cancel"),
                    method: "textDocument/hover".to_string(),
                    params: json!({
                        "textDocument": { "uri": "file:///test/small_file.pl" },
                        "position": { "line": 1, "character": 1 }
                    }),
                    priority: RequestPriority::High,
                },
            ],
            expected_correlation: RequestCorrelation {
                correlation_id: "cancellation_batch".to_string(),
                request_count: 3,
                expected_order: vec![json!("cancel_request"), json!("after_cancel"), json!("long_running")],
                allow_reordering: true,
            },
            expected_responses: vec![
                json!({
                    "jsonrpc": "2.0",
                    "id": "long_running",
                    "error": {
                        "code": -32800,
                        "message": "Request cancelled",
                        "data": {
                            "reason": "client_cancellation"
                        }
                    }
                }),
                // No response for $/cancelRequest (notification)
                json!({
                    "jsonrpc": "2.0",
                    "id": "after_cancel",
                    "result": {
                        "contents": "Post-cancellation hover"
                    }
                }),
            ],
            total_time_ms: Some(100),
            thread_safety: true,
            error_scenarios: vec![ErrorScenario::CancellationRequested],
        },
    ]
}

/// Enhanced error response patterns for comprehensive error handling
#[cfg(test)]
pub fn load_enhanced_error_fixtures() -> Vec<Value> {
    vec![
        // Request timeout error
        json!({
            "jsonrpc": "2.0",
            "id": "timeout_request",
            "error": {
                "code": -32000,
                "message": "Request timeout",
                "data": {
                    "timeout_ms": 5000,
                    "request_method": "textDocument/documentSymbol",
                    "partial_result": null
                }
            }
        }),

        // Server overload error
        json!({
            "jsonrpc": "2.0",
            "id": "overload_request",
            "error": {
                "code": -32000,
                "message": "Server temporarily overloaded",
                "data": {
                    "active_requests": 50,
                    "max_concurrent": 20,
                    "retry_after_ms": 1000
                }
            }
        }),

        // Malformed parameters error
        json!({
            "jsonrpc": "2.0",
            "id": "malformed_params",
            "error": {
                "code": -32602,
                "message": "Invalid parameters",
                "data": {
                    "expected": {
                        "textDocument": "required",
                        "position": "required"
                    },
                    "received": {
                        "textDocument": "missing"
                    }
                }
            }
        }),

        // Parse error in request
        json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32700,
                "message": "Parse error",
                "data": {
                    "line": 1,
                    "column": 45,
                    "context": "Invalid JSON syntax"
                }
            }
        }),

        // Internal server error with diagnostic info
        json!({
            "jsonrpc": "2.0",
            "id": "internal_error",
            "error": {
                "code": -32603,
                "message": "Internal error",
                "data": {
                    "error_type": "parser_crash",
                    "file_uri": "file:///test/problematic.pl",
                    "stack_trace": [
                        "parser::parse_expression at line 245",
                        "parser::parse_statement at line 123",
                        "lsp::handle_hover at line 67"
                    ],
                    "recovery_action": "restart_parser"
                }
            }
        }),
    ]
}

/// Performance metrics collection test data
#[cfg(test)]
pub fn load_performance_metrics_fixtures() -> Vec<Value> {
    vec![
        // Fast request metrics
        json!({
            "request_id": "fast_completion",
            "method": "textDocument/completion",
            "start_time": 1634567890123_u64,
            "end_time": 1634567890145_u64,
            "duration_ms": 22,
            "memory_usage_kb": 1024,
            "thread_id": "worker_1",
            "cache_hit": true,
            "result_size": 15
        }),

        // Medium duration request metrics
        json!({
            "request_id": "medium_symbols",
            "method": "textDocument/documentSymbol",
            "start_time": 1634567890200_u64,
            "end_time": 1634567890350_u64,
            "duration_ms": 150,
            "memory_usage_kb": 2048,
            "thread_id": "worker_2",
            "cache_hit": false,
            "result_size": 156
        }),

        // Slow request metrics
        json!({
            "request_id": "slow_references",
            "method": "textDocument/references",
            "start_time": 1634567890400_u64,
            "end_time": 1634567891200_u64,
            "duration_ms": 800,
            "memory_usage_kb": 4096,
            "thread_id": "worker_3",
            "cache_hit": false,
            "result_size": 324,
            "workspace_scan": true,
            "files_scanned": 45
        }),

        // Cancelled request metrics
        json!({
            "request_id": "cancelled_operation",
            "method": "workspace/symbol",
            "start_time": 1634567891300_u64,
            "end_time": 1634567891450_u64,
            "duration_ms": 150,
            "memory_usage_kb": 512,
            "thread_id": "worker_4",
            "cache_hit": false,
            "result_size": 0,
            "cancelled": true,
            "cancellation_time_ms": 150
        }),

        // Error request metrics
        json!({
            "request_id": "error_request",
            "method": "textDocument/definition",
            "start_time": 1634567891500_u64,
            "end_time": 1634567891505_u64,
            "duration_ms": 5,
            "memory_usage_kb": 128,
            "thread_id": "worker_1",
            "cache_hit": false,
            "result_size": 0,
            "error_code": -32002,
            "error_message": "Document not found"
        }),
    ]
}

/// Mock LSP server responses for infrastructure testing
#[cfg(test)]
pub fn load_mock_infrastructure_responses() -> HashMap<String, Value> {
    let mut responses = HashMap::new();

    // Initialize response
    responses.insert("initialize".to_string(), json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "capabilities": {
                "textDocumentSync": 2,
                "hoverProvider": true,
                "completionProvider": {
                    "triggerCharacters": ["$", "@", "%", ":", ">"]
                },
                "definitionProvider": true,
                "referencesProvider": true,
                "documentSymbolProvider": true,
                "workspaceSymbolProvider": true,
                "executeCommandProvider": {
                    "commands": ["perl.runCritic", "perl.runTests"]
                }
            },
            "serverInfo": {
                "name": "perl-lsp-mock",
                "version": "0.8.9-test"
            }
        }
    }));

    // Document symbols response
    responses.insert("textDocument/documentSymbol".to_string(), json!({
        "jsonrpc": "2.0",
        "id": "symbols_request",
        "result": [
            {
                "name": "test_function",
                "kind": 12,
                "range": {
                    "start": { "line": 5, "character": 0 },
                    "end": { "line": 10, "character": 1 }
                },
                "selectionRange": {
                    "start": { "line": 5, "character": 4 },
                    "end": { "line": 5, "character": 17 }
                }
            }
        ]
    }));

    // Hover response
    responses.insert("textDocument/hover".to_string(), json!({
        "jsonrpc": "2.0",
        "id": "hover_request",
        "result": {
            "contents": {
                "kind": "markdown",
                "value": "**subroutine** `test_function`"
            },
            "range": {
                "start": { "line": 5, "character": 4 },
                "end": { "line": 5, "character": 17 }
            }
        }
    }));

    // Completion response
    responses.insert("textDocument/completion".to_string(), json!({
        "jsonrpc": "2.0",
        "id": "completion_request",
        "result": {
            "items": [
                {
                    "label": "$test_var",
                    "kind": 6,
                    "detail": "scalar variable",
                    "insertText": "$test_var"
                },
                {
                    "label": "test_function",
                    "kind": 3,
                    "detail": "subroutine",
                    "insertText": "test_function"
                }
            ]
        }
    }));

    responses
}

use std::sync::LazyLock;

/// Lazy-loaded concurrent request fixture registry
#[cfg(test)]
pub static CONCURRENT_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, ConcurrentRequestFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_concurrent_request_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get concurrent fixture by name
#[cfg(test)]
pub fn get_concurrent_fixture_by_name(name: &str) -> Option<&'static ConcurrentRequestFixture> {
    CONCURRENT_FIXTURE_REGISTRY.get(name)
}

/// Get fixtures by error scenario type
#[cfg(test)]
pub fn get_fixtures_by_error_scenario(scenario: ErrorScenario) -> Vec<&'static ConcurrentRequestFixture> {
    CONCURRENT_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.error_scenarios.contains(&scenario))
        .collect()
}

/// Get thread-safe concurrent fixtures
#[cfg(test)]
pub fn get_thread_safe_concurrent_fixtures() -> Vec<&'static ConcurrentRequestFixture> {
    CONCURRENT_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.thread_safety)
        .collect()
}