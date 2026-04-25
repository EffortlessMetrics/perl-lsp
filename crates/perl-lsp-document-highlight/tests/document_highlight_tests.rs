use perl_lsp_document_highlight::{
    DocumentHighlight, DocumentHighlightKind, DocumentHighlightProvider,
};
use perl_parser::Parser;

/// Helper to parse code and find highlights at a given byte offset.
fn highlights_at(
    code: &str,
    byte_offset: usize,
) -> Result<Vec<DocumentHighlight>, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = DocumentHighlightProvider::new();
    Ok(provider.find_highlights(&ast, code, byte_offset))
}

// ---------------------------------------------------------------
// Scalar variable highlighting
// ---------------------------------------------------------------

#[test]
fn test_highlight_scalar_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $foo = 42;\nprint $foo;\n$foo = 100;";
    let highlights = highlights_at(code, 3)?; // on $foo

    assert!(!highlights.is_empty());
    Ok(())
}

#[test]
fn scalar_all_occurrences() -> Result<(), Box<dyn std::error::Error>> {
    //              0         1         2         3
    //              0123456789012345678901234567890123456
    let code = "my $foo = 1;\nprint $foo;\n$foo = $foo + 1;";
    let highlights = highlights_at(code, 3)?; // first $foo

    // 4 occurrences: declaration, print arg, assignment lhs, addition operand
    assert_eq!(
        highlights.len(),
        4,
        "Expected 4 highlights for $foo, found {}: {:?}",
        highlights.len(),
        highlights
    );
    Ok(())
}

// ---------------------------------------------------------------
// Array variable cross-sigil highlighting
// ---------------------------------------------------------------

#[test]
fn array_cross_sigil_from_at() -> Result<(), Box<dyn std::error::Error>> {
    // Cursor on @array should highlight @array, $array[0], $#array
    let code = "my @array = (1,2,3);\nmy $x = $array[0];\nmy $len = $#array;";
    // @array starts at offset 3
    let highlights = highlights_at(code, 3)?;

    // Should find: @array (decl), $array (in $array[0]), $#array
    assert!(
        highlights.len() >= 3,
        "Expected at least 3 highlights for @array (got {}): {:?}",
        highlights.len(),
        highlights
    );
    Ok(())
}

#[test]
fn array_cross_sigil_from_dollar_subscript() -> Result<(), Box<dyn std::error::Error>> {
    // Cursor on $array in $array[0] should highlight @array too
    let code = "my @array = (1,2,3);\nmy $x = $array[0];\nmy $len = $#array;";
    // $array in "$array[0]" starts after "my @array = (1,2,3);\nmy $x = "
    let offset = code.find("$array[0]").ok_or("test setup")?;
    let highlights = highlights_at(code, offset)?;

    // Should find: @array (decl), $array (in $array[0]), $#array
    assert!(
        highlights.len() >= 3,
        "Expected at least 3 highlights from $array[0] cursor (got {}): {:?}",
        highlights.len(),
        highlights
    );
    Ok(())
}

#[test]
fn array_cross_sigil_dollar_hash() -> Result<(), Box<dyn std::error::Error>> {
    // Cursor on $#array should highlight @array too
    let code = "my @array = (1,2,3);\nmy $len = $#array;";
    let offset = code.find("$#array").ok_or("test setup")?;
    let highlights = highlights_at(code, offset)?;

    // Should find: @array (decl), $#array
    assert!(
        highlights.len() >= 2,
        "Expected at least 2 highlights from $#array cursor (got {}): {:?}",
        highlights.len(),
        highlights
    );
    Ok(())
}

// ---------------------------------------------------------------
// Hash variable cross-sigil highlighting
// ---------------------------------------------------------------

#[test]
fn hash_cross_sigil_from_percent() -> Result<(), Box<dyn std::error::Error>> {
    // Cursor on %hash should highlight %hash, $hash{key}
    let code = "my %hash = (a => 1);\n$hash{b} = 2;\nmy $v = $hash{a};";
    let highlights = highlights_at(code, 3)?; // on %hash

    // Should find: %hash (decl), $hash (in $hash{b}), $hash (in $hash{a})
    assert!(
        highlights.len() >= 3,
        "Expected at least 3 highlights for %%hash (got {}): {:?}",
        highlights.len(),
        highlights
    );
    Ok(())
}

#[test]
fn hash_cross_sigil_from_dollar_brace() -> Result<(), Box<dyn std::error::Error>> {
    // Cursor on $hash in $hash{key} should highlight %hash too
    let code = "my %hash = (a => 1);\nmy $v = $hash{a};";
    let offset = code.find("$hash{a}").ok_or("test setup")?;
    let highlights = highlights_at(code, offset)?;

    // Should find: %hash (decl), $hash (in $hash{a})
    assert!(
        highlights.len() >= 2,
        "Expected at least 2 highlights from $hash{{a}} cursor (got {}): {:?}",
        highlights.len(),
        highlights
    );
    Ok(())
}

