mod support;

use support::lsp_harness::LspHarness;
use serde_json::{json, Value};

/// Test helper to get declaration result
fn get_declaration(harness: &mut LspHarness, uri: &str, line: u32, character: u32) -> Option<Value> {
    let result = harness.request("textDocument/declaration", json!({
        "textDocument": {"uri": uri},
        "position": {"line": line, "character": character}
    })).ok()?;
    result
}

#[test]
fn test_variable_declaration_same_block() {
    let mut harness = LspHarness::new();
    harness.initialize(json!({})).unwrap();
    
    let uri = "file:///test.pl";
    let content = r#"my $x = 1;
$x++;
print $x;"#;
    
    harness.open(uri, content).unwrap();
    
    // Click on $x on line 1 (character 0)
    let result = get_declaration(&mut harness, uri, 1, 0);
    assert!(result.is_some());
    
    let locations = result.unwrap();
    assert!(locations.is_array());
    let locations = locations.as_array().unwrap();
    assert_eq!(locations.len(), 1);
    
    // Should point to line 0
    let location = &locations[0];
    let target_range = location["targetRange"].as_object().unwrap();
    assert_eq!(target_range["start"]["line"], 0);
}

#[test]
fn test_variable_shadowing() {
    let mut harness = LspHarness::new();
    harness.initialize(json!({})).unwrap();
    
    let uri = "file:///test.pl";
    let content = r#"my $x = 1;
{
    my $x = 2;
    print $x;  # Should resolve to inner $x
}
print $x;  # Should resolve to outer $x"#;
    
    harness.open(uri, content).unwrap();
    
    // Click on inner $x usage (line 3, character 10)
    let result = get_declaration(&mut harness, uri, 3, 10);
    assert!(result.is_some());
    
    let locations = result.unwrap();
    assert!(locations.is_array());
    let locations = locations.as_array().unwrap();
    assert_eq!(locations.len(), 1);
    
    // Should point to line 2 (inner declaration)
    let location = &locations[0];
    let target_range = location["targetRange"].as_object().unwrap();
    assert_eq!(target_range["start"]["line"], 2);
    
    // Click on outer $x usage (line 5, character 6)
    let result = get_declaration(&mut harness, uri, 5, 6);
    assert!(result.is_some());
    
    let locations = result.unwrap();
    assert!(locations.is_array());
    let locations = locations.as_array().unwrap();
    assert_eq!(locations.len(), 1);
    
    // Should point to line 0 (outer declaration)
    let location = &locations[0];
    let target_range = location["targetRange"].as_object().unwrap();
    assert_eq!(target_range["start"]["line"], 0);
}

#[test]
fn test_subroutine_declaration() {
    let mut harness = LspHarness::new();
    harness.initialize(json!({})).unwrap();
    
    let uri = "file:///test.pl";
    let content = r#"sub foo {
    return 42;
}

my $result = foo();"#;
    
    harness.open(uri, content).unwrap();
    
    // Click on foo() call (line 4, character 13)
    let result = get_declaration(&mut harness, uri, 4, 13);
    assert!(result.is_some());
    
    let locations = result.unwrap();
    assert!(locations.is_array());
    let locations = locations.as_array().unwrap();
    assert_eq!(locations.len(), 1);
    
    // Should point to line 0 (sub declaration)
    let location = &locations[0];
    let target_range = location["targetRange"].as_object().unwrap();
    assert_eq!(target_range["start"]["line"], 0);
}

#[test]
fn test_cross_package_subroutine() {
    let mut harness = LspHarness::new();
    harness.initialize(json!({})).unwrap();
    
    let uri = "file:///test.pl";
    let content = r#"package Foo;
sub bar {
    return "hello";
}

package main;
my $result = Foo::bar();"#;
    
    harness.open(uri, content).unwrap();
    
    // Click on Foo::bar() call (line 6, character 13)
    let result = get_declaration(&mut harness, uri, 6, 18);
    assert!(result.is_some());
    
    let locations = result.unwrap();
    assert!(locations.is_array());
    let locations = locations.as_array().unwrap();
    assert_eq!(locations.len(), 1);
    
    // Should point to line 1 (sub bar in package Foo)
    let location = &locations[0];
    let target_range = location["targetRange"].as_object().unwrap();
    assert_eq!(target_range["start"]["line"], 1);
}

#[test]
fn test_constant_declaration() {
    let mut harness = LspHarness::new();
    harness.initialize(json!({})).unwrap();
    
    let uri = "file:///test.pl";
    let content = r#"use constant FOO => 42;
my $x = FOO;"#;
    
    harness.open(uri, content).unwrap();
    
    // Click on FOO usage (line 1, character 8)
    let result = get_declaration(&mut harness, uri, 1, 8);
    assert!(result.is_some());
    
    let locations = result.unwrap();
    assert!(locations.is_array());
    let locations = locations.as_array().unwrap();
    assert_eq!(locations.len(), 1);
    
    // Should point to line 0 (constant declaration)
    let location = &locations[0];
    let target_range = location["targetRange"].as_object().unwrap();
    assert_eq!(target_range["start"]["line"], 0);
}

#[test]
fn test_unicode_variable_name() {
    let mut harness = LspHarness::new();
    harness.initialize(json!({})).unwrap();
    
    let uri = "file:///test.pl";
    let content = r#"my $π = 3.14159;
print $π;"#;
    
    harness.open(uri, content).unwrap();
    
    // Click on $π usage (line 1, character 6)
    // Note: π is 2 UTF-16 code units
    let result = get_declaration(&mut harness, uri, 1, 6);
    assert!(result.is_some());
    
    let locations = result.unwrap();
    assert!(locations.is_array());
    let locations = locations.as_array().unwrap();
    assert_eq!(locations.len(), 1);
    
    // Should point to line 0 (declaration)
    let location = &locations[0];
    let target_range = location["targetRange"].as_object().unwrap();
    assert_eq!(target_range["start"]["line"], 0);
}