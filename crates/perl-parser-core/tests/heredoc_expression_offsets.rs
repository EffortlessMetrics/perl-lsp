//! Regression tests for heredoc bodies declared inside larger expressions.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_parser_core::hir::lower_ast;
use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::must;

fn collect_heredoc_contents(node: &Node, contents: &mut Vec<String>) {
    if let NodeKind::Heredoc { content, .. } = &node.kind {
        contents.push(content.clone());
    }

    for child in node.children() {
        collect_heredoc_contents(child, contents);
    }
}

fn assert_heredoc_bodies(
    source: &str,
    expected: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(source);
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let mut contents = Vec::new();
    collect_heredoc_contents(&ast, &mut contents);
    assert_eq!(contents, expected);
    Ok(())
}

#[test]
fn heredoc_call_argument_uses_the_declaration_line_for_its_body()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies("f(<<'EOF', 1);\nbody\nEOF\n", &["body"])
}

#[test]
fn heredoc_inside_an_array_argument_uses_the_declaration_line_for_its_body()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies("f([<<'EOF']);\nbody\nEOF\n", &["body"])
}

#[test]
fn multiple_heredoc_call_arguments_remain_fifo_aligned() -> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies("f(<<'A', <<'B');\na\nA\nb\nB\n", &["a", "b"])
}

#[test]
fn later_heredoc_declaration_after_the_first_terminator_stays_in_the_same_call()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies("f(<< 'A',\na\nA\n    << 'B',\nb\nB\n    'expected',\n);\n", &["a", "b"])
}

#[test]
fn punctuation_delimiter_heredoc_in_a_call_attaches_its_body()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies("f(split \"\\n\", <<'=');\nbody\n=\n", &["body"])
}

#[test]
fn heredoc_call_continues_with_later_arguments_after_the_terminator()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies("f(<<'EOF',\nbody\nEOF\n    1,\n);\n", &["body"])
}

#[test]
fn spaced_heredoc_call_body_is_not_parsed_as_the_surrounding_expression()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies(
        "f(<< 'EOF',\nBEGIN { $phase++ }\nEND { $phase++ }\nEOF\n    'phase change',\n);\n",
        &["BEGIN { $phase++ }\nEND { $phase++ }"],
    )
}

#[test]
fn heredoc_body_phase_text_does_not_emit_outer_compile_effects()
-> Result<(), Box<dyn std::error::Error>> {
    let source =
        "f(<< 'EOF',\nBEGIN { $phase++ }\nEND { $phase++ }\nEOF\n    'phase change',\n);\n";
    assert_clean_parse(source);
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let hir = lower_ast(&ast);
    assert!(
        hir.compile_environment.phase_blocks.is_empty(),
        "heredoc body text must not become outer compile phase blocks"
    );
    Ok(())
}

#[test]
fn heredoc_array_continues_after_the_terminator() -> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies("f([\n    <<'EOF'\nbody\nEOF\n]);\n", &["body"])
}

#[test]
fn punctuation_delimiter_heredoc_continues_inside_an_array_literal()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies(
        "my $config = { progs => [ split \"\\n\", <<'='\nbody\n=\n] };\n",
        &["body"],
    )
}

#[test]
fn punctuation_delimiter_heredoc_keeps_perl_like_body_out_of_the_outer_call()
-> Result<(), Box<dyn std::error::Error>> {
    assert_heredoc_bodies(
        "like(\n    runperl(\n        progs => [ split \"\\n\", <<'='\nBEGIN { $^P = 0x22; }\nsub DB { return if $__++; }\n=\n        ],\n        stderr => 1,\n    ),\n    qr/ok/,\n);\n",
        &["BEGIN { $^P = 0x22; }\nsub DB { return if $__++; }"],
    )
}
