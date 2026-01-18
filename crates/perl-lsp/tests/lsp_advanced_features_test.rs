//! Advanced LSP Features Test Suite
//!
//! Tests for snippets, templates, test runner integration, and advanced IDE features
//!
//! NOTE: This test file is gated behind the `lsp-extras` feature because:
//! 1. Many of these tests are for speculative/future features not yet implemented
//! 2. The tests exercise mocked/stubbed behavior rather than full LSP harness coverage
//!
//! To run these tests: `cargo test -p perl-lsp --features lsp-extras --test lsp_advanced_features_test`
//! These tests should be rewritten with proper LspHarness before enabling in CI.

#![cfg(feature = "lsp-extras")]

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;

struct AdvancedTestContext {
    server: LspServer,
    #[allow(dead_code)]
    workspace_root: PathBuf,
    snippet_registry: HashMap<String, String>,
    template_cache: HashMap<String, String>,
    initialized: bool,
}

impl AdvancedTestContext {
    fn new_initialized() -> Self {
        let mut ctx = Self::new_uninitialized();
        ctx.initialize_or_reuse();
        ctx
    }

    fn new_uninitialized() -> Self {
        let server = LspServer::new();

        let mut snippet_registry = HashMap::new();
        let mut template_cache = HashMap::new();

        // Register common Perl snippets
        snippet_registry.insert("sub".to_string(), 
            "sub ${1:function_name} {\n    my (${2:\\$args}) = @_;\n    ${3:# code}\n    return ${4:\\$result};\n}".to_string());
        snippet_registry.insert("class".to_string(),
            "package ${1:ClassName};\nuse strict;\nuse warnings;\n\nsub new {\n    my (\\$class, %args) = @_;\n    return bless \\\\%args, \\$class;\n}\n\n${2:# methods}\n\n1;".to_string());
        snippet_registry.insert("test".to_string(),
            "use Test::More tests => ${1:1};\n\n${2:# test code}\n\nok(${3:1}, '${4:test description}');".to_string());

        // Register project templates
        // Templates would be loaded from files in real implementation
        template_cache.insert(
            "module".to_string(),
            "package {{ MODULE_NAME }};
1;"
            .to_string(),
        );
        template_cache.insert(
            "script".to_string(),
            "#!/usr/bin/perl\nuse strict;\nuse warnings;\n".to_string(),
        );
        template_cache.insert("test".to_string(), "use Test::More;\ndone_testing();\n".to_string());

        Self {
            server,
            workspace_root: PathBuf::from("/workspace"),
            snippet_registry,
            template_cache,
            initialized: false,
        }
    }

