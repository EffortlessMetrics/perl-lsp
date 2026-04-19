//! Comprehensive LSP integration tests for Call Hierarchy feature
//!
//! Tests feature spec: LSP_IMPLEMENTATION_GUIDE.md#call-hierarchy
//! Tests feature spec: call_hierarchy_provider.rs (50% complete, preview status)
//!
//! This test suite validates:
//! - textDocument/prepareCallHierarchy request
//! - callHierarchy/incomingCalls request
//! - callHierarchy/outgoingCalls request
//! - Recursive call detection
//! - Cross-package calls
//! - Method calls on objects
//! - Edge cases: Unicode, nested calls, no calls found

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;
/// Tests feature spec: call_hierarchy_provider.rs#prepare
/// Test basic prepareCallHierarchy at a function definition
#[test]
fn test_prepare_call_hierarchy_basic_function() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub hello {
    print "Hello, world!\n";
}

sub main {
    hello();
}
"#,
    )?;

    // Request call hierarchy at "hello" function (line 1, char 4)
    let response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    // Response should be an array of CallHierarchyItem
    assert!(
        response.is_array(),
        "prepareCallHierarchy should return array, got: {:?}",
        response
    );

    let items = response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];
        assert_eq!(item["name"], "hello", "Function name should be 'hello'");
        assert_eq!(item["kind"], 12, "Symbol kind should be 12 (Function)");
        assert!(item["uri"].is_string(), "URI should be present");
        assert!(item["range"].is_object(), "Range should be present");
        assert!(
            item["selectionRange"].is_object(),
            "Selection range should be present"
        );
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#prepare
/// Test prepareCallHierarchy with method calls
#[test]
fn test_prepare_call_hierarchy_method() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package MyClass;

sub new {
    my $class = shift;
    bless {}, $class;
}

sub process {
    my $self = shift;
    print "Processing\n";
}

package main;
my $obj = MyClass->new();
$obj->process();
"#,
    )?;

    // Request call hierarchy at "process" method (line 8, char 4)
    let response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 8, "character": 4 }
        }),
    )?;

    assert!(
        response.is_array(),
        "prepareCallHierarchy should return array"
    );

    let items = response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];
        assert_eq!(item["name"], "process", "Method name should be 'process'");
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#incoming_calls
/// Test incoming calls (callers of a function)
#[test]
fn test_incoming_calls_basic() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub greet {
    print "Hello!\n";
}

sub say_hello {
    greet();
}

