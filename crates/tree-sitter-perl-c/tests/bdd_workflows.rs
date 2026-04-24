//! BDD-style workflow tests for the C tree-sitter Perl binding.
//!
//! These scenarios validate the user-visible behaviors that matter most for
//! this crate: parser setup, successful and failing parses, file parsing, and
//! query/capture interoperability.

use std::{
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tree_sitter::{Query, QueryCursor, StreamingIterator};
use tree_sitter_perl_c::{
    create_parser, get_scanner_config, language, parse_perl_code, parse_perl_file,
    try_create_parser,
};

struct Scenario {
    name: &'static str,
}

impl Scenario {
    fn new(name: &'static str) -> Self {
        eprintln!("[BDD] Scenario: {name}");
        Self { name }
    }

    fn given(&self, message: &str) {
        eprintln!("[{}] Given {message}", self.name);
    }

    fn when(&self, message: &str) {
        eprintln!("[{}] When {message}", self.name);
    }

    fn then(&self, message: &str) {
        eprintln!("[{}] Then {message}", self.name);
    }
}

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0_u128, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!("tree_sitter_perl_c_{name}_{nanos}.pl"))
}

#[test]
fn bdd_language_binding_reports_node_kinds() {
    let scenario = Scenario::new("language binding reports node kinds");

    scenario.given("the C-backed Perl language binding is loaded");
    let perl_language = language();

    scenario.then("node kinds are available for downstream tools");
    assert!(perl_language.node_kind_count() > 0);
}

#[test]
fn bdd_parser_constructors_are_configured() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("parser constructors are configured");

    scenario.given("the parser constructors are available");
    let parser_from_try = try_create_parser()?;
    let parser_from_shim = create_parser();

    scenario.then("both constructors should return parsers with a language");
    assert!(parser_from_try.language().is_some());
    assert!(parser_from_shim.language().is_some());
    Ok(())
}

#[test]
fn bdd_parse_valid_source_returns_an_error_free_tree() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("parse valid source");
    let source = "my $value = 42;\nsub greet { return $value; }\n";

    scenario.given("valid Perl source");
    scenario.when("parse_perl_code is invoked");
    let tree = parse_perl_code(source)?;

    scenario.then("the parse tree should be rooted at source_file and have no errors");
    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    Ok(())
}

#[test]
fn bdd_parse_invalid_source_still_returns_a_tree() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("parse invalid source");
    let source = "my $x = ;\nprint $x;\n";

    scenario.given("a malformed Perl snippet");
    scenario.when("parse_perl_code is invoked");
    let tree = parse_perl_code(source)?;

    scenario.then("callers should still receive a partial tree with syntax errors");
    assert!(tree.root_node().has_error());
    Ok(())
}

#[test]
fn bdd_parse_perl_file_reads_from_disk() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("parse perl file from disk");
    let file = unique_temp_file("parse_file");

    scenario.given("a Perl source file on disk");
    fs::write(&file, "package Demo;\nmy $value = 1;\n")?;

    scenario.when("parse_perl_file is invoked");
    let tree = parse_perl_file(&file)?;

    scenario.then("the file should parse successfully");
    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    fs::remove_file(&file)?;
    Ok(())
}

#[test]
fn bdd_parse_perl_file_allows_non_utf8_bytes() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("parse perl file with non utf8 bytes");
    let file = unique_temp_file("parse_file_non_utf8");
    let source = b"my $value = 1;\n# latin1 byte: \xE9\nprint $value;\n";

    scenario.given("a Perl source file on disk containing non-UTF-8 bytes");
    fs::write(&file, source)?;

    scenario.when("parse_perl_file is invoked");
    let tree = parse_perl_file(&file)?;

    scenario.then("the parser should still return a syntax-valid source_file tree");
    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
    fs::remove_file(&file)?;
    Ok(())
}

#[test]
fn bdd_parse_perl_file_missing_path_reports_context() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("parse perl file missing path reports context");
    let file = unique_temp_file("missing_file");
    let file_display = file.display().to_string();

    scenario.given("a missing Perl source path");
    scenario.when("parse_perl_file is invoked");
    let error = match parse_perl_file(&file) {
        Ok(_) => return Err("expected parse_perl_file to fail for missing file".into()),
        Err(error) => error,
    };

    scenario.then("the error message should contain the path and the reason");
    let message = error.to_string();
    assert!(message.contains(&file_display));
    assert!(message.contains("failed to parse Perl file"));
    Ok(())
}

