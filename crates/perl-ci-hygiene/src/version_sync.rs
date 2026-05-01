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
    /// When true, this site tracks the published/released channel (VS Code Marketplace,
    /// GitHub Releases) and is intentionally allowed to lag behind a pre-release workspace
    /// version. During a pre-release cycle (workspace version contains `-`), mismatches
    /// on channel-split sites are reported as warnings rather than hard failures.
    pub channel_split: bool,
}

impl VersionSite {
    /// Construct a standard (non-channel-split) site.
    fn new(path: PathBuf, line: usize, description: String, found: String) -> Self {
        Self { path, line, description, found, channel_split: false }
    }

    /// Construct a channel-split site that is allowed to lag during pre-release cycles.
    fn channel(path: PathBuf, line: usize, description: String, found: String) -> Self {
        Self { path, line, description, found, channel_split: true }
    }
}

/// Semantic version X.Y.Z[-pre] validation. Accepts stable versions (`X.Y.Z`)
/// and pre-release versions (`X.Y.Z-alpha`, `X.Y.Z-rc1`, `X.Y.Z-beta.2`, etc.).
/// The pre-release suffix must consist of alphanumeric segments separated by dots or
/// dashes. Keep in sync with bump's CLI validation — they must accept the same shape.
pub fn validate_version_format(version: &str) -> Result<()> {
    // Split on the first '-' to separate the base version from the optional pre-release tag.
    let (base, pre_release) =
        version.split_once('-').map(|(b, p)| (b, Some(p))).unwrap_or((version, None));

    let mut parts = base.split('.');

    let major = parts.next().ok_or_else(|| {
        eyre!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)")
    })?;
    let minor = parts.next().ok_or_else(|| {
        eyre!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)")
    })?;
    let patch = parts.next().ok_or_else(|| {
        eyre!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)")
    })?;

    if parts.next().is_some()
        || major.is_empty()
        || minor.is_empty()
        || patch.is_empty()
        || !major.chars().all(|ch| ch.is_ascii_digit())
        || !minor.chars().all(|ch| ch.is_ascii_digit())
        || !patch.chars().all(|ch| ch.is_ascii_digit())
    {
        bail!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)");
    }

    // Validate the pre-release tag if present: alphanumeric segments separated by '.' or '-'.
    if let Some(pre) = pre_release {
        let invalid = pre.is_empty()
            || !pre.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-');
        if invalid {
            bail!(
                "invalid pre-release suffix in {version:?}: {pre:?} (expected alphanumeric segments)"
            );
        }
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

/// Returns `true` when `version` is a pre-release version (contains a `-` suffix,
/// e.g. `0.13.0-rc1`, `1.2.3-alpha`).
pub fn is_pre_release(version: &str) -> bool {
    version.contains('-')
}

/// Check that every discovered site matches the canonical workspace
/// version. Returns `Ok(())` on success or a descriptive error listing
/// every mismatched site.
///
/// Channel-split sites (VS Code Marketplace / GitHub Releases) intentionally
/// lag behind the workspace version during pre-release cycles.  When the
/// workspace version is a pre-release (contains `-`), mismatches on those
/// sites are printed as warnings but do not cause the check to fail.
pub fn check(repo_root: &Path) -> Result<()> {
    let workspace_version = read_workspace_version(repo_root)?;
    let sites = collect_sites(repo_root)?;
    if sites.is_empty() {
        bail!("no version sites discovered — this is a bug in check-version-sync");
    }

    let pre_release = is_pre_release(&workspace_version);

    println!("Version sync check:");
    println!("  Canonical (Cargo.toml workspace): {workspace_version}");
    println!("  Discovered version sites: {}", sites.len());
    if pre_release {
        println!(
            "  Pre-release mode: channel-split sites (vscode-extension) may lag behind {workspace_version}"
        );
    }

    // Hard mismatches: all sites that are NOT channel-split (or channel-split sites
    // during a stable release cycle where they must match exactly).
    let hard_mismatches: Vec<&VersionSite> = sites
        .iter()
        .filter(|s| s.found != workspace_version && (!s.channel_split || !pre_release))
        .collect();

    // Soft mismatches: channel-split sites allowed to lag during pre-release.
    let soft_mismatches: Vec<&VersionSite> = sites
        .iter()
        .filter(|s| s.found != workspace_version && s.channel_split && pre_release)
        .collect();

    for site in &soft_mismatches {
        println!(
            "  [warn] channel-split site {}:{} — {} (found {:?}, workspace is {:?}; \
             will be updated on stable release)",
            site.path.display(),
            site.line,
            site.description,
            site.found,
            workspace_version
        );
    }

    if hard_mismatches.is_empty() {
        let total_in_sync = sites.len() - soft_mismatches.len();
        println!(
            "Version sync check: {total_in_sync} hard site(s) agree on {workspace_version}\
             {} soft warning(s) for channel-split lag",
            if soft_mismatches.is_empty() {
                ", 0".to_string()
            } else {
                format!(", {} ", soft_mismatches.len())
            }
        );
        return Ok(());
    }

    eprintln!(
        "Version mismatch detected: {} site(s) out of sync with workspace version {workspace_version}",
        hard_mismatches.len()
    );
    for site in &hard_mismatches {
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
        hard_mismatches.len()
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

/// Shared fragment for matching a semver string that optionally includes a
/// pre-release suffix (e.g. `0.13.0-rc1`, `1.2.3-beta.2`). Used in all
/// site-discovery regexes so pre-release versions are tracked consistently.
const VERSION_FRAGMENT: &str = r"\d+\.\d+\.\d+(?:-[A-Za-z0-9][A-Za-z0-9.\-]*)?";

static BARE_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r#"^\s*version\s*=\s*"({VERSION_FRAGMENT})""#)));
static WORKSPACE_DEP_WITH_VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(&format!(
        r#"\{{\s*path\s*=\s*["']crates/[^"']+["'][^}}]*version\s*=\s*"({VERSION_FRAGMENT})""#
    ))
});
static CRATE_DEP_WITH_VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(&format!(
        r#"\{{\s*path\s*=\s*["']\.\.?/[^"']+["'][^}}]*version\s*=\s*"({VERSION_FRAGMENT})""#
    ))
});
static JSON_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r#"^\s*"version"\s*:\s*"({VERSION_FRAGMENT})""#)));
static LOCKFILE_ROOT_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r#"^  "version"\s*:\s*"({VERSION_FRAGMENT})""#)));
static LOCKFILE_SELF_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r#"^      "version"\s*:\s*"({VERSION_FRAGMENT})""#)));
static README_RELEASE_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r"\*\*Current release:\s*v({VERSION_FRAGMENT})\*\*")));
static CLAUDE_RELEASE_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r"\*\*Latest Release\*\*:\s*({VERSION_FRAGMENT})")));
static ROADMAP_WORKSPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r"Workspace version line:\s*`v({VERSION_FRAGMENT})`")));
static ROADMAP_PUBLISHED_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(&format!(r"Latest published release:\s*`v({VERSION_FRAGMENT})`"))
});

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
            sites.push(VersionSite::new(
                rel.clone(),
                line_no,
                "[workspace.package] version".to_string(),
                v,
            ));
            seen_package_version = true;
            continue;
        }

        if in_workspace_dependencies
            && let Some(caps) = WORKSPACE_DEP_WITH_VERSION_RE.captures(line)
        {
            // Name is everything before the first `=` on the line.
            let name = line.split_once('=').map(|(n, _)| n.trim()).unwrap_or("<unknown>");
            let v = caps[1].to_string();
            sites.push(VersionSite::new(
                rel.clone(),
                line_no,
                format!("[workspace.dependencies] {name}"),
                v,
            ));
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
                sites.push(VersionSite::new(
                    rel.clone(),
                    line_no,
                    format!(
                        "{} [package] version",
                        crate_dir.file_name().and_then(|n| n.to_str()).unwrap_or("<crate>")
                    ),
                    v,
                ));
                seen_package_version = true;
                continue;
            }

            if in_deps && let Some(caps) = CRATE_DEP_WITH_VERSION_RE.captures(line) {
                let name = line.split_once('=').map(|(n, _)| n.trim()).unwrap_or("<unknown>");
                let v = caps[1].to_string();
                sites.push(VersionSite::new(
                    rel.clone(),
                    line_no,
                    format!(
                        "{} dependency on {name}",
                        crate_dir.file_name().and_then(|n| n.to_str()).unwrap_or("<crate>")
                    ),
                    v,
                ));
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
            sites.push(VersionSite::new(
                rel.clone(),
                line_no,
                "features.toml [meta] version".to_string(),
                caps[1].to_string(),
            ));
            break;
        }
    }
    Ok(())
}

