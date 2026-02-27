//! LSP protocol mock data for server capabilities and response testing
//!
//! Provides comprehensive JSON-RPC test data for LSP executeCommand functionality
//! including server capabilities, request/response patterns, and error scenarios.
//!
//! Features:
//! - Complete LSP initialization sequences with executeCommand support
//! - perl.runCritic request/response patterns with dual analyzer strategy
//! - Enhanced code action responses with refactoring operations
//! - Diagnostic integration patterns with JSON-RPC compliance
//! - Performance validation with <50ms response time testing
//! - Thread-safe mock responses for adaptive threading configuration

use serde_json::{json, Value};
use std::collections::HashMap;

#[cfg(test)]
pub struct LspProtocolFixture {
    pub name: &'static str,
    pub request_method: &'static str,
    pub request_params: Value,
    pub expected_response: Value,
    pub response_time_ms: Option<u64>,
    pub thread_safe: bool,
    pub navigation_type: NavigationType,
    pub protocol_version: &'static str,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationType {
    Initialize,
    ExecuteCommand,
    CodeAction,
    Completion,
    Hover,
    Definition,
    References,
    WorkspaceSymbol,
}

/// Complete LSP server initialization with executeCommand capabilities
#[cfg(test)]
pub fn server_initialization_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "server_initialization_complete",
        request_method: "initialize",
        request_params: json!({
            "processId": 12345,
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "workspaceFolders": [{
                "uri": "file:///test/workspace",
                "name": "test-workspace"
            }],
            "capabilities": {
                "workspace": {
                    "executeCommand": {
                        "dynamicRegistration": true
                    },
                    "workspaceEdit": {
                        "documentChanges": true
                    }
                },
                "textDocument": {
                    "codeAction": {
                        "dynamicRegistration": true,
                        "codeActionLiteralSupport": {
                            "codeActionKind": {
                                "valueSet": [
                                    "quickfix",
                                    "refactor",
                                    "refactor.extract",
                                    "refactor.inline",
                                    "refactor.rewrite",
                                    "source",
                                    "source.organizeImports"
                                ]
                            }
                        },
                        "resolveSupport": {
                            "properties": ["edit"]
                        }
                    }
                }
            }
        }),
        expected_response: json!({
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
                "codeActionProvider": {
                    "codeActionKinds": [
                        "quickfix",
                        "refactor.extract",
                        "refactor.rewrite",
                        "source.organizeImports"
                    ],
                    "resolveProvider": true
                },
                "executeCommandProvider": {
                    "commands": [
                        "perl.runTests",
                        "perl.runFile",
                        "perl.runTestSub",
                        "perl.debugTests",
                        "perl.runCritic"
                    ]
                },
                "semanticTokensProvider": {
                    "legend": {
                        "tokenTypes": [
                            "keyword", "string", "number", "comment", "variable",
                            "function", "class", "operator", "property", "parameter"
                        ],
                        "tokenModifiers": ["declaration", "definition", "readonly"]
                    },
                    "range": true,
                    "full": true
                }
            },
            "serverInfo": {
                "name": "perl-lsp",
                "version": "0.8.9"
            }
        }),
        response_time_ms: Some(100),
        thread_safe: true,
        navigation_type: NavigationType::Initialize,
        protocol_version: "3.17",
    }
}

/// perl.runCritic executeCommand with external perlcritic success
#[cfg(test)]
pub fn perl_run_critic_external_success_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "perl_run_critic_external_success",
        request_method: "workspace/executeCommand",
        request_params: json!({
            "command": "perl.runCritic",
            "arguments": ["file:///test/workspace/violations.pl"]
        }),
        expected_response: json!({
            "status": "success",
            "analyzerUsed": "external_perlcritic",
            "executionTime": 1250,
            "violations": [
                {
                    "policy": "Perl::Critic::Policy::TestingAndDebugging::RequireUseStrict",
                    "severity": 5,
                    "message": "Code before strictures are enabled",
                    "line": 3,
                    "column": 1,
                    "source": "perlcritic"
                },
                {
                    "policy": "Perl::Critic::Policy::TestingAndDebugging::RequireUseWarnings",
                    "severity": 4,
                    "message": "Code before warnings are enabled",
                    "line": 3,
                    "column": 1,
                    "source": "perlcritic"
                },
                {
                    "policy": "Perl::Critic::Policy::InputOutput::RequireBriefOpen",
                    "severity": 4,
                    "message": "Close filehandles as soon as possible after opening them",
                    "line": 12,
                    "column": 1,
                    "source": "perlcritic"
                },
                {
                    "policy": "Perl::Critic::Policy::InputOutput::RequireThreeArgOpen",
                    "severity": 5,
                    "message": "Always use the three-argument form of open",
                    "line": 12,
                    "column": 1,
                    "source": "perlcritic"
                }
            ],
            "summary": {
                "totalViolations": 4,
                "severityBreakdown": {
                    "5": 2,
                    "4": 2,
                    "3": 0,
                    "2": 0,
                    "1": 0
                }
            }
        }),
        response_time_ms: Some(1250),
        thread_safe: true,
        navigation_type: NavigationType::ExecuteCommand,
        protocol_version: "3.17",
    }
}

