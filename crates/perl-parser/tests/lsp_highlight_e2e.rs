use serde_json::json;

mod support;
use support::lsp_client::LspClient;

#[test]
fn highlights_read_and_write() {
    // Build the LSP binary first
    std::process::Command::new("cargo")
        .args(&["build", "-p", "perl-parser", "--bin", "perl-lsp"])
        .output()
        .expect("Failed to build perl-lsp");
    
    let mut client = LspClient::spawn("target/debug/perl-lsp");
    let uri = "file:///test.pl";
    let source = "use strict; use warnings; my $x=1; $x++; print $x;\n";
    
    client.did_open(uri, "perl", source);
    
    // Find column for first "$x"
    let col = source.find("$x").unwrap();
    let response = client.request(2, "textDocument/documentHighlight", json!({
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": col}
    }));
    
    let highlights = response["result"].as_array()
        .expect("documentHighlight should return an array");
    
    // Should find 3 occurrences of $x
    assert_eq!(highlights.len(), 3, "Should find all 3 occurrences of $x");
    
    // Collect kinds (2=Read, 3=Write in LSP spec)
    let kinds: Vec<i64> = highlights.iter()
        .map(|h| h["kind"].as_i64().unwrap_or(2))
        .collect();
    
    // First occurrence is write (declaration)
    assert_eq!(kinds[0], 3, "First occurrence should be Write");
    
    // Second occurrence is write ($x++)
    assert_eq!(kinds[1], 3, "Second occurrence should be Write");
    
    // Third occurrence is read (print $x)
    assert_eq!(kinds[2], 2, "Third occurrence should be Read");
    
    client.shutdown();
}

#[test]
fn highlights_across_scopes() {
    std::process::Command::new("cargo")
        .args(&["build", "-p", "perl-parser", "--bin", "perl-lsp"])
        .output()
        .expect("Failed to build perl-lsp");
    
    let mut client = LspClient::spawn("target/debug/perl-lsp");
    let uri = "file:///scope.pl";
    let source = r#"
my $global = 1;
sub foo {
    my $local = 2;
    $global = 3;
    return $local + $global;
}
$global++;
"#;
    
    client.did_open(uri, "perl", source);
    
    // Highlight $global
    let col = source.find("$global").unwrap();
    let line = source[..col].matches('\n').count();
    
    let response = client.request(3, "textDocument/documentHighlight", json!({
        "textDocument": {"uri": uri},
        "position": {"line": line, "character": col - source[..col].rfind('\n').map(|p| p + 1).unwrap_or(0)}
    }));
    
    let highlights = response["result"].as_array()
        .expect("documentHighlight should return an array");
    
    // Should find 4 occurrences of $global
    assert_eq!(highlights.len(), 4, "Should find all 4 occurrences of $global");
    
    client.shutdown();
}

#[test]
fn no_highlights_for_different_variables() {
    std::process::Command::new("cargo")
        .args(&["build", "-p", "perl-parser", "--bin", "perl-lsp"])
        .output()
        .expect("Failed to build perl-lsp");
    
    let mut client = LspClient::spawn("target/debug/perl-lsp");
    let uri = "file:///different.pl";
    let source = "my $foo = 1; my $bar = 2; $foo++; $bar++;\n";
    
    client.did_open(uri, "perl", source);
    
    // Highlight $foo
    let col = source.find("$foo").unwrap();
    let response = client.request(4, "textDocument/documentHighlight", json!({
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": col}
    }));
    
    let highlights = response["result"].as_array()
        .expect("documentHighlight should return an array");
    
    // Should only find $foo occurrences, not $bar
    assert_eq!(highlights.len(), 2, "Should only find $foo occurrences");
    
    // Verify ranges don't include $bar
    for highlight in highlights {
        let range = &highlight["range"];
        let start_char = range["start"]["character"].as_i64().unwrap() as usize;
        let end_char = range["end"]["character"].as_i64().unwrap() as usize;
        let text = &source[start_char..end_char];
        assert!(text.contains("foo"), "Highlight should only contain 'foo' variable");
        assert!(!text.contains("bar"), "Highlight should not contain 'bar' variable");
    }
    
    client.shutdown();
}