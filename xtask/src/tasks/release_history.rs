//! release-history drift checks.

use crate::utils::project_root;
use color_eyre::eyre::{Result, bail};
use std::collections::HashSet;
use std::fs;
use std::process::Command;

pub fn run() -> Result<()> {
    let root = project_root()?;
    let release_history_path = root.join("RELEASE_HISTORY.md");
    let changelog_path = root.join("CHANGELOG.md");

    let release_history = fs::read_to_string(&release_history_path)?;
    let changelog = fs::read_to_string(&changelog_path)?;

    let all_tags = collect_non_rc_tags(&root)?;
    let cl_only_versions = collect_cl_only_versions(&release_history);

    let mut drift_found = false;

    for tag in &all_tags {
        if cl_only_versions.contains(tag) {
            continue;
        }

        let notes_file = root.join("docs").join("releases").join(format!("v{tag}.md"));
        if !notes_file.exists() {
            if !is_grandfathered_gap(&release_history, tag) {
                eprintln!("ERROR: Missing release notes: docs/releases/v{tag}.md");
                drift_found = true;
            } else {
                eprintln!(
                    "WARN: Grandfathered gap: v{tag} has no notes file (expected — see RELEASE_HISTORY.md)"
                );
            }
        }
    }

    for tag in &all_tags {
        if cl_only_versions.contains(tag) {
            continue;
        }

        if !release_history.contains(tag) {
            eprintln!("ERROR: Missing RELEASE_HISTORY.md entry for {tag}");
            drift_found = true;
        }
    }

    if let Some(newest_tag) = all_tags.last() {
        let changelog_header = format!("## [{newest_tag}]");
        if !changelog.contains(&changelog_header) {
            eprintln!("ERROR: Newest tag v{newest_tag} not found in CHANGELOG.md");
            drift_found = true;
        }
    } else {
        eprintln!("WARN: No non-RC tags found");
    }

    if drift_found {
        bail!("Release history drift detected.");
    }

    println!("Release history drift check passed.");
    Ok(())
}

fn collect_non_rc_tags(root: &std::path::Path) -> Result<Vec<String>> {
    let output =
        Command::new("git").arg("tag").arg("--list").arg("v*").current_dir(root).output()?;

    if !output.status.success() {
        bail!("failed to list git tags");
    }

    let mut tags: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .filter(|tag| !tag.contains("rc"))
        .map(|tag| tag.trim_start_matches('v').to_string())
        .collect();

    tags.sort_by(|a, b| semver_like_cmp(a, b));
    Ok(tags)
}

fn semver_like_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn parse_parts(s: &str) -> Vec<u32> {
        s.split(['.', '-']).map(|part| part.parse::<u32>().unwrap_or(0)).collect()
    }

    let a_parts = parse_parts(a);
    let b_parts = parse_parts(b);
    let max_len = a_parts.len().max(b_parts.len());

    for idx in 0..max_len {
        let left = a_parts.get(idx).copied().unwrap_or(0);
        let right = b_parts.get(idx).copied().unwrap_or(0);
        match left.cmp(&right) {
            std::cmp::Ordering::Equal => continue,
            ordering => return ordering,
        }
    }

    std::cmp::Ordering::Equal
}

fn collect_cl_only_versions(release_history: &str) -> HashSet<String> {
    release_history
        .lines()
        .filter(|line| line.contains("(CL)"))
        .filter_map(extract_bracketed_semver)
        .collect()
}

fn is_grandfathered_gap(release_history: &str, tag: &str) -> bool {
    release_history.contains(tag) && !release_history.contains(&format!("[n-{tag}]:"))
}

fn extract_bracketed_semver(line: &str) -> Option<String> {
    let start = line.find('[')?;
    let rest = &line[(start + 1)..];
    let end = rest.find(']')?;
    let candidate = &rest[..end];

    if candidate.chars().all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-')
        && candidate.contains('.')
    {
        Some(candidate.to_string())
    } else {
        None
    }
}