/// perl.runCritic executeCommand with built-in analyzer fallback
#[cfg(test)]
pub fn perl_run_critic_builtin_fallback_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "perl_run_critic_builtin_fallback",
        request_method: "workspace/executeCommand",
        request_params: json!({
            "command": "perl.runCritic",
            "arguments": ["file:///test/workspace/good_practices.pl"]
        }),
        expected_response: json!({
            "status": "success",
            "analyzerUsed": "builtin_analyzer",
            "executionTime": 180,
            "violations": [
                {
                    "policy": "builtin.unused_variables",
                    "severity": 3,
                    "message": "Variable '$temp' is declared but not used",
                    "line": 15,
                    "column": 8,
                    "source": "perl-lsp-builtin"
                }
            ],
            "summary": {
                "totalViolations": 1,
                "severityBreakdown": {
                    "5": 0,
                    "4": 0,
                    "3": 1,
                    "2": 0,
                    "1": 0
                }
            },
            "fallbackReason": "external_perlcritic_unavailable"
        }),
        response_time_ms: Some(180),
        thread_safe: true,
        navigation_type: NavigationType::ExecuteCommand,
        protocol_version: "3.17",
    }
}

/// perl.runCritic executeCommand error handling
#[cfg(test)]
pub fn perl_run_critic_error_handling_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "perl_run_critic_error_handling",
        request_method: "workspace/executeCommand",
        request_params: json!({
            "command": "perl.runCritic",
            "arguments": ["file:///nonexistent/file.pl"]
        }),
        expected_response: json!({
            "status": "error",
            "error": {
                "code": -32602,
                "message": "File not found or not accessible",
                "data": {
                    "uri": "file:///nonexistent/file.pl",
                    "reason": "file_not_found"
                }
            },
            "analyzerUsed": "none",
            "executionTime": 25
        }),
        response_time_ms: Some(25),
        thread_safe: true,
        navigation_type: NavigationType::ExecuteCommand,
        protocol_version: "3.17",
    }
}

/// Enhanced code action response with extract variable refactoring
#[cfg(test)]
pub fn code_action_extract_variable_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "code_action_extract_variable",
        request_method: "textDocument/codeAction",
        request_params: json!({
            "textDocument": {
                "uri": "file:///test/workspace/refactoring.pl"
            },
            "range": {
                "start": { "line": 7, "character": 17 },
                "end": { "line": 7, "character": 70 }
            },
            "context": {
                "diagnostics": [],
                "only": ["refactor.extract"]
            }
        }),
        expected_response: json!([
            {
                "title": "Extract variable",
                "kind": "refactor.extract",
                "edit": {
                    "changes": {
                        "file:///test/workspace/refactoring.pl": [
                            {
                                "range": {
                                    "start": { "line": 7, "character": 4 },
                                    "end": { "line": 7, "character": 4 }
                                },
                                "newText": "    my $extracted_value = length($input) + substr($input, 0, 5) eq \"hello\" ? 10 : 0;\n"
                            },
                            {
                                "range": {
                                    "start": { "line": 7, "character": 17 },
                                    "end": { "line": 7, "character": 70 }
                                },
                                "newText": "$extracted_value"
                            }
                        ]
                    }
                },
                "data": {
                    "refactoring_type": "extract_variable",
                    "variable_name": "extracted_value"
                }
            }
        ]),
        response_time_ms: Some(45),
        thread_safe: true,
        navigation_type: NavigationType::CodeAction,
        protocol_version: "3.17",
    }
}

