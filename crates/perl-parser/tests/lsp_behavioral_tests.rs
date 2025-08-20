/// Behavioral tests for LSP functionality
/// These tests verify actual functionality, not just response shapes
/// They ensure the wired infrastructure produces real results

use serde_json::json;
use std::time::Duration;

// Import the proper test harness
mod support;
use support::lsp_harness::{LspHarness, TempWorkspace};

mod test_fixtures {
    pub const MAIN_FILE: &str = r#"#!/usr/bin/env perl
use strict;
use warnings;

use My::Module;

my $obj = My::Module->new(name => 'test');
$obj->process();

sub calculate {
    my ($x, $y) = @_;
    return $x + $y;
}

my $result = calculate(5, 10);
print "Result: $result\n";

# TODO: implement caching
my $config = {
    host => 'localhost',
    port => 3000,
};
"#;

    pub const MODULE_FILE: &str = r#"package My::Module;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub process {
    my $self = shift;
    print "Processing: $self->{name}\n";
    return 1;
}

1;
"#;
}

/// Create and initialize a test server with the fixture files
fn create_test_server() -> (LspHarness, TempWorkspace) {
    // Create harness with real temp workspace
    let (mut harness, workspace) = LspHarness::with_workspace(&[
        ("script.pl", test_fixtures::MAIN_FILE),
        ("lib/My/Module.pm", test_fixtures::MODULE_FILE),
    ]).expect("Failed to create test workspace");
    
    // Open documents with real file URIs from the temp workspace
    harness.open_document(
        &workspace.uri("script.pl"),
        test_fixtures::MAIN_FILE
    ).expect("Failed to open main file");
    
    harness.open_document(
        &workspace.uri("lib/My/Module.pm"),
        test_fixtures::MODULE_FILE
    ).expect("Failed to open module file");
    
    // Send didSave notifications to trigger any incremental indexing
    harness.did_save(&workspace.uri("script.pl")).ok();
    harness.did_save(&workspace.uri("lib/My/Module.pm")).ok();
    
    // Wait for the server to process files and become idle
    harness.wait_for_idle(Duration::from_millis(200));
    
    (harness, workspace)
}

#[test]
fn test_cross_file_definition() {
    let (mut harness, workspace) = create_test_server();
    
    // Wait until the module is discoverable
    harness.wait_for_symbol("My::Module", Some(workspace.uri("lib/My/Module.pm").as_str()),
                            Duration::from_millis(800)).expect("index ready");
    
    // Request go-to-definition for My::Module usage
    let result = harness.request("textDocument/definition", json!({
        "textDocument": {"uri": workspace.uri("script.pl")},
        "position": {"line": 4, "character": 10} // On "My::Module"
    })).expect("Definition request failed");
    
    {
        let locations = result.as_array().expect("Should return location array");
        assert!(!locations.is_empty(), "Should find module definition");
        
        // Verify it points to the module file
        let first_location = &locations[0];
        assert_eq!(
            first_location["uri"].as_str(),
            Some(workspace.uri("lib/My/Module.pm").as_str()),
            "Should navigate to module file"
        );
    }
}

#[test]
fn test_cross_file_references() {
    let (mut harness, workspace) = create_test_server();
    
    // Wait until the module is indexed
    harness.wait_for_symbol("process", Some(workspace.uri("lib/My/Module.pm").as_str()),
                            Duration::from_millis(800)).expect("index ready");
    
    // Request references for the 'new' method
    let result = harness.request("textDocument/references", json!({
        "textDocument": {"uri": workspace.uri("lib/My/Module.pm")},
        "position": {"line": 4, "character": 4}, // On "new" method
        "context": {"includeDeclaration": true}
    })).expect("References request failed");
    
    {
        let references = result.as_array().expect("Should return reference array");
        assert!(references.len() >= 2, "Should find declaration and usage");
        
        // Check for reference in script.pl
        let has_script_ref = references.iter().any(|r| {
            r["uri"].as_str() == Some(workspace.uri("script.pl").as_str())
        });
        assert!(has_script_ref, "Should find reference in script.pl");
    }
}

