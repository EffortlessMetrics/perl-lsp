use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::must;

/// Assert a clean parse by walking the AST for Error nodes directly
fn assert_no_errors(source: &str) {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let mut errors = Vec::new();
    walk_all_errors(&ast, &mut errors);

    assert!(
        errors.is_empty(),
        "Found {} error nodes in AST:\n{}\nsexp:\n{}",
        errors.len(),
        errors
            .iter()
            .map(|(pos, msg)| format!("  byte {}: {}", pos, msg))
            .collect::<Vec<_>>()
            .join("\n"),
        ast.to_sexp()
    );
}

fn walk_all_errors(node: &Node, errors: &mut Vec<(usize, String)>) {
    if let NodeKind::Error { message, .. } = &node.kind {
        errors.push((node.location.start, message.clone()));
    }
    node.for_each_child(|child| {
        walk_all_errors(child, errors);
    });
}

// === Bug A: ->local parsed as keyword instead of method name ===

#[test]
fn test_arrow_local_method() {
    assert_no_errors("$self->local;\n");
}

#[test]
fn test_arrow_word_not_method() {
    assert_no_errors("$self->not($self->new->$method(@_));\n");
}

#[test]
fn test_arrow_local_method_full_context() {
    let source = r#"use strict;
use warnings;

package Test::Deep::Cache 1.205;

use Test::Deep::Cache::Simple;

sub new
{
  my $pkg = shift;
  my $self = bless {}, $pkg;
  $self->{expects} = [Test::Deep::Cache::Simple->new];
  $self->{normal} = [Test::Deep::Cache::Simple->new];
  $self->local;
  return $self;
}
"#;
    assert_no_errors(source);
}

// === Local with punctuation special variables ===

#[test]
fn test_local_dollar_slash_no_init() {
    assert_no_errors("local $/;\n");
}

#[test]
fn test_local_dollar_dot_no_init() {
    assert_no_errors("local $.;\n");
}

// === Trailing comma before semicolon ===

#[test]
fn test_trailing_comma_before_semicolon() {
    let source = r#"sprintf
    "defined(%s)",
    $var,
    ;"#;
    assert_no_errors(source);
}
