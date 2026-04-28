use crate::lint::KNOWN_TAGS;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const CORPUS_EXTENSIONS: &[&str] = &["pl", "pm", "t", "psgi", "cgi", "txt"];
const GENERATOR_FAMILIES: &[&str] = &[
    "ambiguity",
    "builtins",
    "control_flow",
    "declarations",
    "expressions",
    "filetest",
    "format_statements",
    "glob",
    "heredoc",
    "io",
    "list_ops",
    "object_oriented",
    "phasers",
    "program",
    "quote_like",
    "qw",
    "regex",
    "sigils",
    "special_vars",
    "tie",
    "whitespace",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryIds {
    pub total: usize,
    pub missing: usize,
    pub duplicates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryTags {
    pub known: Vec<String>,
    pub unknown: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorpusInventory {
    pub schema_version: u32,
    pub files: usize,
    pub sections: usize,
    pub ids: InventoryIds,
    pub tags: InventoryTags,
    pub flags: BTreeMap<String, usize>,
    pub generators: Vec<String>,
    pub fixtures_without_concepts: Vec<String>,
    pub fixtures_without_expectations: Vec<String>,
    pub concept_mapping_available: bool,
    pub expectations_available: bool,
    pub markers: BTreeMap<String, usize>,
}

#[derive(Debug, Default)]
struct SectionEvidence {
    id: Option<String>,
    tags: Vec<String>,
    flags: Vec<String>,
    has_expected_separator: bool,
}

pub fn inventory_from_corpus_dir(corpus_dir: &Path) -> Result<CorpusInventory> {
    let files = collect_corpus_files(corpus_dir)?;

    let mut sections = 0usize;
    let mut ids_seen: HashMap<String, usize> = HashMap::new();
    let mut ids_total = 0usize;
    let mut ids_missing = 0usize;
    let mut known_tags: BTreeSet<String> = BTreeSet::new();
    let mut unknown_tags: BTreeSet<String> = BTreeSet::new();
    let mut flags: BTreeMap<String, usize> = BTreeMap::new();
    let mut markers: BTreeMap<String, usize> = BTreeMap::new();

    for file in &files {
        if !has_extension(file, &["txt"]) {
            continue;
        }

        let contents = fs::read_to_string(file)?;
        let section_entries = parse_section_evidence(&contents);
        for entry in section_entries {
            sections += 1;

            if let Some(id) = entry.id {
                ids_total += 1;
                *ids_seen.entry(id).or_default() += 1;
            } else {
                ids_missing += 1;
            }

            for tag in entry.tags {
                if KNOWN_TAGS.iter().any(|known| *known == tag) {
                    known_tags.insert(tag);
                } else {
                    unknown_tags.insert(tag);
                }
            }

            for flag in entry.flags {
                *flags.entry(flag.clone()).or_default() += 1;
                if flag == "expected-error" || flag == "wip" || flag == "parser-sensitive" {
                    *markers.entry(flag).or_default() += 1;
                }
            }

            if entry.has_expected_separator {
                *markers.entry("has-expectation-separator".to_string()).or_default() += 1;
            }
        }
    }

    for special in ["expected-error", "wip", "parser-sensitive"] {
        markers.entry(special.to_string()).or_default();
    }
    let duplicates: Vec<String> = ids_seen
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let fixtures_without_expectations = find_fixtures_without_expectations(corpus_dir)?;
    let expectations_available = has_gold_fixtures(corpus_dir)?;

    let inventory = CorpusInventory {
        schema_version: 1,
        files: files.len(),
        sections,
        ids: InventoryIds { total: ids_total, missing: ids_missing, duplicates },
        tags: InventoryTags {
            known: known_tags.into_iter().collect(),
            unknown: unknown_tags.into_iter().collect(),
        },
        flags,
        generators: GENERATOR_FAMILIES.iter().map(|family| (*family).to_string()).collect(),
        fixtures_without_concepts: Vec::new(),
        fixtures_without_expectations,
        concept_mapping_available: false,
        expectations_available,
        markers,
    };

    Ok(inventory)
}

pub fn inventory_json_from_corpus_dir(corpus_dir: &Path) -> Result<String> {
    let report = inventory_from_corpus_dir(corpus_dir)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

fn parse_section_evidence(contents: &str) -> Vec<SectionEvidence> {
    let mut entries = Vec::new();
    let lines: Vec<&str> = contents.lines().collect();

    let mut idx = 0usize;
    while idx < lines.len() {
        if !is_delimiter(lines[idx]) {
            idx += 1;
            continue;
        }

        if idx + 1 >= lines.len() {
            break;
        }

        let title_line = lines[idx + 1];
        let looks_like_section = !title_line.trim().is_empty();
        if !looks_like_section {
            idx += 1;
            continue;
        }

        let mut cursor = idx + 2;
        if cursor < lines.len() && is_delimiter(lines[cursor]) {
            cursor += 1;
        }

        let mut entry = SectionEvidence::default();

        while cursor < lines.len() {
            let line = lines[cursor];
            if is_delimiter(line) {
                break;
            }

            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("# @id:") {
                let value = rest.trim();
                if !value.is_empty() {
                    entry.id = Some(value.to_string());
                }
            } else if let Some(rest) = trimmed.strip_prefix("# @tags:") {
                entry.tags = split_words(rest);
            } else if let Some(rest) = trimmed.strip_prefix("# @flags:") {
                entry.flags = split_words(rest);
            }

            if trimmed == "---" {
                entry.has_expected_separator = true;
            }

            cursor += 1;
        }

        entries.push(entry);
        idx = cursor;
    }

    entries
}

fn split_words(input: &str) -> Vec<String> {
    input
        .replace(',', " ")
        .split_whitespace()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn is_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch == '=')
}

fn collect_corpus_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.starts_with('_') {
                continue;
            }

            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file() && has_extension(&path, CORPUS_EXTENSIONS) {
                files.push(path);
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| allowed.iter().any(|candidate| ext.eq_ignore_ascii_case(candidate)))
        .unwrap_or(false)
}

fn has_gold_fixtures(root: &Path) -> Result<bool> {
    let gold_root = root.join("gold");
    if !gold_root.exists() {
        return Ok(false);
    }

    for entry in fs::read_dir(gold_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        if entry.path().join("fixture.pl").exists() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn find_fixtures_without_expectations(root: &Path) -> Result<Vec<String>> {
    let gold_root = root.join("gold");
    if !gold_root.exists() {
        return Ok(Vec::new());
    }

    let mut missing = Vec::new();
    for entry in fs::read_dir(gold_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let path = entry.path();
        if !path.join("fixture.pl").exists() {
            continue;
        }

        let has_expectations = path.join("expected.json").exists()
            || path.join("expected_hover.json").exists()
            || path.join("expected_goto.json").exists()
            || path.join("expected_completion.json").exists();

        if !has_expectations {
            missing.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    missing.sort();
    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        dir.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        must(fs::create_dir_all(&dir));
        dir
    }

    #[test]
    fn inventory_reports_duplicate_and_missing_ids() {
        let root = temp_dir("perl_corpus_inventory_ids");

        let corpus = r#"==========================================
First
==========================================
# @id: duplicate.case
# @tags: regex, unknown-tag
# @flags: parser-sensitive, wip
say 'one';
---
expected

==========================================
Second
==========================================
# @id: duplicate.case
say 'two';

==========================================
Third
==========================================
# @tags: scalar
say 'three';
"#;

        must(fs::write(root.join("sample.txt"), corpus));

        let inventory = must(inventory_from_corpus_dir(&root));

        assert_eq!(inventory.schema_version, 1);
        assert_eq!(inventory.sections, 3);
        assert_eq!(inventory.ids.total, 2);
        assert_eq!(inventory.ids.missing, 1);
        assert_eq!(inventory.ids.duplicates, vec!["duplicate.case".to_string()]);
        assert!(inventory.tags.known.contains(&"regex".to_string()));
        assert!(inventory.tags.unknown.contains(&"unknown-tag".to_string()));
        assert_eq!(inventory.markers.get("parser-sensitive"), Some(&1));
        assert_eq!(inventory.markers.get("wip"), Some(&1));
        assert_eq!(inventory.markers.get("has-expectation-separator"), Some(&1));

        must(fs::remove_dir_all(root));
    }

    #[test]
    fn inventory_reports_expectation_availability_and_missing_expectations() {
        let root = temp_dir("perl_corpus_inventory_gold");
        let gold_root = root.join("gold");
        must(fs::create_dir_all(gold_root.join("with_expected")));
        must(fs::create_dir_all(gold_root.join("without_expected")));

        must(fs::write(gold_root.join("with_expected/fixture.pl"), "say 'ok';\n"));
        must(fs::write(gold_root.join("with_expected/expected.json"), "{\"diagnostics\":[]}"));
        must(fs::write(gold_root.join("without_expected/fixture.pl"), "say 'missing';\n"));

        let inventory = must(inventory_from_corpus_dir(&root));

        assert!(inventory.expectations_available);
        assert_eq!(inventory.fixtures_without_expectations, vec!["without_expected".to_string()]);
        assert!(!inventory.concept_mapping_available);
        assert!(inventory.fixtures_without_concepts.is_empty());

        must(fs::remove_dir_all(root));
    }

    #[test]
    fn inventory_json_is_deterministic() {
        let root = temp_dir("perl_corpus_inventory_json");
        let corpus = r#"==========================================
Alpha
==========================================
# @id: a.id
# @tags: regex, scalar
say 'a';
"#;
        must(fs::write(root.join("a.txt"), corpus));

        let first = must(inventory_json_from_corpus_dir(&root));
        let second = must(inventory_json_from_corpus_dir(&root));
        assert_eq!(first, second);

        must(fs::remove_dir_all(root));
    }
}
