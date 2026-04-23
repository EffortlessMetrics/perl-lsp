//! Workspace version synchronization: discover every place that references
//! the workspace version, check them for drift, and rewrite them on bump.
//!
//! The canonical source of truth is `[workspace.package] version` in the
//! root `Cargo.toml`. Every other site listed here must exactly match that
//! value. Historical references (changelog entries, release notes, blog
//! posts, GitHub Release URLs, and PR references) are immutable and are NOT
//! tracked by this module.
//!
//! Two public entry points:
//! - [`check`] — used by the CI gate to fail on drift.
//! - [`bump`]  — used by `cargo xtask bump-version` to update every site.
//!
//! Both walk exactly the same list of sites, so the CI gate is guaranteed
//! to catch anything the bump command could have updated.

use color_eyre::eyre::{Result, bail, eyre};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// A discovered reference to the workspace version somewhere on disk.
#[derive(Debug, Clone)]
pub struct VersionSite {
    /// Repo-relative path of the file.
    pub path: PathBuf,
    /// 1-based line number inside the file.
    pub line: usize,
    /// Human description of what this site is (for error messages).
    pub description: String,
    /// The version currently written at that site.
    pub found: String,
}

/// Semantic version X.Y.Z validation regex. Keep in sync with bump's CLI
/// validation — they must accept the same shape.
pub fn validate_version_format(version: &str) -> Result<()> {
    if !SEMVER_EXACT_RE.is_match(version) {
        bail!("invalid version format: {version:?} (expected X.Y.Z)");
    }
    Ok(())
}

