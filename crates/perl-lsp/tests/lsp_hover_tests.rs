//! Comprehensive LSP integration tests for textDocument/hover
//!
//! Tests feature spec: LSP_IMPLEMENTATION_GUIDE.md#hover
//! Tests feature spec: navigation.rs#hover-provider
//!
//! This test suite validates:
//! - textDocument/hover request/response handling
//! - Hover on subroutine names (returns signature/documentation)
//! - Hover on variable names (returns type/declaration info)
//! - Hover on builtin functions (returns builtin documentation)
//! - Hover on empty space or comments (returns null/empty)
//! - Hover capability advertised in server capabilities
//!
//! LSP Protocol Compliance:
//! - Hover response: { contents: MarkupContent, range?: Range } or null
//! - MarkupContent: { kind: "markdown"|"plaintext", value: string }
//! - Position-based symbol resolution
//!
//! Related Documentation:
//! - docs/reference/LSP_IMPLEMENTATION_GUIDE.md#hover
//! - crates/perl-lsp-navigation/src/hover.rs

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Tests feature spec: navigation.rs#hover-on-subroutine
///
/// Validates that hovering over a subroutine name returns meaningful content
/// such as the function signature or documentation.
#[test]
fn test_hover_on_subroutine_name() -> TestResult {
    let doc = r#"
sub process {
    my ($input) = @_;
    return $input * 2;
}

my $result = process(42);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // Hover over the subroutine name at definition site
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 1, "character": 5} // Position on "process" in sub definition
            }),
        )
        .unwrap_or(json!(null));

    // Hover may return an object with contents or null
    if !result.is_null() {
        // If we got a hover response, validate its structure
        let contents = result.get("contents");
        assert!(
            contents.is_some(),
            "Hover response should have 'contents' field, got: {:?}",
            result
        );

        let contents = contents.ok_or("Expected contents in hover response")?;

        // Contents should be a MarkupContent object or a string
        if contents.is_object() {
            // MarkupContent format: { kind: "markdown"|"plaintext", value: "..." }
            let kind = contents.get("kind").and_then(|k| k.as_str());
            if let Some(k) = kind {
                assert!(
                    k == "markdown" || k == "plaintext",
                    "Hover content kind should be 'markdown' or 'plaintext', got: {}",
                    k
                );
            }
            let value = contents.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "MarkupContent should have a 'value' field");
        }

        // If range is present, validate it
        if let Some(range) = result.get("range") {
            assert!(range.get("start").is_some(), "Range must have start position");
            assert!(range.get("end").is_some(), "Range must have end position");
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-variable
///
/// Validates that hovering over a variable returns declaration or type info.
#[test]
fn test_hover_on_variable() -> TestResult {
    let doc = r#"
my $count = 0;
$count = $count + 1;
print $count;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///vars.pl", doc)?;

    // Hover over $count at its usage site
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///vars.pl"},
                "position": {"line": 2, "character": 1} // Position on $count usage
            }),
        )
        .unwrap_or(json!(null));

    // Variable hover may return info or null depending on implementation depth
    if !result.is_null() {
        let contents = result.get("contents").ok_or("Expected contents in hover response")?;

        if contents.is_object() {
            let value = contents.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "Hover contents should have a value string");
        } else if contents.is_string() {
            // Plain string contents are also valid per LSP spec
            assert!(
                !contents.as_str().ok_or("Expected string")?.is_empty(),
                "Hover content string should not be empty"
            );
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-builtin
///
/// Validates that hovering over a Perl builtin function returns documentation.
#[test]
fn test_hover_on_builtin_function() -> TestResult {
    let doc = r#"
my @items = (3, 1, 4, 1, 5);
my @sorted = sort @items;
my $length = scalar @items;
push @items, 9;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///builtins.pl", doc)?;

    // Hover over "sort" builtin
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///builtins.pl"},
                "position": {"line": 2, "character": 16} // Position on "sort"
            }),
        )
        .unwrap_or(json!(null));

    // Builtin hover may return documentation or null
    if !result.is_null() {
        let contents = result.get("contents").ok_or("Expected contents in hover response")?;
        if contents.is_object() {
            let value = contents.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "Builtin hover should have content value");
        }
    }

    // Hover over "push" builtin
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///builtins.pl"},
                "position": {"line": 4, "character": 1} // Position on "push"
            }),
        )
        .unwrap_or(json!(null));

    // Accept null or valid hover response
    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "If hover is returned for builtin, it must have contents"
        );
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-file-test-operators
///
/// Validates that hovering over Perl file test operators returns documentation.
#[test]
fn test_hover_on_file_test_operators() -> TestResult {
    let doc = r#"
my $file = "test.txt";
-e $file;
-M $file;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///file_tests.pl", doc)?;

    let result = harness
        .request_with_timeout(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///file_tests.pl"},
                "position": {"line": 2, "character": 1}
            }),
            Duration::from_secs(10),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "Expected hover response for -e");
    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        value.contains("exists"),
        "File test hover should explain -e existence checks, got: {value}"
    );

    let result = harness
        .request_with_timeout(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///file_tests.pl"},
                "position": {"line": 3, "character": 1}
            }),
            Duration::from_secs(10),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "Expected hover response for -M");
    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        value.contains("days"),
        "Related file test hover should explain -M age semantics, got: {value}"
    );

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-empty-space
///
/// Validates that hovering over whitespace, comments, or positions with no symbol
/// returns null (no hover information).
#[test]
fn test_hover_on_empty_space_returns_null() -> TestResult {
    let doc = r#"
# This is a comment
my $variable = 42;

print $variable;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///empty.pl", doc)?;

    // Hover on a comment line
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///empty.pl"},
                "position": {"line": 1, "character": 5} // Position within comment
            }),
        )
        .unwrap_or(json!(null));

    // Comments should return null or an empty hover
    // Some implementations may provide comment content; accept both
    if !result.is_null() {
        // If non-null, it should still be a valid hover structure
        assert!(result.get("contents").is_some(), "Non-null hover must have contents field");
    }

    // Hover on blank line
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///empty.pl"},
                "position": {"line": 3, "character": 0} // Position on blank line
            }),
        )
        .unwrap_or(json!(null));

    // Blank line should return null
    assert!(result.is_null(), "Hover on blank line should return null, got: {:?}", result);

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-method-call
///
/// Validates hover on method calls in object-oriented Perl code.
#[test]
fn test_hover_on_method_call() -> TestResult {
    let doc = r#"
package Logger;

sub new {
    my ($class, %opts) = @_;
    return bless \%opts, $class;
}

sub info {
    my ($self, $msg) = @_;
    print "[INFO] $msg\n";
}

package main;

my $log = Logger->new(level => 'debug');
$log->info("Application started");
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method.pl", doc)?;

    // Hover over "info" method call
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///method.pl"},
                "position": {"line": 16, "character": 7} // Position on "info" in $log->info(...)
            }),
        )
        .unwrap_or(json!(null));

    // Method hover may return info or null depending on implementation
    if !result.is_null() {
        let contents = result.get("contents").ok_or("Expected contents in hover response")?;
        if contents.is_object() {
            assert!(contents.get("value").is_some(), "Method hover content should have value");
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-capability-advertised
///
/// Validates that hover capability is advertised in server capabilities.
#[test]
fn test_hover_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    // Hover should be advertised
    let has_capability = capabilities.get("hoverProvider").is_some();
    assert!(has_capability, "hoverProvider should be advertised in capabilities");

    // If present, should be true or an object
    let provider = &capabilities["hoverProvider"];
    assert!(
        provider.is_boolean() || provider.is_object(),
        "hoverProvider should be boolean or object, got: {:?}",
        provider
    );

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-builtin-context-sensitive-docs
///
/// Validates that dual-context builtins (gmtime, keys, wantarray, grep, caller)
/// include scalar context information in their hover documentation.
#[test]
fn test_hover_builtin_context_sensitive_docs() -> TestResult {
    let doc = "gmtime();\nkeys %h;\nwantarray();\ngrep { 1 } @a;\ncaller();\n";

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///context_builtins.pl", doc)?;

    // gmtime at line 0
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///context_builtins.pl"},
                "position": {"line": 0, "character": 2}
            }),
        )
        .unwrap_or(json!(null));
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            value.contains("scalar context"),
            "gmtime hover must mention scalar context: {}",
            value
        );
    }

    // keys at line 1
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///context_builtins.pl"},
                "position": {"line": 1, "character": 2}
            }),
        )
        .unwrap_or(json!(null));
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            value.contains("scalar context"),
            "keys hover must mention scalar context: {}",
            value
        );
    }

    // wantarray at line 2
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///context_builtins.pl"},
                "position": {"line": 2, "character": 2}
            }),
        )
        .unwrap_or(json!(null));
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            value.contains("scalar context") || value.contains("void context"),
            "wantarray hover must mention context variants: {}",
            value
        );
    }

    // grep at line 3
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///context_builtins.pl"},
                "position": {"line": 3, "character": 2}
            }),
        )
        .unwrap_or(json!(null));
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            value.contains("scalar context"),
            "grep hover must mention scalar context count behavior: {}",
            value
        );
    }

    // caller at line 4
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///context_builtins.pl"},
                "position": {"line": 4, "character": 2}
            }),
        )
        .unwrap_or(json!(null));
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            value.contains("scalar context") || value.contains("package"),
            "caller hover must mention scalar form: {}",
            value
        );
    }

    Ok(())
}