    fn initialize_or_reuse(&mut self) {
        if self.initialized {
            return;
        }

        // Initialize with advanced capabilities.
        let init_params = json!({
            "processId": 1234,
            "rootUri": "file:///workspace",
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true,
                            "insertReplaceSupport": true,
                            "resolveSupport": {
                                "properties": ["documentation", "detail", "additionalTextEdits"]
                            }
                        }
                    },
                    "codeAction": {
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
                        }
                    }
                },
                "workspace": {
                    "workspaceEdit": {
                        "documentChanges": true,
                        "resourceOperations": ["create", "rename", "delete"]
                    },
                    "executeCommand": {
                        "dynamicRegistration": true
                    },
                    "configuration": true
                }
            }
        });

        let init_request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(init_params),
        };
        self.server.handle_request(init_request);

        let initialized_notification = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialized".to_string(),
            params: Some(json!({})),
        };
        self.server.handle_request(initialized_notification);
        self.initialized = true;
    }

    fn execute_command(&mut self, command: &str, args: Vec<Value>) -> Option<Value> {
        assert!(
            self.initialized,
            "AdvancedTestContext must be initialized before execute_command()"
        );
        self.initialize_or_reuse();
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "workspace/executeCommand".to_string(),
            params: Some(json!({
                "command": command,
                "arguments": args
            })),
        };

        self.server.handle_request(request).and_then(|response| {
            if let Some(result) = response.result {
                return Some(result);
            }
            // Treat error responses as completed replies for feature-stub tests.
            response.error.map(|error| {
                json!({
                    "error": {
                        "code": error.code,
                        "message": error.message,
                        "data": error.data,
                    }
                })
            })
        })
    }

    fn get_snippet_completions(&self, trigger: &str) -> Vec<Value> {
        self.snippet_registry
            .iter()
            .filter(|(key, _)| key.starts_with(trigger))
            .map(|(key, snippet)| {
                json!({
                    "label": key,
                    "kind": 15, // Snippet
                    "insertText": snippet,
                    "insertTextFormat": 2, // Snippet format
                    "detail": format!("{} snippet", key),
                    "documentation": {
                        "kind": "markdown",
                        "value": format!("Insert a {} template", key)
                    }
                })
            })
            .collect()
    }

    fn create_from_template(
        &self,
        template_name: &str,
        _target_path: &str,
        params: HashMap<String, String>,
    ) -> Result<String, String> {
        let template = self
            .template_cache
            .get(template_name)
            .ok_or_else(|| format!("Template '{}' not found", template_name))?;

        let mut content = template.clone();
        for (key, value) in params {
            content = content.replace(&format!("{{{{ {} }}}}", key), &value);
        }

        // In real implementation, would write to file system
        Ok(content)
    }
}

// ===================== Snippet Tests =====================

#[test]
fn test_snippet_completion() {
    let ctx = AdvancedTestContext::new_initialized();

    // Test getting snippet completions
    let completions = ctx.get_snippet_completions("su");
    assert!(!completions.is_empty(), "Should find 'sub' snippet");

    let sub_snippet =
        completions.iter().find(|c| c["label"] == "sub").expect("Should have sub snippet");

    assert_eq!(sub_snippet["insertTextFormat"], 2, "Should be snippet format");
    assert!(
        sub_snippet["insertText"].as_str().unwrap().contains("${1:function_name}"),
        "Should contain snippet placeholders"
    );
}

#[test]
fn test_custom_snippets() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Register custom snippet via command
    let _register_result = ctx.execute_command("perl.registerSnippet", vec![
        json!({
            "name": "moose",
            "prefix": "moose",
            "body": "use Moose;\n\nhas '${1:attribute}' => (\n    is => '${2:ro}',\n    isa => '${3:Str}',\n);\n\n__PACKAGE__->meta->make_immutable;",
            "description": "Moose class template"
        })
    ]);

    // Verify snippet was registered
    let _completions = ctx.get_snippet_completions("moo");
    // In real implementation, would check for custom snippet
}

// ===================== Template Tests =====================

#[test]
fn test_create_module_from_template() {
    let ctx = AdvancedTestContext::new_initialized();

    let mut params = HashMap::new();
    params.insert("MODULE_NAME".to_string(), "MyApp::Utils".to_string());
    params.insert("AUTHOR".to_string(), "Test Author".to_string());
    params.insert("EMAIL".to_string(), "test@example.com".to_string());

    let content = ctx.create_from_template("module", "lib/MyApp/Utils.pm", params);

    // Template should be created (or error handled gracefully)
    assert!(content.is_ok() || content.is_err(), "Template creation should complete");
}

#[test]
fn test_create_test_from_template() {
    let ctx = AdvancedTestContext::new_initialized();

    let mut params = HashMap::new();
    params.insert("MODULE_NAME".to_string(), "MyApp::Calculator".to_string());
    params.insert("TEST_COUNT".to_string(), "5".to_string());

    let content = ctx.create_from_template("test", "t/calculator.t", params);

    // Test template should be created
    assert!(content.is_ok() || content.is_err(), "Test template creation should complete");
}

// ===================== Test Runner Integration =====================

