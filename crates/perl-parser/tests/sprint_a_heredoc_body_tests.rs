use perl_parser::Parser;
use perl_parser::ast::{Node, NodeKind};

/// Depth-first traversal using for_each_child_mut to collect all heredoc nodes
fn collect_heredocs(node: &mut Node, out: &mut Vec<(String, String, bool, bool)>) {
    // Check if current node is a heredoc and collect its data
    if let NodeKind::Heredoc { delimiter, content, interpolated, indented } = &node.kind {
        out.push((delimiter.clone(), content.clone(), *interpolated, *indented));
    }

    // Recursively traverse all children using the mutable for_each_child_mut pattern
    node.for_each_child_mut(|child| {
        collect_heredocs(child, out);
    });
}

#[test]
fn heredoc_body_basic() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print <<EOT;\nhello\nworld\nEOT\n";
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 1, "Expected exactly one heredoc node");
    let (delimiter, content, interpolated, indented) = &heredocs[0];
    assert_eq!(delimiter, "EOT");
    assert_eq!(content, "hello\nworld", "Content should be normalized with \\n");
    assert!(*interpolated, "Default heredoc should be interpolated");
    assert!(!*indented);
    Ok(())
}

#[test]
fn heredoc_body_indented_crlf() -> Result<(), Box<dyn std::error::Error>> {
    // For <<~, the terminator's indent is the baseline for stripping
    // In this test, the terminator has 2 spaces, so 2 spaces are stripped from content
    let src = "my $x = <<~EOF;\r\n  a\r\n  b\r\n  EOF\r\n";
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 1);
    let (delimiter, content, _interpolated, indented) = &heredocs[0];
    assert_eq!(delimiter, "EOF");
    assert_eq!(content, "a\nb", "Indented heredoc strips terminator's indent from content");
    assert!(*indented);
    Ok(())
}

#[test]
fn heredoc_body_single_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print <<'END';\nNo $interpolation here\nEND\n";
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 1);
    let (_delimiter, content, interpolated, _indented) = &heredocs[0];
    assert_eq!(content, "No $interpolation here");
    assert!(!*interpolated, "Single-quoted heredoc should not be interpolated");
    Ok(())
}

#[test]
fn heredoc_body_double_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"print <<"END";
Line 1
Line 2
END
"#;
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 1);
    let (delimiter, content, interpolated, _indented) = &heredocs[0];
    assert_eq!(delimiter, "END");
    assert_eq!(content, "Line 1\nLine 2");
    assert!(*interpolated);
    Ok(())
}

#[test]
fn heredoc_body_empty() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print <<EOT;\nEOT\n";
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 1);
    let (_delimiter, content, _interpolated, _indented) = &heredocs[0];
    assert_eq!(content, "", "Empty heredoc should have empty content");
    Ok(())
}

#[test]
fn heredoc_body_multiple_in_statement() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print <<A, <<B;\nfirst\nA\nsecond\nB\n";
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 2, "Should find both heredocs");

    let (delim1, content1, _, _) = &heredocs[0];
    assert_eq!(delim1, "A");
    assert_eq!(content1, "first");

    let (delim2, content2, _, _) = &heredocs[1];
    assert_eq!(delim2, "B");
    assert_eq!(content2, "second");
    Ok(())
}

#[test]
fn heredoc_body_in_expression() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = <<END . ' suffix';\ncontent\nEND\n";
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 1);
    let (_delimiter, content, _interpolated, _indented) = &heredocs[0];
    assert_eq!(content, "content");
    Ok(())
}

#[test]
fn heredoc_indented_mixed_spaces_tabs() -> Result<(), Box<dyn std::error::Error>> {
    // Test <<~ baseline stripping with mixed spaces and tabs
    // Terminator has 2 spaces, so exactly 2 leading bytes are stripped per line
    // Content lines have mixed whitespace:
    //   - "  \tfoo" -> strip 2 bytes (the two spaces) -> "\tfoo"
    //   - "  bar"   -> strip 2 bytes (two spaces) -> "bar"
    //   - "  baz"   -> strip 2 bytes (two spaces) -> "baz"
    let src = "say <<~TXT;\n  \tfoo\n  bar\n  baz\n  TXT\n";
    let mut parser = Parser::new(src);
    let mut root = parser.parse()?;

    let mut heredocs = Vec::new();
    collect_heredocs(&mut root, &mut heredocs);

    assert_eq!(heredocs.len(), 1, "Expected exactly one heredoc node");
    let (delimiter, content, _interpolated, indented) = &heredocs[0];
    assert_eq!(delimiter, "TXT");
    assert_eq!(
        content, "\tfoo\nbar\nbaz",
        "Terminator's 2-space indent should strip exactly 2 leading bytes per line"
    );
    assert!(*indented, "<<~ heredoc should be marked as indented");
    Ok(())
}
