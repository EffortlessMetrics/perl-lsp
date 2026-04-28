use crate::lint::KNOWN_TAGS;
use crate::meta::Section;
use crate::parse_file;
use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Stable inventory schema for corpus shape reporting.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InventoryIds {
    pub total: usize,
    pub missing: usize,
    pub duplicates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InventoryTags {
    pub known: Vec<String>,
    pub unknown: Vec<String>,
}

#[derive(Debug)]
struct SectionAudit {
    section: Section,
    id_missing: bool,
    has_expectation_block: bool,
}

/// Build an inventory report for a section-based corpus directory.
pub fn build_inventory(dir: &Path) -> Result<CorpusInventory> {
    let corpus_files = discover_corpus_txt_files(dir)?;
    let mut audited_sections = Vec::new();

    for file in &corpus_files {
        audited_sections.extend(parse_file_with_audit(file)?);
    }

    let sections: Vec<&Section> = audited_sections.iter().map(|audit| &audit.section).collect();

    let mut seen_ids = HashSet::new();
    let mut duplicate_ids = BTreeSet::new();
    let mut missing_ids = 0usize;

    for audit in &audited_sections {
        if audit.id_missing {
            missing_ids += 1;
        }

        if !seen_ids.insert(audit.section.id.clone()) {
            duplicate_ids.insert(audit.section.id.clone());
        }
    }

    let known_tag_set: HashSet<&str> = KNOWN_TAGS.iter().copied().collect();
    let mut known_tags = BTreeSet::new();
    let mut unknown_tags = BTreeSet::new();

    for section in &sections {
        for tag in &section.tags {
            if known_tag_set.contains(tag.as_str()) {
                known_tags.insert(tag.clone());
            } else {
                unknown_tags.insert(tag.clone());
            }
        }
    }

    let mut flags: BTreeMap<String, usize> = BTreeMap::new();
    for section in &sections {
        for flag in &section.flags {
            *flags.entry(flag.clone()).or_default() += 1;
        }
    }

    let marker_counts = [
        ("expected-error", marker_count(&sections, "expected-error")),
        ("wip", marker_count(&sections, "wip")),
        ("parser-sensitive", marker_count(&sections, "parser-sensitive")),
    ];

    for (marker, count) in marker_counts {
        if count > 0 {
            flags.insert(marker.to_string(), count);
        }
    }

    let fixtures_without_expectations = audited_sections
        .iter()
        .filter(|audit| !audit.has_expectation_block)
        .map(|audit| format!("{}:{}", audit.section.file, audit.section.id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(CorpusInventory {
        schema_version: 1,
        files: corpus_files.len(),
        sections: audited_sections.len(),
        ids: InventoryIds {
            total: audited_sections.len(),
            missing: missing_ids,
            duplicates: duplicate_ids.into_iter().collect(),
        },
        tags: InventoryTags {
            known: known_tags.into_iter().collect(),
            unknown: unknown_tags.into_iter().collect(),
        },
        flags,
        generators: generator_families(),
        fixtures_without_concepts: vec!["unavailable".to_string()],
        fixtures_without_expectations,
    })
}

/// Exposed generator families from `perl-corpus::gen`.
pub fn generator_families() -> Vec<String> {
    [
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
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn discover_corpus_txt_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current_dir) = stack.pop() {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if name.starts_with('.') || name.starts_with('_') {
                continue;
            }

            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                files.push(path);
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn parse_file_with_audit(path: &Path) -> Result<Vec<SectionAudit>> {
    let text = fs::read_to_string(path)?;
    let sections = parse_file(path)?;
    let chunks = split_sections(&text);

    let audited = sections
        .into_iter()
        .enumerate()
        .map(|(idx, section)| {
            let chunk = chunks.get(idx).copied().unwrap_or_default();
            SectionAudit {
                id_missing: !chunk_has_metadata(chunk, "id"),
                has_expectation_block: chunk.lines().any(|line| line.trim() == "---"),
                section,
            }
        })
        .collect();

    Ok(audited)
}

fn split_sections(text: &str) -> Vec<&str> {
    let mut line_starts = vec![0usize];
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            line_starts.push(idx + 1);
        }
    }

    let mut raw_delims = Vec::new();
    for start in line_starts {
        if start < text.len() && is_delimiter_at(text, start) {
            raw_delims.push(start);
        }
    }

    let mut opening_delims = Vec::new();
    let mut index = 0usize;
    while index < raw_delims.len() {
        opening_delims.push(raw_delims[index]);
        if index + 1 < raw_delims.len() {
            let between = &text[raw_delims[index]..raw_delims[index + 1]];
            if between.lines().count() == 2 {
                index += 2;
                continue;
            }
        }
        index += 1;
    }

    if opening_delims.is_empty() {
        return Vec::new();
    }

    let mut sections = Vec::new();
    for window in opening_delims.windows(2) {
        sections.push(&text[window[0]..window[1]]);
    }
    if let Some(last) = opening_delims.last().copied() {
        sections.push(&text[last..]);
    }

    sections
}

fn is_delimiter_at(text: &str, offset: usize) -> bool {
    let line = text[offset..].lines().next().unwrap_or_default().trim();
    !line.is_empty() && line.chars().all(|ch| ch == '=')
}

fn chunk_has_metadata(chunk: &str, key: &str) -> bool {
    chunk.lines().any(|line| {
        let line = line.trim();
        line.starts_with("#")
            && line
                .strip_prefix('#')
                .map(|rest| rest.trim_start().starts_with(&format!("@{key}:")))
                .unwrap_or(false)
    })
}

fn marker_count(sections: &[&Section], marker: &str) -> usize {
    sections.iter().filter(|section| section.has_flag(marker) || section.has_tag(marker)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        path.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        path
    }

    #[test]
    fn inventory_reports_duplicate_and_missing_ids() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("perl_corpus_inventory");
        fs::create_dir_all(&root)?;

        let corpus = r#"==========================================
Case A
==========================================
# @id: duplicate.case
# @tags: regex, custom-tag
# @flags: parser-sensitive
say 'a';
---
(expected)

==========================================
Case B
==========================================
# @id: duplicate.case
# @tags: wip
say 'b';

==========================================
Case C
==========================================
# @flags: expected-error
say 'c';
"#;

        let path = root.join("sample.txt");
        must(fs::write(&path, corpus));

        let inventory = build_inventory(&root)?;

        assert_eq!(inventory.schema_version, 1);
        assert_eq!(inventory.files, 1);
        assert_eq!(inventory.sections, 3);
        assert_eq!(inventory.ids.total, 3);
        assert_eq!(inventory.ids.missing, 1);
        assert_eq!(inventory.ids.duplicates, vec!["duplicate.case"]);
        assert_eq!(inventory.tags.known, vec!["regex"]);
        assert_eq!(inventory.tags.unknown, vec!["custom-tag", "wip"]);
        assert_eq!(inventory.flags.get("expected-error"), Some(&1));
        assert_eq!(inventory.flags.get("parser-sensitive"), Some(&1));
        assert_eq!(inventory.flags.get("wip"), Some(&1));
        assert_eq!(inventory.fixtures_without_concepts, vec!["unavailable"]);
        assert_eq!(inventory.fixtures_without_expectations.len(), 2);
        assert!(inventory.fixtures_without_expectations[0].starts_with("sample.txt:"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn generator_families_are_deterministic() {
        let first = generator_families();
        let second = generator_families();
        assert_eq!(first, second);
        assert_eq!(first.first().map(String::as_str), Some("ambiguity"));
        assert_eq!(first.last().map(String::as_str), Some("whitespace"));
    }
}