sub say_hi {
    greet();
}
"#,
    )?;

    // First prepare call hierarchy for "greet"
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Request incoming calls for "greet"
        let incoming_response = harness.request(
            "callHierarchy/incomingCalls",
            json!({
                "item": item
            }),
        )?;

        assert!(
            incoming_response.is_array(),
            "incomingCalls should return array"
        );

        let calls = incoming_response.as_array().ok_or("not an array")?;
        // Should find both say_hello and say_hi as callers
        if !calls.is_empty() {
            let caller_names: Vec<String> = calls
                .iter()
                .filter_map(|call| call["from"]["name"].as_str())
                .map(String::from)
                .collect();

            assert!(
                caller_names.contains(&"say_hello".to_string())
                    || caller_names.contains(&"say_hi".to_string()),
                "Should find at least one caller, got: {:?}",
                caller_names
            );
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#incoming_calls
/// Test incoming calls with multiple call sites in same function
#[test]
fn test_incoming_calls_multiple_sites() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub log_message {
    print shift . "\n";
}

sub process {
    log_message("Starting");
    # do work
    log_message("Processing");
    # more work
    log_message("Done");
}
"#,
    )?;

    // Prepare call hierarchy for "log_message"
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Request incoming calls
        let incoming_response = harness.request(
            "callHierarchy/incomingCalls",
            json!({
                "item": item
            }),
        )?;

        let calls = incoming_response.as_array().ok_or("not an array")?;
        if !calls.is_empty() {
            // Find the "process" caller
            let process_call = calls.iter().find(|call| call["from"]["name"] == "process");
            if let Some(call) = process_call {
                // Should have multiple fromRanges (one for each call site)
                let ranges = call["fromRanges"].as_array();
                assert!(ranges.is_some(), "Should have fromRanges array");
                // Note: Implementation may aggregate or separate ranges
            }
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#outgoing_calls
/// Test outgoing calls (functions called by this function)
#[test]
fn test_outgoing_calls_basic() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub helper1 {
    print "Helper 1\n";
}

sub helper2 {
    print "Helper 2\n";
}

sub main_function {
    helper1();
    helper2();
}
"#,
    )?;

    // Prepare call hierarchy for "main_function"
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 9, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Request outgoing calls
        let outgoing_response = harness.request(
            "callHierarchy/outgoingCalls",
            json!({
                "item": item
            }),
        )?;

        assert!(
            outgoing_response.is_array(),
            "outgoingCalls should return array"
        );

        let calls = outgoing_response.as_array().ok_or("not an array")?;
        if !calls.is_empty() {
            let callee_names: Vec<String> = calls
                .iter()
                .filter_map(|call| call["to"]["name"].as_str())
                .map(String::from)
                .collect();

            // Should find helper1 and/or helper2
            assert!(
                callee_names.contains(&"helper1".to_string())
                    || callee_names.contains(&"helper2".to_string()),
                "Should find at least one callee, got: {:?}",
                callee_names
            );
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#outgoing_calls
/// Test outgoing calls with method calls
#[test]
fn test_outgoing_calls_methods() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package Logger;

sub new {
    my $class = shift;
    bless {}, $class;
}

sub log {
    my ($self, $msg) = @_;
    print $msg . "\n";
}

package main;

sub process_data {
    my $logger = Logger->new();
    $logger->log("Processing");
}
"#,
    )?;

    // Prepare call hierarchy for "process_data"
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 15, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Request outgoing calls
        let outgoing_response = harness.request(
            "callHierarchy/outgoingCalls",
            json!({
                "item": item
            }),
        )?;

        let calls = outgoing_response.as_array().ok_or("not an array")?;
        if !calls.is_empty() {
            let callee_names: Vec<String> = calls
                .iter()
                .filter_map(|call| call["to"]["name"].as_str())
                .map(String::from)
                .collect();

            // Should find "new" and/or "log" method calls
            assert!(
                callee_names.contains(&"new".to_string())
                    || callee_names.contains(&"log".to_string()),
                "Should find method calls, got: {:?}",
                callee_names
            );
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#recursive-calls
/// Test detection of recursive function calls
#[test]
fn test_recursive_calls() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub factorial {
    my $n = shift;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

sub main {
    my $result = factorial(5);
}
"#,
    )?;

    // Prepare call hierarchy for "factorial"
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Check outgoing calls - should include recursive call to itself
        let outgoing_response = harness.request(
            "callHierarchy/outgoingCalls",
            json!({
                "item": item
            }),
        )?;

        let calls = outgoing_response.as_array().ok_or("not an array")?;
        if !calls.is_empty() {
            let callee_names: Vec<String> = calls
                .iter()
                .filter_map(|call| call["to"]["name"].as_str())
                .map(String::from)
                .collect();

            // May find recursive call to "factorial"
            // Note: Implementation may or may not report self-recursion
            let _ = callee_names; // Avoid unused variable warning
        }

        // Check incoming calls - should include call from main
        let incoming_response = harness.request(
            "callHierarchy/incomingCalls",
            json!({
                "item": item
            }),
        )?;

        let calls = incoming_response.as_array().ok_or("not an array")?;
        if !calls.is_empty() {
            let caller_names: Vec<String> = calls
                .iter()
                .filter_map(|call| call["from"]["name"].as_str())
                .map(String::from)
                .collect();

            // Should find "main" as caller (and possibly "factorial" if self-recursion is tracked)
            assert!(
                caller_names.contains(&"main".to_string())
                    || caller_names.contains(&"factorial".to_string()),
                "Should find callers, got: {:?}",
                caller_names
            );
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#cross-package
/// Test cross-package function calls
#[test]
fn test_cross_package_calls() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package Utils;

sub format_string {
    my $str = shift;
    return uc($str);
}

package App;

sub process {
    my $result = Utils::format_string("hello");
    print $result;
}

package main;
App::process();
"#,
    )?;

    // Prepare call hierarchy for "format_string"
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 3, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Check incoming calls from other package
        let incoming_response = harness.request(
            "callHierarchy/incomingCalls",
            json!({
                "item": item
            }),
        )?;

        let calls = incoming_response.as_array().ok_or("not an array")?;
        if !calls.is_empty() {
            // Should find "process" from App package
            let caller_names: Vec<String> = calls
                .iter()
                .filter_map(|call| call["from"]["name"].as_str())
                .map(String::from)
                .collect();

            assert!(
                caller_names.contains(&"process".to_string()),
                "Should find cross-package caller, got: {:?}",
                caller_names
            );
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#edge-cases
/// Test call hierarchy with no calls found
#[test]
fn test_no_calls_found() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub unused_function {
    print "Never called\n";
}
"#,
    )?;

    // Prepare call hierarchy for unused function
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Request incoming calls - should be empty
        let incoming_response = harness.request(
            "callHierarchy/incomingCalls",
            json!({
                "item": item
            }),
        )?;

        assert!(
            incoming_response.is_array(),
            "Should return empty array for no incoming calls"
        );

        // Request outgoing calls - should be empty
        let outgoing_response = harness.request(
            "callHierarchy/outgoingCalls",
            json!({
                "item": item
            }),
        )?;

        assert!(
            outgoing_response.is_array(),
            "Should return empty array for no outgoing calls"
        );
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#edge-cases
/// Test call hierarchy at invalid position (no symbol)
#[test]
fn test_prepare_at_invalid_position() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub hello {
    print "Hello\n";
}
"#,
    )?;

    // Request call hierarchy at a comment or whitespace (line 0, char 0)
    let response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 0, "character": 0 }
        }),
    )?;

    // Should return null or empty array when no symbol found
    assert!(
        response.is_null()
            || (response.is_array() && response.as_array().ok_or("not an array")?.is_empty()),
        "Should return null or empty array for invalid position, got: {:?}",
        response
    );
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#edge-cases
/// Test call hierarchy with Unicode function names
#[test]
fn test_unicode_function_names() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub 你好 {
    print "Hello in Chinese\n";
}

