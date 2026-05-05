//! Snapshot tests for the C-backed tree-sitter Perl grammar S-expression output.
//!
//! These test the same 11 Perl constructs as `tree-sitter-perl-rs/tests/snapshots.rs`
//! but capture the upstream C grammar's S-expression shape. Run
//! `INSTA_UPDATE=always cargo test -p tree-sitter-perl-c --test snapshots`
//! to regenerate snapshots when the upstream grammar is updated.
//!
//! Snapshot content intentionally differs from `tree-sitter-perl-rs` — the C grammar
//! produces different node kinds (upstream tree-sitter grammar vs. native v3 AST).
//! Both sets of snapshots together form the cross-backend comparison baseline.

use std::error::Error;

use tree_sitter_perl_c::parse_perl_code;

fn sexp(code: &str) -> Result<String, Box<dyn Error>> {
    let tree = parse_perl_code(code)?;
    Ok(tree.root_node().to_sexp())
}

#[test]
fn snapshot_variable_declaration() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!("variable_declaration", sexp("my $x = 42;")?);
    Ok(())
}

#[test]
fn snapshot_subroutine() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!("subroutine", sexp("sub foo { return $_[0] + 1; }")?);
    Ok(())
}

#[test]
fn snapshot_heredoc() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!("heredoc", sexp("my $text = <<END;\nhello world\nEND\n")?);
    Ok(())
}

#[test]
fn snapshot_regex() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!("regex", sexp(r"my $matched = ($str =~ /^\d+$/);")?);
    Ok(())
}

#[test]
fn snapshot_package_declaration() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(
        "package_declaration",
        sexp("package My::Module;\nuse strict;\nuse warnings;")?
    );
    Ok(())
}

#[test]
fn snapshot_package_with_multiple_subs() -> Result<(), Box<dyn Error>> {
    let src = "package Animal;\n\nsub new { my ($class, %args) = @_; bless {}, $class; }\n\nsub speak { return \"...\"; }\n\nsub name { return $_[0]->{name}; }";
    insta::assert_snapshot!("package_with_multiple_subs", sexp(src)?);
    Ok(())
}

#[test]
fn snapshot_nested_blocks() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!("nested_blocks", sexp("sub outer { if (1) { while (1) { last; } } }")?);
    Ok(())
}

#[test]
fn snapshot_complex_regex() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(
        "complex_regex",
        sexp(r#"my @matches = ($text =~ /(\w+)\s+=\s+(\d+)/g);"#)?
    );
    Ok(())
}

#[test]
fn snapshot_control_flow_with_postfix_condition() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(
        "control_flow_with_postfix_condition",
        sexp("my $x = 3;\nprint \"odd\\n\" if $x % 2;\n")?
    );
    Ok(())
}

#[test]
fn snapshot_data_structure_dereference() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(
        "data_structure_dereference",
        sexp("my $name = $user->{profile}->{name} // 'unknown';")?
    );
    Ok(())
}

#[test]
fn snapshot_for_loop_with_lexical_iterator() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(
        "for_loop_with_lexical_iterator",
        sexp("for my $item (@items) { print $item, \"\\n\"; }")?
    );
    Ok(())
}
