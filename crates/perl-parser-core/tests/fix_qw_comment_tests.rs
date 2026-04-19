//! Tests for qw() comment stripping.
//!
//! In Perl, `#` inside `qw()` is a comment character — it strips to end of line.
//! See perlop: "A # character within the list is treated as a comment character"
//!
//! `qw(foo # comment\n bar)` must yield `['foo', 'bar']`, NOT `['foo', '#', 'comment', 'bar']`.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

/// Parse a qw() literal and return the list of string values.
fn parse_qw_words(source: &str) -> Vec<String> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    // Walk the AST to find the first ArrayLiteral node and extract its string children.
    collect_array_words(&ast)
}

fn collect_array_words(node: &perl_parser_core::Node) -> Vec<String> {
    if let NodeKind::ArrayLiteral { elements } = &node.kind {
        return elements
            .iter()
            .filter_map(|e| {
                if let NodeKind::String { value, .. } = &e.kind {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .collect();
    }
    for child in node.children() {
        let result = collect_array_words(child);
        if !result.is_empty() {
            return result;
        }
    }
    vec![]
}

// --- Semantic correctness tests (these FAIL before the fix) ---

#[test]
fn qw_with_inline_comment_word_count() {
    // qw(foo # this is a comment\n bar) => ['foo', 'bar'] in Perl
    let words = parse_qw_words("my @x = qw(foo # this is a comment\n bar);");
    assert_eq!(
        words,
        vec!["foo", "bar"],
        "qw comment not stripped: got {:?}",
        words
    );
}

#[test]
fn qw_comment_at_start_word_count() {
    // qw(# all comment\n foo bar) => ['foo', 'bar'] in Perl
    let words = parse_qw_words("my @x = qw(# all comment\n foo bar);");
    assert_eq!(
        words,
        vec!["foo", "bar"],
        "qw leading comment not stripped: got {:?}",
        words
    );
}

#[test]
fn qw_multiple_comment_lines_word_count() {
    // qw(a # first\n b # second\n c) => ['a', 'b', 'c'] in Perl
    let words = parse_qw_words("my @x = qw(a # first\n b # second\n c);");
    assert_eq!(
        words,
        vec!["a", "b", "c"],
        "qw multiple comments not stripped: got {:?}",
        words
    );
}

#[test]
fn qw_trailing_comment_word_count() {
    // qw(foo bar # trailing) => ['foo', 'bar'] in Perl
    let words = parse_qw_words("my @x = qw(foo bar # trailing comment);");
    assert_eq!(
        words,
        vec!["foo", "bar"],
        "qw trailing comment not stripped: got {:?}",
        words
    );
}

// --- No-comment regression (must always pass) ---

#[test]
fn qw_no_comment_regression() {
    let words = parse_qw_words("my @x = qw(foo bar baz);");
    assert_eq!(words, vec!["foo", "bar", "baz"]);
}

// --- Non-paren delimiter ---

#[test]
fn qw_brace_delimiter_with_comment() {
    let words = parse_qw_words("my @x = qw{foo # comment\n bar};");
    assert_eq!(
        words,
        vec!["foo", "bar"],
        "qw brace delimiter comment not stripped: got {:?}",
        words
    );
}

// --- use-statement context ---

#[test]
fn use_statement_qw_with_comment() {
    assert_clean_parse("use Scalar::Util qw(looks_like_number # check\n blessed);");
}