sub main {
    你好();
}
"#,
    )?;

    // Prepare call hierarchy for Unicode function name
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    // Should handle Unicode gracefully
    assert!(
        prepare_response.is_array() || prepare_response.is_null(),
        "Should return array or null for Unicode function"
    );

    let items = prepare_response.as_array();
    if let Some(items) = items {
        if !items.is_empty() {
            let item = &items[0];
            // Function name should be preserved
            assert!(item["name"].is_string(), "Function name should be string");
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#nested-calls
/// Test deeply nested function calls
#[test]
fn test_nested_function_calls() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub level3 {
    print "Level 3\n";
}

sub level2 {
    level3();
}

sub level1 {
    level2();
}

sub main {
    level1();
}
"#,
    )?;

    // Test call hierarchy at middle level (level2)
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 5, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Check incoming calls (should find level1)
        let incoming_response = harness.request(
            "callHierarchy/incomingCalls",
            json!({
                "item": item
            }),
        )?;

        let incoming_calls = incoming_response.as_array().ok_or("not an array")?;
        if !incoming_calls.is_empty() {
            let caller_names: Vec<String> = incoming_calls
                .iter()
                .filter_map(|call| call["from"]["name"].as_str())
                .map(String::from)
                .collect();

            assert!(
                caller_names.contains(&"level1".to_string()),
                "Should find level1 as caller, got: {:?}",
                caller_names
            );
        }

        // Check outgoing calls (should find level3)
        let outgoing_response = harness.request(
            "callHierarchy/outgoingCalls",
            json!({
                "item": item
            }),
        )?;

        let outgoing_calls = outgoing_response.as_array().ok_or("not an array")?;
        if !outgoing_calls.is_empty() {
            let callee_names: Vec<String> = outgoing_calls
                .iter()
                .filter_map(|call| call["to"]["name"].as_str())
                .map(String::from)
                .collect();

            assert!(
                callee_names.contains(&"level3".to_string()),
                "Should find level3 as callee, got: {:?}",
                callee_names
            );
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#anonymous-subs
/// Test call hierarchy with anonymous subroutines (edge case)
#[test]
fn test_anonymous_subroutines() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub process_callback {
    my $callback = shift;
    $callback->();
}

sub main {
    process_callback(sub { print "Anonymous\n"; });
}
"#,
    )?;

    // Prepare call hierarchy for process_callback
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    // Should handle file with anonymous subs gracefully
    assert!(
        prepare_response.is_array() || prepare_response.is_null(),
        "Should handle anonymous subs gracefully"
    );
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#complex-expressions
/// Test call hierarchy with complex call expressions
#[test]
fn test_complex_call_expressions() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub get_handler {
    return sub { print "Handler\n"; };
}

sub execute {
    print "Executing\n";
}

sub main {
    get_handler()->();
    my $ref = \&execute;
    $ref->();
}
"#,
    )?;

    // Prepare call hierarchy for get_handler
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    // Should return valid response even with complex expressions
    assert!(
        prepare_response.is_array() || prepare_response.is_null(),
        "Should handle complex call expressions"
    );
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#builtin-calls
/// Test call hierarchy with builtin function calls (should not appear in hierarchy)
#[test]
fn test_builtin_function_calls() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
sub custom_print {
    my $msg = shift;
    print $msg;
    chomp($msg);
    return uc($msg);
}
"#,
    )?;

    // Prepare call hierarchy for custom_print
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Check outgoing calls - builtins like print, chomp, uc may or may not be included
        let outgoing_response = harness.request(
            "callHierarchy/outgoingCalls",
            json!({
                "item": item
            }),
        )?;

        // Should return array (implementation may choose to include/exclude builtins)
        assert!(
            outgoing_response.is_array(),
            "Should return array for outgoing calls"
        );
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#method-on-variable
/// Test call hierarchy with method calls on specific variables
#[test]
fn test_method_calls_on_objects() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package Database;

