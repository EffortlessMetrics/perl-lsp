#![allow(clippy::unwrap_used, clippy::expect_used)]

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

#[test]
fn multi_package_comment_scenarios() {
    let src = r#"
# Package level comment for Foo
package Foo;

# This is for sub bar
sub bar {
    return 42;
}

# Package level comment for Baz
package Baz;

# This is for sub qux
sub qux {
    return "hello";
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);

    // Check bar function in Foo package
    let bar_symbols = table.symbols.get("bar").expect("symbol bar");
    assert_eq!(bar_symbols[0].documentation.as_deref(), Some("This is for sub bar"));
    assert_eq!(bar_symbols[0].qualified_name, "Foo::bar");

    // Check qux function in Baz package
    let qux_symbols = table.symbols.get("qux").expect("symbol qux");
    assert_eq!(qux_symbols[0].documentation.as_deref(), Some("This is for sub qux"));
    assert_eq!(qux_symbols[0].qualified_name, "Baz::qux");
}

#[test]
fn complex_comment_formatting() {
    let src = r#"
### START OF DOCUMENTATION
### This function does something important
###   - It takes a parameter
###   - It returns a value
### 
### Example usage:
###   my $result = foo(42);
### END OF DOCUMENTATION
sub foo {
    my $param = shift;
    return $param * 2;
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("foo").expect("symbol foo");

    let expected = "START OF DOCUMENTATION\nThis function does something important\n- It takes a parameter\n- It returns a value\n\nExample usage:\nmy $result = foo(42);\nEND OF DOCUMENTATION";
    assert_eq!(symbols[0].documentation.as_deref(), Some(expected));
}

#[test]
fn mixed_comment_styles_and_blank_lines() {
    let src = r#"
# Single hash comment
## Double hash comment
### Triple hash comment

# This comment is separated by blank line - should NOT be captured

### This is the actual documentation for the variable
##  with mixed indentation
#   and varying hash counts
my $complex_var = "test";
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("complex_var").expect("symbol complex_var");

    let expected = "This is the actual documentation for the variable\nwith mixed indentation\nand varying hash counts";
    assert_eq!(symbols[0].documentation.as_deref(), Some(expected));
}

#[test]
fn variable_list_declarations_with_comments() {
    let src = r#"
# Documentation for multiple variables
# These variables are used together
my ($first, $second, @array, %hash) = (1, 2, (3, 4), (key => 'value'));
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);

    let expected = "Documentation for multiple variables\nThese variables are used together";

    // All variables in the list should get the same documentation
    let first_symbols = table.symbols.get("first").expect("symbol first");
    assert_eq!(first_symbols[0].documentation.as_deref(), Some(expected));

    let second_symbols = table.symbols.get("second").expect("symbol second");
    assert_eq!(second_symbols[0].documentation.as_deref(), Some(expected));

    let array_symbols = table.symbols.get("array").expect("symbol array");
    assert_eq!(array_symbols[0].documentation.as_deref(), Some(expected));

    let hash_symbols = table.symbols.get("hash").expect("symbol hash");
    assert_eq!(hash_symbols[0].documentation.as_deref(), Some(expected));
}

#[test]
fn method_comments_in_class() {
    let src = r#"
class MyClass {
    # This method does something
    # with multiple parameters
    method do_something($param1, $param2) {
        return $param1 + $param2;
    }
    
    # Another method with no parameters
    method simple_method() {
        return "simple";
    }
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);

    let do_something_symbols = table.symbols.get("do_something").expect("symbol do_something");
    assert_eq!(
        do_something_symbols[0].documentation.as_deref(),
        Some("This method does something\nwith multiple parameters")
    );

    let simple_symbols = table.symbols.get("simple_method").expect("symbol simple_method");
    assert_eq!(
        simple_symbols[0].documentation.as_deref(),
        Some("Another method with no parameters")
    );
}

#[test]
fn whitespace_only_lines_vs_blank_lines() {
    let src = "# First comment\n# Second comment\n   \t  \n# Third comment (should not be included)\n\n# This is the actual documentation\nsub test_func {}\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("test_func").expect("symbol test_func");

    // Should only capture the comment immediately preceding the function
    // Blank lines (even with whitespace) should stop the capture
    assert_eq!(symbols[0].documentation.as_deref(), Some("This is the actual documentation"));
}

#[test]
fn unicode_in_comments() {
    let src = "# Документация на русском языке\n# Documentation with émojis 🚀\n# and Unicode symbols ∑∏∆\nmy $unicode_var = 42;\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("unicode_var").expect("symbol unicode_var");

    let expected =
        "Документация на русском языке\nDocumentation with émojis 🚀\nand Unicode symbols ∑∏∆";
    assert_eq!(symbols[0].documentation.as_deref(), Some(expected));
}

