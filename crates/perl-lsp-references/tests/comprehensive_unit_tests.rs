//! Comprehensive unit tests for `perl-lsp-references`.

use perl_lsp_references::find_references_single_file;
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};

fn parse_ast(code: &str) -> perl_parser_core::ast::Node {
    let mut parser = Parser::new(code);
    must(parser.parse())
}

#[test]
fn refs_finds_variable_references() {
    let code = "my $count = 0; $count++; print $count;";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$count"));
    let refs = must_some(find_references_single_file(&ast, offset));
    assert!(refs.len() >= 2, "should find at least 2 references, found {}", refs.len());
}

#[test]
fn refs_finds_function_call_references() {
    let code = "sub greet { } greet();";
    let ast = parse_ast(code);

    let offset = must_some(code.find("greet"));
    let refs = must_some(find_references_single_file(&ast, offset));
    assert!(refs.len() >= 2, "should find definition + call, found {}", refs.len());
}

#[test]
fn refs_returns_none_for_non_symbol_offset() {
    let code = "my $x = 42;";
    let ast = parse_ast(code);
    let offset = must_some(code.find("42"));

    let _ = find_references_single_file(&ast, offset);
}

#[test]
fn refs_returns_none_for_out_of_range_offset() {
    let code = "my $x = 1;";
    let ast = parse_ast(code);
    let refs = find_references_single_file(&ast, 99999);
    assert!(refs.is_none(), "out-of-range offset should return None");
}

#[test]
fn refs_empty_source() {
    let code = "";
    let ast = parse_ast(code);
    let _ = find_references_single_file(&ast, 0);
}

#[test]
fn refs_variable_with_different_sigils_not_confused() {
    let code = "my $foo = 1; my @foo = (2, 3);";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$foo"));
    if let Some(refs) = find_references_single_file(&ast, offset) {
        for &(start, end) in &refs {
            let fragment = &code[start..end.min(code.len())];
            assert!(
                !fragment.starts_with('@'),
                "scalar $foo reference should not match @foo: '{}'",
                fragment
            );
        }
    }
}