/// Read the canonical workspace version from `Cargo.toml`.
pub fn read_workspace_version(repo_root: &Path) -> Result<String> {
    let path = repo_root.join("Cargo.toml");
    let raw = fs::read_to_string(&path).map_err(|e| eyre!("reading {}: {e}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&raw).map_err(|e| eyre!("parsing {}: {e}", path.display()))?;
    let version = value
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre!("Cargo.toml is missing [workspace.package] version"))?;
    Ok(version.to_string())
}

/// Discover every version site in the repo.
///
/// Each site's `found` field records the version currently written there.
/// A consistent repo has all sites equal to [`read_workspace_version`].
pub fn collect_sites(repo_root: &Path) -> Result<Vec<VersionSite>> {
    let mut sites = Vec::new();

    // 1. Root Cargo.toml — [workspace.package] version + every
    //    [workspace.dependencies] path = "crates/..." version entry.
    collect_root_cargo_toml_sites(repo_root, &mut sites)?;

    // 2. Each crate's Cargo.toml — package version (if hardcoded) and any
    //    path-based internal dependency that specifies a version field.
    collect_crate_cargo_toml_sites(repo_root, &mut sites)?;

    // 3. features.toml — `[meta] version`.
    collect_features_toml_site(repo_root, &mut sites)?;

    // 4. vscode-extension/package.json (and package-lock.json).
    collect_vscode_sites(repo_root, &mut sites)?;

    // 5. Doc surface: README.md, CLAUDE.md, docs/project/ROADMAP.md.
    collect_doc_sites(repo_root, &mut sites)?;

    Ok(sites)
}

/// Check that every discovered site matches the canonical workspace
/// version. Returns `Ok(())` on success or a descriptive error listing
/// every mismatched site.
pub fn check(repo_root: &Path) -> Result<()> {
    let workspace_version = read_workspace_version(repo_root)?;
    let sites = collect_sites(repo_root)?;
    if sites.is_empty() {
        bail!("no version sites discovered — this is a bug in check-version-sync");
    }

    println!("Version sync check:");
    println!("  Canonical (Cargo.toml workspace): {workspace_version}");
    println!("  Discovered version sites: {}", sites.len());

    let mismatches: Vec<&VersionSite> =
        sites.iter().filter(|s| s.found != workspace_version).collect();

    if mismatches.is_empty() {
        println!("Version sync check: all {} sites agree on {workspace_version}", sites.len());
        return Ok(());
    }

    eprintln!(
        "Version mismatch detected: {} site(s) out of sync with workspace version {workspace_version}",
        mismatches.len()
    );
    for site in &mismatches {
        eprintln!(
            "  {}:{} — {} (found {:?}, expected {:?})",
            site.path.display(),
            site.line,
            site.description,
            site.found,
            workspace_version
        );
    }
    bail!(
        "version mismatch: {} site(s) drifted from workspace version {workspace_version}; \
         run `cargo xtask bump-version {workspace_version}` to resynchronize",
        mismatches.len()
    );
}

/// Rewrite every discovered site to `new_version`. Idempotent — sites
/// already at `new_version` are left untouched.
pub fn bump(repo_root: &Path, new_version: &str) -> Result<BumpReport> {
    validate_version_format(new_version)?;

    let sites = collect_sites(repo_root)?;
    if sites.is_empty() {
        bail!("no version sites discovered — this is a bug in bump-version");
    }

    // Group sites by file to minimize I/O and keep edits atomic per file.
    let mut by_file: std::collections::BTreeMap<PathBuf, Vec<VersionSite>> =
        std::collections::BTreeMap::new();
    for site in sites {
        by_file.entry(site.path.clone()).or_default().push(site);
    }

    let mut report = BumpReport::default();
    for (rel_path, file_sites) in by_file {
        let abs_path = repo_root.join(&rel_path);
        let content = fs::read_to_string(&abs_path)
            .map_err(|e| eyre!("reading {}: {e}", abs_path.display()))?;

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut file_updated = 0usize;
        let mut file_unchanged = 0usize;

        for site in &file_sites {
            let idx = site
                .line
                .checked_sub(1)
                .ok_or_else(|| eyre!("invalid line number 0 in {}", rel_path.display()))?;
            if idx >= lines.len() {
                bail!(
                    "line {} out of range in {} (file has {} lines)",
                    site.line,
                    rel_path.display(),
                    lines.len()
                );
            }
            let line = &lines[idx];
            let updated = rewrite_version_in_line(line, &site.found, new_version);
            if updated == *line {
                file_unchanged += 1;
            } else {
                lines[idx] = updated;
                file_updated += 1;
            }
        }

        if file_updated > 0 {
            // Preserve exact trailing whitespace (including multiple blank
            // lines and whether the file ended with a newline at all). We
            // compute the suffix once from the original content and append
            // it to the reconstituted body.
            let trailing = trailing_newline_suffix(&content);
            let new_content = lines.join("\n") + trailing;
            fs::write(&abs_path, new_content)
                .map_err(|e| eyre!("writing {}: {e}", abs_path.display()))?;
            report.files_updated += 1;
            report.sites_updated += file_updated;
            report.touched_files.push(rel_path.clone());
        }
        report.sites_unchanged += file_unchanged;
        report.sites_total += file_updated + file_unchanged;
    }

    Ok(report)
}

/// Summary returned from [`bump`].
#[derive(Debug, Default)]
pub struct BumpReport {
    pub sites_total: usize,
    pub sites_updated: usize,
    pub sites_unchanged: usize,
    pub files_updated: usize,
    pub touched_files: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Line rewriter
// ---------------------------------------------------------------------------

/// Compute the trailing newline suffix of a string that `str::lines()`
/// would discard. `str::lines()` strips the final `\n` if present; our
/// round-trip must add it back to preserve file shape. We return `"\n"`
/// if the content ends with a newline, otherwise `""`.
///
/// Note: a file ending in `\n\n` has its penultimate `\n` preserved by
/// `lines().join("\n")` (because an empty string becomes its own entry),
/// so we still only need to append a single `\n` here.
fn trailing_newline_suffix(content: &str) -> &'static str {
    if content.ends_with('\n') { "\n" } else { "" }
}

/// Rewrite the first occurrence of `old` (as a whole semver string) to `new`
/// inside a single line. This is intentionally narrow: we only ever replace
/// the exact semver string we already identified at this site, so there is
/// no risk of clobbering unrelated numbers.
fn rewrite_version_in_line(line: &str, old: &str, new: &str) -> String {
    if old == new {
        return line.to_string();
    }
    // Only replace the first occurrence — every site points to exactly one
    // version token on its line.
    if let Some(idx) = line.find(old) {
        let mut out = String::with_capacity(line.len() - old.len() + new.len());
        out.push_str(&line[..idx]);
        out.push_str(new);
        out.push_str(&line[idx + old.len()..]);
        out
    } else {
        line.to_string()
    }
}

// ---------------------------------------------------------------------------
// Collectors
// ---------------------------------------------------------------------------

static SEMVER_EXACT_RE: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"^\d+\.\d+\.\d+$"));
static BARE_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r#"^\s*version\s*=\s*"(\d+\.\d+\.\d+)""#));
static WORKSPACE_DEP_WITH_VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(r#"\{\s*path\s*=\s*"crates/[^"]+"[^}]*version\s*=\s*"(\d+\.\d+\.\d+)""#)
});
static CRATE_DEP_WITH_VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(r#"\{\s*path\s*=\s*"\.\.?/[^"]+"[^}]*version\s*=\s*"(\d+\.\d+\.\d+)""#)
});
static JSON_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r#"^\s*"version"\s*:\s*"(\d+\.\d+\.\d+)""#));
static LOCKFILE_ROOT_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r#"^  "version"\s*:\s*"(\d+\.\d+\.\d+)""#));
static LOCKFILE_SELF_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r#"^      "version"\s*:\s*"(\d+\.\d+\.\d+)""#));
static README_RELEASE_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"\*\*Current release:\s*v(\d+\.\d+\.\d+)\*\*"));
static CLAUDE_RELEASE_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"\*\*Latest Release\*\*:\s*(\d+\.\d+\.\d+)"));
static ROADMAP_WORKSPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"Workspace version line:\s*`v(\d+\.\d+\.\d+)`"));
static ROADMAP_PUBLISHED_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"Latest published release:\s*`v(\d+\.\d+\.\d+)"));