/// Import organization code action response
#[cfg(test)]
pub fn code_action_organize_imports_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "code_action_organize_imports",
        request_method: "textDocument/codeAction",
        request_params: json!({
            "textDocument": {
                "uri": "file:///test/workspace/imports.pl"
            },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 20, "character": 0 }
            },
            "context": {
                "diagnostics": [],
                "only": ["source.organizeImports"]
            }
        }),
        expected_response: json!([
            {
                "title": "Organize imports",
                "kind": "source.organizeImports",
                "edit": {
                    "changes": {
                        "file:///test/workspace/imports.pl": [
                            {
                                "range": {
                                    "start": { "line": 5, "character": 0 },
                                    "end": { "line": 18, "character": 0 }
                                },
                                "newText": "# Organized imports (alphabetical, duplicates removed)\nuse Carp;\nuse File::Path qw(make_path);\nuse My::Custom::Module;\nuse POSIX qw(strftime);\nuse Scalar::Util qw(blessed);\n\n"
                            }
                        ]
                    }
                },
                "data": {
                    "refactoring_type": "organize_imports",
                    "removed_duplicates": ["File::Path"],
                    "removed_unused": ["Data::Dumper", "List::Util"]
                }
            }
        ]),
        response_time_ms: Some(38),
        thread_safe: true,
        navigation_type: NavigationType::CodeAction,
        protocol_version: "3.17",
    }
}

/// Cross-file definition response with dual indexing
#[cfg(test)]
pub fn definition_cross_file_dual_indexing_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "definition_cross_file_dual_indexing",
        request_method: "textDocument/definition",
        request_params: json!({
            "textDocument": {
                "uri": "file:///test/workspace/main.pl"
            },
            "position": { "line": 12, "character": 15 }
        }),
        expected_response: json!([
            {
                "uri": "file:///test/workspace/MyModule/Utils.pm",
                "range": {
                    "start": { "line": 8, "character": 0 },
                    "end": { "line": 8, "character": 32 }
                }
            },
            {
                "uri": "file:///test/workspace/MyModule/Utils.pm",
                "range": {
                    "start": { "line": 15, "character": 0 },
                    "end": { "line": 15, "character": 15 }
                }
            }
        ]),
        response_time_ms: Some(125),
        thread_safe: true,
        navigation_type: NavigationType::Definition,
        protocol_version: "3.17",
    }
}

/// Workspace symbols response with enhanced navigation
#[cfg(test)]
pub fn workspace_symbols_enhanced_navigation_fixture() -> LspProtocolFixture {
    LspProtocolFixture {
        name: "workspace_symbols_enhanced_navigation",
        request_method: "workspace/symbol",
        request_params: json!({
            "query": "process"
        }),
        expected_response: json!([
            {
                "name": "process_data",
                "kind": 12, // Function
                "location": {
                    "uri": "file:///test/workspace/MyModule/Utils.pm",
                    "range": {
                        "start": { "line": 8, "character": 0 },
                        "end": { "line": 8, "character": 32 }
                    }
                },
                "containerName": "MyModule::Utils"
            },
            {
                "name": "MyModule::Utils::process_data",
                "kind": 12, // Function
                "location": {
                    "uri": "file:///test/workspace/MyModule/Utils.pm",
                    "range": {
                        "start": { "line": 8, "character": 0 },
                        "end": { "line": 8, "character": 32 }
                    }
                },
                "containerName": "MyModule::Utils"
            },
            {
                "name": "process_file",
                "kind": 12, // Function
                "location": {
                    "uri": "file:///test/workspace/file_utils.pl",
                    "range": {
                        "start": { "line": 25, "character": 0 },
                        "end": { "line": 25, "character": 21 }
                    }
                }
            }
        ]),
        response_time_ms: Some(85),
        thread_safe: true,
        navigation_type: NavigationType::WorkspaceSymbol,
        protocol_version: "3.17",
    }
}

