use perl_corpus::index::write_indices;
use perl_corpus::lint::{LintConfig, lint_with_config};
use perl_corpus::{find_by_flag, find_by_tag, parse_dir};
use perl_tdd_support::{must, must_some};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn scenario_temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    path.push(format!("{}_{}_{}", prefix, pid, nanos));
    path
}

fn write_corpus_file(dir: &Path, name: &str, body: &str) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join(name);
    fs::write(&path, body)?;
    Ok(path)
}

#[test]
fn bdd_given_corpus_when_parsing_then_sections_are_discoverable_by_metadata()
-> Result<(), Box<dyn Error>> {
    // Given: a corpus directory with section metadata and mixed tags/flags.
    let dir = scenario_temp_dir("perl_corpus_bdd_parse");
    let corpus_text = r#"==========================================
Variable declaration
==========================================
# @id: scalar.decl
# @tags: scalar, declaration
my $value = 41;

==========================================
Smartmatch branch
==========================================
# @id: flow.smartmatch
# @tags: flow, given, when
# @flags: parser-sensitive
use v5.10;
given ($value) {
    when (41) { say 'forty one'; }
}
"#;

    must(write_corpus_file(&dir, "sample.txt", corpus_text));

    // When: we parse and query by tag and flag.
    let sections = must(parse_dir(&dir));
    let flow_sections = find_by_tag(&sections, "flow");
    let parser_sensitive = find_by_flag(&sections, "parser-sensitive");

    // Then: the expected section ids and normalized metadata are present.
    let scalar_decl = must_some(sections.iter().find(|section| section.id == "scalar.decl"));
    assert_eq!(scalar_decl.tags, vec!["scalar", "declaration"]);
    assert_eq!(flow_sections.len(), 1);
    assert_eq!(flow_sections[0].id, "flow.smartmatch");
    assert_eq!(parser_sensitive.len(), 1);
    assert_eq!(parser_sensitive[0].id, "flow.smartmatch");

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn bdd_given_valid_sections_when_lint_and_index_then_outputs_are_written()
-> Result<(), Box<dyn Error>> {
    // Given: a valid corpus directory.
    let dir = scenario_temp_dir("perl_corpus_bdd_index");
    let corpus_text = r#"==========================================
Regex capture
==========================================
# @id: regex.capture
# @tags: regex, capture, test
my ($lhs, $rhs) = ('left', 'right') =~ /(left).*(right)/;
"#;

    must(write_corpus_file(&dir, "regex.txt", corpus_text));
    let sections = must(parse_dir(&dir));

    // When: linting and index generation run.
    let lint_config = LintConfig {
        max_sections_per_file: 8,
        check_unknown_tags: false,
        check_unknown_flags: true,
        require_perl_version: false,
    };
    must(lint_with_config(&sections, &lint_config));
    must(write_indices(&dir, &sections));

    // Then: index artifacts exist and include the generated id.
    let index_path = dir.join("_index.json");
    let tags_path = dir.join("_tags.json");
    let coverage_path = dir.join("COVERAGE_SUMMARY.md");

    assert!(index_path.exists());
    assert!(tags_path.exists());
    assert!(coverage_path.exists());

    let index_json = fs::read_to_string(index_path)?;
    let tags_json = fs::read_to_string(tags_path)?;
    assert!(index_json.contains("regex.capture"));
    assert!(tags_json.contains("regex"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}