fn compile_regex(pattern: &str) -> Regex {
    match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(err) => unreachable!("internal regex must be valid: {err}"),
    }
}

fn collect_root_cargo_toml_sites(repo_root: &Path, sites: &mut Vec<VersionSite>) -> Result<()> {
    let rel = PathBuf::from("Cargo.toml");
    let abs = repo_root.join(&rel);
    let raw = fs::read_to_string(&abs).map_err(|e| eyre!("reading {}: {e}", abs.display()))?;

    let mut in_workspace_package = false;
    let mut in_workspace_dependencies = false;
    let mut seen_package_version = false;

    for (idx, line) in raw.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();

        if trimmed.starts_with('[') {
            in_workspace_package = trimmed.starts_with("[workspace.package]");
            in_workspace_dependencies = trimmed.starts_with("[workspace.dependencies]");
            continue;
        }

        if in_workspace_package
            && !seen_package_version
            && let Some(caps) = BARE_VERSION_RE.captures(line)
        {
            let v = caps[1].to_string();
            sites.push(VersionSite {
                path: rel.clone(),
                line: line_no,
                description: "[workspace.package] version".to_string(),
                found: v,
            });
            seen_package_version = true;
            continue;
        }

        if in_workspace_dependencies
            && let Some(caps) = WORKSPACE_DEP_WITH_VERSION_RE.captures(line)
        {
            // Name is everything before the first `=` on the line.
            let name = line.split_once('=').map(|(n, _)| n.trim()).unwrap_or("<unknown>");
            let v = caps[1].to_string();
            sites.push(VersionSite {
                path: rel.clone(),
                line: line_no,
                description: format!("[workspace.dependencies] {name}"),
                found: v,
            });
        }
    }

    Ok(())
}

/// Crate directories that are NOT workspace members and therefore do NOT
/// track the workspace version. They are listed in `[workspace.exclude]`
/// in the root `Cargo.toml` and may drift to their own version cadence.
const EXCLUDED_CRATE_DIRS: &[&str] = &["tree-sitter-perl-c"];