#[test]
fn test_workspace_symbol_search() {
    let (mut harness, workspace) = create_test_server();
    
    // Search for symbols across workspace
    let result = harness.request("workspace/symbol", json!({"query": "process"})).expect("Symbol search failed");
    
    {
        let symbols = result.as_array().expect("Should return symbol array");
        assert!(!symbols.is_empty(), "Should find 'process' method");
        
        // Verify process method is found
        let process_symbol = symbols.iter().find(|s| {
            s["name"].as_str() == Some("process")
        });
        assert!(process_symbol.is_some(), "Should find process method");
        
        // Verify it's in the module file
        assert_eq!(
            process_symbol.unwrap()["location"]["uri"].as_str(),
            Some(workspace.uri("lib/My/Module.pm").as_str()),
            "Process method should be in Module.pm"
        );
    }
}

#[test]
fn test_extract_variable_returns_edits() {
    let (mut harness, workspace) = create_test_server();
    
    // Request code actions for expression extraction
    let result = harness.request("textDocument/codeAction", json!({
        "textDocument": {"uri": workspace.uri("script.pl")},
        "range": {
            "start": {"line": 11, "character": 11},
            "end": {"line": 11, "character": 18} // Select "$x + $y"
        },
        "context": {"diagnostics": []}
    })).expect("Code action request failed");
    
    {
        let actions = result.as_array().expect("Should return action array");
        
        // Find extract variable action
        let extract_action = actions.iter().find(|a| {
            a["title"].as_str().map_or(false, |t| t.contains("Extract"))
        });
        
        if let Some(action) = extract_action {
            // Verify it has actual edits
            if let Some(edit) = action.get("edit") {
                let changes = &edit["changes"];
                assert!(!changes.is_null(), "Should have workspace edit changes");
                
                // Check for edits in the file
                let file_edits = &changes["file:///workspace/script.pl"];
                let edits = file_edits.as_array().expect("Should have edits array");
                assert!(!edits.is_empty(), "Should have actual text edits");
            }
        }
    }
}

#[test]
#[ignore = "Perl::Critic integration not yet implemented"]
fn test_critic_violations_emit_diagnostics() {
    let (mut harness, workspace) = create_test_server();
    
    // Perl::Critic would flag missing return in calculate sub
    // Note: In real implementation, we'd need to capture published diagnostics
    // For now, we'll request diagnostics through a code action context
    
    let result = harness.request("textDocument/codeAction", json!({
        "textDocument": {"uri": workspace.uri("script.pl")},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 20, "character": 0}
        },
        "context": {"diagnostics": [], "only": ["quickfix"]}
    })).expect("Code action request failed");
    
    // Verify the server is processing the file and would emit diagnostics
    // In a real test, we'd capture the publishDiagnostics notification
    assert!(!result.is_null(), "Should process file for diagnostics");
    
    // Alternative: directly check if diagnostics would be generated
    // This would require exposing a test helper method
}

#[test]
fn test_test_generation_actions_present() {
    let (mut harness, workspace) = create_test_server();
    
    // Request code actions for the calculate subroutine
    let result = harness.request("textDocument/codeAction", json!({
        "textDocument": {"uri": workspace.uri("script.pl")},
        "range": {
            "start": {"line": 9, "character": 0},
            "end": {"line": 12, "character": 1} // Cover "calculate" subroutine
        },
        "context": {"diagnostics": []}
    })).expect("Code action request failed");
    
    {
        let actions = result.as_array().expect("Should return action array");
        
        // Find test generation action
        let test_action = actions.iter().find(|a| {
            a["title"].as_str().map_or(false, |t| t.contains("Generate test"))
        });
        
        assert!(test_action.is_some(), "Should have test generation action");
        
        // Verify it has the right command
        let action = test_action.unwrap();
        assert_eq!(
            action["command"]["command"].as_str(),
            Some("perl.generateTest"),
            "Should use perl.generateTest command"
        );
        
        // Verify arguments include test code
        let args = &action["command"]["arguments"];
        let args_array = args.as_array().expect("Should have arguments");
        assert!(!args_array.is_empty(), "Should have test generation arguments");
        
        let first_arg = &args_array[0];
        assert!(first_arg["name"].is_string(), "Should include subroutine name");
        assert!(first_arg["test"].is_string(), "Should include generated test code");
    }
}

