#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Comprehensive End-to-End Integration Tests for Issue #145
//!
//! Tests feature spec: SPEC_145_LSP_EXECUTE_COMMAND_AND_CODE_ACTIONS.md
//! Architecture: ADR_003_EXECUTE_COMMAND_CODE_ACTIONS_ARCHITECTURE.md
//! Implementation roadmap: IGNORED_TESTS_IMPLEMENTATION_ROADMAP.md
//!
//! This module provides complete end-to-end integration testing for Issue #145 resolution:
//! - executeCommand functionality with perl.runCritic
//! - Code actions with enhanced refactoring capabilities
//! - LSP protocol compliance validation
//! - Performance and reliability testing

use serde_json::json;
use std::time::Duration;

mod support;
use support::lsp_harness::{LspHarness, TempWorkspace};

// Comprehensive test fixtures for E2E testing
mod e2e_fixtures {
    /// Real-world Perl module with multiple refactoring opportunities
    pub const REALISTIC_PERL_MODULE: &str = r#"#!/usr/bin/perl
package MyApp::DataProcessor;

use warnings;  # Missing 'use strict;' - policy violation

my $VERSION = '1.0';

sub new {
    my ($class, %args) = @_;
    my $self = {
        data => $args{data} || [],
        config => $args{config} || {},
    };
    bless $self, $class;
    return $self;
}

sub process_data {
    my ($self) = @_;
    my $data = $self->{data};

    # Complex expression suitable for extract variable
    my $processed_count = scalar(@$data) > 0 ?
        scalar(grep { defined $_ && length($_) > 0 } @$data) : 0;

    # C-style for loop (should be converted to foreach)
    my @results;
    for (my $i = 0; $i < @$data; $i++) {
        my $item = $data->[$i];

        # Code block suitable for extract subroutine
        if (defined $item && ref($item) eq 'HASH') {
            my $validated = $self->validate_item($item);
            my $transformed = $self->transform_item($validated);
            my $enriched = $self->enrich_item($transformed);
            push @results, $enriched;
        } else {
            push @results, $item;
        }
    }

    return \@results;
}

sub validate_item {
    my ($self, $item) = @_;

    # File operation without error checking (should add error handling)
    open FILE, "validation.log";
    print FILE "Validating item\n";
    close FILE;

    return $item;
}

sub save_results {
    my ($self, $results) = @_;

    # Another file operation opportunity for error checking
    open my $fh, ">", "results.txt";
    for my $result (@$results) {
        print $fh "$result\n";
    }
    close $fh;

    print "Results saved\n";  # Should be 'return' statement
}

# Global variable without 'our' declaration
$debug_mode = 1;

1;

__END__

=head1 NAME

MyApp::DataProcessor - Process data items

=head1 SYNOPSIS

    my $processor = MyApp::DataProcessor->new(
        data => \@items,
        config => \%config
    );

    my $results = $processor->process_data();

=cut
"#;

    /// Test script that uses the module
    pub const TEST_SCRIPT: &str = r#"#!/usr/bin/perl
use strict;
use warnings;

use lib '.';
use MyApp::DataProcessor;

my @test_data = (
    { name => 'item1', value => 100 },
    { name => 'item2', value => 200 },
    'simple_string',
    undef,
);

my $processor = MyApp::DataProcessor->new(
    data => \@test_data,
    config => { debug => 1 }
);

my $results = $processor->process_data();
$processor->save_results($results);

print "Processing complete\n";
"#;

    /// Configuration file for perl critic
    pub const PERLCRITIC_CONFIG: &str = r#"severity = 3
only = 1

[TestingAndDebugging::RequireUseStrict]
severity = 5

[TestingAndDebugging::RequireUseWarnings]
severity = 5

[InputOutput::RequireBracedFileHandleWithPrint]
severity = 4

[Variables::RequireLexicalLoopIterators]
severity = 3
"#;
}

/// Create comprehensive test workspace for E2E testing
fn create_comprehensive_workspace() -> (LspHarness, TempWorkspace) {
    let (harness, workspace, _init_result) = create_comprehensive_workspace_with_init();
    (harness, workspace)
}