fn collect_crate_cargo_toml_sites(repo_root: &Path, sites: &mut Vec<VersionSite>) -> Result<()> {
    let crates_dir = repo_root.join("crates");
    if !crates_dir.is_dir() {
        return Ok(());
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(&crates_dir)
        .map_err(|e| eyre!("reading {}: {e}", crates_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| !EXCLUDED_CRATE_DIRS.contains(&n))
                .unwrap_or(true)
        })
        .collect();
    entries.sort();

    for crate_dir in entries {
        let manifest = crate_dir.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let rel = manifest
            .strip_prefix(repo_root)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| manifest.clone());
        let raw = fs::read_to_string(&manifest)
            .map_err(|e| eyre!("reading {}: {e}", manifest.display()))?;

        let mut in_package = false;
        let mut seen_package_version = false;
        let mut in_deps = false;

        for (idx, line) in raw.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim_start();

            if trimmed.starts_with('[') {
                in_package = trimmed.starts_with("[package]");
                // Any [dependencies] / [dev-dependencies] / [build-dependencies]
                // / [target.*.dependencies] section.
                in_deps = trimmed.contains("dependencies]");
                continue;
            }

            if in_package
                && !seen_package_version
                && let Some(caps) = BARE_VERSION_RE.captures(line)
            {
                let v = caps[1].to_string();
                sites.push(VersionSite {
                    path: rel.clone(),
                    line: line_no,
                    description: format!(
                        "{} [package] version",
                        crate_dir.file_name().and_then(|n| n.to_str()).unwrap_or("<crate>")
                    ),
                    found: v,
                });
                seen_package_version = true;
                continue;
            }

            if in_deps && let Some(caps) = CRATE_DEP_WITH_VERSION_RE.captures(line) {
                let name = line.split_once('=').map(|(n, _)| n.trim()).unwrap_or("<unknown>");
                let v = caps[1].to_string();
                sites.push(VersionSite {
                    path: rel.clone(),
                    line: line_no,
                    description: format!(
                        "{} dependency on {name}",
                        crate_dir.file_name().and_then(|n| n.to_str()).unwrap_or("<crate>")
                    ),
                    found: v,
                });
            }
        }
    }

    Ok(())
}

fn collect_features_toml_site(repo_root: &Path, sites: &mut Vec<VersionSite>) -> Result<()> {
    let rel = PathBuf::from("features.toml");
    let abs = repo_root.join(&rel);
    if !abs.is_file() {
        return Ok(());
    }
    let raw = fs::read_to_string(&abs).map_err(|e| eyre!("reading {}: {e}", abs.display()))?;

    let mut in_meta = false;
    for (idx, line) in raw.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_meta = trimmed.starts_with("[meta]");
            continue;
        }
        if in_meta && let Some(caps) = BARE_VERSION_RE.captures(line) {
            sites.push(VersionSite {
                path: rel.clone(),
                line: line_no,
                description: "features.toml [meta] version".to_string(),
                found: caps[1].to_string(),
            });
            break;
        }
    }
    Ok(())
}

fn collect_vscode_sites(repo_root: &Path, sites: &mut Vec<VersionSite>) -> Result<()> {
    // package.json: exactly one top-level "version" field.
    let pkg_rel = PathBuf::from("vscode-extension/package.json");
    let pkg_abs = repo_root.join(&pkg_rel);
    if pkg_abs.is_file() {
        let raw = fs::read_to_string(&pkg_abs)
            .map_err(|e| eyre!("reading {}: {e}", pkg_abs.display()))?;
        // First top-level "version" line (indented by 2 spaces in our formatted JSON).
        for (idx, line) in raw.lines().enumerate() {
            if let Some(caps) = JSON_VERSION_RE.captures(line) {
                sites.push(VersionSite {
                    path: pkg_rel.clone(),
                    line: idx + 1,
                    description: "vscode-extension package.json version".to_string(),
                    found: caps[1].to_string(),
                });
                break;
            }
        }
    }

    // package-lock.json: the lockfile has two top-level version references —
    // the root "version" and the "" package entry — both pinned to the
    // workspace version.
    let lock_rel = PathBuf::from("vscode-extension/package-lock.json");
    let lock_abs = repo_root.join(&lock_rel);
    if lock_abs.is_file() {
        let raw = fs::read_to_string(&lock_abs)
            .map_err(|e| eyre!("reading {}: {e}", lock_abs.display()))?;
        // Match only the first two version lines at the root and at the ""
        // package entry. The lockfile has many other `"version"` references
        // for transitive deps that we must NOT touch.
        //
        // Strategy: we look at indentation. The root "version" is at indent
        // of 2 spaces (top level of the JSON object). The "" package entry
        // is inside `"packages": { "": { ... "version": ... } }` and sits at
        // indent 6. Any deeper indentation is a transitive dep.
        let mut found_root = false;
        let mut found_self = false;
        let mut in_empty_package = false;
        for (idx, line) in raw.lines().enumerate() {
            let line_no = idx + 1;
            if !found_root && let Some(caps) = LOCKFILE_ROOT_VERSION_RE.captures(line) {
                sites.push(VersionSite {
                    path: lock_rel.clone(),
                    line: line_no,
                    description: "vscode-extension package-lock.json root version".to_string(),
                    found: caps[1].to_string(),
                });
                found_root = true;
                continue;
            }
            if !found_self {
                if line.trim_start().starts_with("\"\": {") {
                    in_empty_package = true;
                    continue;
                }
                if in_empty_package && let Some(caps) = LOCKFILE_SELF_VERSION_RE.captures(line) {
                    sites.push(VersionSite {
                        path: lock_rel.clone(),
                        line: line_no,
                        description: "vscode-extension package-lock.json self-package version"
                            .to_string(),
                        found: caps[1].to_string(),
                    });
                    found_self = true;
                }
            }
            if found_root && found_self {
                break;
            }
        }
    }

    Ok(())
}

