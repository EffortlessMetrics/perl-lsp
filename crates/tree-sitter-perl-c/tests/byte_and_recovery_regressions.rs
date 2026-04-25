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

    // The upstream C grammar does not strip the BOM automatically, so has_error()
    // may be true on BOM input. The stable regression invariant is that the parser
    // does NOT hard-fail (i.e., parse_perl_bytes returns Ok, not Err), and the root
    // node kind is always "source_file". Do NOT assert !has_error() here — whether
    // the BOM becomes an error node is a grammar detail that can vary across versions.
    let _tree = parse_bytes_must_return_tree(source)?;

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
fn regression_malformed_statement_produces_error_nodes_but_tree_has_multiple_children()
-> Result<(), Box<dyn Error>> {
    // Regression: recovery after a malformed expression must not collapse the rest
    // of the tree. The statement `my $x = ;` is invalid but the subsequent `print`
    // must still appear as a child node (i.e., recovery advances past the bad token).
    let source = "my $x = ;\nmy $y = 10;\nprint $y;\n";

    let tree = parse_perl_code(source)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(tree.root_node().has_error(), "malformed expression should produce error node");
    // The tree must have more than one top-level child: the broken statement and
    // the subsequent valid statements. This distinguishes from a total-failure parse.
    assert!(
        tree.root_node().child_count() >= 2,
        "recovery should preserve subsequent valid statements as children; got child_count={}",
        tree.root_node().child_count()
    );
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

    // Remove the file before asserting so it is cleaned up even on assertion failure.
    let _ = fs::remove_file(&file);

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(tree.root_node().has_error(), "invalid trailing bytes should produce error nodes, not hard failure");

    Ok(())
}

#[test]
fn regression_file_parse_with_unclosed_construct_is_recoverable() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("file_unclosed_construct");
    let source = "if ($flag) {\n  print \"open\";\n";

    fs::write(&file, source)?;
    let tree = parse_perl_file(&file)?;

    // Remove the file before asserting so it is cleaned up even on assertion failure.
    let _ = fs::remove_file(&file);

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(tree.root_node().has_error(), "unclosed block should still return a partial tree");

    Ok(())
}
