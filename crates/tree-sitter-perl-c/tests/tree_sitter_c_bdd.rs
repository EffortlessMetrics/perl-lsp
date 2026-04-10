use std::{
    error::Error,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use tree_sitter_perl_c::{
    create_parser, get_scanner_config, language, parse_perl_code, parse_perl_file,
    try_create_parser,
};

#[test]
fn given_language_binding_when_loading_then_node_kinds_are_available() {
    let lang = language();
    assert!(lang.node_kind_count() > 0);
}

#[test]
fn given_simple_perl_code_when_parsing_then_tree_has_no_errors() -> Result<(), Box<dyn Error>> {
    let code = "my $value = 42;";
    let tree = parse_perl_code(code)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());

    Ok(())
}

#[test]
fn given_invalid_perl_fragment_when_parsing_then_tree_reports_errors() -> Result<(), Box<dyn Error>>
{
    let code = "my $value = ;";
    let tree = parse_perl_code(code)?;

    assert!(tree.root_node().has_error());

    Ok(())
}

#[test]
fn given_perl_source_file_when_parsing_then_file_and_string_paths_match()
-> Result<(), Box<dyn Error>> {
    let code = "package Demo;\nmy $name = 'tree-sitter';\n1;\n";
    let mut path = std::env::temp_dir();
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    path.push(format!("tree_sitter_perl_c_bdd_{ts}.pl"));

    fs::write(&path, code)?;

    let parsed_from_file = parse_perl_file(&path)?;
    let parsed_from_string = parse_perl_code(code)?;

    assert_eq!(parsed_from_file.root_node().to_sexp(), parsed_from_string.root_node().to_sexp());

    fs::remove_file(&path)?;

    Ok(())
}

#[test]
fn given_parser_constructors_when_initialized_then_language_is_set() -> Result<(), Box<dyn Error>> {
    let parser = create_parser();
    let checked_parser = try_create_parser()?;

    assert!(parser.language().is_some());
    assert!(checked_parser.language().is_some());

    Ok(())
}

#[test]
fn given_c_backend_crate_when_querying_scanner_config_then_c_scanner_is_reported() {
    assert_eq!(get_scanner_config(), "c-scanner");
}