fn collect_doc_sites(repo_root: &Path, sites: &mut Vec<VersionSite>) -> Result<()> {
    // README.md: "**Current release: v<version>**"
    collect_single_line_doc_site(
        repo_root,
        "README.md",
        "README current release line",
        &README_RELEASE_RE,
        sites,
    )?;

    // CLAUDE.md: "**Latest Release**: <version>"
    collect_single_line_doc_site(
        repo_root,
        "CLAUDE.md",
        "CLAUDE.md latest release line",
        &CLAUDE_RELEASE_RE,
        sites,
    )?;

    // docs/project/ROADMAP.md: "Workspace version line: `v<version>`"
    collect_single_line_doc_site(
        repo_root,
        "docs/project/ROADMAP.md",
        "ROADMAP workspace version line",
        &ROADMAP_WORKSPACE_RE,
        sites,
    )?;

    // docs/project/ROADMAP.md: "Latest published release: `v<version>`"
    collect_single_line_doc_site(
        repo_root,
        "docs/project/ROADMAP.md",
        "ROADMAP latest published release",
        &ROADMAP_PUBLISHED_RE,
        sites,
    )?;

    Ok(())
}

fn collect_single_line_doc_site(
    repo_root: &Path,
    rel_path: &str,
    description: &str,
    pattern: &Regex,
    sites: &mut Vec<VersionSite>,
) -> Result<()> {
    let rel = PathBuf::from(rel_path);
    let abs = repo_root.join(&rel);
    if !abs.is_file() {
        return Ok(());
    }
    let raw = fs::read_to_string(&abs).map_err(|e| eyre!("reading {}: {e}", abs.display()))?;
    for (idx, line) in raw.lines().enumerate() {
        if let Some(caps) = pattern.captures(line) {
            sites.push(VersionSite {
                path: rel.clone(),
                line: idx + 1,
                description: description.to_string(),
                found: caps[1].to_string(),
            });
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_version_in_line_replaces_only_target() {
        let line = r#"perl-foo = { path = "crates/perl-foo", version = "0.12.2" }"#;
        let updated = rewrite_version_in_line(line, "0.12.2", "0.13.0");
        assert_eq!(updated, r#"perl-foo = { path = "crates/perl-foo", version = "0.13.0" }"#);
    }

    #[test]
    fn rewrite_version_in_line_is_idempotent() {
        let line = r#"version = "0.12.2""#;
        let updated = rewrite_version_in_line(line, "0.12.2", "0.12.2");
        assert_eq!(updated, line);
    }

    #[test]
    fn rewrite_version_in_line_leaves_unmatched_line_alone() {
        let line = r#"description = "perl-foo""#;
        let updated = rewrite_version_in_line(line, "0.12.2", "0.13.0");
        assert_eq!(updated, line);
    }

    #[test]
    fn validate_version_format_accepts_semver() {
        assert!(validate_version_format("0.12.2").is_ok());
        assert!(validate_version_format("1.0.0").is_ok());
        assert!(validate_version_format("12.345.6789").is_ok());
    }

    #[test]
    fn validate_version_format_rejects_garbage() {
        assert!(validate_version_format("v0.12.2").is_err());
        assert!(validate_version_format("0.12").is_err());
        assert!(validate_version_format("0.12.2-rc1").is_err());
        assert!(validate_version_format("").is_err());
    }
}