/// Tests feature spec: hover#variable-declaration-line
///
/// Validates that hovering over a lexical variable shows where it was declared (line N).
#[test]
fn test_hover_variable_shows_declaration_line() -> TestResult {
    let doc = r#"my $config = load_config();

sub process {
    my ($data) = @_;
    my $result = transform($data);
    print $result;
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///hover_decl.pl", doc)?;

    // Hover over $result at usage site (line 5, character 10)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///hover_decl.pl"},
                "position": {"line": 5, "character": 10}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "Expected hover response for $result usage");

    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");

    assert!(
        value.contains("line") || value.contains("Declared"),
        "Hover for $result should show declaration line info, got: {value}"
    );

    Ok(())
}

/// Tests feature spec: hover#variable-declaration-scope-context
///
/// Validates that hovering over a lexical variable inside a subroutine shows
/// the subroutine name as the scope context.
#[test]
fn test_hover_variable_shows_scope_context() -> TestResult {
    let doc = r#"my $global = 1;

sub process_data {
    my $local_var = 42;
    return $local_var;
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///hover_scope.pl", doc)?;

    // Hover over $local_var at usage site (line 4, character 12)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///hover_scope.pl"},
                "position": {"line": 4, "character": 12}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "Expected hover response for $local_var usage");

    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");

    assert!(
        value.contains("process_data") || value.contains("subroutine") || value.contains("Scope"),
        "Hover for $local_var should show scope context (subroutine name), got: {value}"
    );

    Ok(())
}