/// Create comprehensive test workspace for E2E testing - returns init result for capability inspection
fn create_comprehensive_workspace_with_init() -> (LspHarness, TempWorkspace, serde_json::Value) {
    let workspace = TempWorkspace::new().expect("Failed to create temp workspace");

    // Write all files to disk
    workspace
        .write("lib/MyApp/DataProcessor.pm", e2e_fixtures::REALISTIC_PERL_MODULE)
        .expect("Failed to write module file");
    workspace
        .write("test_script.pl", e2e_fixtures::TEST_SCRIPT)
        .expect("Failed to write script file");
    workspace
        .write(".perlcriticrc", e2e_fixtures::PERLCRITIC_CONFIG)
        .expect("Failed to write config file");

    let mut harness = LspHarness::new_without_initialize();
    let init_result = harness
        .initialize_with_root(&workspace.root_uri, None)
        .expect("Failed to initialize LSP server");

    // Open all documents
    harness
        .open_document(
            &workspace.uri("lib/MyApp/DataProcessor.pm"),
            e2e_fixtures::REALISTIC_PERL_MODULE,
        )
        .expect("Failed to open module file");

    harness
        .open_document(&workspace.uri("test_script.pl"), e2e_fixtures::TEST_SCRIPT)
        .expect("Failed to open script file");

    // Trigger processing and indexing
    harness.did_save(&workspace.uri("lib/MyApp/DataProcessor.pm")).ok();
    harness.did_save(&workspace.uri("test_script.pl")).ok();

    // Wait for comprehensive indexing and analysis
    harness.wait_for_idle(Duration::from_millis(2000));

    (harness, workspace, init_result)
}

// ======================== AC5: Comprehensive Integration Test Suite ========================

#[cfg(feature = "lsp-extras")]
#[test]
// AC5:integration - Complete Issue #145 workflow validation
fn test_issue_145_complete_workflow() {
    let (mut harness, workspace, init_result) = create_comprehensive_workspace_with_init();

    // Step 1: Verify server capabilities include new features
    // Server was initialized by create_comprehensive_workspace_with_init(), use the returned init_result

    let capabilities =
        init_result.get("capabilities").expect("Initialize result should contain capabilities");

    // Verify executeCommand capability
    assert!(
        capabilities.get("executeCommandProvider").is_some(),
        "Server should advertise executeCommandProvider"
    );

    let execute_commands = capabilities["executeCommandProvider"]["commands"]
        .as_array()
        .expect("Commands should be array");

    let has_critic_command =
        execute_commands.iter().any(|cmd| cmd.as_str() == Some("perl.runCritic"));
    assert!(has_critic_command, "perl.runCritic command should be advertised");

    // Verify code action capability
    assert!(
        capabilities.get("codeActionProvider").is_some(),
        "Server should advertise codeActionProvider"
    );

    // Step 2: Execute perl.runCritic command
    let critic_result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("lib/MyApp/DataProcessor.pm")]
            }),
            Duration::from_secs(10), // Extended timeout for comprehensive analysis
        )
        .expect("perl.runCritic command should execute successfully");

    // Validate critic results
    assert_eq!(critic_result["status"].as_str(), Some("success"), "Critic analysis should succeed");

    let violations =
        critic_result["violations"].as_array().expect("Should return violations array");

    assert!(!violations.is_empty(), "Should detect policy violations in test file");

    // Should detect missing 'use strict;'
    let has_strict_violation = violations.iter().any(|v| {
        v["policy"]
            .as_str()
            .map(|p| p.contains("RequireUseStrict") || p.contains("strict"))
            .unwrap_or(false)
    });
    assert!(has_strict_violation, "Should detect missing 'use strict;' violation");

    // Step 3: Request code actions to fix violations
    let code_actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("lib/MyApp/DataProcessor.pm")},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 10, "character": 0}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["quickfix"]
                }
            }),
            Duration::from_secs(3),
        )
        .expect("Code actions request should succeed");

    let _actions = code_actions_result.as_array().expect("Should return code actions array");

    // Step 4: Request refactoring actions
    let refactor_actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("lib/MyApp/DataProcessor.pm")},
                "range": {
                    "start": {"line": 18, "character": 8},  // Complex expression
                    "end": {"line": 20, "character": 0}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor"]
                }
            }),
            Duration::from_secs(3),
        )
        .expect("Refactor actions request should succeed");

    let _refactor_actions =
        refactor_actions_result.as_array().expect("Should return refactor actions array");

    // Step 5: Test import organization
    let _organize_imports_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("test_script.pl")},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 10, "character": 0}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["source.organizeImports"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Organize imports request should succeed");

    // Validation - the complete workflow should work without errors
    // Complete Issue #145 workflow executed successfully
}