/// Performance validation fixtures with <50ms response requirement
#[cfg(test)]
pub fn load_performance_validation_fixtures() -> Vec<LspProtocolFixture> {
    vec![
        // Fast completion response
        LspProtocolFixture {
            name: "completion_performance_validation",
            request_method: "textDocument/completion",
            request_params: json!({
                "textDocument": {
                    "uri": "file:///test/workspace/large_file.pl"
                },
                "position": { "line": 50, "character": 10 }
            }),
            expected_response: json!({
                "items": [
                    {
                        "label": "$variable",
                        "kind": 6, // Variable
                        "detail": "scalar variable",
                        "insertText": "$variable"
                    },
                    {
                        "label": "validate_input",
                        "kind": 3, // Function
                        "detail": "subroutine",
                        "insertText": "validate_input"
                    }
                ]
            }),
            response_time_ms: Some(25),
            thread_safe: true,
            navigation_type: NavigationType::Completion,
            protocol_version: "3.17",
        },

        // Fast hover response
        LspProtocolFixture {
            name: "hover_performance_validation",
            request_method: "textDocument/hover",
            request_params: json!({
                "textDocument": {
                    "uri": "file:///test/workspace/large_file.pl"
                },
                "position": { "line": 35, "character": 8 }
            }),
            expected_response: json!({
                "contents": {
                    "kind": "markdown",
                    "value": "**subroutine** `process_data`\n\n```perl\nsub process_data {\n    my ($input) = @_;\n    # ... implementation\n}\n```"
                },
                "range": {
                    "start": { "line": 35, "character": 4 },
                    "end": { "line": 35, "character": 16 }
                }
            }),
            response_time_ms: Some(18),
            thread_safe: true,
            navigation_type: NavigationType::Hover,
            protocol_version: "3.17",
        },
    ]
}

/// Error response fixtures for comprehensive error handling testing
#[cfg(test)]
pub fn load_error_response_fixtures() -> Vec<LspProtocolFixture> {
    vec![
        // Invalid command error
        LspProtocolFixture {
            name: "invalid_command_error",
            request_method: "workspace/executeCommand",
            request_params: json!({
                "command": "perl.invalidCommand",
                "arguments": []
            }),
            expected_response: json!({
                "error": {
                    "code": -32601,
                    "message": "Unknown command: perl.invalidCommand",
                    "data": {
                        "supportedCommands": [
                            "perl.runTests",
                            "perl.runFile",
                            "perl.runTestSub",
                            "perl.debugTests",
                            "perl.runCritic"
                        ]
                    }
                }
            }),
            response_time_ms: Some(5),
            thread_safe: true,
            navigation_type: NavigationType::ExecuteCommand,
            protocol_version: "3.17",
        },

        // File not found error
        LspProtocolFixture {
            name: "file_not_found_error",
            request_method: "textDocument/definition",
            request_params: json!({
                "textDocument": {
                    "uri": "file:///nonexistent/file.pl"
                },
                "position": { "line": 0, "character": 0 }
            }),
            expected_response: json!({
                "error": {
                    "code": -32002,
                    "message": "Document not found",
                    "data": {
                        "uri": "file:///nonexistent/file.pl"
                    }
                }
            }),
            response_time_ms: Some(8),
            thread_safe: true,
            navigation_type: NavigationType::Definition,
            protocol_version: "3.17",
        },
    ]
}

use std::sync::LazyLock;

/// Comprehensive LSP protocol fixture registry
#[cfg(test)]
pub static PROTOCOL_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, LspProtocolFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        // Core fixtures
        let core_fixtures = vec![
            server_initialization_fixture(),
            perl_run_critic_external_success_fixture(),
            perl_run_critic_builtin_fallback_fixture(),
            perl_run_critic_error_handling_fixture(),
            code_action_extract_variable_fixture(),
            code_action_organize_imports_fixture(),
            definition_cross_file_dual_indexing_fixture(),
            workspace_symbols_enhanced_navigation_fixture(),
        ];

        for fixture in core_fixtures {
            registry.insert(fixture.name, fixture);
        }

        // Performance fixtures
        for fixture in load_performance_validation_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        // Error fixtures
        for fixture in load_error_response_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get fixture by name with efficient lookup
#[cfg(test)]
pub fn get_protocol_fixture_by_name(name: &str) -> Option<&'static LspProtocolFixture> {
    PROTOCOL_FIXTURE_REGISTRY.get(name)
}

/// Get fixtures by navigation type
#[cfg(test)]
pub fn get_fixtures_by_navigation_type(nav_type: NavigationType) -> Vec<&'static LspProtocolFixture> {
    PROTOCOL_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.navigation_type == nav_type)
        .collect()
}

/// Get performance validation fixtures (response time <= 50ms)
#[cfg(test)]
pub fn get_performance_fixtures() -> Vec<&'static LspProtocolFixture> {
    PROTOCOL_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| {
            fixture.response_time_ms.map_or(false, |time| time <= 50)
        })
        .collect()
}

/// Get thread-safe fixtures for adaptive threading tests
#[cfg(test)]
pub fn get_thread_safe_fixtures() -> Vec<&'static LspProtocolFixture> {
    PROTOCOL_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.thread_safe)
        .collect()
}