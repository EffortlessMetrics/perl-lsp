use serde_json::json;

mod support;
use support::lsp_client::LspClient;

#[test]
fn highlights_read_and_write() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let uri = "file:///test.pl";
    let source = "use strict; use warnings; my $x=1; $x++; print $x;\n";

    client.did_open(uri, "perl", source)?;

    // Find column for first "$x"
    let col = source.find("$x").ok_or("Could not find '$x' in source")?;
    let response = client.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": col}
        }),
    )?;

    let highlights =
        response["result"].as_array().ok_or("documentHighlight should return an array")?;

    // Should find 3 occurrences of $x
    assert_eq!(highlights.len(), 3, "Should find all 3 occurrences of $x");

    // Verify exact ranges for each highlight
    // Sort highlights by position to make order-independent
    let mut sorted_highlights: Vec<_> = highlights
        .iter()
        .map(|h| -> Result<(usize, usize, i64), Box<dyn std::error::Error>> {
            let range = &h["range"];
            let start_char =
                range["start"]["character"].as_u64().ok_or("Missing start character")? as usize;
            let end_char =
                range["end"]["character"].as_u64().ok_or("Missing end character")? as usize;
            let kind = h["kind"].as_i64().unwrap_or(2);
            Ok((start_char, end_char, kind))
        })
        .collect::<Result<Vec<_>, _>>()?;
    sorted_highlights.sort_by_key(|&(start, _, _)| start);

    let expected_positions = [
        (29, 31, 3), // First $x at "my $x=1" - Write (declaration)
        (35, 37, 3), // Second $x at "$x++" - Write (modification)
        (47, 49, 2), // Third $x at "print $x" - Read
    ];

    for (i, &(start, end, kind)) in sorted_highlights.iter().enumerate() {
        assert_eq!(
            (start, end),
            (expected_positions[i].0, expected_positions[i].1),
            "Highlight {} should have correct range",
            i
        );
        assert_eq!(kind, expected_positions[i].2, "Highlight {} should have correct kind", i);
    }

    // Also verify all line numbers (all on line 0)
    for highlight in highlights {
        let range = &highlight["range"];
        assert_eq!(range["start"]["line"], 0, "All highlights on line 0");
        assert_eq!(range["end"]["line"], 0, "All highlights on line 0");
    }

    client.shutdown()?;
    Ok(())
}

#[test]
fn highlights_across_scopes() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
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

    client.did_open(uri, "perl", source)?;

    // Highlight $global
    let col = source.find("$global").ok_or("Could not find '$global' in source")?;
    let line = source[..col].matches('\n').count();

    let response = client.request("textDocument/documentHighlight", json!({
        "textDocument": {"uri": uri},
        "position": {"line": line, "character": col - source[..col].rfind('\n').map(|p| p + 1).unwrap_or(0)}
    }))?;

    let highlights =
        response["result"].as_array().ok_or("documentHighlight should return an array")?;

    // Should find 4 occurrences of $global
    assert_eq!(highlights.len(), 4, "Should find all 4 occurrences of $global");

    client.shutdown()?;
    Ok(())
}

#[test]
fn no_highlights_for_different_variables() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let uri = "file:///different.pl";
    let source = "my $foo = 1; my $bar = 2; $foo++; $bar++;\n";

    client.did_open(uri, "perl", source)?;

    // Highlight $foo
    let col = source.find("$foo").ok_or("Could not find '$foo' in source")?;
    let response = client.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": col}
        }),
    )?;

    let highlights =
        response["result"].as_array().ok_or("documentHighlight should return an array")?;

    // Should only find $foo occurrences, not $bar
    assert_eq!(highlights.len(), 2, "Should only find $foo occurrences");

    // Verify ranges don't include $bar
    for highlight in highlights {
        let range = &highlight["range"];
        let start_char =
            range["start"]["character"].as_i64().ok_or("Missing start character")? as usize;
        let end_char = range["end"]["character"].as_i64().ok_or("Missing end character")? as usize;
        let text = &source[start_char..end_char];
        assert!(text.contains("foo"), "Highlight should only contain 'foo' variable");
        assert!(!text.contains("bar"), "Highlight should not contain 'bar' variable");
    }

    client.shutdown()?;
    Ok(())
}
