//! Perl parser corpus - test data and property-based testing infrastructure
#![allow(clippy::pedantic)] // Corpus crate - focus on core clippy lints only

pub mod r#gen;
pub mod index;
pub mod lint;
pub mod meta;

use anyhow::{Context, Result};
use meta::Section;
use regex::Regex;
use std::collections::HashMap;
use std::{fs, path::Path};

static SEC_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^=+\s*$")
        .unwrap_or_else(|_| panic!("SEC_RE regex is invalid - this is a bug in the corpus parser"))
});
static META_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$")
        .unwrap_or_else(|_| panic!("META_RE regex is invalid - this is a bug in the corpus parser"))
});

/// Parse a corpus file into sections.
pub fn parse_file(path: &Path) -> Result<Vec<Section>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut sections = Vec::new();

    // Find all section delimiters
    let mut offs = vec![0usize];
    for m in SEC_RE.find_iter(&text) {
        offs.push(m.start());
    }
    // Add EOF sentinel
    offs.push(text.len());

    for w in offs.windows(2) {
        let start = w[0];
        let end = w[1];
        if start == 0 {
            continue;
        } // skip prelude

        // Extract section text
        let section_text = &text[start..end];
        let lines: Vec<&str> = section_text.lines().collect();

        if lines.len() < 2 {
            continue;
        } // malformed section

        // Title is the line after "===="
        let title = lines[1].trim().to_string();

        // Gather metadata lines following title
        let mut meta = HashMap::<String, String>::new();
        let mut body_start_idx = 2;

        for (i, line) in lines.iter().enumerate().skip(2) {
            if let Some(cap) = META_RE.captures(line) {
                meta.insert(cap["k"].to_string(), cap["v"].trim().to_string());
                body_start_idx = i + 1;
            } else if !line.starts_with('#') || line.trim().is_empty() {
                body_start_idx = i;
                break;
            }
        }

        // Extract metadata fields
        let id = meta.get("id").cloned().unwrap_or_default();
        let tags = meta
            .get("tags")
            .map(|s| {
                s.replace(',', " ").split_whitespace().map(|t| t.to_lowercase()).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let perl = meta.get("perl").cloned().filter(|s| !s.is_empty());
        let flags = meta
            .get("flags")
            .map(|s| {
                s.replace(',', " ").split_whitespace().map(|t| t.to_string()).collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Extract body (code after metadata)
        let body = lines[body_start_idx..].join("\n").trim().to_string();

        // Calculate line number (for error reporting)
        let line_num = text[..start].lines().count() + 1;

        sections.push(Section {
            id,
            title,
            file: path.file_name().unwrap_or_default().to_string_lossy().into(),
            tags,
            perl,
            flags,
            body,
            line: Some(line_num),
        });
    }

    Ok(sections)
}

/// Scan the `test/corpus/` directory.
pub fn parse_dir(dir: &Path) -> Result<Vec<Section>> {
    let mut all = Vec::new();

    // Build glob pattern
    let pattern = format!("{}/**/*.txt", dir.display());

    for entry in glob::glob(&pattern)? {
        let p = entry?;

        // Skip index/tag files
        let filename = p.file_name().unwrap_or_default().to_string_lossy();
        if filename.starts_with('_') || filename.starts_with('.') {
            continue;
        }

        all.extend(parse_file(&p)?);
    }

    // Sort by file and ID for stable output
    all.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.id.cmp(&b.id)));

    Ok(all)
}

/// Find sections by tag
pub fn find_by_tag<'a>(sections: &'a [Section], tag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_tag(tag)).collect()
}

/// Find sections by flag
pub fn find_by_flag<'a>(sections: &'a [Section], flag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_flag(flag)).collect()
}