#[test]
fn hash_slice_cross_sigil() -> Result<(), Box<dyn std::error::Error>> {
    // @hash{@keys} should match %hash
    let code = "my %hash = (a => 1, b => 2);\nmy @vals = @hash{qw(a b)};";
    let highlights = highlights_at(code, 3)?; // on %hash

    // Should find: %hash (decl), @hash (in @hash{qw(a b)})
    assert!(
        highlights.len() >= 2,
        "Expected at least 2 highlights for %%hash with slice (got {}): {:?}",
        highlights.len(),
        highlights
    );
    Ok(())
}

// ---------------------------------------------------------------
// Write vs Read highlighting
// ---------------------------------------------------------------

#[test]
fn write_vs_read_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $foo = 42;\nprint $foo;";
    let highlights = highlights_at(code, 3)?;

    assert!(highlights.len() >= 2, "Expected at least 2 highlights");

    // First highlight (declaration) should be Write
    let decl_highlight = highlights.iter().find(|h| h.location.start == 3);
    assert!(
        decl_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
        "Declaration should be Write"
    );

    // Second highlight (print usage) should be Read
    let print_offset = code.find("print $foo").ok_or("test setup")? + 6;
    let read_highlight = highlights.iter().find(|h| h.location.start == print_offset);
    assert!(
        read_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Read),
        "Usage in print should be Read"
    );

    Ok(())
}

#[test]
fn write_vs_read_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\n$x = 2;\nprint $x;";
    let highlights = highlights_at(code, 3)?;

    assert!(highlights.len() >= 3, "Expected at least 3 highlights");

    // Declaration is Write
    assert_eq!(highlights[0].kind, DocumentHighlightKind::Write);

    // Assignment is Write
    let assign_offset = code.find("\n$x = 2").ok_or("test setup")? + 1;
    let assign_highlight = highlights.iter().find(|h| h.location.start == assign_offset);
    assert!(
        assign_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
        "Assignment LHS should be Write"
    );

    Ok(())
}

#[test]
fn write_vs_read_increment() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 0;\n$x++;\nprint $x;";
    let highlights = highlights_at(code, 3)?;

    assert!(highlights.len() >= 3, "Expected at least 3 highlights");

    // Increment should be Write
    let incr_offset = code.find("\n$x++").ok_or("test setup")? + 1;
    let incr_highlight = highlights.iter().find(|h| h.location.start == incr_offset);
    assert!(
        incr_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
        "Increment should be Write"
    );

    Ok(())
}

#[test]
fn write_vs_read_foreach_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @items = (1,2,3);\nfor my $item (@items) {\n    print $item;\n}";
    // Find the offset of "$item" in "for my $item"
    let item_offset = code.find("$item").ok_or("test setup")?;
    let highlights = highlights_at(code, item_offset)?;

    assert!(
        highlights.len() >= 2,
        "Expected at least 2 highlights for $item (got {}): {:?}",
        highlights.len(),
        highlights
    );

    // The loop variable declaration should be Write
    let decl_highlight = highlights.iter().find(|h| h.location.start == item_offset);
    assert!(
        decl_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
        "Foreach loop variable should be Write"
    );

    Ok(())
}

// ---------------------------------------------------------------
// Existing tests (preserved)
// ---------------------------------------------------------------

#[test]
fn test_highlight_function_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub hello { print \"Hello\" }\nhello();\nhello();";
    let highlights = highlights_at(code, 29)?; // first hello() call

    assert!(
        highlights.len() >= 2,
        "Expected at least 2 highlights for function calls, found {}",
        highlights.len()
    );
    Ok(())
}

#[test]
fn test_no_highlights_for_non_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = \"Hello World\";";
    let highlights = highlights_at(code, 12)?; // inside string "Hello"

    assert_eq!(highlights.len(), 0);
    Ok(())
}

#[test]
fn test_highlight_statement_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 5;\nprint $x if $x > 0;";
    let highlights = highlights_at(code, 3)?; // first $x

    assert!(
        highlights.len() >= 3,
        "Expected at least 3 highlights for $x, found {}",
        highlights.len()
    );
    Ok(())
}

#[test]
fn qualified_call_highlights_bare_definition() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"package Utils;
sub format_string { return shift }
my $x = Utils::format_string("hi");
"#;
    let call_offset = code.find("Utils::format_string").ok_or("test setup")? + "Utils::".len();
    let def_offset = code.find("format_string {").ok_or("test setup")?;

    let highlights = highlights_at(code, call_offset)?;

    assert!(
        highlights.iter().any(|h| h.location.start == def_offset),
        "expected qualified call to highlight bare definition, got {highlights:?}"
    );
    Ok(())
}