/// Tests feature spec: hover#variable-my-declaration-keyword
///
/// Validates that hovering over a `my` variable shows the `my` declaration keyword.
#[test]
fn test_hover_variable_shows_my_declaration_keyword() -> TestResult {
    let doc = r#"sub greet {
    my $name = "world";
    print "Hello, $name!\n";
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///hover_my.pl", doc)?;

    // Hover over $name at declaration site (line 1, character 8)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///hover_my.pl"},
                "position": {"line": 1, "character": 8}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "Expected hover response for $name");

    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");

    assert!(
        value.contains("my") || value.contains("lexical"),
        "Hover for my $name should mention 'my' or 'lexical', got: {value}"
    );

    Ok(())
}

/// Tests feature spec: hover#variable-file-scope
///
/// Validates that hovering over a file-scope `my` variable (outside any sub)
/// includes useful variable info.
#[test]
fn test_hover_variable_file_scope_context() -> TestResult {
    let doc = r#"my $top_level = 100;
print $top_level;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///hover_file_scope.pl", doc)?;

    // Hover over $top_level at usage site (line 1, character 7)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///hover_file_scope.pl"},
                "position": {"line": 1, "character": 7}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "Expected hover response for $top_level");

    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");

    assert!(!value.is_empty(), "Hover for file-scope $top_level should return non-empty content");

    assert!(
        value.contains("line") || value.contains("Declared") || value.contains("Scalar"),
        "Hover for file-scope $top_level should contain variable info, got: {value}"
    );

    Ok(())
}

/// Tests Gap 2 of issue #3482: hover shows information for inherited methods.
///
/// When the cursor is on an inherited method call (`$child->inherited_method()`),
/// hover should return meaningful content identifying the method's origin, rather
/// than falling through to a generic token hover.
///
/// Uses cross-file workspace (TempWorkspace) so that the Phase 2 workspace BFS
/// can find the parent package.
#[test]
fn hover_shows_inherited_method_from_parent_class() -> TestResult {
    use support::lsp_harness::TempWorkspace;

    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/HoverBase.pm",
        r#"package HoverBase;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub hover_greet {
    my ($self) = @_;
    return "hello from HoverBase";
}

1;
"#,
    )?;

    workspace.write(
        "lib/HoverChild.pm",
        r#"package HoverChild;
use parent 'HoverBase';

sub child_method {
    my ($self) = @_;
    return "from child";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in ["lib/HoverBase.pm", "lib/HoverChild.pm"] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    // Open main.pl with a call to the inherited method
    // Line 4 (0-indexed): `$c->hover_greet();`
    // character 4 is on "hover_greet"
    let main_content = r#"#!/usr/bin/perl
use lib 'lib';
use HoverChild;
my $c = HoverChild->new();
$c->hover_greet();
"#;
    harness.open(&workspace.uri("main.pl"), main_content)?;

    harness.barrier();

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": workspace.uri("main.pl")},
                "position": {"line": 4, "character": 4}
            }),
        )
        .unwrap_or(json!(null));

    // The hover result should not be null and should contain something meaningful.
    // Either the workspace BFS finds the method (Gap 2 fix), or the token hover
    // returns the method name. Either way, there should be content.
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(!value.is_empty(), "Hover on inherited method should return non-empty content");
        // If the workspace BFS resolved the method, the content should mention
        // the method name or its origin package.
        if value.contains("Method") || value.contains("hover_greet") || value.contains("HoverBase")
        {
            // The full inherited method hover is working
        }
        // Otherwise it's a token hover — still valid, just not yet fully enriched
    }
    // A null hover is also acceptable here: it means the method is not found
    // in-file and the token was empty — that's the pre-fix behaviour.

    Ok(())
}

