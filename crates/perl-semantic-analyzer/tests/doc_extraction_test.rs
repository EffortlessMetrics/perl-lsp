use perl_semantic_analyzer::semantic::SemanticAnalyzer;
use perl_parser_core::Parser;

#[test]
fn test_doc_extraction_performance() {
    let mut code = String::new();
    // Generate a large file with many POD blocks
    for i in 0..1000 {
        code.push_str(&format!("=head1 Section {}\nThis is some documentation for section {}.\n=cut\n\nsub func_{} {{}}\n\n", i, i, i));
    }

    let start = std::time::Instant::now();
    let mut parser = Parser::new(&code);
    let ast = parser.parse().unwrap();
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, &code);

    // Measure time to access hover info (which triggers extraction if not cached,
    // but wait, extract_documentation is called during analyze_node and stored in hover_info map!)

    // Ah, analyze_node calls extract_documentation eagerly for every declaration.
    // So analyze_with_source effectively benchmarks it.

    let duration = start.elapsed();
    println!("Analysis took: {:?}", duration);

    // Verify some documentation
    let symbol_table = analyzer.symbol_table();
    let symbols = symbol_table.find_symbol("func_500", 0, perl_semantic_analyzer::symbol::SymbolKind::Subroutine);
    assert!(!symbols.is_empty());

    let hover = analyzer.hover_at(symbols[0].location).unwrap();
    // With the fix, we expect correct documentation for section 500
    // Currently (with bug), it returns Section 0 because of greedy regex matching from start
    assert!(hover.documentation.as_ref().unwrap().contains("This is some documentation for section 500."));
}

#[test]
fn test_comment_extraction_edge_cases() {
    let code = r#"
# Comment 1
# Comment 2

sub foo {}

# Comment 3
sub bar {}
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let foo = analyzer.symbol_table().find_symbol("foo", 0, perl_semantic_analyzer::symbol::SymbolKind::Subroutine)[0].clone();
    let hover_foo = analyzer.hover_at(foo.location).unwrap();
    // Should NOT have comments because there is a blank line separating them
    assert!(hover_foo.documentation.is_none());

    let bar = analyzer.symbol_table().find_symbol("bar", 0, perl_semantic_analyzer::symbol::SymbolKind::Subroutine)[0].clone();
    let hover_bar = analyzer.hover_at(bar.location).unwrap();
    assert_eq!(hover_bar.documentation.as_deref(), Some("Comment 3"));
}
