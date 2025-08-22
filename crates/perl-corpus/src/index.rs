use crate::meta::Section;
use anyhow::Result;
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Serialize)]
struct IndexEntry<'a> {
    id: &'a str,
    file: &'a str,
    title: &'a str,
    tags: &'a [String],
    perl: &'a Option<String>,
    flags: &'a [String],
}

/// Write index files and coverage report
pub fn write_indices(dir: &Path, sections: &[Section]) -> Result<()> {
    // Build flat index
    let index: Vec<IndexEntry> = sections
        .iter()
        .map(|s| IndexEntry {
            id: &s.id,
            file: &s.file,
            title: &s.title,
            tags: &s.tags,
            perl: &s.perl,
            flags: &s.flags,
        })
        .collect();

    // Write _index.json
    let idx_path = dir.join("_index.json");
    fs::write(&idx_path, serde_json::to_vec_pretty(&index)?)?;

    // Build tag map
    let mut tagmap: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for s in sections {
        for tag in &s.tags {
            tagmap.entry(tag.clone()).or_default().push(&s.id);
        }
    }

    // Write _tags.json
    let tags_path = dir.join("_tags.json");
    fs::write(&tags_path, serde_json::to_vec_pretty(&tagmap)?)?;

    // Generate coverage summary
    write_coverage_summary(dir, sections)?;

    Ok(())
}

fn write_coverage_summary(dir: &Path, sections: &[Section]) -> Result<()> {
    let mut lines = vec![
        "# Corpus Coverage Summary".to_string(),
        String::new(),
        format!("Generated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")),
        String::new(),
    ];

    // Stats by file
    let mut by_file: BTreeMap<&str, usize> = BTreeMap::new();
    for s in sections {
        *by_file.entry(&s.file).or_default() += 1;
    }

    lines.push("## By File".to_string());
    lines.push(String::new());
    lines.push("| File | Sections |".to_string());
    lines.push("|------|----------|".to_string());
    for (file, count) in by_file {
        lines.push(format!("| {} | {} |", file, count));
    }

    // Stats by tag
    let mut by_tag: BTreeMap<&str, usize> = BTreeMap::new();
    for s in sections {
        for tag in &s.tags {
            *by_tag.entry(tag).or_default() += 1;
        }
    }

    lines.push(String::new());
    lines.push("## By Tag (Top 20)".to_string());
    lines.push(String::new());
    lines.push("| Tag | Count |".to_string());
    lines.push("|-----|-------|".to_string());

    let mut tag_counts: Vec<_> = by_tag.into_iter().collect();
    tag_counts.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (tag, count) in tag_counts.iter().take(20) {
        lines.push(format!("| {} | {} |", tag, count));
    }

    // Stats by flag
    let mut by_flag: BTreeMap<&str, usize> = BTreeMap::new();
    for s in sections {
        for flag in &s.flags {
            *by_flag.entry(flag).or_default() += 1;
        }
    }

    if !by_flag.is_empty() {
        lines.push(String::new());
        lines.push("## By Flag".to_string());
        lines.push(String::new());
        lines.push("| Flag | Count |".to_string());
        lines.push("|------|-------|".to_string());
        for (flag, count) in by_flag {
            lines.push(format!("| {} | {} |", flag, count));
        }
    }

    // Summary stats
    lines.push(String::new());
    lines.push("## Summary".to_string());
    lines.push(String::new());
    lines.push(format!("- Total sections: {}", sections.len()));
    lines.push(format!(
        "- Total files: {}",
        sections.iter().map(|s| &s.file).collect::<std::collections::HashSet<_>>().len()
    ));
    lines.push(format!("- Unique tags: {}", tag_counts.len()));

    let report_path = dir.join("COVERAGE_SUMMARY.md");
    fs::write(report_path, lines.join("\n") + "\n")?;

    Ok(())
}
