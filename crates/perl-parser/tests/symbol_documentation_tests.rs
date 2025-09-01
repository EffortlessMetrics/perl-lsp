use perl_parser::{Parser, SymbolExtractor};

#[test]
fn sub_comment_is_captured() {
    let src = "# sub docs\n# line two\nsub foo {}\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("foo").expect("symbol foo");
    assert_eq!(symbols[0].documentation.as_deref(), Some("sub docs\nline two"));
}

#[test]
fn variable_comment_is_captured() {
    let src = "# var docs\nmy $x = 1;\n$x;\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("x").expect("symbol x");
    assert_eq!(symbols[0].documentation.as_deref(), Some("var docs"));
}

#[test]
fn comment_separated_by_blank_line_is_not_captured() {
    let src = "# this is not for foo\n\n# foo docs\nsub foo {}\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("foo").expect("symbol foo");
    assert_eq!(symbols[0].documentation.as_deref(), Some("foo docs"));
}

#[test]
fn symbol_with_no_comment() {
    let src = "\n\nsub foo {}\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("foo").expect("symbol foo");
    assert_eq!(symbols[0].documentation, None);
}

#[test]
fn comment_with_extra_hashes_and_spaces() {
    let src = "  ###   var docs\n  my $x = 1;\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("x").expect("symbol x");
    assert_eq!(symbols[0].documentation.as_deref(), Some("var docs"));
}
