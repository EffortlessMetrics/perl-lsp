use anyhow::Result;
use perl_corpus::{find_by_flag, find_by_tag, parse_dir, parse_file};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Result<Self> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn scenario_given_corpus_file_with_metadata_when_parsing_then_metadata_and_body_are_extracted()
-> Result<()> {
    let temp = TempDirGuard::new("perl_corpus_bdd_parse_file")?;
    let file = temp.path().join("feature.txt");

    fs::write(
        &file,
        r#"==========================================
Regex Scenario
==========================================
# @id: regex.named.capture
# @tags: Regex, Capture, Parser
# @flags: parser-sensitive, expected-error
my $value = "abc123";
$value =~ /(?<letters>[a-z]+)(?<digits>\d+)/;
---
(source_file
  (expression_statement ...))
"#,
    )?;

    let sections = parse_file(&file)?;
    let section = sections
        .iter()
        .find(|candidate| candidate.id == "regex.named.capture")
        .ok_or_else(|| anyhow::anyhow!("expected section with explicit id"))?;

    assert_eq!(section.title, "Regex Scenario");
    assert_eq!(section.tags, vec!["regex", "capture", "parser"]);
    assert_eq!(section.flags, vec!["parser-sensitive", "expected-error"]);
    assert!(section.body.contains("my $value = \"abc123\";"));
    assert!(!section.body.contains("(source_file"));

    Ok(())
}

#[test]
fn scenario_given_directory_when_parsing_then_sections_are_sorted_and_hidden_indexes_ignored()
-> Result<()> {
    let temp = TempDirGuard::new("perl_corpus_bdd_parse_dir")?;

    let alpha = temp.path().join("alpha.txt");
    fs::write(
        &alpha,
        r#"==========================================
Alpha Case
==========================================
# @id: zeta.case
print "alpha";
"#,
    )?;

    let beta = temp.path().join("nested").join("beta.txt");
    if let Some(parent) = beta.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &beta,
        r#"==========================================
Beta Case
==========================================
# @id: alpha.case
print "beta";
"#,
    )?;

    // Should be ignored by parse_dir because it is an index-style file.
    fs::write(
        temp.path().join("_index.txt"),
        r#"==========================================
Ignored
==========================================
# @id: ignored.case
print "ignore";
"#,
    )?;

    let sections = parse_dir(temp.path())?;
    let ids: Vec<&str> = sections.iter().map(|section| section.id.as_str()).collect();

    assert_eq!(ids, vec!["zeta.case", "alpha.case"]);

    Ok(())
}

#[test]
fn scenario_given_parsed_sections_when_filtering_then_tag_and_flag_queries_return_matching_cases()
-> Result<()> {
    let temp = TempDirGuard::new("perl_corpus_bdd_filter")?;
    let file = temp.path().join("filters.txt");

    fs::write(
        &file,
        r#"==========================================
Parser Sensitive Regex
==========================================
# @id: filters.regex.one
# @tags: regex, lexer
# @flags: parser-sensitive
my $x = "abc";

==========================================
Expected Error Parse
==========================================
# @id: filters.error.one
# @tags: parser
# @flags: expected-error
my $ = 42;

==========================================
No Flags
==========================================
# @id: filters.misc.one
# @tags: parser
my $ok = 42;
"#,
    )?;

    let sections = parse_file(&file)?;

    let parser_tag = find_by_tag(&sections, "parser");
    let parser_sensitive_flag = find_by_flag(&sections, "parser-sensitive");
    let expected_error_flag = find_by_flag(&sections, "expected-error");

    assert_eq!(parser_tag.len(), 2);
    assert_eq!(parser_sensitive_flag.len(), 1);
    assert_eq!(expected_error_flag.len(), 1);
    assert_eq!(parser_sensitive_flag[0].id, "filters.regex.one");
    assert_eq!(expected_error_flag[0].id, "filters.error.one");

    Ok(())
}
