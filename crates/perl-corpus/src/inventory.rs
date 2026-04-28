use crate::lint::KNOWN_TAGS;
use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::{fs, path::Path};

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

#[derive(Debug, Clone)]
struct ParsedInventorySection {
    id: Option<String>,
    tags: Vec<String>,
    flags: Vec<String>,
    has_expectation_separator: bool,
    has_expectation_content: bool,
    locator: String,
}

static SEC_RE: once_cell::sync::Lazy<Option<Regex>> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(?m)^=+\s*$").ok());
static META_RE: once_cell::sync::Lazy<Option<Regex>> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$").ok()
});

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

pub fn inventory_dir(dir: &Path) -> Result<CorpusInventory> {
    let pattern = format!("{}/**/*.txt", dir.display());
    let mut files = Vec::new();

    for entry in glob::glob(&pattern)? {
        let path = entry?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        if filename.starts_with('_') || filename.starts_with('.') {
            continue;
        }
        files.push(path);
    }

    files.sort();

    let mut parsed_sections = Vec::new();
    for file in &files {
        parsed_sections.extend(parse_file_sections_for_inventory(file)?);
    }

    let mut id_counts: HashMap<String, usize> = HashMap::new();
    let mut ids_total = 0usize;
    let mut ids_missing = 0usize;
    for section in &parsed_sections {
        if let Some(id) = &section.id {
            ids_total += 1;
            *id_counts.entry(id.clone()).or_default() += 1;
        } else {
            ids_missing += 1;
        }
    }

    let mut duplicates: Vec<String> =
        id_counts.into_iter().filter_map(|(id, count)| (count > 1).then_some(id)).collect();
    duplicates.sort();

    let known_tag_set: HashSet<&str> = KNOWN_TAGS.iter().copied().collect();
    let mut known_tags = BTreeSet::new();
    let mut unknown_tags = BTreeSet::new();
    for section in &parsed_sections {
        for tag in &section.tags {
            if known_tag_set.contains(tag.as_str()) {
                known_tags.insert(tag.clone());
            } else {
                unknown_tags.insert(tag.clone());
            }
        }
    }

    let mut flags: BTreeMap<String, usize> = BTreeMap::new();
    for section in &parsed_sections {
        for flag in &section.flags {
            *flags.entry(flag.clone()).or_default() += 1;
        }
    }

    // Marker-focused counters required by inventory report.
    for marker in ["expected-error", "wip", "parser-sensitive"] {
        let marker_count = parsed_sections
            .iter()
            .filter(|section| {
                section.flags.iter().any(|flag| flag == marker)
                    || section.tags.iter().any(|tag| tag == marker)
            })
            .count();
        flags.entry(marker.to_string()).or_insert(marker_count);
    }

    let expectation_info_available =
        parsed_sections.iter().any(|section| section.has_expectation_separator);
    let fixtures_without_expectations = if expectation_info_available {
        let mut missing: Vec<String> = parsed_sections
            .iter()
            .filter(|section| !section.has_expectation_content)
            .map(|section| section.locator.clone())
            .collect();
        missing.sort();
        missing
    } else {
        vec!["unavailable".to_string()]
    };

    Ok(CorpusInventory {
        schema_version: 1,
        files: files.len(),
        sections: parsed_sections.len(),
        ids: InventoryIds { total: ids_total, missing: ids_missing, duplicates },
        tags: InventoryTags {
            known: known_tags.into_iter().collect(),
            unknown: unknown_tags.into_iter().collect(),
        },
        flags,
        generators: GENERATOR_FAMILIES.iter().map(|name| (*name).to_string()).collect(),
        fixtures_without_concepts: vec!["unavailable".to_string()],
        fixtures_without_expectations,
    })
}

