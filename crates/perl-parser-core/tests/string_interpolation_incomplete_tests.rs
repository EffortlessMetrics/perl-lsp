use perl_parser_core::{Node, NodeKind, Parser};

fn collect_string_literals(node: &Node, out: &mut Vec<String>) {
    if let NodeKind::String { value, .. } = &node.kind {
        out.push(value.clone());
    }
    for child in node.children() {
        collect_string_literals(child, out);
    }
}

fn parse_and_assert_no_error_nodes(
    code: &str,
) -> Result<(Node, Vec<String>, usize), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "Parse of `{code}` produced ERROR node(s): {sexp}");
    let mut strings = Vec::new();
    collect_string_literals(&ast, &mut strings);
    let error_count = parser.errors().len();
    Ok((ast, strings, error_count))
}

#[test]
fn double_quote_incomplete_hash_key() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $msg = "Key: $hash{incomplete";"#;
    let (_ast, strings, error_count) = parse_and_assert_no_error_nodes(code)?;

    assert!(
        strings.iter().any(|s| s.contains("$hash{incomplete")),
        "Interpolated string content should preserve `$hash` even with incomplete hash indexing"
    );
    assert!(error_count > 0, "Incomplete hash interpolation should still record a diagnostic");
    Ok(())
}

#[test]
fn double_quote_incomplete_array_index() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $item = "Element: $array[0";"#;
    let (_ast, strings, error_count) = parse_and_assert_no_error_nodes(code)?;

    assert!(
        strings.iter().any(|s| s.contains("$array[0")),
        "Interpolated string content should preserve `$array` even with incomplete indexing"
    );
    assert!(error_count > 0, "Incomplete array interpolation should still record a diagnostic");
    Ok(())
}

#[test]
fn double_quote_incomplete_arrow_hash_index() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $msg = "Nested: $obj->{field";"#;
    let (_ast, strings, error_count) = parse_and_assert_no_error_nodes(code)?;

    assert!(
        strings.iter().any(|s| s.contains("$obj->{field")),
        "Interpolated string content should preserve `$obj` even with incomplete arrow hash dereference"
    );
    assert!(
        error_count > 0,
        "Incomplete arrow hash interpolation should still record a diagnostic"
    );
    Ok(())
}

#[test]
fn double_quote_incomplete_mixed_array_index() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $msg = "Mixed: $array[$i";"#;
    let (_ast, strings, error_count) = parse_and_assert_no_error_nodes(code)?;

    assert!(
        strings.iter().any(|s| s.contains("$array[$i")),
        "Interpolated string content should preserve `$array` in mixed indexing form"
    );
    assert!(error_count > 0, "Incomplete mixed interpolation should still record a diagnostic");
    Ok(())
}

#[test]
fn double_quote_complete_interpolation_still_clean() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $ok1 = "Key: $hash{complete}";
my $ok2 = "Element: $array[0]";
my $ok3 = "Nested: $obj->{field}";
my $ok4 = "Mixed: $array[$i]";
"#;

    let (_ast, _strings, error_count) = parse_and_assert_no_error_nodes(code)?;
    assert_eq!(error_count, 0, "Complete interpolation should not produce diagnostics");
    Ok(())
}
