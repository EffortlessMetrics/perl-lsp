//! Regression tests for byte-level and recovery-oriented parse behavior.

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

    std::env::temp_dir().join(format!("tree_sitter_perl_c_{name}_{nanos}.pl"))
}

fn parse_bytes_must_return_tree(source: &[u8]) -> Result<tree_sitter::Tree, Box<dyn Error>> {
    let tree = parse_perl_bytes(source)?;
    assert_eq!(tree.root_node().kind(), "source_file");
    Ok(tree)
}

#[test]
fn regression_utf8_bom_prefix_returns_tree_without_hard_failure() -> Result<(), Box<dyn Error>> {
    let source = b"\xEF\xBB\xBFmy $value = 1;\n";

    let tree = parse_bytes_must_return_tree(source)?;

    // Current grammar snapshot accepts a leading BOM for this snippet; the key regression check
    // is that BOM bytes do not cause a hard parse failure.
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn regression_completely_empty_file_is_valid_and_error_free() -> Result<(), Box<dyn Error>> {
    let tree = parse_perl_code("")?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn regression_malformed_statement_is_recoverable_with_error_nodes() -> Result<(), Box<dyn Error>> {
    let source = "my $x = ;\nprint $x;\n";

    let tree = parse_perl_code(source)?;

    assert!(tree.root_node().has_error());
    Ok(())
}

#[test]
fn regression_heredoc_heavy_inline_input_still_parses() -> Result<(), Box<dyn Error>> {
    let source = "my $sql = <<'SQL';\nSELECT * FROM users;\nSQL\nmy $json = <<'JSON';\n{\"k\": \"v\"}\nJSON\nmy $tmpl = <<'TMPL';\nHello ${name}\nTMPL\n";

    let tree = parse_perl_code(source)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn regression_quote_like_operator_forms_return_a_tree() -> Result<(), Box<dyn Error>> {
    let source = "my $a = q{literal};\nmy $b = qq(interpolate $a);\nmy @words = qw/alpha beta gamma/;\nmy $rx = qr{^foo\\d+$};\n";

    let tree = parse_perl_code(source)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn regression_file_parse_with_trailing_junk_keeps_partial_tree() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("file_with_trailing_junk");
    let source = "my $ok = 1;\nsub stable { return $ok; }\n@@@\n";

    fs::write(&file, source)?;
    let tree = parse_perl_file(&file)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(tree.root_node().has_error(), "invalid trailing bytes should produce error nodes, not hard failure");

    fs::remove_file(&file)?;
    Ok(())
}

#[test]
fn regression_file_parse_with_unclosed_construct_is_recoverable() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("file_unclosed_construct");
    let source = "if ($flag) {\n  print \"open\";\n";

    fs::write(&file, source)?;
    let tree = parse_perl_file(&file)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(tree.root_node().has_error(), "unclosed block should still return a partial tree");

    fs::remove_file(&file)?;
    Ok(())
}
