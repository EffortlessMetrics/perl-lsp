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

mod model;
mod validate;

pub use model::{BumpReport, VersionSite};
pub use validate::{is_pre_release, validate_version_format};

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
///
/// Also appends a row + 4 link refs to `RELEASE_HISTORY.md` if the new
/// version is not already present there. The bundling guarantee is
/// load-bearing: missing ledger rows fail the release-history drift gate
/// for *every* PR opened against master, not just release PRs. See
/// `docs/forensics/2026-05-03-release-history-ledger-drift.md`.
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

    // Bundle the RELEASE_HISTORY ledger update with the version bump so
    // master cannot be left in drift after a release-prep PR merges.
    if append_release_history_row(repo_root, new_version)? {
        report.files_updated += 1;
        report.touched_files.push(PathBuf::from("RELEASE_HISTORY.md"));
    }

    Ok(report)
}

/// Append a row + 4 link refs to `RELEASE_HISTORY.md` for `new_version`.
/// Idempotent: returns `Ok(false)` if a row already exists for this version.
///
/// The row uses placeholder values for fields that aren't known at bump
/// time (tag commit SHA — release-orchestration backfills) and best-effort
/// values for fields that are roughly knowable (today's date, schema-shape
/// asset/crate counts that match recent releases).
///
/// If `RELEASE_HISTORY.md` does not exist, this is a no-op (some forks may
/// not maintain a ledger). Existing files must be parseable (have a prior
/// version row to anchor insertion against), or this returns an error so
/// the bump fails fast rather than silently emitting a malformed ledger.
fn append_release_history_row(repo_root: &Path, new_version: &str) -> Result<bool> {
    let path = repo_root.join("RELEASE_HISTORY.md");
    if !path.exists() {
        return Ok(false);
    }
    let content =
        fs::read_to_string(&path).map_err(|e| eyre!("reading {}: {e}", path.display()))?;

    // Idempotency: the ledger row begins with "| [<version>]". If we find
    // that prefix already, this version has been added — don't duplicate.
    let row_marker = format!("| [{new_version}]");
    if content.contains(&row_marker) {
        return Ok(false);
    }

    // Find the previous (topmost data) version by scanning rows of the form
    // "| [<semver>]". The topmost match is the most recent prior release.
    let prev_version = find_topmost_ledger_version(&content).ok_or_else(|| {
        eyre!(
            "RELEASE_HISTORY.md has no prior version row to anchor insertion against; \
             cannot synthesize a row for {new_version} automatically"
        )
    })?;

    let today = today_iso_date();
    let new_row = format!(
        "| [{v}] | `v{v}` | [yes][gh-{v}] | {date} | `pending` | [v{prev}...v{v}] | \
         10 (7 binaries, VSIX, SHA256SUMS, SBOM) | {v} (31 crates) | [perl-lsp-rs][vsce] | \
         [v{v}][n-{v}] |",
        v = new_version,
        prev = prev_version,
        date = today,
    );
    let new_n_ref = format!("[n-{v}]: docs/releases/v{v}.md", v = new_version);
    let new_v_ref = format!("[{v}]: docs/releases/v{v}.md", v = new_version);
    let new_gh_ref = format!(
        "[gh-{v}]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v{v}",
        v = new_version,
    );
    let new_compare_ref = format!(
        "[v{prev}...v{v}]: https://github.com/EffortlessMetrics/perl-lsp/compare/v{prev}...v{v}",
        v = new_version,
        prev = prev_version,
    );

    // Insert each new line above the topmost matching prior line. We rely
    // on stable patterns from the existing schema rather than trying to
    // parse markdown structurally.
    let prev_row_prefix = format!("| [{prev_version}]");
    let prev_n_ref_prefix = format!("[n-{prev_version}]:");
    let prev_v_ref_prefix = format!("[{prev_version}]:");
    let prev_gh_ref_prefix = format!("[gh-{prev_version}]:");
    let prev_compare_ref_anchor = format!("...v{prev_version}]:");

    let mut updated = String::with_capacity(content.len() + 1024);
    let mut inserted_row = false;
    let mut inserted_n_ref = false;
    let mut inserted_v_ref = false;
    let mut inserted_gh_ref = false;
    let mut inserted_compare_ref = false;

    for line in content.lines() {
        if !inserted_row && line.starts_with(&prev_row_prefix) {
            updated.push_str(&new_row);
            updated.push('\n');
            inserted_row = true;
        }
        if !inserted_n_ref && line.starts_with(&prev_n_ref_prefix) {
            updated.push_str(&new_n_ref);
            updated.push('\n');
            inserted_n_ref = true;
        } else if !inserted_v_ref
            && line.starts_with(&prev_v_ref_prefix)
            && !line.starts_with("[n-")
            && !line.starts_with("[gh-")
        {
            updated.push_str(&new_v_ref);
            updated.push('\n');
            inserted_v_ref = true;
        }
        if !inserted_gh_ref && line.starts_with(&prev_gh_ref_prefix) {
            updated.push_str(&new_gh_ref);
            updated.push('\n');
            inserted_gh_ref = true;
        }
        if !inserted_compare_ref
            && line.starts_with("[v")
            && line.contains(&prev_compare_ref_anchor)
        {
            updated.push_str(&new_compare_ref);
            updated.push('\n');
            inserted_compare_ref = true;
        }
        updated.push_str(line);
        updated.push('\n');
    }

    if !inserted_row {
        bail!(
            "could not find prior version row '{prev_row_prefix}' in RELEASE_HISTORY.md; \
             ledger schema may have changed"
        );
    }
    if !inserted_n_ref || !inserted_v_ref || !inserted_gh_ref || !inserted_compare_ref {
        bail!(
            "could not find all four prior link refs for {prev_version} in RELEASE_HISTORY.md; \
             ledger schema may have changed (n_ref={inserted_n_ref}, v_ref={inserted_v_ref}, \
             gh_ref={inserted_gh_ref}, compare_ref={inserted_compare_ref})"
        );
    }

    // Preserve the file's trailing-newline shape. `lines()` strips the
    // final newline; if the original ended without one, drop ours.
    if !content.ends_with('\n') && updated.ends_with('\n') {
        updated.pop();
    }

    fs::write(&path, updated).map_err(|e| eyre!("writing {}: {e}", path.display()))?;
    Ok(true)
}

