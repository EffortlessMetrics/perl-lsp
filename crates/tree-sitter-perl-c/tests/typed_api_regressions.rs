use std::{
    error::Error,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tree_sitter_perl_c::{
    ParsePerlError, try_create_parser, try_parse_perl_code_with_parser, try_parse_perl_file,
};

fn unique_missing_file(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0_u128, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!("tree_sitter_perl_c_missing_{name}_{nanos}.pl"))
}

#[test]
fn regression_try_parse_perl_file_surfaces_typed_io_error() {
    let missing = unique_missing_file("typed_io_error");

    let result = try_parse_perl_file(&missing);

    assert!(matches!(result, Err(ParsePerlError::Io(_))));
}

#[test]
fn regression_reused_parser_typed_api_recovers_after_invalid_input() -> Result<(), Box<dyn Error>> {
    let mut parser = try_create_parser()?;

    let invalid = try_parse_perl_code_with_parser(&mut parser, "my $x = ;\n")?;
    assert!(invalid.root_node().has_error());

    let valid = try_parse_perl_code_with_parser(&mut parser, "my $x = 42;\n")?;
    assert!(!valid.root_node().has_error());

    Ok(())
}
