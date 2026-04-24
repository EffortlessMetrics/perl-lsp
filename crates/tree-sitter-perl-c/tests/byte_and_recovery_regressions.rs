//! Regression tests for byte-level and parser-recovery edge cases.

use std::{
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tree_sitter_perl_c::{parse_perl_code, parse_perl_file};

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0_u128, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!("tree_sitter_perl_c_{name}_{nanos}.pl"))
}

fn assert_parse_returns_tree(
    result: Result<tree_sitter::Tree, Box<dyn Error>>,
) -> Result<tree_sitter::Tree, Box<dyn Error>> {
    let tree = result?;
    assert_eq!(tree.root_node().kind(), "source_file");
    Ok(tree)
}

fn assert_parse_returns_tree_with_expected_error_state(
    result: Result<tree_sitter::Tree, Box<dyn Error>>,
    expect_error_nodes: bool,
) -> Result<(), Box<dyn Error>> {
    let tree = assert_parse_returns_tree(result)?;
    assert_eq!(tree.root_node().has_error(), expect_error_nodes);
    Ok(())
}

#[test]
fn regression_utf8_bom_file_returns_tree_without_hard_failure() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("utf8_bom");
    let source = b"\xEF\xBB\xBFmy $value = 42;\n";
    fs::write(&file, source)?;

    let result = parse_perl_file(&file);
    fs::remove_file(&file)?;

    let _tree = assert_parse_returns_tree(result)?;
    Ok(())
}

#[test]
fn regression_empty_file_returns_error_free_tree() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("empty_file");
    fs::write(&file, b"")?;

    let result = parse_perl_file(&file);
    fs::remove_file(&file)?;

    assert_parse_returns_tree_with_expected_error_state(result, false)
}

#[test]
fn regression_malformed_but_recoverable_code_still_returns_tree() -> Result<(), Box<dyn Error>> {
    let source = "my $x = ;\nprint $x;\nmy $y = 7;\n";
    let result = parse_perl_code(source);

    assert_parse_returns_tree_with_expected_error_state(result, true)
}

#[test]
fn regression_heredoc_heavy_input_still_parses() -> Result<(), Box<dyn Error>> {
    let source = "my $sql = <<'SQL';\nSELECT id, name FROM users;\nSQL\nmy $json = <<\"JSON\";\n{\"ok\":true}\nJSON\nprint $sql;\nprint $json;\n";
    let result = parse_perl_code(source);

    assert_parse_returns_tree_with_expected_error_state(result, false)
}

#[test]
fn regression_quote_like_operator_snippets_return_tree() -> Result<(), Box<dyn Error>> {
    let source = "my $single = q{literal};\nmy $double = qq/interpolate $single/;\nmy @words = qw(alpha beta gamma);\nmy $pattern = qr/ab+c/;\nmy $cmd = qx/echo perl/;\n";
    let result = parse_perl_code(source);

    assert_parse_returns_tree_with_expected_error_state(result, false)
}

#[test]
fn regression_file_parse_malformed_source_returns_tree_with_error_nodes() -> Result<(), Box<dyn Error>> {
    let file = unique_temp_file("malformed_file_recovery");
    let source = "sub broken {\n    my $x = 1;\n\nprint $x;\n";
    fs::write(&file, source)?;

    let result = parse_perl_file(&file);
    fs::remove_file(&file)?;

    assert_parse_returns_tree_with_expected_error_state(result, true)
}