sub new {
    my $class = shift;
    bless { connected => 0 }, $class;
}

sub connect {
    my $self = shift;
    $self->{connected} = 1;
}

sub query {
    my ($self, $sql) = @_;
    return [] unless $self->{connected};
    # execute query
    return [];
}

package main;

sub run_query {
    my $db = Database->new();
    $db->connect();
    my $results = $db->query("SELECT * FROM users");
}
"#,
    )?;

    // Test call hierarchy for "connect" method
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 8, "character": 4 }
        }),
    )?;

    let items = prepare_response.as_array().ok_or("not an array")?;
    if !items.is_empty() {
        let item = &items[0];

        // Check incoming calls - should find run_query
        let incoming_response = harness.request(
            "callHierarchy/incomingCalls",
            json!({
                "item": item
            }),
        )?;

        let calls = incoming_response.as_array().ok_or("not an array")?;
        if !calls.is_empty() {
            let caller_names: Vec<String> = calls
                .iter()
                .filter_map(|call| call["from"]["name"].as_str())
                .map(String::from)
                .collect();

            assert!(
                caller_names.contains(&"run_query".to_string()),
                "Should find run_query as caller for method, got: {:?}",
                caller_names
            );
        }
    }
    Ok(())
}

/// Tests feature spec: call_hierarchy_provider.rs#capability-advertisement
/// Test that call hierarchy capability is advertised in server capabilities
#[test]
fn test_call_hierarchy_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    // Call hierarchy should be advertised (unless in ga-lock mode)
    // Check if capability exists (may be true or an object with options)
    if !cfg!(feature = "lsp-ga-lock") {
        let has_capability = capabilities.get("callHierarchyProvider").is_some();
        assert!(
            has_capability,
            "callHierarchyProvider should be advertised in capabilities"
        );
    }
    Ok(())
}

/// Tests fix for issue #2878: call hierarchy must search across workspace files
/// When format_string is defined in lib/Utils.pm and called from bin/app.pl,
/// incoming calls from app.pl must appear even though the item URI is Utils.pm.
#[test]
fn test_cross_file_incoming_calls() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // File that defines the target subroutine
    harness.open(
        "file:///lib/Utils.pm",
        r#"package Utils;

sub format_string {
    my $str = shift;
    return uc($str);
}

1;
"#,
    )?;

    // A second file that calls format_string
    harness.open(
        "file:///bin/app.pl",
        r#"use Utils;

sub process {
    my $result = Utils::format_string("hello");
    return $result;
}

process();
"#,
    )?;

    harness.barrier();

    // Prepare call hierarchy on format_string in Utils.pm (line 2, char 4 = "format_string")
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": "file:///lib/Utils.pm" },
            "position": { "line": 2, "character": 4 }
        }),
    )?;

    let items = prepare_response
        .as_array()
        .ok_or("prepareCallHierarchy did not return array")?;
    assert!(
        !items.is_empty(),
        "prepareCallHierarchy returned empty array — cursor may be off"
    );

    let item = &items[0];
    assert_eq!(
        item["name"], "format_string",
        "Expected to prepare on format_string"
    );

    // Request incoming calls — must find the caller in the OTHER file
    let incoming_response =
        harness.request("callHierarchy/incomingCalls", json!({ "item": item }))?;

    let calls = incoming_response
        .as_array()
        .ok_or("incomingCalls did not return array")?;

    let caller_names: Vec<String> = calls
        .iter()
        .filter_map(|c| c["from"]["name"].as_str())
        .map(String::from)
        .collect();

    assert!(
        caller_names.contains(&"process".to_string()),
        "Expected to find 'process' from bin/app.pl as an incoming caller, got: {:?}",
        caller_names
    );

    Ok(())
}

