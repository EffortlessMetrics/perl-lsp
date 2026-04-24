//! Regression tests for byte-oriented and recovery-oriented parsing behaviors.
//!
//! These tests focus on real-world compatibility cases where parsing should
//! return a tree even when syntax errors are present.

use std::{
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tree_sitter_perl_c::{parse_perl_bytes, parse_perl_code, parse_perl_file};

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0_u128, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("tree_sitter_perl_c_regression_{name}_{nanos}.pl"))
}

fn assert_parses_to_tree(bytes: &[u8]) -> Result<tree_sitter::Tree, Box<dyn Error>> {
    let tree = parse_perl_bytes(bytes)?;
    assert_eq!(tree.root_node().kind(), "source_file");
    Ok(tree)
}

#[test]
fn parse_bom_prefixed_file_returns_tree_without_hard_failure() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("bom_prefixed");
    let source = b"\xEF\xBB\xBFmy $value = 1;\nprint $value;\n";
    fs::write(&file, source)?;

    let tree = parse_perl_file(&file)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(
        !tree.root_node().has_error(),
        "Current grammar behavior accepts BOM-prefixed files without syntax errors"
    );

    fs::remove_file(&file)?;
    Ok(())
}

#[test]
fn parse_empty_file_returns_error_free_tree() -> Result<(), Box<dyn Error>> {
    let tree = parse_perl_code("")?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn parse_recoverable_malformed_assignment_returns_tree_with_errors() -> Result<(), Box<dyn Error>> {
    let source = b"my $value = ;\nmy $next = 2;\n";

    let tree = assert_parses_to_tree(source)?;

    assert!(
        tree.root_node().has_error(),
        "Malformed assignment should still produce a recoverable tree"
    );
    Ok(())
}

#[test]
fn parse_heredoc_heavy_input_remains_error_free() -> Result<(), Box<dyn Error>> {
    let source = r#"my $sql = <<'SQL';
SELECT id, name FROM users;
SQL

my $json = <<"JSON";
{"ok":true,"count":2}
JSON

print $sql;
print $json;
"#;

    let tree = parse_perl_code(source)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn parse_quote_like_operators_returns_tree() -> Result<(), Box<dyn Error>> {
    let source = r#"my $a = q/plain text/;
my $b = qq{value=$a};
my @c = qw(alpha beta gamma);
my $d = qr/^foo\d+$/;
my $e = qx(echo perl);
"#;

    let tree = parse_perl_code(source)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn parse_malformed_quote_like_operator_returns_tree_with_errors() -> Result<(), Box<dyn Error>> {
    let source = b"my $pattern = qr/[a-z]+;\nprint $pattern;\n";

    let tree = assert_parses_to_tree(source)?;

    assert!(
        tree.root_node().has_error(),
        "Malformed quote-like operator should recover with error nodes"
    );
    Ok(())
}

#[test]
fn parse_file_with_partial_syntax_errors_still_returns_tree() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("partial_syntax_errors");
    let source = b"package Demo;\nmy $x = 1\nsub ok { return $x; }\n";
    fs::write(&file, source)?;

    let tree = parse_perl_file(&file)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(
        tree.root_node().has_error(),
        "File parsing should succeed and surface syntax recovery in-tree"
    );

    fs::remove_file(&file)?;
    Ok(())
}
