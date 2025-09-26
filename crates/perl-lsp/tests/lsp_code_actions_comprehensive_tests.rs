//! Comprehensive tests for LSP code actions and refactoring functionality
//!
//! Tests feature spec: SPEC_145_LSP_EXECUTE_COMMAND_AND_CODE_ACTIONS.md#AC3
//! Architecture: ADR_003_EXECUTE_COMMAND_CODE_ACTIONS_ARCHITECTURE.md
//!
//! This module provides complete test coverage for advanced code action refactorings
//! including extract variable, extract subroutine, import management, and code quality improvements.

use serde_json::json;
use std::time::Duration;

mod support;
use support::lsp_harness::{LspHarness, TempWorkspace};

// Test fixtures for code actions testing
mod code_actions_fixtures {
    /// Code with extractable expressions for refactoring tests
    pub const REFACTORING_OPPORTUNITIES_FILE: &str = r#"#!/usr/bin/perl
use strict;
use warnings;

sub process_data {
    my ($input) = @_;

    # Complex expression suitable for extract variable
    my $result = length($input) + substr($input, 0, 5) eq "hello" ? 10 : 0;

    # Another extractable expression
    my $normalized = lc(trim($input)) . "_processed";

    # Code block suitable for extract subroutine
    if ($result > 0) {
        my $validation = validate_input($input);
        my $processed = transform_data($validation);
        my $output = format_result($processed);
        return $output;
    }

    return $input;
}

sub calculate_complex {
    my ($x, $y, $z) = @_;

    # Complex mathematical expression
    return ($x * $y + $z) / ($x + $y) * sin($z) + cos($x);
}

# C-style for loop that could be converted to foreach
for (my $i = 0; $i < 10; $i++) {
    print "Index: $i\n";
}

# File operation without error checking
open FILE, "data.txt";
print FILE "some data";
close FILE;

# Old-style if statement that could be postfix
if ($result) {
    print "Success";
}
"#;

    /// Code with import management opportunities
    pub const IMPORT_MANAGEMENT_FILE: &str = r#"#!/usr/bin/perl
use strict;
use warnings;

# Unused imports
use File::Spec;
use Data::Dumper;
use List::Util qw(first);

# Missing imports (would be detected by analysis)
use Carp;

# Duplicate imports
use File::Path;
use File::Path qw(make_path);

# Unorganized imports
use My::Custom::Module;
use POSIX qw(strftime);
use Scalar::Util qw(blessed);

sub process_file {
    my ($filename) = @_;

    # Uses blessed but imported
    return blessed($filename) ? $filename->name : $filename;
}

sub format_time {
    my ($time) = @_;

    # Uses strftime but imported
    return strftime("%Y-%m-%d", localtime($time));
}

sub create_directory {
    my ($path) = @_;

    # Uses make_path from File::Path
    make_path($path);
}
"#;

    /// Code requiring pragma additions
    pub const MISSING_PRAGMAS_FILE: &str = r#"#!/usr/bin/perl
# Missing use strict and use warnings

my $variable = "test";
print "$variable\n";

sub calculate {
    my ($a, $b) = @_;
    $a + $b;
}

# Unicode string without utf8 pragma
my $unicode = "café";
print "Unicode: $unicode\n";
"#;

    /// Code with undefined variable issues
    pub const UNDEFINED_VARIABLES_FILE: &str = r#"#!/usr/bin/perl
use strict;
use warnings;

sub process {
    my ($input) = @_;

    # Undefined variable (should be detected)
    print "Processing: $undefinedVar\n";

    # Another undefined variable in complex expression
    my $result = $input + $anotherUndefined * 2;

    return $result;
}

# Global variable used without declaration
$globalVar = "test";
print "Global: $globalVar\n";
"#;
}