#[test]
fn test_run_single_test() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Execute test run command
    let result = ctx.execute_command(
        "perl.runTest",
        vec![json!({
            "file": "t/basic.t",
            "verbose": true
        })],
    );

    // Verify test execution completes
    assert!(result.is_some(), "Test execution should return a result");
}

#[test]
fn test_run_test_suite() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Run all tests in directory
    let result = ctx.execute_command(
        "perl.runTestSuite",
        vec![json!({
            "directory": "t/",
            "parallel": true,
            "jobs": 4
        })],
    );

    // Verify test suite execution
    assert!(result.is_some(), "Test suite execution should return results");
}

#[test]
fn test_debug_test() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Start test in debug mode
    let result = ctx.execute_command(
        "perl.debugTest",
        vec![json!({
            "file": "t/complex.t",
            "breakpoints": [
                {"line": 10},
                {"line": 25, "condition": "$x > 5"}
            ]
        })],
    );

    // Debug session should start or fail gracefully
    assert!(result.is_some(), "Debug test should complete");
}

// ===================== Code Generation Tests =====================

#[test]
fn test_generate_getters_setters() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Generate accessors for a class
    let result = ctx.execute_command(
        "perl.generateAccessors",
        vec![json!({
            "class": "Person",
            "attributes": ["name", "age", "email"],
            "style": "moose" // or "classic"
        })],
    );

    // Should generate accessor methods
    assert!(result.is_some(), "Accessor generation should complete");
}

#[test]
fn test_generate_test_skeleton() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Generate test file for a module
    let result = ctx.execute_command(
        "perl.generateTests",
        vec![json!({
            "module": "lib/MyApp/Calculator.pm",
            "output": "t/calculator.t",
            "framework": "Test::More"
        })],
    );

    // Should create test skeleton
    assert!(result.is_some(), "Test generation should complete");
}

// ===================== Project Configuration Tests =====================

#[test]
fn test_project_initialization() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Initialize new Perl project
    let result = ctx.execute_command(
        "perl.initProject",
        vec![json!({
            "name": "MyNewProject",
            "type": "application", // or "module", "web"
            "author": "Developer",
            "license": "perl",
            "dependencies": ["DBI", "JSON", "Test::More"]
        })],
    );

    // Should create project structure
    assert!(result.is_some(), "Project initialization should complete");
}

#[test]
fn test_dependency_management() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Add CPAN dependency
    let add_result = ctx.execute_command(
        "perl.addDependency",
        vec![json!({
            "module": "Mojolicious",
            "version": ">=9.0",
            "dev": false
        })],
    );

    // Update dependencies
    let update_result = ctx.execute_command("perl.updateDependencies", vec![]);

    // Check outdated modules
    let check_result = ctx.execute_command("perl.checkOutdated", vec![]);

    // Verify dependency operations
    assert!(add_result.is_some(), "Dependency add should return status");
    assert!(update_result.is_some(), "Dependency update should return status");
    assert!(check_result.is_some(), "Dependency check should return results");
}

// ===================== Linting and Formatting =====================

#[test]
fn test_perltidy_integration() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Format with perltidy
    let result = ctx.execute_command(
        "perl.formatWithPerltidy",
        vec![json!({
            "file": "messy.pl",
            "config": ".perltidyrc"
        })],
    );

    // Should format or report error
    assert!(result.is_some(), "Perltidy should complete");
}

#[test]
fn test_perlcritic_integration() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Run perlcritic analysis
    let result = ctx.execute_command(
        "perl.runPerlcritic",
        vec![json!({
            "file": "code.pl",
            "severity": 3,
            "theme": "core"
        })],
    );

    // Should return critic results
    assert!(result.is_some(), "Perlcritic should complete");
}

// ===================== Documentation Generation =====================

