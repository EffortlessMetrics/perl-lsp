use anyhow::Result;
use perl_corpus::{
    EdgeCaseGenerator, find_by_flag, find_by_tag, generate_perl_code_with_seed, parse_dir,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempCorpusDir {
    path: PathBuf,
}

impl TempCorpusDir {
    fn new(prefix: &str) -> Result<Self> {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        path.push(format!("{prefix}_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempCorpusDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn given_nested_corpus_files_when_parse_dir_then_sections_are_discovered_sorted_and_filtered()
-> Result<()> {
    let dir = TempCorpusDir::new("perl_corpus_bdd_parse")?;

    let corpus_a = r#"==========================================
Regex Case
==========================================
# @id: bdd.regex.case
# @tags: regex, bdd
# @flags: parser-sensitive
m/foo/;

==========================================
Baseline Case
==========================================
# @id: bdd.base.case
# @tags: basic
my $x = 42;
"#;

    let nested = dir.path().join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(dir.path().join("a.txt"), corpus_a)?;
    fs::write(nested.join("_ignored.txt"), corpus_a)?;

    let corpus_b = r#"==========================================
Tie Case
==========================================
# @id: bdd.tie.case
# @tags: tie, bdd
# @flags: expected-error
my %h; tie %h, 'Store';
"#;
    fs::write(nested.join("b.txt"), corpus_b)?;

    let sections = parse_dir(dir.path())?;

    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0].id, "bdd.base.case");
    assert_eq!(sections[1].id, "bdd.regex.case");
    assert_eq!(sections[2].id, "bdd.tie.case");

    let bdd_tagged = find_by_tag(&sections, "bdd");
    let parser_sensitive = find_by_flag(&sections, "parser-sensitive");

    assert_eq!(bdd_tagged.len(), 2);
    assert_eq!(parser_sensitive.len(), 1);
    assert_eq!(parser_sensitive[0].id, "bdd.regex.case");

    Ok(())
}

#[test]
fn given_seeded_inputs_when_generating_code_or_sampling_edge_cases_then_results_are_deterministic()
{
    let code_1 = generate_perl_code_with_seed(12, 2026);
    let code_2 = generate_perl_code_with_seed(12, 2026);
    assert_eq!(code_1, code_2);

    let sample_1 = EdgeCaseGenerator::sample(77);
    let sample_2 = EdgeCaseGenerator::sample(77);

    assert_eq!(sample_1.id, sample_2.id);
    assert_eq!(sample_1.source, sample_2.source);
}

#[test]
fn given_known_tags_when_querying_edge_cases_then_bdd_related_cases_are_present() {
    let cases = EdgeCaseGenerator::by_tag("regex");

    assert!(!cases.is_empty());
    assert!(cases.iter().all(|case| case.tags.contains(&"regex")));
}