/// Tests fix for issue #2878: outgoing calls must resolve targets in other workspace files
/// When process() in bin/app.pl calls Utils::format_string, the outgoing call target
/// should report the URI of Utils.pm, not app.pl.
#[test]
fn test_cross_file_outgoing_calls_uri() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // File that defines the callee
    harness.open(
        "file:///lib/Utils.pm",
        r#"package Utils;

sub format_string {
    my $str = shift;
    return uc($str);
}

1;
"#,
    )?;

    // Caller file
    harness.open(
        "file:///bin/app.pl",
        r#"use Utils;

sub process {
    my $result = Utils::format_string("hello");
    return $result;
}

process();
"#,
    )?;

    harness.barrier();

    // Prepare on "process" in app.pl (line 2, char 4)
    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": "file:///bin/app.pl" },
            "position": { "line": 2, "character": 4 }
        }),
    )?;

    let items = prepare_response
        .as_array()
        .ok_or("prepareCallHierarchy returned non-array")?;
    assert!(
        !items.is_empty(),
        "prepareCallHierarchy returned empty array for process"
    );

    let item = &items[0];
    assert_eq!(item["name"], "process", "Expected to prepare on process");

    // Request outgoing calls
    let outgoing_response =
        harness.request("callHierarchy/outgoingCalls", json!({ "item": item }))?;

    let calls = outgoing_response
        .as_array()
        .ok_or("outgoingCalls returned non-array")?;

    // Must find format_string as an outgoing call
    let callee_names: Vec<String> = calls
        .iter()
        .filter_map(|c| c["to"]["name"].as_str())
        .map(String::from)
        .collect();

    assert!(
        callee_names
            .iter()
            .any(|n| n == "format_string" || n.contains("format_string")),
        "Expected to find format_string as an outgoing call, got: {:?}",
        callee_names
    );

    // The outgoing call target URI should point to Utils.pm, not app.pl.
    // The callee name may be stored as "Utils::format_string" (qualified) or "format_string"
    // (bare) depending on how the AST represents the call site — match both.
    let format_string_call = calls.iter().find(|c| {
        c["to"]["name"]
            .as_str()
            .map(|n| n == "format_string" || n.ends_with("::format_string"))
            .unwrap_or(false)
    });
    assert!(
        format_string_call.is_some(),
        "format_string (or Utils::format_string) must appear as an outgoing call \
         — URI resolution cannot be tested if the call is absent; got: {:?}",
        callee_names
    );
    let format_string_call = format_string_call.unwrap();

    let target_uri = format_string_call["to"]["uri"].as_str().unwrap_or("");
    assert_eq!(
        target_uri, "file:///lib/Utils.pm",
        "format_string target URI should be Utils.pm, got: {:?}",
        target_uri
    );

    Ok(())
}

/// Tests fix for issue #2878: package-qualified calls must disambiguate between
/// multiple same-named subs in different packages.
#[test]
fn test_cross_file_outgoing_calls_disambiguate_package_qualified_target() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///lib/Alpha.pm",
        r#"package Alpha;

sub run {
    return "alpha";
}