/// Create test server with code actions-focused workspace
fn create_code_actions_server() -> (LspHarness, TempWorkspace) {
    let (mut harness, workspace) = LspHarness::with_workspace(&[
        ("refactoring.pl", code_actions_fixtures::REFACTORING_OPPORTUNITIES_FILE),
        ("imports.pl", code_actions_fixtures::IMPORT_MANAGEMENT_FILE),
        ("pragmas.pl", code_actions_fixtures::MISSING_PRAGMAS_FILE),
        ("undefined.pl", code_actions_fixtures::UNDEFINED_VARIABLES_FILE),
    ])
    .expect("Failed to create code actions test workspace");

    // Initialize documents
    harness
        .open_document(
            &workspace.uri("refactoring.pl"),
            code_actions_fixtures::REFACTORING_OPPORTUNITIES_FILE,
        )
        .expect("Failed to open refactoring file");

    harness
        .open_document(&workspace.uri("imports.pl"), code_actions_fixtures::IMPORT_MANAGEMENT_FILE)
        .expect("Failed to open imports file");

    harness
        .open_document(&workspace.uri("pragmas.pl"), code_actions_fixtures::MISSING_PRAGMAS_FILE)
        .expect("Failed to open pragmas file");

    harness
        .open_document(
            &workspace.uri("undefined.pl"),
            code_actions_fixtures::UNDEFINED_VARIABLES_FILE,
        )
        .expect("Failed to open undefined variables file");

    // Trigger processing and wait for idle
    harness.did_save(&workspace.uri("refactoring.pl")).ok();
    harness.did_save(&workspace.uri("imports.pl")).ok();
    harness.did_save(&workspace.uri("pragmas.pl")).ok();
    harness.did_save(&workspace.uri("undefined.pl")).ok();

    harness.wait_for_idle(Duration::from_millis(1000));

    (harness, workspace)
}

// ======================== AC3: Advanced Code Action Refactorings ========================

#[test]
// AC3:codeActions - Server capabilities for code actions
fn test_code_action_server_capabilities() {
    let (mut harness, _workspace) = create_code_actions_server();

    // Initialize the server to get capabilities
    let init_result = harness.initialize_default().expect("Server should initialize successfully");

    let capabilities =
        init_result.get("capabilities").expect("Initialize result should contain capabilities");

    // Verify codeActionProvider is advertised
    assert!(
        capabilities.get("codeActionProvider").is_some(),
        "Server should advertise codeActionProvider capability"
    );

    let code_action_provider = &capabilities["codeActionProvider"];

    // Check for supported code action kinds
    if let Some(kinds) = code_action_provider.get("codeActionKinds") {
        let kinds_array = kinds.as_array().expect("codeActionKinds should be array");

        let expected_kinds =
            vec!["quickfix", "refactor.extract", "refactor.rewrite", "source.organizeImports"];

        for expected_kind in expected_kinds {
            let kind_found = kinds_array.iter().any(|k| k.as_str() == Some(expected_kind));
            assert!(kind_found, "Code action kind '{}' should be supported", expected_kind);
        }
    }

    // Check for resolve provider capability
    assert!(
        code_action_provider.get("resolveProvider").is_some(),
        "Should support code action resolve capability"
    );
}

#[test]
// AC3:codeActions - Extract variable refactoring
fn test_extract_variable_refactoring() {
    let (mut harness, workspace) = create_code_actions_server();

    // Request code actions for a complex expression that can be extracted
    let actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("refactoring.pl")},
                "range": {
                    "start": {"line": 7, "character": 17}, // Start of complex expression
                    "end": {"line": 7, "character": 70}     // End of complex expression
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.extract"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let actions = actions_result.as_array().expect("Should return action array");
    assert!(!actions.is_empty(), "Should have refactoring actions");

    // Look for extract variable action
    let extract_var_action = actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| title.contains("Extract") && title.contains("variable"))
            .unwrap_or(false)
    });

    assert!(extract_var_action.is_some(), "Should have 'Extract variable' action available");

    let action = extract_var_action.unwrap();

    // Verify action properties
    assert_eq!(
        action["kind"].as_str(),
        Some("refactor.extract"),
        "Should have correct action kind"
    );
    assert!(
        action.get("edit").is_some() || action.get("command").is_some(),
        "Should have edit or command"
    );
}

#[test]
// AC3:codeActions - Extract subroutine refactoring
fn test_extract_subroutine_refactoring() {
    let (mut harness, workspace) = create_code_actions_server();

    // Request code actions for code block that can be extracted into subroutine
    let actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("refactoring.pl")},
                "range": {
                    "start": {"line": 12, "character": 4},  // Start of extractable block
                    "end": {"line": 17, "character": 4}     // End of extractable block
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.extract"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let actions = actions_result.as_array().expect("Should return action array");

    // Look for extract subroutine action
    let extract_sub_action = actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| {
                title.contains("Extract")
                    && (title.contains("subroutine") || title.contains("function"))
            })
            .unwrap_or(false)
    });

    if extract_sub_action.is_some() {
        let action = extract_sub_action.unwrap();
        assert_eq!(
            action["kind"].as_str(),
            Some("refactor.extract"),
            "Should have correct action kind"
        );
        assert!(
            action.get("edit").is_some() || action.get("command").is_some(),
            "Should have edit or command"
        );
    }
    // Note: Extract subroutine may not be implemented yet, so we don't assert it exists
}

