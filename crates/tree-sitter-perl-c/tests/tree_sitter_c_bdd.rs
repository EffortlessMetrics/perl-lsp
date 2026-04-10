use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tree_sitter_perl_c::{
    create_parser, get_scanner_config, language, parse_perl_code, parse_perl_file,
    try_create_parser,
};

fn unique_temp_file(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!("tree_sitter_perl_c_{name}_{nanos}.pl")))
}

#[test]
fn given_the_c_language_binding_when_loaded_then_node_kinds_are_available() {
    let perl_language = language();
    assert!(perl_language.node_kind_count() > 0);
}

#[test]
fn given_valid_perl_source_when_parse_perl_code_is_called_then_a_tree_is_returned_without_errors()
-> Result<(), Box<dyn Error>> {
    let code = "my $value = 42;\nsub greet { return $value; }\n";

    let tree = parse_perl_code(code)?;

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn given_unbalanced_perl_source_when_parse_perl_code_is_called_then_the_tree_reports_syntax_errors()
-> Result<(), Box<dyn Error>> {
    let code = "if ($x { print $x; }";

    let tree = parse_perl_code(code)?;

    assert!(tree.root_node().has_error());
    Ok(())
}

#[test]
fn given_a_real_perl_file_when_parse_perl_file_is_called_then_the_contents_are_parsed()
-> Result<(), Box<dyn Error>> {
    let path = unique_temp_file("parse_file")?;
    let code = "package Demo;\nuse strict;\n1;\n";

    fs::write(&path, code)?;
    let tree = parse_perl_file(&path)?;
    let _ = fs::remove_file(&path);

    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn given_parser_constructors_when_invoked_then_each_parser_is_configured_with_the_perl_language()
-> Result<(), Box<dyn Error>> {
    let parser_from_try = try_create_parser()?;
    let parser_from_shim = create_parser();

    assert!(parser_from_try.language().is_some());
    assert!(parser_from_shim.language().is_some());
    Ok(())
}

#[test]
fn given_the_backend_configuration_when_requested_then_it_reports_the_c_scanner() {
    assert_eq!(get_scanner_config(), "c-scanner");
}