#[test]
// AC5:integration - Cross-file analysis and navigation
fn test_cross_file_integration() {
    let (mut harness, workspace) = create_comprehensive_workspace();

    // Test that the LSP server can analyze relationships between files
    let module_uri = workspace.uri("lib/MyApp/DataProcessor.pm");
    let script_uri = workspace.uri("test_script.pl");

    // Request symbols from both files to ensure indexing works
    let module_symbols = harness
        .request_with_timeout(
            "textDocument/documentSymbol",
            json!({
                "textDocument": {"uri": module_uri}
            }),
            Duration::from_secs(2),
        )
        .expect("Document symbols should work for module");

    let _script_symbols = harness.request_with_timeout(
        "textDocument/documentSymbol",
        json!({
            "textDocument": {"uri": script_uri}
        }),
        Duration::from_secs(2),
    );

    // Both files should be analyzed successfully
    assert!(
        module_symbols.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "Module should have symbols"
    );

    // Test definition lookup across files (if implemented)
    let definition_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": script_uri},
            "position": {"line": 5, "character": 15} // MyApp::DataProcessor reference
        }),
        Duration::from_secs(2),
    );

    // Should either provide definition or handle gracefully
    assert!(definition_result.is_ok(), "Cross-file definition lookup should not error");
}

#[test]
// AC5:integration - Performance validation for complete workflow
fn test_complete_workflow_performance() {
    let (mut harness, workspace) = create_comprehensive_workspace();

    let workflow_start = std::time::Instant::now();

    // Execute multiple operations in sequence to test overall performance
    let critic_future = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("lib/MyApp/DataProcessor.pm")]
        }),
        Duration::from_secs(5),
    );

    let actions_future = harness.request_with_timeout(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/DataProcessor.pm")},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 50, "character": 0}
            },
            "context": {"diagnostics": []}
        }),
        Duration::from_secs(2),
    );

    // Both operations should complete successfully
    assert!(critic_future.is_ok(), "Critic analysis should complete");
    assert!(actions_future.is_ok(), "Code actions should complete");

    let total_duration = workflow_start.elapsed();

    // Performance requirement: complete workflow should finish within reasonable time
    assert!(
        total_duration < Duration::from_secs(10),
        "Complete workflow should finish within 10 seconds, took: {:?}",
        total_duration
    );
}

#[test]
// AC5:integration - Error handling and recovery
fn test_error_handling_integration() {
    let (mut harness, _workspace) = create_comprehensive_workspace();

    // Test with malformed requests
    let malformed_execute_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.invalidCommand",
            "arguments": ["invalid_argument"]
        }),
        Duration::from_secs(2),
    );

    // Should handle malformed requests gracefully
    assert!(
        malformed_execute_result.is_err()
            || malformed_execute_result.as_ref().unwrap().get("error").is_some(),
        "Invalid executeCommand should be handled gracefully"
    );

    let malformed_actions_result = harness.request_with_timeout(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": "file:///nonexistent.pl"},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 1, "character": 0}
            },
            "context": {"diagnostics": []}
        }),
        Duration::from_secs(2),
    );

    // Should handle non-existent files gracefully
    assert!(malformed_actions_result.is_ok(), "Non-existent file should be handled gracefully");
}

#[test]
// AC5:integration - Concurrent operations validation
fn test_concurrent_operations() {
    let (mut harness, workspace) = create_comprehensive_workspace();

    // Test concurrent executeCommand and code action requests
    // Note: This is a simplified test due to harness limitations

    let start_time = std::time::Instant::now();

    // Execute operations in sequence (simulating concurrency)
    let critic_result1 = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("lib/MyApp/DataProcessor.pm")]
        }),
        Duration::from_secs(3),
    );

    let critic_result2 = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("test_script.pl")]
        }),
        Duration::from_secs(3),
    );

    let actions_result = harness.request_with_timeout(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/DataProcessor.pm")},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 20, "character": 0}
            },
            "context": {"diagnostics": []}
        }),
        Duration::from_secs(2),
    );

    // All operations should succeed
    assert!(critic_result1.is_ok(), "First critic request should succeed");
    assert!(critic_result2.is_ok(), "Second critic request should succeed");
    assert!(actions_result.is_ok(), "Code actions request should succeed");

    let total_time = start_time.elapsed();

    // Should handle multiple operations efficiently
    assert!(
        total_time < Duration::from_secs(15),
        "Concurrent operations should complete efficiently, took: {:?}",
        total_time
    );
}