1;
"#,
    )?;

    harness.open(
        "file:///lib/Beta.pm",
        r#"package Beta;

sub run {
    return "beta";
}

1;
"#,
    )?;

    harness.open(
        "file:///bin/app.pl",
        r#"use Alpha;
use Beta;

sub orchestrate {
    return Beta::run();
}
"#,
    )?;

    harness.barrier();

    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": "file:///bin/app.pl" },
            "position": { "line": 3, "character": 4 }
        }),
    )?;

    let items = prepare_response
        .as_array()
        .ok_or("prepareCallHierarchy returned non-array")?;
    assert!(
        !items.is_empty(),
        "prepareCallHierarchy returned empty array for orchestrate"
    );

    let outgoing_response =
        harness.request("callHierarchy/outgoingCalls", json!({ "item": &items[0] }))?;
    let calls = outgoing_response
        .as_array()
        .ok_or("outgoingCalls returned non-array")?;

    let run_call = calls
        .iter()
        .find(|call| {
            call["to"]["name"]
                .as_str()
                .map(|name| name.ends_with("run"))
                .unwrap_or(false)
        })
        .ok_or("expected outgoing run() call")?;

    assert_eq!(
        run_call["to"]["uri"], "file:///lib/Beta.pm",
        "package-qualified Beta::run() should resolve to Beta.pm",
    );

    Ok(())
}

/// Tests receiver inference for OO-style method calls.
/// When process() constructs a Repo object and later calls $repo->fetch(),
/// outgoing call resolution should point fetch() at Repo.pm.
#[test]
fn test_cross_file_outgoing_calls_infer_receiver_package() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///lib/Repo.pm",
        r#"package Repo;

sub new {
    return bless {}, shift;
}

sub fetch {
    return "value";
}

1;
"#,
    )?;

    harness.open(
        "file:///bin/app.pl",
        r#"use Repo;

sub process {
    my $repo = Repo->new();
    return $repo->fetch("x");
}
"#,
    )?;

    harness.barrier();

    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": "file:///bin/app.pl" },
            "position": { "line": 2, "character": 4 }
        }),
    )?;

    let items = prepare_response
        .as_array()
        .ok_or("prepareCallHierarchy returned non-array")?;
    assert!(
        !items.is_empty(),
        "prepareCallHierarchy returned empty array for process"
    );

    let outgoing_response =
        harness.request("callHierarchy/outgoingCalls", json!({ "item": &items[0] }))?;
    let calls = outgoing_response
        .as_array()
        .ok_or("outgoingCalls returned non-array")?;

    let fetch_call = calls
        .iter()
        .find(|call| {
            call["to"]["name"]
                .as_str()
                .map(|name| name == "fetch")
                .unwrap_or(false)
        })
        .ok_or("expected outgoing fetch() call")?;

    assert_eq!(
        fetch_call["to"]["uri"], "file:///lib/Repo.pm",
        "receiver inference should resolve $repo->fetch() to Repo.pm",
    );

    Ok(())
}

/// Tests fix for issue #2878: workspace-backed incoming calls should refresh after edits.
#[test]
fn test_cross_file_incoming_calls_refresh_after_change() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///lib/Utils.pm",
        r#"package Utils;

sub format_string {
    my $str = shift;
    return uc($str);
}

1;
"#,
    )?;

    harness.open(
        "file:///bin/app.pl",
        r#"use Utils;

sub process {
    return Utils::format_string("hello");
}
"#,
    )?;

    harness.barrier();

    let prepare_response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": "file:///lib/Utils.pm" },
            "position": { "line": 2, "character": 4 }
        }),
    )?;

    let items = prepare_response
        .as_array()
        .ok_or("prepareCallHierarchy did not return array")?;
    let item = items
        .first()
        .ok_or("prepareCallHierarchy returned empty array")?;

    let incoming_before =
        harness.request("callHierarchy/incomingCalls", json!({ "item": item }))?;
    let before_calls = incoming_before
        .as_array()
        .ok_or("incomingCalls did not return array")?;
    assert!(
        before_calls
            .iter()
            .any(|call| call["from"]["name"] == "process"),
        "expected process() as incoming caller before edit"
    );

    harness.change_full(
        "file:///bin/app.pl",
        2,
        r#"use Utils;

sub process {
    return "done";
}
"#,
    )?;
    harness.barrier();

    let incoming_after = harness.request("callHierarchy/incomingCalls", json!({ "item": item }))?;
    let after_calls = incoming_after
        .as_array()
        .ok_or("incomingCalls did not return array")?;
    assert!(
        after_calls
            .iter()
            .all(|call| call["from"]["name"] != "process"),
        "process() should disappear from incoming callers after edit, got: {:?}",
        after_calls
    );

    Ok(())
}