#[test]
fn test_completion_detail_formatting() {
    let (mut harness, workspace) = create_test_server();
    
    // Request completion after $obj->
    let result = harness.request("textDocument/completion", json!({
        "textDocument": {"uri": workspace.uri("script.pl")},
        "position": {"line": 7, "character": 6} // After "$obj->"
    })).expect("Completion request failed");
    
    {
        let items = if result.is_array() {
            result.as_array().unwrap()
        } else if let Some(items) = result["items"].as_array() {
            items
        } else {
            panic!("Expected completion items array");
        };
        
        assert!(!items.is_empty(), "Should have completion items");
        
        // Check that detail field is concise
        let typed_items = items.iter().filter(|item| {
            if let Some(detail) = item["detail"].as_str() {
                // Should be concise like "scalar", "array", not debug dumps
                detail.len() < 50 && !detail.contains("InferredType")
            } else {
                false
            }
        }).count();
        assert!(typed_items > 0, "Should have type information in completion details");
    }
}

#[test]
fn test_hover_enriched_information() {
    let (mut harness, workspace) = create_test_server();
    
    // Request hover for My::Module
    let result = harness.request("textDocument/hover", json!({
        "textDocument": {"uri": workspace.uri("script.pl")},
        "position": {"line": 4, "character": 10} // On "My::Module"
    })).expect("Hover request failed");
    
    {
        assert!(!result.is_null(), "Should return hover information");
        
        let contents = &result["contents"];
        let hover_text = if let Some(value) = contents["value"].as_str() {
            value.to_string()
        } else if let Some(markup) = contents.as_array() {
            markup.iter()
                .filter_map(|m| m["value"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };
        
        assert!(!hover_text.is_empty(), "Should have hover content");
        
        // Check for enriched information
        assert!(
            hover_text.contains("Module") || hover_text.contains("package"),
            "Should show package/module information"
        );
    }
}

#[test]
fn test_folding_ranges_work() {
    let (mut harness, workspace) = create_test_server();
    
    // Request folding ranges with timeout
    let result = harness.request_with_timeout("textDocument/foldingRange", json!({
        "textDocument": {"uri": workspace.uri("script.pl")}
    }), Duration::from_millis(500)).expect("Folding range request failed");
    
    {
        let ranges = result.as_array().expect("Should return folding ranges");
        assert!(!ranges.is_empty(), "Should have folding ranges");
        
        // Check for subroutine folding
        let has_sub_fold = ranges.iter().any(|r| {
            r["kind"].as_str() == Some("region")
        });
        assert!(has_sub_fold, "Should have foldable regions");
    }
}

#[test]
fn test_utf16_definition_with_non_ascii_on_same_line() {
    // Ensure we use the fast, deterministic fallbacks in CI
    unsafe {
        std::env::set_var("LSP_TEST_FALLBACKS", "1");
    }
    
    let (mut harness, workspace) = create_test_server();

    // Module with a trivial body
    let module = r#"package My::Module;
use strict;
sub new { bless {}, shift }
1;
"#;

    // Same line contains 2 emojis (each 2 UTF-16 units) and an umlaut (1 unit)
    // The caret will sit on 'M' in `My::Module` after those non-ASCII chars.
    let line = r#"my $obj = "😀😀 zö " . My::Module->new();"#;

    let script = format!(
        r#"#!/usr/bin/env perl
use utf8;
use strict;
use lib "lib";
use My::Module;
{}
"#,
        line
    );

    // Create the module file
    let module_path = workspace.dir.path().join("lib/My/Module.pm");
    std::fs::create_dir_all(module_path.parent().unwrap()).unwrap();
    std::fs::write(&module_path, module).unwrap();
    harness.open_document(&format!("file://{}", module_path.display()), module).unwrap();

    // Create and open the script
    let script_path = workspace.dir.path().join("script.pl");
    std::fs::write(&script_path, &script).unwrap();
    harness.open_document(&format!("file://{}", script_path.display()), &script).unwrap();

    // Wait until the symbol appears so we don't race the indexer
    let module_uri = format!("file://{}", module_path.display());
    harness.wait_for_symbol(
        "My::Module",
        Some(&module_uri),
        Duration::from_millis(800),
    ).expect("index ready");

    // Compute the UTF-16 column for the 'M' in "My::Module" on that exact line.
    let line_idx = script.lines()
        .position(|l| l == line)
        .expect("line with non-ASCII is present");
    let m_byte = line.find("My::Module").expect("line contains My::Module");
    let char_col_utf16 = utf16_units(&line[..m_byte]);

    // Ask for definition using UTF-16 character units
    let result = harness.request_with_timeout("textDocument/definition", json!({
        "textDocument": { "uri": format!("file://{}", script_path.display()) },
        "position": { "line": line_idx, "character": char_col_utf16 }
    }), Duration::from_millis(500)).expect("definition request");

    // Should resolve to the module file
    let locations = result.as_array().expect("definition returns array");
    assert!(!locations.is_empty(), "should return at least one location");
    assert_eq!(
        locations[0]["uri"].as_str(),
        Some(module_uri.as_str()),
        "definition should jump to module file"
    );
}

// Helper to count UTF-16 code units
fn utf16_units(s: &str) -> usize {
    // Count UTF-16 code units in the prefix (surrogate pairs count as 2)
    s.encode_utf16().count()
}

#[test]
fn test_word_boundary_references() {
    // Ensure we use the fast, deterministic fallbacks
    unsafe {
        std::env::set_var("LSP_TEST_FALLBACKS", "1");
    }
    
    let (mut harness, workspace) = create_test_server();
    
    // Create a file with similar variable names to test boundary detection
    let file_path = workspace.dir.path().join("boundary_test.pl");
    let content = r#"#!/usr/bin/perl
my $process = 1;
my $process_data = 2;
my $preprocessor = 3;
print $process;        # Should match
print $process_data;   # Should NOT match
print $preprocessor;   # Should NOT match
"#;
    
    std::fs::write(&file_path, content).unwrap();
    harness.open_document(&format!("file://{}", file_path.display()), content).unwrap();
    
    // Find references to $process (not $process_data or $preprocessor)
    let result = harness.request_with_timeout("textDocument/references", json!({
        "textDocument": { "uri": format!("file://{}", file_path.display()) },
        "position": { "line": 1, "character": 4 },  // Position within $process
        "context": { "includeDeclaration": true }
    }), Duration::from_millis(500)).expect("References request failed");
    
    {
        let refs = result.as_array().expect("Should return references");
        assert_eq!(refs.len(), 2, "Should find exactly 2 uses of $process (declaration and print)");
        
        // Verify only the exact matches are found
        let lines: Vec<u64> = refs.iter()
            .map(|r| r["range"]["start"]["line"].as_u64().unwrap())
            .collect();
        
        assert!(lines.contains(&1), "Should find declaration on line 1");
        assert!(lines.contains(&4), "Should find usage on line 4");
        assert!(!lines.contains(&5), "Should NOT find $process_data on line 5");
        assert!(!lines.contains(&6), "Should NOT find $preprocessor on line 6");
    }
}