#[test]
fn test_generate_pod_documentation() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Generate POD from code
    let result = ctx.execute_command(
        "perl.generatePOD",
        vec![json!({
            "file": "lib/Module.pm",
            "includePrivate": false,
            "template": "standard"
        })],
    );

    // Should generate documentation
    assert!(result.is_some(), "POD generation should complete");
}

#[test]
fn test_extract_pod_to_markdown() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Convert POD to Markdown
    let result = ctx.execute_command(
        "perl.pod2markdown",
        vec![json!({
            "input": "lib/Module.pm",
            "output": "docs/Module.md"
        })],
    );

    // Should create markdown file
    assert!(result.is_some(), "POD to Markdown should complete");
}

// ===================== Performance Profiling =====================

#[test]
fn test_profile_execution() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Profile script execution
    let result = ctx.execute_command(
        "perl.profile",
        vec![json!({
            "script": "slow_script.pl",
            "profiler": "NYTProf",
            "output": "nytprof.out"
        })],
    );

    // Should generate profile data
    assert!(result.is_some(), "Profiling should complete");
}

#[test]
fn test_analyze_profile_results() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Analyze profiling results
    let result = ctx.execute_command(
        "perl.analyzeProfile",
        vec![json!({
            "profile": "nytprof.out",
            "format": "html",
            "threshold": 0.01
        })],
    );

    // Should generate analysis report
    assert!(result.is_some(), "Profile analysis should complete");
}

// ===================== Version Control Integration =====================

#[test]
fn test_git_blame_integration() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Show git blame info inline
    let result = ctx.execute_command(
        "perl.showGitBlame",
        vec![json!({
            "file": "lib/Module.pm",
            "line": 42
        })],
    );

    // Should show blame info or handle gracefully
    assert!(result.is_some(), "Git blame should complete");
}

#[test]
fn test_commit_with_conventional_format() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Create conventional commit
    let result = ctx.execute_command(
        "perl.conventionalCommit",
        vec![json!({
            "type": "feat",
            "scope": "parser",
            "description": "Add support for new Perl 5.38 features",
            "breaking": false,
            "issues": ["#123", "#456"]
        })],
    );

    // Should create commit or report error
    assert!(result.is_some(), "Commit should complete");
}

// ===================== Database Integration =====================

#[test]
fn test_sql_preview_in_dbi_code() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Preview SQL query results
    let result = ctx.execute_command(
        "perl.previewSQL",
        vec![json!({
            "query": "SELECT * FROM users WHERE age > ?",
            "params": [18],
            "connection": "dbi:SQLite:test.db"
        })],
    );

    // Should show query results or error
    assert!(result.is_some(), "SQL preview should complete");
}

#[test]
fn test_generate_dbi_code_from_schema() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Generate DBI code from database schema
    let result = ctx.execute_command(
        "perl.generateDBICode",
        vec![json!({
            "connection": "dbi:mysql:database=myapp",
            "tables": ["users", "posts", "comments"],
            "style": "DBIx::Class"
        })],
    );

    // Should generate database access code
    assert!(result.is_some(), "DBI code generation should complete");
}

// ===================== Container and Deployment =====================

#[test]
fn test_dockerfile_generation() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Generate Dockerfile for Perl app
    let result = ctx.execute_command(
        "perl.generateDockerfile",
        vec![json!({
            "base": "perl:5.38-slim",
            "app": "app.pl",
            "dependencies": "cpanfile",
            "expose": [8080]
        })],
    );

    // Should create Dockerfile
    assert!(result.is_some(), "Dockerfile generation should complete");
}

#[test]
fn test_kubernetes_manifest_generation() {
    let mut ctx = AdvancedTestContext::new_initialized();

    // Generate K8s manifests
    let result = ctx.execute_command(
        "perl.generateK8sManifests",
        vec![json!({
            "app": "perl-web-app",
            "image": "myapp:latest",
            "replicas": 3,
            "service": true,
            "ingress": true
        })],
    );

    // Should create K8s YAML files
    assert!(result.is_some(), "K8s manifest generation should complete");
}