#[test]
fn bdd_parse_perl_file_unreadable_path_reports_context() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("parse perl file unreadable path reports context");
    let directory = unique_temp_file("unreadable_path");
    let directory_display = directory.display().to_string();

    scenario.given("a directory path where a file is expected");
    fs::create_dir(&directory)?;

    scenario.when("parse_perl_file is invoked");
    let error = match parse_perl_file(&directory) {
        Ok(_) => return Err("expected parse_perl_file to fail for directory path".into()),
        Err(error) => error,
    };

    scenario.then("the error message should contain the path and parse context");
    let message = error.to_string();
    assert!(message.contains(&directory_display));
    assert!(message.contains("failed to parse Perl file"));

    fs::remove_dir(&directory)?;
    Ok(())
}

#[test]
fn bdd_injections_query_matches_inline_cpp_heredoc_content() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("injections query matches inline cpp heredoc content");
    let source = "use Inline CPP => <<'END_CPP';\n#include <string>\nclass Greet {};\nEND_CPP\n";
    let injections_query = include_str!("../../../tree-sitter-perl/queries/injections.scm");

    scenario.given("an Inline::CPP heredoc snippet");
    scenario.when("the upstream injections query is executed");
    let tree = parse_perl_code(source)?;
    let query = Query::new(&language(), injections_query)?;
    let mut cursor = QueryCursor::new();

    let mut saw_inline_package = false;
    let mut saw_inline_language = false;
    let mut saw_injection_content = false;

    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let capture_name =
                query.capture_names().get(capture.index as usize).copied().unwrap_or_default();
            let text = capture.node.utf8_text(source.as_bytes()).unwrap_or_default();

            match capture_name {
                "inline.package" => saw_inline_package = text == "Inline",
                "inline.language" => saw_inline_language = text == "CPP",
                "injection.content" => {
                    saw_injection_content = capture.node.kind() == "heredoc_content"
                        && text.contains("#include <string>");
                }
                _ => {}
            }
        }
    }

    scenario.then("all expected captures should be present");
    assert!(saw_inline_package, "expected inline.package capture");
    assert!(saw_inline_language, "expected inline.language capture");
    assert!(saw_injection_content, "expected injection.content capture");
    Ok(())
}

#[test]
fn bdd_injections_query_matches_inline_c_heredoc_content() -> Result<(), Box<dyn Error>> {
    let scenario = Scenario::new("injections query matches inline c heredoc content");
    let source = "use Inline C => <<'END_C';\n#include <math.h>\ndouble calc(double x) { return sqrt(x); }\nEND_C\n";
    let injections_query = include_str!("../../../tree-sitter-perl/queries/injections.scm");

    scenario.given("an Inline::C heredoc snippet");
    scenario.when("the upstream injections query is executed");
    let tree = parse_perl_code(source)?;
    let query = Query::new(&language(), injections_query)?;
    let mut cursor = QueryCursor::new();

    let mut saw_inline_package = false;
    let mut saw_inline_language = false;
    let mut saw_injection_content = false;

    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let capture_name =
                query.capture_names().get(capture.index as usize).copied().unwrap_or_default();
            let text = capture.node.utf8_text(source.as_bytes()).unwrap_or_default();

            match capture_name {
                "inline.package" => saw_inline_package = text == "Inline",
                "inline.language" => saw_inline_language = text == "C",
                "injection.content" => {
                    saw_injection_content = capture.node.kind() == "heredoc_content"
                        && text.contains("#include <math.h>");
                }
                _ => {}
            }
        }
    }

    scenario.then("all expected captures should be present");
    assert!(saw_inline_package, "expected inline.package capture");
    assert!(saw_inline_language, "expected inline.language capture");
    assert!(saw_injection_content, "expected injection.content capture");
    Ok(())
}

#[test]
fn bdd_scanner_configuration_is_stable() {
    let scenario = Scenario::new("scanner configuration is stable");

    scenario.given("the crate backend is queried for scanner metadata");
    scenario.then("the backend should report the C scanner");
    assert_eq!(get_scanner_config(), "c-scanner");
}