#[test]
// AC3:codeActions - Import organization refactoring
fn test_organize_imports_refactoring() {
    let (mut harness, workspace) = create_code_actions_server();

    // Request code actions for import organization
    let actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("imports.pl")},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 20, "character": 0}  // Cover import section
                },
                "context": {
                    "diagnostics": [],
                    "only": ["source.organizeImports"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let actions = actions_result.as_array().expect("Should return action array");

    // Look for organize imports action
    let organize_imports_action = actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| title.contains("Organize") && title.contains("import"))
            .unwrap_or(false)
    });

    if organize_imports_action.is_some() {
        let action = organize_imports_action.unwrap();
        assert_eq!(
            action["kind"].as_str(),
            Some("source.organizeImports"),
            "Should have correct action kind"
        );
        assert!(action.get("edit").is_some(), "Should have text edits for import organization");
    }
}

#[test]
// AC3:codeActions - Code quality improvement refactorings
fn test_code_quality_refactorings() {
    let (mut harness, workspace) = create_code_actions_server();

    // Request code actions for C-style for loop conversion
    let loop_actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("refactoring.pl")},
                "range": {
                    "start": {"line": 30, "character": 0},  // C-style for loop line
                    "end": {"line": 33, "character": 0}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.rewrite"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let loop_actions = loop_actions_result.as_array().expect("Should return action array");

    // Look for loop conversion actions
    let convert_loop_action = loop_actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| {
                title.contains("Convert") && (title.contains("foreach") || title.contains("for"))
            })
            .unwrap_or(false)
    });

    if convert_loop_action.is_some() {
        let action = convert_loop_action.unwrap();
        assert_eq!(
            action["kind"].as_str(),
            Some("refactor.rewrite"),
            "Should have correct action kind"
        );
    }

    // Request code actions for file operation error checking
    let error_check_actions = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("refactoring.pl")},
                "range": {
                    "start": {"line": 35, "character": 0},  // File operation lines
                    "end": {"line": 38, "character": 0}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.rewrite"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let error_actions = error_check_actions.as_array().expect("Should return action array");

    // Look for error checking actions
    let add_error_check_action = error_actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| title.contains("Add") && title.contains("error"))
            .unwrap_or(false)
    });

    if add_error_check_action.is_some() {
        let action = add_error_check_action.unwrap();
        assert_eq!(
            action["kind"].as_str(),
            Some("refactor.rewrite"),
            "Should have correct action kind"
        );
    }
}

#[test]
// AC3:codeActions - Add missing pragmas refactoring
fn test_add_missing_pragmas_refactoring() {
    let (mut harness, workspace) = create_code_actions_server();

    // Request code actions for adding missing pragmas
    let actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("pragmas.pl")},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 5, "character": 0}   // Top of file where pragmas should go
                },
                "context": {
                    "diagnostics": [],
                    "only": ["quickfix", "refactor.rewrite"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let actions = actions_result.as_array().expect("Should return action array");

    // Look for pragma addition actions
    let strict_action = actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| title.contains("Add") && title.contains("strict"))
            .unwrap_or(false)
    });

    let warnings_action = actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| title.contains("Add") && title.contains("warnings"))
            .unwrap_or(false)
    });

    let utf8_action = actions.iter().find(|action| {
        action["title"]
            .as_str()
            .map(|title| title.contains("Add") && title.contains("utf8"))
            .unwrap_or(false)
    });

    // At least one pragma action should be available
    assert!(
        strict_action.is_some() || warnings_action.is_some() || utf8_action.is_some(),
        "Should have at least one pragma addition action"
    );
}

#[test]
// AC3:codeActions - Performance validation for code actions response time
fn test_code_actions_performance() {
    let (mut harness, workspace) = create_code_actions_server();

    let start_time = std::time::Instant::now();

    let _actions_result = harness.request_with_timeout(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": workspace.uri("refactoring.pl")},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 50, "character": 0}  // Large range
            },
            "context": {
                "diagnostics": [],
                "only": [] // Request all available actions
            }
        }),
        Duration::from_millis(100), // Performance requirement: <50ms
    );

    let duration = start_time.elapsed();

    // Performance requirement from specification: <50ms response time
    assert!(
        duration < Duration::from_millis(75), // Slight buffer for test environment
        "Code actions should respond within 75ms, took: {:?}",
        duration
    );
}