fn collect_vscode_sites(repo_root: &Path, sites: &mut Vec<VersionSite>) -> Result<()> {
    // package.json: exactly one top-level "version" field.
    //
    // Note: the VS Code Marketplace requires a pure X.Y.Z semver version; it does not
    // accept pre-release suffixes.  The extension version therefore intentionally lags
    // behind a pre-release workspace version (e.g. `0.13.0-rc1`) until a final release
    // is cut.  These sites are marked `channel_split = true` so that `check` can treat
    // them as warnings rather than hard failures when the workspace is on a pre-release.
    let pkg_rel = PathBuf::from("vscode-extension/package.json");
    let pkg_abs = repo_root.join(&pkg_rel);
    if pkg_abs.is_file() {
        let raw = fs::read_to_string(&pkg_abs)
            .map_err(|e| eyre!("reading {}: {e}", pkg_abs.display()))?;
        // First top-level "version" line (indented by 2 spaces in our formatted JSON).
        for (idx, line) in raw.lines().enumerate() {
            if let Some(caps) = JSON_VERSION_RE.captures(line) {
                sites.push(VersionSite::channel(
                    pkg_rel.clone(),
                    idx + 1,
                    "vscode-extension package.json version".to_string(),
                    caps[1].to_string(),
                ));
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
                sites.push(VersionSite::channel(
                    lock_rel.clone(),
                    line_no,
                    "vscode-extension package-lock.json root version".to_string(),
                    caps[1].to_string(),
                ));
                found_root = true;
                continue;
            }
            if !found_self {
                if line.trim_start().starts_with("\"\": {") {
                    in_empty_package = true;
                    continue;
                }
                if in_empty_package && let Some(caps) = LOCKFILE_SELF_VERSION_RE.captures(line) {
                    sites.push(VersionSite::channel(
                        lock_rel.clone(),
                        line_no,
                        "vscode-extension package-lock.json self-package version".to_string(),
                        caps[1].to_string(),
                    ));
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
            sites.push(VersionSite::new(
                rel.clone(),
                idx + 1,
                description.to_string(),
                caps[1].to_string(),
            ));
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_repo_dir(label: &str) -> Result<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| eyre!("system clock before unix epoch: {e}"))?
            .as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("perl-ci-hygiene-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| eyre!("creating {}: {e}", dir.display()))?;
        Ok(dir)
    }

    #[test]
    fn rewrite_version_in_line_replaces_only_target() {
        let line = r#"perl-foo = { path = "crates/perl-foo", version = "0.12.2" }"#;
        let updated = rewrite_version_in_line(line, "0.12.2", "0.13.0");
        assert_eq!(updated, r#"perl-foo = { path = "crates/perl-foo", version = "0.13.0" }"#);
    }

    #[test]
    fn rewrite_version_in_line_handles_pre_release_target() {
        let line = r#"perl-foo = { path = "crates/perl-foo", version = "0.13.0-rc1" }"#;
        let updated = rewrite_version_in_line(line, "0.13.0-rc1", "0.13.0");
        assert_eq!(updated, r#"perl-foo = { path = "crates/perl-foo", version = "0.13.0" }"#);
    }

    #[test]
    fn rewrite_version_in_line_stable_to_rc() {
        let line = r#"perl-foo = { path = "crates/perl-foo", version = "0.12.4" }"#;
        let updated = rewrite_version_in_line(line, "0.12.4", "0.13.0-rc1");
        assert_eq!(updated, r#"perl-foo = { path = "crates/perl-foo", version = "0.13.0-rc1" }"#);
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
    fn validate_version_format_accepts_pre_release_suffixes() {
        assert!(validate_version_format("0.13.0-rc1").is_ok());
        assert!(validate_version_format("1.0.0-alpha").is_ok());
        assert!(validate_version_format("0.12.0-beta.2").is_ok());
        assert!(validate_version_format("2.0.0-rc.1").is_ok());
    }

    #[test]
    fn validate_version_format_rejects_garbage() {
        assert!(validate_version_format("v0.12.2").is_err());
        assert!(validate_version_format("0.12").is_err());
        assert!(validate_version_format("").is_err());
        assert!(validate_version_format("1..2").is_err());
        assert!(validate_version_format("1.2.3.4").is_err());
        assert!(validate_version_format("1.two.3").is_err());
        // pre-release suffix with invalid characters
        assert!(validate_version_format("0.13.0-").is_err());
    }

    #[test]
    fn rewrite_version_in_line_updates_only_first_match() {
        let line = r#"version = "0.12.2" # historical "0.12.2""#;
        let updated = rewrite_version_in_line(line, "0.12.2", "0.13.0");
        assert_eq!(updated, r#"version = "0.13.0" # historical "0.12.2""#);
    }

    #[test]
    fn trailing_newline_suffix_preserves_expected_shape() {
        assert_eq!(trailing_newline_suffix("a"), "");
        assert_eq!(trailing_newline_suffix("a\n"), "\n");
        assert_eq!(trailing_newline_suffix("a\n\n"), "\n");
    }

    #[test]
    fn collect_vscode_sites_ignores_transitive_lockfile_versions() -> Result<()> {
        let repo_root = unique_temp_repo_dir("lockfile-scan")?;
        let vscode_dir = repo_root.join("vscode-extension");
        fs::create_dir_all(&vscode_dir)
            .map_err(|e| eyre!("creating {}: {e}", vscode_dir.display()))?;

        let package_json = r#"{
  "name": "perl-lsp",
  "version": "0.42.0"
}"#;
        fs::write(vscode_dir.join("package.json"), package_json)
            .map_err(|e| eyre!("writing package.json: {e}"))?;

        let package_lock = r#"{
  "name": "perl-lsp",
  "version": "0.42.0",
  "packages": {
    "": {
      "version": "0.42.0"
    },
    "node_modules/x": {
      "version": "9.9.9"
    }
  }
}"#;
        fs::write(vscode_dir.join("package-lock.json"), package_lock)
            .map_err(|e| eyre!("writing package-lock.json: {e}"))?;

        let mut sites = Vec::new();
        collect_vscode_sites(&repo_root, &mut sites)?;

        let versions: Vec<String> = sites.iter().map(|site| site.found.clone()).collect();
        assert_eq!(
            versions,
            vec!["0.42.0".to_string(), "0.42.0".to_string(), "0.42.0".to_string()]
        );
        assert!(
            !versions.iter().any(|version| version == "9.9.9"),
            "transitive lockfile versions must not be collected"
        );

        fs::remove_dir_all(&repo_root)
            .map_err(|e| eyre!("cleanup {}: {e}", repo_root.display()))?;
        Ok(())
    }

    #[test]
    fn collect_crate_cargo_toml_sites_scans_all_dependency_sections() -> Result<()> {
        let repo_root = unique_temp_repo_dir("deps-sections")?;
        let crate_dir = repo_root.join("crates/example-crate");
        fs::create_dir_all(&crate_dir).map_err(|e| eyre!("creating crate dir: {e}"))?;

        let cargo_toml = r#"[package]
name = "example-crate"
version = "0.42.0"

[dependencies]
perl-lexer = { path = "../perl-lexer", version = "0.42.0" }

[target.'cfg(unix)'.dependencies]
perl-parser = { path = "../perl-parser", version = "0.42.0" }

[build-dependencies]
perl-token = { path = "../perl-token", version = "0.42.0" }
"#;
        fs::write(crate_dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| eyre!("writing test Cargo.toml: {e}"))?;

        let mut sites = Vec::new();
        collect_crate_cargo_toml_sites(&repo_root, &mut sites)?;

        let dep_sites =
            sites.iter().filter(|site| site.description.contains("dependency on")).count();
        assert_eq!(dep_sites, 3, "all dependency sections must be discovered");
        assert!(
            sites.iter().any(|site| site.description.contains("[package] version")),
            "package version should also be discovered"
        );

        fs::remove_dir_all(&repo_root)
            .map_err(|e| eyre!("cleanup {}: {e}", repo_root.display()))?;
        Ok(())
    }

    #[test]
    fn is_pre_release_identifies_rc_versions() {
        assert!(is_pre_release("0.13.0-rc1"));
        assert!(is_pre_release("1.0.0-alpha"));
        assert!(is_pre_release("2.0.0-beta.3"));
        assert!(!is_pre_release("0.13.0"));
        assert!(!is_pre_release("1.2.3"));
    }

    #[test]
    fn vscode_sites_are_marked_channel_split() -> Result<()> {
        let repo_root = unique_temp_repo_dir("channel-split")?;
        let vscode_dir = repo_root.join("vscode-extension");
        fs::create_dir_all(&vscode_dir)
            .map_err(|e| eyre!("creating {}: {e}", vscode_dir.display()))?;

        let package_json = r#"{
  "name": "perl-lsp",
  "version": "0.12.4"
}"#;
        fs::write(vscode_dir.join("package.json"), package_json)
            .map_err(|e| eyre!("writing package.json: {e}"))?;

        let package_lock = r#"{
  "name": "perl-lsp",
  "version": "0.12.4",
  "packages": {
    "": {
      "version": "0.12.4"
    }
  }
}"#;
        fs::write(vscode_dir.join("package-lock.json"), package_lock)
            .map_err(|e| eyre!("writing package-lock.json: {e}"))?;

        let mut sites = Vec::new();
        collect_vscode_sites(&repo_root, &mut sites)?;

        assert_eq!(sites.len(), 3, "should find 3 vscode sites");
        assert!(sites.iter().all(|s| s.channel_split), "all vscode sites must be channel-split");

        fs::remove_dir_all(&repo_root)
            .map_err(|e| eyre!("cleanup {}: {e}", repo_root.display()))?;
        Ok(())
    }

    #[test]
    fn bare_version_re_matches_pre_release() {
        let line = r#"version = "0.13.0-rc1""#;
        let caps = BARE_VERSION_RE.captures(line);
        assert!(caps.is_some(), "BARE_VERSION_RE must match pre-release versions");
        assert_eq!(&caps.unwrap()[1], "0.13.0-rc1");
    }

    #[test]
    fn workspace_dep_re_matches_pre_release() {
        let line = r#"perl-foo = { path = "crates/perl-foo", version = "0.13.0-rc1" }"#;
        let caps = WORKSPACE_DEP_WITH_VERSION_RE.captures(line);
        assert!(caps.is_some(), "WORKSPACE_DEP_WITH_VERSION_RE must match pre-release versions");
        assert_eq!(&caps.unwrap()[1], "0.13.0-rc1");
    }

    #[test]
    fn workspace_dep_re_matches_single_quoted_path() {
        let line = r#"perl-foo = { path = 'crates/perl-foo', version = "0.13.0-rc1" }"#;
        let caps = WORKSPACE_DEP_WITH_VERSION_RE.captures(line);
        assert!(caps.is_some(), "WORKSPACE_DEP_WITH_VERSION_RE must match single-quoted paths");
        assert_eq!(&caps.unwrap()[1], "0.13.0-rc1");
    }

    #[test]
    fn claude_re_matches_pre_release_full_version() {
        let line = "**Latest Release**: 0.13.0-rc1 | **Metrics**: [status]";
        let caps = CLAUDE_RELEASE_RE.captures(line);
        assert!(caps.is_some(), "CLAUDE_RELEASE_RE must match pre-release versions");
        assert_eq!(
            &caps.unwrap()[1],
            "0.13.0-rc1",
            "CLAUDE_RELEASE_RE must capture the full version including pre-release suffix"
        );
    }

    #[test]
    fn roadmap_workspace_re_matches_pre_release() {
        let line = "- Workspace version line: `v0.13.0-rc1`";
        let caps = ROADMAP_WORKSPACE_RE.captures(line);
        assert!(caps.is_some(), "ROADMAP_WORKSPACE_RE must match pre-release versions");
        assert_eq!(&caps.unwrap()[1], "0.13.0-rc1");
    }
}
