//! Snapshot tests for tree-sitter-compatible S-expression output.
//!
//! Each test parses a representative Perl snippet and asserts the `to_sexp()` output
//! matches the stored snapshot. Run `cargo insta review` to update snapshots when the
//! output changes intentionally.

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::Parser;

#[test]
fn snapshot_variable_declaration() {
    let mut parser = Parser::new();
    let tree = must_some(parser.parse("my $x = 42;"));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("variable_declaration", sexp);
}

#[test]
fn snapshot_subroutine() {
    let mut parser = Parser::new();
    let tree = must_some(parser.parse("sub foo { return $_[0] + 1; }"));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("subroutine", sexp);
}

#[test]
fn snapshot_heredoc() {
    let mut parser = Parser::new();
    let tree = must_some(parser.parse("my $text = <<END;\nhello world\nEND\n"));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("heredoc", sexp);
}

#[test]
fn snapshot_regex() {
    let mut parser = Parser::new();
    let tree = must_some(parser.parse(r"my $matched = ($str =~ /^\d+$/);"));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("regex", sexp);
}

#[test]
fn snapshot_package_declaration() {
    let mut parser = Parser::new();
    let tree = must_some(parser.parse("package My::Module;\nuse strict;\nuse warnings;"));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("package_declaration", sexp);
}