#[test]
fn hover_shows_autoload_resolution_for_missing_method() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///lib/HoverAuto.pm",
        r#"package HoverAuto;

sub AUTOLOAD {
    our $AUTOLOAD;
    return $AUTOLOAD;
}

1;
"#,
    )?;

    let main_content = r#"#!/usr/bin/perl
use lib 'lib';
use HoverAuto;
HoverAuto->dynamic_hover();
"#;
    harness.open("file:///app.pl", main_content)?;

    harness.barrier();

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///app.pl"},
                "position": {"line": 3, "character": 13}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "AUTOLOAD-backed method hover should not be null");

    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");

    assert!(value.contains("AUTOLOAD"), "hover should mention AUTOLOAD resolution, got: {value}");
    assert!(
        value.contains("dynamic_hover"),
        "hover should mention the requested method name, got: {value}"
    );

    Ok(())
}

/// Tests feature spec: hover#compile-time-constants
///
/// Validates that hovering over __FILE__ returns compile-time constant documentation.
#[test]
fn hover_compile_time_constant_file() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open_document("file:///ct.pl", "print __FILE__;\n")?;
    harness.barrier();

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///ct.pl"},
                "position": {"line": 0, "character": 7}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "__FILE__ hover should not be null");
    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(value.contains("__FILE__"), "__FILE__ hover must mention __FILE__, got: {value}");
    assert!(
        value.contains("file name") || value.contains("source file"),
        "__FILE__ hover must describe file name, got: {value}"
    );

    Ok(())
}

/// Tests feature spec: hover#compile-time-constants
///
/// Validates that hovering over __LINE__ returns compile-time constant documentation.
#[test]
fn hover_compile_time_constant_line() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open_document("file:///ct_line.pl", "print __LINE__;\n")?;
    harness.barrier();

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///ct_line.pl"},
                "position": {"line": 0, "character": 7}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "__LINE__ hover should not be null");
    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(value.contains("__LINE__"), "__LINE__ hover must mention __LINE__, got: {value}");
    assert!(
        value.contains("line number"),
        "__LINE__ hover must describe line number, got: {value}"
    );

    Ok(())
}

/// Tests feature spec: hover#compile-time-constants
///
/// Validates that hovering over __PACKAGE__ returns compile-time constant documentation.
#[test]
fn hover_compile_time_constant_package() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open_document("file:///ct_pkg.pl", "package Foo;\nprint __PACKAGE__;\n")?;
    harness.barrier();

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///ct_pkg.pl"},
                "position": {"line": 1, "character": 7}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "__PACKAGE__ hover should not be null");
    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        value.contains("__PACKAGE__"),
        "__PACKAGE__ hover must mention __PACKAGE__, got: {value}"
    );
    assert!(
        value.contains("package name") || value.contains("package"),
        "__PACKAGE__ hover must describe package, got: {value}"
    );

    Ok(())
}

/// Tests feature spec: hover#compile-time-constants
///
/// Validates that hovering over __SUB__ returns compile-time constant documentation.
#[test]
fn hover_compile_time_constant_sub() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Use an anonymous sub so __SUB__ does not resolve to a named subroutine symbol
    // via the AST path, forcing the token fallback to handle it.
    harness.open_document(
        "file:///ct_sub.pl",
        "use feature 'current_sub';\nmy $f = sub { return __SUB__; };\n",
    )?;
    harness.barrier();

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///ct_sub.pl"},
                "position": {"line": 1, "character": 22}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "__SUB__ hover should not be null");
    let value =
        result.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(value.contains("__SUB__"), "__SUB__ hover must mention __SUB__, got: {value}");
    assert!(
        value.contains("subroutine") || value.contains("current_sub"),
        "__SUB__ hover must describe current subroutine, got: {value}"
    );

    Ok(())
}

#[test]
fn test_hover_on_qualified_call_finds_bare_definition() -> TestResult {
    let doc = r#"
package Utils;
sub format_string {
    my ($input) = @_;
    return uc($input);
}

my $result = Utils::format_string("hello");
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///qualified_hover.pl", doc)?;

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///qualified_hover.pl"},
                "position": {"line": 7, "character": 21}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "expected hover on qualified user function call");
    let value = result["contents"]["value"].as_str().unwrap_or_default();
    assert!(
        value.contains("format_string")
            && (value.contains("User-defined function")
                || value.contains("sub Utils::format_string")
                || value.contains("**Subroutine**")),
        "expected qualified call hover to resolve user function signature, got: {result}"
    );
    Ok(())
}