/// Find the most recent version listed in the `RELEASE_HISTORY.md` ledger.
/// The ledger is ordered most-recent-first, so the topmost data row is the
/// previous release.
fn find_topmost_ledger_version(content: &str) -> Option<String> {
    static ROW_PATTERN: LazyLock<Regex> =
        LazyLock::new(|| compile_regex(r"^\|\s*\[(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)\]"));
    for line in content.lines() {
        if let Some(caps) = ROW_PATTERN.captures(line)
            && let Some(m) = caps.get(1)
        {
            return Some(m.as_str().to_string());
        }
    }
    None
}

/// Today's date as `YYYY-MM-DD` in UTC. Best-effort — release-orchestration
/// can backfill the actual publish date if needed.
fn today_iso_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    // Ymd from unix epoch: standard civil-from-days conversion (Howard Hinnant).
    let z = now.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
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
/// Matches Nix `version = "X.Y.Z";` lines. The trailing semicolon is Nix-syntax-specific
/// and distinguishes this pattern from TOML `version = "X.Y.Z"` lines.
static FLAKE_NIX_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(&format!(r#"^\s*version\s*=\s*"({VERSION_FRAGMENT})"\s*;"#)));

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

    // flake.nix: `version = "X.Y.Z";` in the perl-lsp package derivation.
    // Nix syntax uses a trailing semicolon, so FLAKE_NIX_VERSION_RE is used
    // instead of the TOML-oriented BARE_VERSION_RE. See issue #4357.
    collect_single_line_doc_site(
        repo_root,
        "flake.nix",
        "flake.nix perl-lsp package version",
        &FLAKE_NIX_VERSION_RE,
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

    // -----------------------------------------------------------------------
    // RELEASE_HISTORY ledger generator tests
    //
    // These lock the bundling guarantee documented in
    // docs/forensics/2026-05-03-release-history-ledger-drift.md: bump-version
    // must append the ledger row in the same operation as the version-site
    // updates so master cannot be left in drift after a release-prep PR
    // merges.
    // -----------------------------------------------------------------------

    fn ledger_fixture() -> String {
        // Minimal fixture matching the schema of the real RELEASE_HISTORY.md.
        r#"# Release History

## Release ledger

| Version | Tag | GitHub Release | Released | Tag commit | Compare | Assets | crates.io | VS Code Marketplace | Notes file |
|---------|-----|----------------|----------|------------|---------|--------|-----------|---------------------|------------|
| [0.13.3] | `v0.13.3` | [yes][gh-0.13.3] | 2026-05-03 | `06fc1443` | [v0.13.2...v0.13.3] | 10 (7 binaries, VSIX, SHA256SUMS, SBOM) | 0.13.3 (31 crates) | [perl-lsp-rs][vsce] | [v0.13.3][n-0.13.3] |
| [0.13.2] | `v0.13.2` | [yes][gh-0.13.2] | 2026-05-02 | `0e9c5d78` | [v0.13.1...v0.13.2] | 10 (7 binaries, VSIX, SHA256SUMS, SBOM) | 0.13.2 (31 crates) | [perl-lsp-rs][vsce] | [v0.13.2][n-0.13.2] |

## Links

<!-- Notes files -->
[n-0.13.3]: docs/releases/v0.13.3.md
[n-0.13.2]: docs/releases/v0.13.2.md

<!-- Version links (to notes files) -->
[0.13.3]: docs/releases/v0.13.3.md
[0.13.2]: docs/releases/v0.13.2.md

<!-- GitHub Releases -->
[gh-0.13.3]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.13.3
[gh-0.13.2]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.13.2

<!-- Compare ranges -->
[v0.13.2...v0.13.3]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.2...v0.13.3
[v0.13.1...v0.13.2]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.1...v0.13.2

<!-- Channels -->
[vsce]: https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs
"#
        .to_string()
    }

    #[test]
    fn append_release_history_inserts_row_and_four_link_refs() -> Result<()> {
        let dir = unique_temp_repo_dir("ledger-append")?;
        let path = dir.join("RELEASE_HISTORY.md");
        fs::write(&path, ledger_fixture())?;

        let inserted = append_release_history_row(&dir, "0.13.4")?;
        assert!(inserted, "first call should insert the row");

        let content = fs::read_to_string(&path)?;
        // New row is now topmost data row.
        let row_idx =
            content.find("| [0.13.4]").ok_or_else(|| eyre!("row for 0.13.4 should be inserted"))?;
        let prev_row_idx =
            content.find("| [0.13.3]").ok_or_else(|| eyre!("row for 0.13.3 not found"))?;
        assert!(row_idx < prev_row_idx, "0.13.4 row should appear above 0.13.3");

        // Link refs all present in their respective sections.
        assert!(content.contains("[n-0.13.4]: docs/releases/v0.13.4.md"));
        assert!(content.contains("[0.13.4]: docs/releases/v0.13.4.md"));
        assert!(content.contains(
            "[gh-0.13.4]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.13.4"
        ));
        assert!(content.contains(
            "[v0.13.3...v0.13.4]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.3...v0.13.4"
        ));

        // Compare-range link references the correct prev version.
        assert!(content.contains("[v0.13.3...v0.13.4]"));

        // Tag-commit SHA is the placeholder.
        assert!(
            content.contains("`pending`"),
            "tag-commit SHA should be `pending` placeholder for release-orchestration to fill in"
        );

        fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    #[test]
    fn append_release_history_is_idempotent() -> Result<()> {
        let dir = unique_temp_repo_dir("ledger-idempotent")?;
        let path = dir.join("RELEASE_HISTORY.md");
        fs::write(&path, ledger_fixture())?;

        let first = append_release_history_row(&dir, "0.13.4")?;
        assert!(first, "first call inserts");
        let after_first = fs::read_to_string(&path)?;

        let second = append_release_history_row(&dir, "0.13.4")?;
        assert!(!second, "second call should detect existing row and skip");
        let after_second = fs::read_to_string(&path)?;

        assert_eq!(
            after_first, after_second,
            "ledger content must be byte-identical after redundant append"
        );

        fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    #[test]
    fn append_release_history_no_op_when_file_missing() -> Result<()> {
        let dir = unique_temp_repo_dir("ledger-missing")?;
        // Deliberately do not create RELEASE_HISTORY.md.
        let inserted = append_release_history_row(&dir, "0.13.4")?;
        assert!(!inserted, "missing ledger file is a no-op (forks may not maintain a ledger)");
        fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    #[test]
    fn append_release_history_fails_when_no_prior_row_exists() -> Result<()> {
        let dir = unique_temp_repo_dir("ledger-empty")?;
        let path = dir.join("RELEASE_HISTORY.md");
        // Ledger exists but has no data rows.
        fs::write(&path, "# Release History\n\n## Links\n")?;
        let result = append_release_history_row(&dir, "0.13.4");
        assert!(
            result.is_err(),
            "empty ledger must fail loudly so the bump aborts rather than emitting a malformed row"
        );
        fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    #[test]
    fn find_topmost_ledger_version_picks_first_data_row() {
        let content = ledger_fixture();
        assert_eq!(find_topmost_ledger_version(&content).as_deref(), Some("0.13.3"));
    }

    #[test]
    fn find_topmost_ledger_version_handles_pre_release() {
        let content = "## Release ledger\n\n| [0.14.0-rc1] | `v0.14.0-rc1` | ... |\n";
        assert_eq!(find_topmost_ledger_version(content).as_deref(), Some("0.14.0-rc1"));
    }

    #[test]
    fn today_iso_date_is_well_formed() {
        let today = today_iso_date();
        assert_eq!(today.len(), 10);
        assert!(
            today.chars().nth(4) == Some('-') && today.chars().nth(7) == Some('-'),
            "expected YYYY-MM-DD, got {today}"
        );
        let parts: Vec<&str> = today.split('-').collect();
        assert_eq!(parts.len(), 3);
        let year: i32 = parts[0].parse().unwrap();
        let month: u32 = parts[1].parse().unwrap();
        let day: u32 = parts[2].parse().unwrap();
        assert!((2025..=2100).contains(&year), "year {year} out of expected range");
        assert!((1..=12).contains(&month), "month {month} invalid");
        assert!((1..=31).contains(&day), "day {day} invalid");
    }

    // -----------------------------------------------------------------------
    // flake.nix version collector tests (issue #4357)
    // -----------------------------------------------------------------------

    #[test]
    fn flake_nix_re_matches_nix_version_line() -> Result<()> {
        // Exact format from flake.nix: leading spaces, Nix assignment, trailing semicolon.
        let line = r#"            version = "0.14.0";  # Synced manually"#;
        let caps = FLAKE_NIX_VERSION_RE
            .captures(line)
            .ok_or_else(|| eyre!("FLAKE_NIX_VERSION_RE must match Nix version lines"))?;
        assert_eq!(&caps[1], "0.14.0");
        Ok(())
    }

    #[test]
    fn flake_nix_re_matches_pre_release_version() -> Result<()> {
        let line = r#"            version = "0.14.0-rc1";"#;
        let caps = FLAKE_NIX_VERSION_RE.captures(line).ok_or_else(|| {
            eyre!("FLAKE_NIX_VERSION_RE must match pre-release Nix version lines")
        })?;
        assert_eq!(&caps[1], "0.14.0-rc1");
        Ok(())
    }

    #[test]
    fn flake_nix_re_does_not_match_toml_version_line() {
        // TOML lines do not have a trailing semicolon — must not be captured by
        // FLAKE_NIX_VERSION_RE (BARE_VERSION_RE handles those).
        let line = r#"version = "0.14.0""#;
        assert!(
            FLAKE_NIX_VERSION_RE.captures(line).is_none(),
            "FLAKE_NIX_VERSION_RE must not match TOML-style version lines (no semicolon)"
        );
    }

    #[test]
    fn collect_doc_sites_discovers_flake_nix_version() -> Result<()> {
        let repo_root = unique_temp_repo_dir("flake-nix-collect")?;

        // Minimal flake.nix with the perl-lsp package version line.
        let flake_content = r#"
{
  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    packages.${system}.perl-lsp = pkgs.rustPlatform.buildRustPackage {
      pname = "perl-lsp";
      version = "0.42.7";
    };
  };
}
"#;
        fs::write(repo_root.join("flake.nix"), flake_content)
            .map_err(|e| eyre!("writing flake.nix: {e}"))?;

        let mut sites = Vec::new();
        collect_doc_sites(&repo_root, &mut sites)?;

        let flake_site = sites
            .iter()
            .find(|s| s.description.contains("flake.nix"))
            .ok_or_else(|| eyre!("flake.nix version site should be collected"))?;

        assert_eq!(flake_site.found, "0.42.7", "collector should capture the flake.nix version");
        assert!(
            !flake_site.channel_split,
            "flake.nix version must not be channel-split (it must match workspace version exactly)"
        );

        fs::remove_dir_all(&repo_root).ok();
        Ok(())
    }

    #[test]
    fn collect_doc_sites_skips_missing_flake_nix() -> Result<()> {
        let repo_root = unique_temp_repo_dir("flake-nix-missing")?;
        // Do not create flake.nix — collector must silently skip.
        let mut sites = Vec::new();
        collect_doc_sites(&repo_root, &mut sites)?;

        assert!(
            !sites.iter().any(|s| s.description.contains("flake.nix")),
            "missing flake.nix must be silently skipped (some forks may not use Nix)"
        );

        fs::remove_dir_all(&repo_root).ok();
        Ok(())
    }
}
