use crate::meta::Section;
use anyhow::Result;
use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn write_coverage_summary(dir: &Path, sections: &[Section]) -> Result<()> {
    let mut lines = summary_header();

    append_file_stats(&mut lines, sections);
    let tag_count = append_tag_stats(&mut lines, sections);
    append_flag_stats(&mut lines, sections);
    append_summary_stats(&mut lines, sections, tag_count);

    let report_path = dir.join("COVERAGE_SUMMARY.md");
    fs::write(report_path, lines.join("\n") + "\n")?;

    Ok(())
}

fn summary_header() -> Vec<String> {
    vec![
        "# Corpus Coverage Summary".to_string(),
        String::new(),
        format!("Generated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")),
        String::new(),
    ]
}

fn append_file_stats(lines: &mut Vec<String>, sections: &[Section]) {
    let mut by_file: BTreeMap<&str, usize> = BTreeMap::new();
    for section in sections {
        *by_file.entry(&section.file).or_default() += 1;
    }

    lines.push("## By File".to_string());
    lines.push(String::new());
    lines.push("| File | Sections |".to_string());
    lines.push("|------|----------|".to_string());
    for (file, count) in by_file {
        lines.push(format!("| {} | {} |", file, count));
    }
}

fn append_tag_stats(lines: &mut Vec<String>, sections: &[Section]) -> usize {
    let mut by_tag: BTreeMap<&str, usize> = BTreeMap::new();
    for section in sections {
        for tag in &section.tags {
            *by_tag.entry(tag).or_default() += 1;
        }
    }

    lines.push(String::new());
    lines.push("## By Tag (Top 20)".to_string());
    lines.push(String::new());
    lines.push("| Tag | Count |".to_string());
    lines.push("|-----|-------|".to_string());

    let unique_tag_count = by_tag.len();
    let mut tag_counts: Vec<_> = by_tag.into_iter().collect();
    tag_counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (tag, count) in tag_counts.iter().take(20) {
        lines.push(format!("| {} | {} |", tag, count));
    }

    unique_tag_count
}

fn append_flag_stats(lines: &mut Vec<String>, sections: &[Section]) {
    let mut by_flag: BTreeMap<&str, usize> = BTreeMap::new();
    for section in sections {
        for flag in &section.flags {
            *by_flag.entry(flag).or_default() += 1;
        }
    }

    if by_flag.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("## By Flag".to_string());
    lines.push(String::new());
    lines.push("| Flag | Count |".to_string());
    lines.push("|------|-------|".to_string());
    for (flag, count) in by_flag {
        lines.push(format!("| {} | {} |", flag, count));
    }
}

fn append_summary_stats(lines: &mut Vec<String>, sections: &[Section], unique_tag_count: usize) {
    lines.push(String::new());
    lines.push("## Summary".to_string());
    lines.push(String::new());
    lines.push(format!("- Total sections: {}", sections.len()));
    lines.push(format!(
        "- Total files: {}",
        sections
            .iter()
            .map(|section| &section.file)
            .collect::<std::collections::HashSet<_>>()
            .len()
    ));
    lines.push(format!("- Unique tags: {}", unique_tag_count));
}