#[test]
fn performance_with_large_comment_blocks() {
    // Test performance with large comment blocks to ensure no significant overhead
    let mut src = String::new();

    // Add 100 lines of comments
    for i in 0..100 {
        src.push_str(&format!("# Comment line number {}\n", i + 1));
    }
    src.push_str("sub large_comment_function {}\n");

    let start = std::time::Instant::now();
    let mut parser = Parser::new(&src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(&src);
    let table = extractor.extract(&ast);
    let duration = start.elapsed();

    // Should complete within reasonable time (< 10ms even for large comment blocks)
    assert!(duration.as_millis() < 10, "Comment extraction took too long: {:?}", duration);

    let symbols =
        table.symbols.get("large_comment_function").expect("symbol large_comment_function");
    assert!(symbols[0].documentation.is_some());

    // Check that all 100 lines are captured
    let doc = symbols[0].documentation.as_ref().unwrap();
    let line_count = doc.lines().count();
    assert_eq!(line_count, 100, "Should capture all 100 comment lines");
}

#[test]
fn performance_benchmark_comment_extraction() {
    // Benchmark comment extraction specifically
    let src = "# Short comment\nsub func1 {}\n";

    let iterations = 1000;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        let mut parser = Parser::new(src);
        let ast = parser.parse().expect("parse");
        let extractor = SymbolExtractor::new_with_source(src);
        let _table = extractor.extract(&ast);
    }

    let duration = start.elapsed();
    let per_iteration = duration.as_nanos() / iterations as u128;

    // Should be very fast - less than 100 microseconds per iteration
    assert!(
        per_iteration < 100_000,
        "Comment extraction too slow: {} ns per iteration",
        per_iteration
    );

    // Print performance info for manual verification
    if per_iteration > 50_000 {
        println!(
            "Warning: Comment extraction slower than expected: {} ns per iteration",
            per_iteration
        );
    }
}

#[test]
fn edge_case_empty_comments() {
    let src = "#\n# \n#\t\n# Actual documentation\nmy $var = 1;\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("var").expect("symbol var");

    // Should handle empty comments and capture the actual documentation
    let expected = "\n\n\nActual documentation";
    assert_eq!(symbols[0].documentation.as_deref(), Some(expected));
}

#[test]
fn edge_case_source_boundaries() {
    // Test edge cases with source boundaries
    let src = "# Comment at start\nmy $var = 1;";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("var").expect("symbol var");

    assert_eq!(symbols[0].documentation.as_deref(), Some("Comment at start"));
}

#[test]
fn edge_case_non_ascii_whitespace() {
    // Test with non-ASCII whitespace characters
    let src = "# Comment with various whitespace\u{00A0}\u{2000}\u{2028}\nmy $var = 1;\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("var").expect("symbol var");

    // Should handle non-ASCII whitespace properly
    assert!(symbols[0].documentation.is_some());
    assert!(symbols[0].documentation.as_ref().unwrap().contains("Comment with various whitespace"));
}

#[test]
fn edge_case_malformed_utf8_handling() {
    // Test with valid UTF-8 strings that might cause issues
    let src = "#  Comment with control chars\nmy $var = 1;\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().expect("parse");
    let extractor = SymbolExtractor::new_with_source(src);
    let table = extractor.extract(&ast);
    let symbols = table.symbols.get("var").expect("symbol var");

    // Should not panic and should extract some form of documentation
    assert!(symbols[0].documentation.is_some());
}

#[test]
fn bless_with_comment_documentation() {
    // Regression test: ensure comment extraction doesn't interfere with bless parsing
    let src = "# This creates a blessed object\nmy $obj = bless { foo => 1 }, 'MyClass';";
    let mut parser = Parser::new(src);
    let ast = parser.parse().unwrap();

    // Verify parsing succeeds and generates correct AST structure
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(call bless"));
    assert!(sexp.contains("(hash"));
    assert!(sexp.contains("MyClass"));

    // Also verify symbol extraction works
    let extractor = SymbolExtractor::new_with_source(src);
    let symbol_table = extractor.extract(&ast);

    // Should have extracted the variable symbol
    assert!(symbol_table.symbols.contains_key("obj"));
    let obj_symbols = &symbol_table.symbols["obj"];
    assert_eq!(obj_symbols.len(), 1);
    // Variable should have the preceding comment as documentation
    assert_eq!(obj_symbols[0].documentation, Some("This creates a blessed object".to_string()));
}

#[test]
fn subroutine_with_bless_return() {
    // Regression test: ensure subroutines that return blessed objects work correctly
    let src = "# Constructor\nsub new {\n    return bless {}, shift;\n}";
    let mut parser = Parser::new(src);
    let ast = parser.parse().unwrap();

    // Verify parsing succeeds
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(call bless"));

    // Verify symbol extraction captures subroutine documentation
    let extractor = SymbolExtractor::new_with_source(src);
    let symbol_table = extractor.extract(&ast);

    assert!(symbol_table.symbols.contains_key("new"));
    let new_symbols = &symbol_table.symbols["new"];
    assert_eq!(new_symbols[0].documentation, Some("Constructor".to_string()));
}