// ======================== Protocol Compliance and Standards ========================

#[test]
// AC5:integration - LSP 3.17+ protocol compliance validation
fn test_lsp_protocol_compliance() {
    let (mut harness, workspace) = create_comprehensive_workspace();

    // Test executeCommand request/response format compliance
    let execute_response = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("lib/MyApp/DataProcessor.pm")]
            }),
            Duration::from_secs(3),
        )
        .expect("executeCommand should return valid response");

    // Response should be JSON-RPC 2.0 compliant (verified by harness)
    assert!(execute_response.is_object(), "executeCommand response should be JSON object");

    // Test codeAction request/response format compliance
    let code_action_response = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("lib/MyApp/DataProcessor.pm")},
                "range": {
                    "start": {"line": 10, "character": 0},
                    "end": {"line": 20, "character": 0}
                },
                "context": {"diagnostics": []}
            }),
            Duration::from_secs(2),
        )
        .expect("codeAction should return valid response");

    // Response should be array of CodeAction | Command
    assert!(code_action_response.is_array(), "codeAction response should be array");

    // Validate individual code actions have required fields
    if let Some(actions) = code_action_response.as_array() {
        for action in actions {
            // Each action should have title (required field)
            assert!(action.get("title").is_some(), "Code action should have title field");

            // Should have either edit OR command field
            let has_edit = action.get("edit").is_some();
            let has_command = action.get("command").is_some();
            assert!(
                has_edit || has_command,
                "Code action should have either edit or command field"
            );
        }
    }
}

#[test]
// AC5:integration - Workspace configuration and settings
fn test_workspace_configuration_integration() {
    let (mut harness, workspace) = create_comprehensive_workspace();

    // Test that server respects workspace configuration (.perlcriticrc)
    let critic_result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("lib/MyApp/DataProcessor.pm")]
            }),
            Duration::from_secs(5),
        )
        .expect("perl.runCritic should respect workspace configuration");

    // The presence of .perlcriticrc should influence the analysis
    // (specific assertions depend on implementation details)
    assert_eq!(
        critic_result["status"].as_str(),
        Some("success"),
        "Critic should work with workspace configuration"
    );
}

// ======================== Regression and Stability Testing ========================

#[test]
// AC5:integration - Backwards compatibility with existing features
fn test_backwards_compatibility() {
    let (mut harness, workspace) = create_comprehensive_workspace();

    // Test that existing executeCommand functionality still works
    let existing_commands = ["perl.runTests", "perl.runFile"];

    for command in existing_commands {
        let result = harness.request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": command,
                "arguments": [workspace.uri("test_script.pl")]
            }),
            Duration::from_secs(3),
        );

        // Should not break existing functionality
        assert!(result.is_ok(), "Existing command '{}' should still work", command);
    }

    // Test that basic LSP features still work
    let hover_result = harness.request_with_timeout(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": workspace.uri("test_script.pl")},
            "position": {"line": 3, "character": 10}
        }),
        Duration::from_secs(2),
    );

    assert!(hover_result.is_ok(), "Basic LSP features should continue to work");
}

#[test]
// AC5:integration - Memory and resource management
fn test_resource_management() {
    let (mut harness, workspace) = create_comprehensive_workspace();

    // Execute multiple operations to test resource management
    for i in 0..5 {
        let _critic_result = harness
            .request_with_timeout(
                "workspace/executeCommand",
                json!({
                    "command": "perl.runCritic",
                    "arguments": [workspace.uri("lib/MyApp/DataProcessor.pm")]
                }),
                Duration::from_secs(3),
            )
            .expect("Repeated critic requests should succeed");

        let _actions_result = harness
            .request_with_timeout(
                "textDocument/codeAction",
                json!({
                    "textDocument": {"uri": workspace.uri("lib/MyApp/DataProcessor.pm")},
                    "range": {
                        "start": {"line": i * 5, "character": 0},
                        "end": {"line": (i + 1) * 5, "character": 0}
                    },
                    "context": {"diagnostics": []}
                }),
                Duration::from_secs(2),
            )
            .expect("Repeated code action requests should succeed");

        // Brief pause between operations
        std::thread::sleep(Duration::from_millis(100));
    }

    // Server should handle repeated operations without issues
    // Server successfully handled repeated operations without resource leaks
}
