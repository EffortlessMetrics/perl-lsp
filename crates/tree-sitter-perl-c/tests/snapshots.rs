//! Snapshot tests for tree-sitter-perl-c S-expression output.
//!
//! Each test parses a representative Perl snippet and asserts the `to_sexp()` output
//! matches the stored snapshot. These tests mirror the snapshot tests in
//! `tree-sitter-perl-rs` to enable cross-backend comparison of parse output.
//!
//! Run `INSTA_UPDATE=always cargo test -p tree-sitter-perl-c --test snapshots` to
//! update snapshots when the output changes intentionally.

use tree_sitter_perl_c::parse_perl_code;

/// Helper function that parses Perl code and returns the S-expression representation
/// of the root node.
fn sexp(code: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(parse_perl_code(code)?.root_node().to_sexp())
}

#[test]
fn snapshot_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("my $x = 42;")?;
    insta::assert_snapshot!("variable_declaration", sexp);
    Ok(())
}

#[test]
fn snapshot_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("sub foo { return $_[0] + 1; }")?;
    insta::assert_snapshot!("subroutine", sexp);
    Ok(())
}

#[test]
fn snapshot_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("my $text = <<END;\nhello world\nEND\n")?;
    insta::assert_snapshot!("heredoc", sexp);
    Ok(())
}

#[test]
fn snapshot_regex() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp(r"my $matched = ($str =~ /^\d+$/);")?;
    insta::assert_snapshot!("regex", sexp);
    Ok(())
}

#[test]
fn snapshot_package_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("package My::Module;\nuse strict;\nuse warnings;")?;
    insta::assert_snapshot!("package_declaration", sexp);
    Ok(())
}

#[test]
fn snapshot_package_with_multiple_subs() -> Result<(), Box<dyn std::error::Error>> {
    let src = "package Animal;\n\nsub new { my ($class, %args) = @_; bless {}, $class; }\n\nsub speak { return \"...\"; }\n\nsub name { return $_[0]->{name}; }";
    let sexp = sexp(src)?;
    insta::assert_snapshot!("package_with_multiple_subs", sexp);
    Ok(())
}

#[test]
fn snapshot_nested_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("sub outer { if (1) { while (1) { last; } } }")?;
    insta::assert_snapshot!("nested_blocks", sexp);
    Ok(())
}

#[test]
fn snapshot_complex_regex() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"my @matches = ($text =~ /(\w+)\s+=\s+(\d+)/g);"#;
    let sexp = sexp(src)?;
    insta::assert_snapshot!("complex_regex", sexp);
    Ok(())
}

#[test]
fn snapshot_control_flow_with_postfix_condition() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("my $x = 3;\nprint \"odd\\n\" if $x % 2;\n")?;
    insta::assert_snapshot!("control_flow_with_postfix_condition", sexp);
    Ok(())
}

#[test]
fn snapshot_data_structure_dereference() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("my $name = $user->{profile}->{name} // 'unknown';")?;
    insta::assert_snapshot!("data_structure_dereference", sexp);
    Ok(())
}

#[test]
fn snapshot_for_loop_with_lexical_iterator() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = sexp("for my $item (@items) { print $item, \"\\n\"; }")?;
    insta::assert_snapshot!("for_loop_with_lexical_iterator", sexp);
    Ok(())
}
