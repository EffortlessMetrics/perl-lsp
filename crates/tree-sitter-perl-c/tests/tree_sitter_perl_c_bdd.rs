use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tree_sitter_perl_c::{
    create_parser, get_scanner_config, language, parse_perl_code, parse_perl_file,
    try_create_parser,
};

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0_u128, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!("tree-sitter-perl-c-{name}-{nanos}.pl"))
}

#[test]
fn given_valid_perl_when_parse_perl_code_then_tree_is_error_free()
-> Result<(), Box<dyn std::error::Error>> {
    let tree = parse_perl_code("my $x = 42; print $x;")?;

    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn given_invalid_perl_when_parse_perl_code_then_tree_reports_parse_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let tree = parse_perl_code("my $x = ;")?;

    assert!(tree.root_node().has_error());
    Ok(())
}

#[test]
fn given_missing_file_when_parse_perl_file_then_error_is_returned() {
    let missing_path = unique_temp_file("missing");

    let result = parse_perl_file(&missing_path);

    assert!(result.is_err());
}

#[test]
fn given_existing_perl_file_when_parse_perl_file_then_tree_is_returned()
-> Result<(), Box<dyn std::error::Error>> {
    let file_path = unique_temp_file("valid-source");
    fs::write(&file_path, "package Demo;\nmy $value = 1;\n")?;

    let tree = parse_perl_file(&file_path)?;

    assert!(!tree.root_node().has_error());
    fs::remove_file(file_path)?;
    Ok(())
}

#[test]
fn given_perl_language_when_creating_parser_then_parser_is_configured() {
    let mut parser = create_parser();

    assert!(parser.language().is_some());
    let parse = parser.parse("my $flag = 1;", None);
    assert!(parse.is_some());
}

#[test]
fn given_try_create_parser_when_parsing_multiple_snippets_then_parser_is_reusable()
-> Result<(), Box<dyn std::error::Error>> {
    let mut parser = try_create_parser()?;

    let first = parser.parse("my $x = 1;", None);
    let second = parser.parse("my $y = $x + 1;", None);

    assert!(first.is_some());
    assert!(second.is_some());
    Ok(())
}

#[test]
fn given_crate_metadata_when_querying_scanner_and_language_then_values_are_stable() {
    assert_eq!(get_scanner_config(), "c-scanner");
    assert!(language().node_kind_count() > 0);
}