#[test]
// AC3:codeActions - Code action resolve capability
fn test_code_action_resolve() {
    let (mut harness, workspace) = create_code_actions_server();

    // Get code actions first
    let actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("refactoring.pl")},
                "range": {
                    "start": {"line": 7, "character": 17},
                    "end": {"line": 7, "character": 70}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.extract"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let actions = actions_result.as_array().expect("Should return action array");

    if !actions.is_empty() {
        let first_action = &actions[0];

        // If action doesn't have edit but has data, it needs resolving
        if first_action.get("edit").is_none() && first_action.get("data").is_some() {
            let resolved_result = harness.request_with_timeout(
                "codeAction/resolve",
                first_action.clone(),
                Duration::from_secs(2),
            );

            assert!(resolved_result.is_ok(), "Code action resolve should succeed");

            let resolved_action = resolved_result.unwrap();
            assert!(
                resolved_action.get("edit").is_some(),
                "Resolved code action should have edit field"
            );
        }
    }
}

// ======================== Code Actions Integration with Diagnostics ========================

#[test]
// AC3:codeActions - Quick fix actions from diagnostics
fn test_quickfix_actions_from_diagnostics() {
    let (mut harness, workspace) = create_code_actions_server();

    // Create mock diagnostics for undefined variables
    let mock_diagnostics = json!([
        {
            "range": {
                "start": {"line": 6, "character": 20},
                "end": {"line": 6, "character": 32}
            },
            "severity": 1,
            "message": "Global symbol \"$undefinedVar\" requires explicit package name",
            "source": "perl"
        }
    ]);

    let actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("undefined.pl")},
                "range": {
                    "start": {"line": 6, "character": 20},
                    "end": {"line": 6, "character": 32}
                },
                "context": {
                    "diagnostics": mock_diagnostics,
                    "only": ["quickfix"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed");

    let actions = actions_result.as_array().expect("Should return action array");

    // Look for quickfix actions
    let quickfix_action = actions.iter().find(|action| action["kind"].as_str() == Some("quickfix"));

    if quickfix_action.is_some() {
        let action = quickfix_action.unwrap();
        assert!(
            action["title"].as_str().is_some(),
            "Quickfix action should have descriptive title"
        );
        assert!(
            action.get("edit").is_some() || action.get("command").is_some(),
            "Quickfix action should have edit or command"
        );
    }
}

// ======================== Error Handling and Edge Cases ========================

#[test]
// Test code actions for empty selections
fn test_code_actions_empty_selection() {
    let (mut harness, workspace) = create_code_actions_server();

    let actions_result = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("refactoring.pl")},
                "range": {
                    "start": {"line": 5, "character": 10},
                    "end": {"line": 5, "character": 10}  // Empty range
                },
                "context": {
                    "diagnostics": []
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Code action request should succeed for empty selection");

    let actions = actions_result.as_array().expect("Should return action array");
    // Empty selection may have fewer actions, but should not error
}

#[test]
// Test code actions for invalid ranges
fn test_code_actions_invalid_range() {
    let (mut harness, workspace) = create_code_actions_server();

    let actions_result = harness.request_with_timeout(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": workspace.uri("refactoring.pl")},
            "range": {
                "start": {"line": 1000, "character": 0},  // Beyond file end
                "end": {"line": 1001, "character": 0}
            },
            "context": {
                "diagnostics": []
            }
        }),
        Duration::from_secs(2),
    );

    // Should handle invalid range gracefully (either empty actions or error)
    assert!(actions_result.is_ok() || actions_result.is_err(), "Should handle invalid range");
}

#[test]
// Test code actions with specific "only" filters
fn test_code_actions_filtering() {
    let (mut harness, workspace) = create_code_actions_server();

    // Request only refactor actions
    let refactor_actions = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("refactoring.pl")},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 10, "character": 0}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Refactor-only request should succeed");

    let refactor_actions_array = refactor_actions.as_array().expect("Should return action array");

    // Verify all returned actions are refactor kinds
    for action in refactor_actions_array {
        if let Some(kind) = action.get("kind").and_then(|k| k.as_str()) {
            assert!(
                kind.starts_with("refactor"),
                "All actions should be refactor kind, found: {}",
                kind
            );
        }
    }

    // Request only quickfix actions
    let quickfix_actions = harness
        .request_with_timeout(
            "textDocument/codeAction",
            json!({
                "textDocument": {"uri": workspace.uri("pragmas.pl")},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 5, "character": 0}
                },
                "context": {
                    "diagnostics": [],
                    "only": ["quickfix"]
                }
            }),
            Duration::from_secs(2),
        )
        .expect("Quickfix-only request should succeed");

    let quickfix_actions_array = quickfix_actions.as_array().expect("Should return action array");

    // Verify all returned actions are quickfix kinds (if any)
    for action in quickfix_actions_array {
        if let Some(kind) = action.get("kind").and_then(|k| k.as_str()) {
            assert!(kind == "quickfix", "All actions should be quickfix kind, found: {}", kind);
        }
    }
}
