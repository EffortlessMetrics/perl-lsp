//! LSP workflow integration test fixtures
//!
//! Provides comprehensive test data for validating the complete LSP workflow:
//! Parse → Index → Navigate → Complete → Analyze
//!
//! Features:
//! - End-to-end workflow validation scenarios
//! - Cross-file navigation test data with dual indexing
//! - Completion context scenarios for all trigger points
//! - Code analysis integration with diagnostic workflows
//! - Workspace-wide symbol resolution testing

use serde_json::{json, Value};
use std::collections::HashMap;

#[cfg(test)]
pub struct WorkflowFixture {
    pub name: &'static str,
    pub description: &'static str,
    pub workspace_files: HashMap<String, String>,
    pub workflow_steps: Vec<WorkflowStep>,
    pub expected_final_state: WorkflowState,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub step_name: &'static str,
    pub step_type: WorkflowStepType,
    pub input_data: Value,
    pub expected_output: Value,
    pub performance_target_ms: u64,
    pub dependencies: Vec<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowStepType {
    Parse,
    Index,
    Navigate,
    Complete,
    Analyze,
    Diagnostic,
    CodeAction,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct WorkflowState {
    pub parsed_files: usize,
    pub indexed_symbols: usize,
    pub navigation_targets: usize,
    pub completion_items: usize,
    pub diagnostics: usize,
    pub code_actions: usize,
}

/// Comprehensive LSP workflow integration fixtures
#[cfg(test)]
pub fn load_workflow_fixtures() -> Vec<WorkflowFixture> {
    vec![
        // Basic single-file workflow
        WorkflowFixture {
            name: "single_file_workflow",
            description: "Complete workflow for a single Perl file with function definitions",
            workspace_files: create_single_file_workspace(),
            workflow_steps: vec![
                WorkflowStep {
                    step_name: "parse_initial_file",
                    step_type: WorkflowStepType::Parse,
                    input_data: json!({
                        "uri": "file:///test/single.pl",
                        "content": "use strict;\nuse warnings;\n\nsub calculate {\n    my ($a, $b) = @_;\n    return $a + $b;\n}\n\nmy $result = calculate(5, 3);\nprint \"Result: $result\\n\";"
                    }),
                    expected_output: json!({
                        "success": true,
                        "ast_nodes": 25,
                        "parsing_time_ms": 15
                    }),
                    performance_target_ms: 50,
                    dependencies: vec![],
                },
                WorkflowStep {
                    step_name: "index_symbols",
                    step_type: WorkflowStepType::Index,
                    input_data: json!({
                        "uri": "file:///test/single.pl"
                    }),
                    expected_output: json!({
                        "symbols": [
                            {
                                "name": "calculate",
                                "type": "function",
                                "line": 3,
                                "character": 4
                            },
                            {
                                "name": "$result",
                                "type": "variable",
                                "line": 8,
                                "character": 4
                            }
                        ],
                        "indexing_time_ms": 8
                    }),
                    performance_target_ms: 25,
                    dependencies: vec!["parse_initial_file".to_string()],
                },
                WorkflowStep {
                    step_name: "navigate_to_function",
                    step_type: WorkflowStepType::Navigate,
                    input_data: json!({
                        "uri": "file:///test/single.pl",
                        "position": { "line": 8, "character": 17 }, // "calculate" call
                        "method": "textDocument/definition"
                    }),
                    expected_output: json!({
                        "locations": [
                            {
                                "uri": "file:///test/single.pl",
                                "range": {
                                    "start": { "line": 3, "character": 4 },
                                    "end": { "line": 3, "character": 13 }
                                }
                            }
                        ],
                        "navigation_time_ms": 12
                    }),
                    performance_target_ms: 100,
                    dependencies: vec!["index_symbols".to_string()],
                },
                WorkflowStep {
                    step_name: "complete_at_function_call",
                    step_type: WorkflowStepType::Complete,
                    input_data: json!({
                        "uri": "file:///test/single.pl",
                        "position": { "line": 8, "character": 15 }, // Inside "calculate"
                        "trigger_character": null
                    }),
                    expected_output: json!({
                        "items": [
                            {
                                "label": "calculate",
                                "kind": 3,
                                "detail": "subroutine",
                                "insertText": "calculate"
                            }
                        ],
                        "completion_time_ms": 5
                    }),
                    performance_target_ms: 50,
                    dependencies: vec!["index_symbols".to_string()],
                },
                WorkflowStep {
                    step_name: "analyze_code_quality",
                    step_type: WorkflowStepType::Analyze,
                    input_data: json!({
                        "uri": "file:///test/single.pl",
                        "analysis_type": "comprehensive"
                    }),
                    expected_output: json!({
                        "diagnostics": [],
                        "quality_score": 8.5,
                        "analysis_time_ms": 25
                    }),
                    performance_target_ms: 100,
                    dependencies: vec!["parse_initial_file".to_string()],
                },
            ],
            expected_final_state: WorkflowState {
                parsed_files: 1,
                indexed_symbols: 2,
                navigation_targets: 1,
                completion_items: 1,
                diagnostics: 0,
                code_actions: 0,
            },
        },

        // Multi-file cross-reference workflow
        WorkflowFixture {
            name: "multi_file_cross_reference_workflow",
            description: "Complete workflow with cross-file navigation and dual indexing",
            workspace_files: create_multi_file_workspace(),
            workflow_steps: vec![
                WorkflowStep {
                    step_name: "parse_main_file",
                    step_type: WorkflowStepType::Parse,
                    input_data: json!({
                        "uri": "file:///test/main.pl",
                        "content": get_main_file_content()
                    }),
                    expected_output: json!({
                        "success": true,
                        "ast_nodes": 35,
                        "imports": ["MyModule::Utils"],
                        "parsing_time_ms": 20
                    }),
                    performance_target_ms: 100,
                    dependencies: vec![],
                },
                WorkflowStep {
                    step_name: "parse_module_file",
                    step_type: WorkflowStepType::Parse,
                    input_data: json!({
                        "uri": "file:///test/MyModule/Utils.pm",
                        "content": get_module_file_content()
                    }),
                    expected_output: json!({
                        "success": true,
                        "ast_nodes": 45,
                        "package": "MyModule::Utils",
                        "parsing_time_ms": 18
                    }),
                    performance_target_ms: 100,
                    dependencies: vec![],
                },
                WorkflowStep {
                    step_name: "index_workspace_symbols",
                    step_type: WorkflowStepType::Index,
                    input_data: json!({
                        "workspace_uri": "file:///test/"
                    }),
                    expected_output: json!({
                        "total_symbols": 8,
                        "qualified_symbols": 4,
                        "bare_symbols": 4,
                        "dual_indexed": true,
                        "indexing_time_ms": 35
                    }),
                    performance_target_ms: 150,
                    dependencies: vec!["parse_main_file".to_string(), "parse_module_file".to_string()],
                },
                WorkflowStep {
                    step_name: "navigate_cross_file_qualified",
                    step_type: WorkflowStepType::Navigate,
                    input_data: json!({
                        "uri": "file:///test/main.pl",
                        "position": { "line": 6, "character": 25 }, // "MyModule::Utils::process_data"
                        "method": "textDocument/definition"
                    }),
                    expected_output: json!({
                        "locations": [
                            {
                                "uri": "file:///test/MyModule/Utils.pm",
                                "range": {
                                    "start": { "line": 5, "character": 4 },
                                    "end": { "line": 5, "character": 16 }
                                }
                            }
                        ],
                        "dual_indexing_used": true,
                        "navigation_time_ms": 45
                    }),
                    performance_target_ms: 200,
                    dependencies: vec!["index_workspace_symbols".to_string()],
                },
                WorkflowStep {
                    step_name: "navigate_cross_file_bare",
                    step_type: WorkflowStepType::Navigate,
                    input_data: json!({
                        "uri": "file:///test/main.pl",
                        "position": { "line": 7, "character": 15 }, // "validate_input" (bare call)
                        "method": "textDocument/definition"
                    }),
                    expected_output: json!({
                        "locations": [
                            {
                                "uri": "file:///test/MyModule/Utils.pm",
                                "range": {
                                    "start": { "line": 12, "character": 4 },
                                    "end": { "line": 12, "character": 18 }
                                }
                            }
                        ],
                        "dual_indexing_used": true,
                        "navigation_time_ms": 38
                    }),
                    performance_target_ms: 200,
                    dependencies: vec!["index_workspace_symbols".to_string()],
                },
                WorkflowStep {
                    step_name: "complete_with_imports",
                    step_type: WorkflowStepType::Complete,
                    input_data: json!({
                        "uri": "file:///test/main.pl",
                        "position": { "line": 10, "character": 20 }, // After "MyModule::Utils::"
                        "trigger_character": ":"
                    }),
                    expected_output: json!({
                        "items": [
                            {
                                "label": "process_data",
                                "kind": 3,
                                "detail": "subroutine from MyModule::Utils",
                                "insertText": "process_data"
                            },
                            {
                                "label": "validate_input",
                                "kind": 3,
                                "detail": "subroutine from MyModule::Utils",
                                "insertText": "validate_input"
                            }
                        ],
                        "completion_time_ms": 15
                    }),
                    performance_target_ms: 100,
                    dependencies: vec!["index_workspace_symbols".to_string()],
                },
                WorkflowStep {
                    step_name: "analyze_import_usage",
                    step_type: WorkflowStepType::Analyze,
                    input_data: json!({
                        "uri": "file:///test/main.pl",
                        "analysis_type": "import_optimization"
                    }),
                    expected_output: json!({
                        "used_imports": ["MyModule::Utils"],
                        "unused_imports": [],
                        "missing_imports": [],
                        "optimization_suggestions": [],
                        "analysis_time_ms": 30
                    }),
                    performance_target_ms: 150,
                    dependencies: vec!["index_workspace_symbols".to_string()],
                },
            ],
            expected_final_state: WorkflowState {
                parsed_files: 2,
                indexed_symbols: 8,
                navigation_targets: 2,
                completion_items: 2,
                diagnostics: 0,
                code_actions: 0,
            },
        },

        // Error handling and diagnostic workflow
        WorkflowFixture {
            name: "error_handling_diagnostic_workflow",
            description: "Workflow with syntax errors and diagnostic generation",
            workspace_files: create_error_workspace(),
            workflow_steps: vec![
                WorkflowStep {
                    step_name: "parse_file_with_errors",
                    step_type: WorkflowStepType::Parse,
                    input_data: json!({
                        "uri": "file:///test/errors.pl",
                        "content": get_error_file_content()
                    }),
                    expected_output: json!({
                        "success": false,
                        "errors": [
                            {
                                "line": 5,
                                "message": "Syntax error: unterminated string",
                                "severity": "error"
                            },
                            {
                                "line": 8,
                                "message": "Undefined subroutine",
                                "severity": "warning"
                            }
                        ],
                        "partial_ast_nodes": 15,
                        "parsing_time_ms": 25
                    }),
                    performance_target_ms: 100,
                    dependencies: vec![],
                },
                WorkflowStep {
                    step_name: "generate_diagnostics",
                    step_type: WorkflowStepType::Diagnostic,
                    input_data: json!({
                        "uri": "file:///test/errors.pl"
                    }),
                    expected_output: json!({
                        "diagnostics": [
                            {
                                "range": {
                                    "start": { "line": 4, "character": 15 },
                                    "end": { "line": 4, "character": 30 }
                                },
                                "severity": 1,
                                "message": "Unterminated string literal",
                                "source": "perl-parser"
                            },
                            {
                                "range": {
                                    "start": { "line": 7, "character": 8 },
                                    "end": { "line": 7, "character": 25 }
                                },
                                "severity": 2,
                                "message": "Undefined subroutine 'unknown_function'",
                                "source": "perl-lsp"
                            }
                        ],
                        "diagnostic_time_ms": 20
                    }),
                    performance_target_ms: 100,
                    dependencies: vec!["parse_file_with_errors".to_string()],
                },
                WorkflowStep {
                    step_name: "suggest_code_actions",
                    step_type: WorkflowStepType::CodeAction,
                    input_data: json!({
                        "uri": "file:///test/errors.pl",
                        "range": {
                            "start": { "line": 4, "character": 15 },
                            "end": { "line": 4, "character": 30 }
                        },
                        "context": {
                            "diagnostics": [
                                {
                                    "range": {
                                        "start": { "line": 4, "character": 15 },
                                        "end": { "line": 4, "character": 30 }
                                    },
                                    "severity": 1,
                                    "message": "Unterminated string literal"
                                }
                            ]
                        }
                    }),
                    expected_output: json!({
                        "code_actions": [
                            {
                                "title": "Close string literal",
                                "kind": "quickfix",
                                "edit": {
                                    "changes": {
                                        "file:///test/errors.pl": [
                                            {
                                                "range": {
                                                    "start": { "line": 4, "character": 30 },
                                                    "end": { "line": 4, "character": 30 }
                                                },
                                                "newText": "\""
                                            }
                                        ]
                                    }
                                }
                            }
                        ],
                        "action_time_ms": 12
                    }),
                    performance_target_ms: 50,
                    dependencies: vec!["generate_diagnostics".to_string()],
                },
            ],
            expected_final_state: WorkflowState {
                parsed_files: 1,
                indexed_symbols: 0,
                navigation_targets: 0,
                completion_items: 0,
                diagnostics: 2,
                code_actions: 1,
            },
        },

        // Performance-intensive workflow
        WorkflowFixture {
            name: "performance_intensive_workflow",
            description: "Large workspace with performance requirements",
            workspace_files: create_large_workspace(),
            workflow_steps: vec![
                WorkflowStep {
                    step_name: "parse_large_workspace",
                    step_type: WorkflowStepType::Parse,
                    input_data: json!({
                        "workspace_uri": "file:///test/",
                        "file_count": 10
                    }),
                    expected_output: json!({
                        "parsed_files": 10,
                        "total_ast_nodes": 850,
                        "avg_parsing_time_ms": 35,
                        "max_parsing_time_ms": 65
                    }),
                    performance_target_ms: 500,
                    dependencies: vec![],
                },
                WorkflowStep {
                    step_name: "index_large_symbol_table",
                    step_type: WorkflowStepType::Index,
                    input_data: json!({
                        "workspace_uri": "file:///test/"
                    }),
                    expected_output: json!({
                        "total_symbols": 150,
                        "packages": 10,
                        "functions": 85,
                        "variables": 55,
                        "indexing_time_ms": 120
                    }),
                    performance_target_ms: 300,
                    dependencies: vec!["parse_large_workspace".to_string()],
                },
                WorkflowStep {
                    step_name: "workspace_symbol_search",
                    step_type: WorkflowStepType::Navigate,
                    input_data: json!({
                        "query": "process",
                        "method": "workspace/symbol"
                    }),
                    expected_output: json!({
                        "symbols": [
                            {
                                "name": "process_data",
                                "containerName": "MyModule::Utils",
                                "kind": 12,
                                "location": {
                                    "uri": "file:///test/MyModule/Utils.pm",
                                    "range": {
                                        "start": { "line": 5, "character": 4 },
                                        "end": { "line": 5, "character": 16 }
                                    }
                                }
                            }
                        ],
                        "search_time_ms": 45
                    }),
                    performance_target_ms: 200,
                    dependencies: vec!["index_large_symbol_table".to_string()],
                },
                WorkflowStep {
                    step_name: "bulk_completion_test",
                    step_type: WorkflowStepType::Complete,
                    input_data: json!({
                        "uri": "file:///test/main.pl",
                        "position": { "line": 15, "character": 5 },
                        "trigger_character": "$"
                    }),
                    expected_output: json!({
                        "items": [
                            {
                                "label": "$large_data",
                                "kind": 6,
                                "detail": "scalar variable"
                            },
                            {
                                "label": "$result",
                                "kind": 6,
                                "detail": "scalar variable"
                            }
                        ],
                        "completion_time_ms": 25
                    }),
                    performance_target_ms: 100,
                    dependencies: vec!["index_large_symbol_table".to_string()],
                },
            ],
            expected_final_state: WorkflowState {
                parsed_files: 10,
                indexed_symbols: 150,
                navigation_targets: 1,
                completion_items: 2,
                diagnostics: 0,
                code_actions: 0,
            },
        },
    ]
}

/// Create workspace files for different test scenarios
#[cfg(test)]
fn create_single_file_workspace() -> HashMap<String, String> {
    let mut files = HashMap::new();
    files.insert(
        "file:///test/single.pl".to_string(),
        r#"use strict;
use warnings;

sub calculate {
    my ($a, $b) = @_;
    return $a + $b;
}

my $result = calculate(5, 3);
print "Result: $result\n";"#.to_string(),
    );
    files
}

#[cfg(test)]
fn create_multi_file_workspace() -> HashMap<String, String> {
    let mut files = HashMap::new();
    files.insert("file:///test/main.pl".to_string(), get_main_file_content());
    files.insert("file:///test/MyModule/Utils.pm".to_string(), get_module_file_content());
    files
}

#[cfg(test)]
fn create_error_workspace() -> HashMap<String, String> {
    let mut files = HashMap::new();
    files.insert("file:///test/errors.pl".to_string(), get_error_file_content());
    files
}

#[cfg(test)]
fn create_large_workspace() -> HashMap<String, String> {
    let mut files = HashMap::new();

    // Generate 10 test files
    for i in 1..=10 {
        let uri = format!("file:///test/module_{}.pl", i);
        let content = format!(r#"#!/usr/bin/perl
use strict;
use warnings;

package Module{};

sub process_data_{} {{
    my ($input) = @_;
    return transform_data_{}($input);
}}

sub transform_data_{} {{
    my ($data) = @_;
    return uc($data) . "_{}";
}}

sub validate_{} {{
    my ($value) = @_;
    return length($value) > 0;
}}

my $test_data_{} = "test_data_{}";
my $result_{} = process_data_{}($test_data_{});

1;
"#, i, i, i, i, i, i, i, i, i, i, i);

        files.insert(uri, content);
    }

    files.insert("file:///test/main.pl".to_string(), r#"#!/usr/bin/perl
use strict;
use warnings;

use Module1;
use Module2;
use Module3;

my $large_data = "comprehensive test data";
my $result = Module1::process_data_1($large_data);
print "Final result: $result\n";

for my $i (1..10) {
    my $module_result = eval "Module${i}::validate_${i}('test')";
    print "Module $i validation: $module_result\n" if $module_result;
}

1;
"#.to_string());

    files
}

/// Get file content for test scenarios
#[cfg(test)]
fn get_main_file_content() -> String {
    r#"#!/usr/bin/perl
use strict;
use warnings;
use MyModule::Utils;

my $data = "test input";
my $processed = MyModule::Utils::process_data($data);
my $valid = validate_input($processed);

if ($valid) {
    my $final = MyModule::Utils::finalize($processed);
    print "Final result: $final\n";
}

1;"#.to_string()
}

#[cfg(test)]
fn get_module_file_content() -> String {
    r#"#!/usr/bin/perl
use strict;
use warnings;

package MyModule::Utils;

sub process_data {
    my ($input) = @_;
    return transform_string($input);
}

sub transform_string {
    my ($str) = @_;
    return uc($str);
}

sub validate_input {
    my ($data) = @_;
    return defined $data && length($data) > 0;
}

sub finalize {
    my ($processed_data) = @_;
    return $processed_data . "_FINAL";
}

1;"#.to_string()
}

#[cfg(test)]
fn get_error_file_content() -> String {
    r#"#!/usr/bin/perl
use strict;
use warnings;

my $broken_string = "unterminated string
my $valid_var = "this is fine";

my $result = unknown_function("test");
print "Result: $result\n";

sub incomplete_function {
    my ($param = @_;  # Syntax error
    return $param;
"#.to_string()
}

/// Workflow execution utilities
#[cfg(test)]
pub struct WorkflowExecutor {
    pub current_state: WorkflowState,
    pub step_results: HashMap<String, Value>,
    pub performance_metrics: HashMap<String, u64>,
}

#[cfg(test)]
impl WorkflowExecutor {
    pub fn new() -> Self {
        Self {
            current_state: WorkflowState {
                parsed_files: 0,
                indexed_symbols: 0,
                navigation_targets: 0,
                completion_items: 0,
                diagnostics: 0,
                code_actions: 0,
            },
            step_results: HashMap::new(),
            performance_metrics: HashMap::new(),
        }
    }

    pub fn execute_step(&mut self, step: &WorkflowStep) -> Result<Value, String> {
        // Mock execution - in real tests, this would call actual LSP methods
        let start_time = std::time::Instant::now();

        // Simulate step execution based on step type
        let result = match step.step_type {
            WorkflowStepType::Parse => {
                self.current_state.parsed_files += 1;
                step.expected_output.clone()
            },
            WorkflowStepType::Index => {
                if let Some(symbols) = step.expected_output.get("total_symbols") {
                    self.current_state.indexed_symbols = symbols.as_u64().unwrap_or(0) as usize;
                }
                step.expected_output.clone()
            },
            WorkflowStepType::Navigate => {
                self.current_state.navigation_targets += 1;
                step.expected_output.clone()
            },
            WorkflowStepType::Complete => {
                if let Some(items) = step.expected_output.get("items") {
                    if let Some(items_array) = items.as_array() {
                        self.current_state.completion_items += items_array.len();
                    }
                }
                step.expected_output.clone()
            },
            WorkflowStepType::Analyze => {
                step.expected_output.clone()
            },
            WorkflowStepType::Diagnostic => {
                if let Some(diagnostics) = step.expected_output.get("diagnostics") {
                    if let Some(diag_array) = diagnostics.as_array() {
                        self.current_state.diagnostics += diag_array.len();
                    }
                }
                step.expected_output.clone()
            },
            WorkflowStepType::CodeAction => {
                if let Some(actions) = step.expected_output.get("code_actions") {
                    if let Some(actions_array) = actions.as_array() {
                        self.current_state.code_actions += actions_array.len();
                    }
                }
                step.expected_output.clone()
            },
        };

        let execution_time = start_time.elapsed().as_millis() as u64;
        self.performance_metrics.insert(step.step_name.to_string(), execution_time);
        self.step_results.insert(step.step_name.to_string(), result.clone());

        // Validate performance target
        if execution_time > step.performance_target_ms {
            return Err(format!(
                "Step '{}' exceeded performance target: {}ms > {}ms",
                step.step_name, execution_time, step.performance_target_ms
            ));
        }

        Ok(result)
    }

    pub fn validate_final_state(&self, expected: &WorkflowState) -> Result<(), String> {
        if self.current_state.parsed_files != expected.parsed_files {
            return Err(format!(
                "Parsed files mismatch: {} != {}",
                self.current_state.parsed_files, expected.parsed_files
            ));
        }

        if self.current_state.indexed_symbols != expected.indexed_symbols {
            return Err(format!(
                "Indexed symbols mismatch: {} != {}",
                self.current_state.indexed_symbols, expected.indexed_symbols
            ));
        }

        if self.current_state.diagnostics != expected.diagnostics {
            return Err(format!(
                "Diagnostics mismatch: {} != {}",
                self.current_state.diagnostics, expected.diagnostics
            ));
        }

        Ok(())
    }
}

use std::sync::LazyLock;

/// Lazy-loaded workflow fixture registry
#[cfg(test)]
pub static WORKFLOW_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, WorkflowFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_workflow_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get workflow fixture by name
#[cfg(test)]
pub fn get_workflow_fixture_by_name(name: &str) -> Option<&'static WorkflowFixture> {
    WORKFLOW_FIXTURE_REGISTRY.get(name)
}

/// Get fixtures by step type
#[cfg(test)]
pub fn get_fixtures_by_step_type(step_type: WorkflowStepType) -> Vec<&'static WorkflowFixture> {
    WORKFLOW_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| {
            fixture.workflow_steps.iter().any(|step| step.step_type == step_type)
        })
        .collect()
}

/// Get fixtures with specific performance requirements
#[cfg(test)]
pub fn get_fixtures_by_performance_requirement(max_step_time_ms: u64) -> Vec<&'static WorkflowFixture> {
    WORKFLOW_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| {
            fixture.workflow_steps.iter().all(|step| step.performance_target_ms <= max_step_time_ms)
        })
        .collect()
}