fn parse_file_sections_for_inventory(path: &Path) -> Result<Vec<ParsedInventorySection>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut sections = Vec::new();

    let Some(sec_re) = SEC_RE.as_ref() else {
        return Ok(sections);
    };

    let raw_delims: Vec<usize> = sec_re.find_iter(&text).map(|m| m.start()).collect();

    let mut opening_delims: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < raw_delims.len() {
        opening_delims.push(raw_delims[i]);
        if i + 1 < raw_delims.len() {
            let between = &text[raw_delims[i]..raw_delims[i + 1]];
            if between.lines().count() == 2 {
                i += 2;
                continue;
            }
        }
        i += 1;
    }

    let mut offs = vec![0usize];
    offs.extend(&opening_delims);
    offs.dedup();
    offs.push(text.len());

    for window in offs.windows(2) {
        let start = window[0];
        let end = window[1];

        let first_line = text[start..end].lines().next().unwrap_or("");
        if !sec_re.is_match(first_line) {
            continue;
        }

        let section_text = &text[start..end];
        let lines: Vec<&str> = section_text.lines().collect();
        if lines.len() < 2 {
            continue;
        }

        let title = lines[1].trim().to_string();
        let after_title_idx = if lines.len() > 2 && sec_re.is_match(lines[2]) { 3 } else { 2 };

        let mut meta = HashMap::<String, String>::new();
        let mut body_start_idx = after_title_idx;
        if let Some(meta_re) = META_RE.as_ref() {
            for (line_idx, line) in lines.iter().enumerate().skip(after_title_idx) {
                if let Some(cap) = meta_re.captures(line) {
                    meta.insert(cap["k"].to_string(), cap["v"].trim().to_string());
                    body_start_idx = line_idx + 1;
                } else if !line.starts_with('#') || line.trim().is_empty() {
                    body_start_idx = line_idx;
                    break;
                }
            }
        }

        let id = meta.get("id").cloned().and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        });

        let tags = meta
            .get("tags")
            .map(|value| {
                value
                    .replace(',', " ")
                    .split_whitespace()
                    .map(|tag| tag.to_lowercase())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let flags = meta
            .get("flags")
            .map(|value| {
                value
                    .replace(',', " ")
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let body_lines = if body_start_idx < lines.len() { &lines[body_start_idx..] } else { &[] };
        let expectation_separator_idx = body_lines.iter().position(|line| line.trim() == "---");
        let has_expectation_separator = expectation_separator_idx.is_some();
        let has_expectation_content = expectation_separator_idx
            .is_some_and(|idx| body_lines[idx + 1..].iter().any(|line| !line.trim().is_empty()));

        let locator =
            format!("{}:{}", path.file_name().unwrap_or_default().to_string_lossy(), title);

        sections.push(ParsedInventorySection {
            id,
            tags,
            flags,
            has_expectation_separator,
            has_expectation_content,
            locator,
        });
    }

    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        path.push(format!("{}_{}", prefix, nanos));
        path
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            must(fs::create_dir_all(parent));
        }
        must(fs::write(path, content));
    }

    #[test]
    fn inventory_reports_missing_and_duplicate_ids() {
        let dir = temp_dir("perl_corpus_inventory_ids");
        must(fs::create_dir_all(&dir));

        write_file(
            &dir.join("sample.txt"),
            r#"==========================================
One
==========================================
# @id: duplicate.id
# @tags: regex, made-up
# @flags: parser-sensitive, wip
print "a";
---
(expect one)

==========================================
Two
==========================================
# @id: duplicate.id
# @tags: scalar
print "b";

==========================================
Three
==========================================
# @tags: hash
print "c";
"#,
        );

        let inventory = must(inventory_dir(&dir));
        must(fs::remove_dir_all(&dir));

        assert_eq!(inventory.schema_version, 1);
        assert_eq!(inventory.files, 1);
        assert_eq!(inventory.sections, 3);
        assert_eq!(inventory.ids.total, 2);
        assert_eq!(inventory.ids.missing, 1);
        assert_eq!(inventory.ids.duplicates, vec!["duplicate.id".to_string()]);
        assert_eq!(inventory.tags.known, vec!["hash", "regex", "scalar"]);
        assert_eq!(inventory.tags.unknown, vec!["made-up"]);
        assert_eq!(inventory.flags.get("parser-sensitive"), Some(&1));
        assert_eq!(inventory.flags.get("wip"), Some(&1));
        assert_eq!(inventory.flags.get("expected-error"), Some(&0));
    }

    #[test]
    fn inventory_output_is_deterministic() {
        let dir = temp_dir("perl_corpus_inventory_deterministic");
        must(fs::create_dir_all(&dir));

        write_file(
            &dir.join("a.txt"),
            r#"==========================================
A
==========================================
# @id: z.id
# @tags: unknown-z, regex
print "a";
"#,
        );

        write_file(
            &dir.join("b.txt"),
            r#"==========================================
B
==========================================
# @id: a.id
# @tags: scalar
# @flags: parser-sensitive
print "b";
---
(expectation)
"#,
        );

        let first = must(inventory_dir(&dir));
        let second = must(inventory_dir(&dir));
        must(fs::remove_dir_all(&dir));

        assert_eq!(first, second);
        assert_eq!(
            first.generators,
            GENERATOR_FAMILIES.iter().map(|name| (*name).to_string()).collect::<Vec<_>>()
        );
        assert_eq!(first.fixtures_without_concepts, vec!["unavailable".to_string()]);
    }
}
