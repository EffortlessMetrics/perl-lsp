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

#[test]
fn snapshot_package_with_multiple_subs() {
    let mut parser = Parser::new();
    let src = "package Animal;\n\nsub new { my ($class, %args) = @_; bless {}, $class; }\n\nsub speak { return \"...\"; }\n\nsub name { return $_[0]->{name}; }";
    let tree = must_some(parser.parse(src));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("package_with_multiple_subs", sexp);
}

#[test]
fn snapshot_nested_blocks() {
    let mut parser = Parser::new();
    let src = "sub outer { if (1) { while (1) { last; } } }";
    let tree = must_some(parser.parse(src));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("nested_blocks", sexp);
}

#[test]
fn snapshot_complex_regex() {
    let mut parser = Parser::new();
    let src = r#"my @matches = ($text =~ /(\w+)\s+=\s+(\d+)/g);"#;
    let tree = must_some(parser.parse(src));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("complex_regex", sexp);
}

#[test]
fn snapshot_control_flow_with_postfix_condition() {
    let mut parser = Parser::new();
    let src = "my $x = 3;\nprint \"odd\\n\" if $x % 2;\n";
    let tree = must_some(parser.parse(src));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("control_flow_with_postfix_condition", sexp);
}

#[test]
fn snapshot_data_structure_dereference() {
    let mut parser = Parser::new();
    let src = "my $name = $user->{profile}->{name} // 'unknown';";
    let tree = must_some(parser.parse(src));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("data_structure_dereference", sexp);
}

#[test]
fn snapshot_for_loop_with_lexical_iterator() {
    let mut parser = Parser::new();
    let src = "for my $item (@items) { print $item, \"\\n\"; }";
    let tree = must_some(parser.parse(src));
    let sexp = tree.root_node().to_sexp();
    insta::assert_snapshot!("for_loop_with_lexical_iterator", sexp);
}
