//! Keep release narrative docs synchronized from one source of truth.
//!
//! The workspace version is read from `Cargo.toml` and the published crate
//! surface count is derived from `[workspace.metadata.publish.allow]`.
//! This keeps active narrative docs aligned with the true release surface without
//! hand-editing version/count literals.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

/// Values used to hydrate release narrative files.
#[derive(Debug)]
struct ReleaseSurface {
    version: String,
    published_crate_count: usize,
}

/// Synchronize active release docs from workspace-derived values.
pub fn run(write: bool) -> Result<()> {
    let root = project_root()?;
    let surface = collect_release_surface(&root)?;

    let mut changed_files: Vec<(PathBuf, String)> = Vec::new();

    apply_sync(
        &root.join("README.md"),
        |content| sync_readme(content, &surface),
        &mut changed_files,
    )?;
    apply_sync(
        &root.join("docs/project/CURRENT_STATUS.md"),
        |content| sync_current_status(content, &surface),
        &mut changed_files,
    )?;
    apply_sync(
        &root.join("docs/project/ROADMAP.md"),
        |content| sync_roadmap(content, &surface),
        &mut changed_files,
    )?;
    apply_sync(
        &root.join("docs/project/status/index.md"),
        |content| sync_status_index(content, &surface),
        &mut changed_files,
    )?;
    apply_sync(
        &root.join("docs/project/status/release.md"),
        |content| sync_release_notes(content, &surface),
        &mut changed_files,
    )?;

    if changed_files.is_empty() {
        println!("Release docs are in sync.");
        return Ok(());
    }

    if write {
        for (path, content) in &changed_files {
            fs::write(path, content)
                .with_context(|| format!("failed to write {}", path.display()))?;
            println!("Updated {}", path.display());
        }
        println!("Synced {} file(s) from workspace metadata.", changed_files.len());
        return Ok(());
    }

    for (path, _) in &changed_files {
        eprintln!("{} is out of date", path.display());
    }
    bail!("{} release doc file(s) out of date; rerun with --write", changed_files.len());
}

fn apply_sync<F>(path: &Path, mut sync: F, changed_files: &mut Vec<(PathBuf, String)>) -> Result<()>
where
    F: FnMut(&str) -> Result<String>,
{
    let current =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let updated = sync(&current)?;
    if updated != current {
        changed_files.push((path.to_path_buf(), updated));
    }
    Ok(())
}

fn collect_release_surface(root: &Path) -> Result<ReleaseSurface> {
    let cargo_toml = root.join("Cargo.toml");
    let raw = fs::read_to_string(&cargo_toml)
        .with_context(|| format!("reading {}", cargo_toml.display()))?;
    let parsed: Value = toml::from_str(&raw).context("parsing Cargo.toml")?;

    let version = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(toml::Value::as_str)
        .map(|v| v.to_string())
        .ok_or_else(|| {
            color_eyre::eyre::eyre!("Cargo.toml is missing [workspace.package].version")
        })?;

    let publish_allow = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("metadata"))
        .and_then(|metadata| metadata.get("publish"))
        .and_then(|publish| publish.get("allow"))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            color_eyre::eyre::eyre!("Cargo.toml is missing [workspace.metadata.publish.allow]")
        })?;

    let published_crate_count = publish_allow.len();
    if published_crate_count == 0 {
        bail!("publish allowlist is empty");
    }

    Ok(ReleaseSurface { version, published_crate_count })
}

fn sync_readme(content: &str, surface: &ReleaseSurface) -> Result<String> {
    let mut release_track_seen = false;
    let mut published_surface_seen = false;

    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.starts_with("| Release track | `") {
            lines.push(format!("| Release track | `v{}` public-alpha patch |", surface.version));
            release_track_seen = true;
        } else if line.starts_with("| Published crate surface | ") {
            lines.push(format!(
                "| Published crate surface | {} crates in `[workspace.metadata.publish.allow]` |",
                surface.published_crate_count
            ));
            published_surface_seen = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !release_track_seen {
        bail!("README.md: release track line not found");
    }
    if !published_surface_seen {
        bail!("README.md: published crate surface line not found");
    }
    Ok(restore_trailing_newline(content, &lines))
}

fn sync_current_status(content: &str, surface: &ReleaseSurface) -> Result<String> {
    let mut version_seen = false;
    let mut train_seen = false;
    let mut count_seen = false;
    let mut milestone_seen = false;

    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.starts_with("| **Workspace version line** | ") {
            lines.push(format!(
                "| **Workspace version line** | `v{}` | [`Cargo.toml`](../../Cargo.toml) |",
                surface.version
            ));
            version_seen = true;
        } else if line.starts_with("| **Current release train** | ") {
            lines.push(format!(
                "| **Current release train** | `v{}` public-alpha patch prep | [docs/releases/v{}.md](../releases/v{}.md) |",
                surface.version, surface.version, surface.version
            ));
            train_seen = true;
        } else if line.starts_with("| **Published crate surface** | ") {
            lines.push(format!(
                "| **Published crate surface** | {} crates | [`[workspace.metadata.publish.allow]`](../../Cargo.toml) |",
                surface.published_crate_count
            ));
            count_seen = true;
        } else if line.starts_with("| **Active milestone** | `") {
            lines.push(format!(
                "| **Active milestone** | `v{}` release proof and channel prep | [status/index.md](status/index.md) |",
                surface.version
            ));
            milestone_seen = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !version_seen {
        bail!("CURRENT_STATUS.md: workspace version row not found");
    }
    if !train_seen {
        bail!("CURRENT_STATUS.md: current release train row not found");
    }
    if !count_seen {
        bail!("CURRENT_STATUS.md: published crate surface row not found");
    }
    if !milestone_seen {
        bail!("CURRENT_STATUS.md: active milestone row not found");
    }
    Ok(restore_trailing_newline(content, &lines))
}

fn sync_roadmap(content: &str, surface: &ReleaseSurface) -> Result<String> {
    let mut workspace_version_seen = false;
    let mut current_release_seen = false;
    let mut published_surface_seen = false;
    let mut publication_discipline_seen = false;
    let mut active_section_seen = false;
    let mut now_section_seen = false;
    let mut now_gate_seen = false;
    let mut next_header_seen = false;

    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.starts_with("- Workspace version line: `v") {
            lines.push(format!("- Workspace version line: `v{}`", surface.version));
            workspace_version_seen = true;
        } else if line.starts_with("- Current release train: `v") {
            lines.push(format!(
                "- Current release train: `v{}` public-alpha patch prep, with release dispatch intentionally pending",
                surface.version
            ));
            current_release_seen = true;
        } else if line.starts_with("- Published crate surface target: ") {
            lines.push(format!(
                "- Published crate surface target: {} crates from `[workspace.metadata.publish.allow]`",
                surface.published_crate_count
            ));
            published_surface_seen = true;
        } else if line.starts_with("Publication discipline: `v") {
            lines.push(format!(
                "Publication discipline: `v{}` uses a normal SemVer package version for release channels while the human-facing product posture remains public alpha. See [RELEASE_HISTORY.md](../../RELEASE_HISTORY.md) for the cross-channel ledger, and do not dispatch the release until the prep checks pass.",
                surface.version
            ));
            publication_discipline_seen = true;
        } else if line.starts_with("## Active: Public-Alpha Release Prep (v") {
            lines.push(format!(
                "## Active: Public-Alpha Release Prep (v{})",
                surface.version
            ));
            active_section_seen = true;
        } else if line.starts_with("### Now (v") && line.contains("public-alpha patch prep)") {
            lines.push(format!("### Now (v{} public-alpha patch prep)", surface.version));
            now_section_seen = true;
        } else if line.starts_with("- `v") && line.contains("is staged as the next public-alpha patch release; run the release-prep checks before dispatching the train") {
            lines.push(format!(
                "- `v{}` is staged as the next public-alpha patch release; run the release-prep checks before dispatching the train",
                surface.version
            ));
            now_gate_seen = true;
        } else if line.starts_with("### Next (post v") {
            lines.push(format!("### Next (post v{})", surface.version));
            next_header_seen = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !workspace_version_seen {
        bail!("ROADMAP.md: workspace version line not found");
    }
    if !current_release_seen {
        bail!("ROADMAP.md: current release train line not found");
    }
    if !published_surface_seen {
        bail!("ROADMAP.md: published crate surface target line not found");
    }
    if !publication_discipline_seen {
        bail!("ROADMAP.md: publication discipline line not found");
    }
    if !active_section_seen {
        bail!("ROADMAP.md: Active: Public-Alpha Release Prep heading not found");
    }
    if !now_section_seen {
        bail!("ROADMAP.md: current-release Now section not found");
    }
    if !now_gate_seen {
        bail!("ROADMAP.md: release-prep check gate line not found");
    }
    if !next_header_seen {
        bail!("ROADMAP.md: Next (post ...) heading not found");
    }
    Ok(restore_trailing_newline(content, &lines))
}

fn sync_status_index(content: &str, surface: &ReleaseSurface) -> Result<String> {
    let mut release_posture_seen = false;
    let mut now_section_seen = false;
    let mut now_gate_seen = false;
    let mut published_surface_bullet_seen = false;
    let mut next_section_seen = false;

    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.starts_with("- **Release posture**: `v") {
            lines.push(format!(
                "- **Release posture**: `v{}` is staged as the next public-alpha patch train. The workspace version line is `v{}`, the published crate surface is {} crates, and release dispatch is intentionally pending until the prep checks pass.",
                surface.version, surface.version, surface.published_crate_count
            ));
            release_posture_seen = true;
        } else if line.starts_with("**Now (active milestone: v") {
            lines.push(format!(
                "**Now (active milestone: v{} public-alpha release prep)**",
                surface.version
            ));
            now_section_seen = true;
        } else if line.starts_with("- Run the `v")
            && line.contains(" release-prep checks before dispatching release orchestration")
        {
            lines.push(format!(
                "- Run the `v{}` release-prep checks before dispatching release orchestration",
                surface.version
            ));
            now_gate_seen = true;
        } else if line.contains("published surface")
            && line.starts_with("- Keep the top-level README")
        {
            lines.push(format!(
                "- Keep the top-level README, status docs, and release runbooks aligned with the actual `perllsp` asset line, the `perl-lsp-rs` extension package, and the {}-crate published surface",
                surface.published_crate_count
            ));
            published_surface_bullet_seen = true;
        } else if line.starts_with("**Next (post v") {
            lines.push(format!("**Next (post v{} public alpha)**", surface.version));
            next_section_seen = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !release_posture_seen {
        bail!("status/index.md: release posture line not found");
    }
    if !now_section_seen {
        bail!("status/index.md: now section heading not found");
    }
    if !now_gate_seen {
        bail!("status/index.md: now release-prep checks line not found");
    }
    if !published_surface_bullet_seen {
        bail!("status/index.md: published-surface alignment line not found");
    }
    if !next_section_seen {
        bail!("status/index.md: next section heading not found");
    }
    Ok(restore_trailing_newline(content, &lines))
}

fn sync_release_notes(content: &str, surface: &ReleaseSurface) -> Result<String> {
    let mut train_seen = false;
    let mut workspace_seen = false;
    let mut surface_seen = false;
    let mut remaining_seen = false;

    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.starts_with("**Current release train**: `v") {
            lines.push(format!(
                "**Current release train**: `v{}` public-alpha patch prep",
                surface.version
            ));
            train_seen = true;
        } else if line.starts_with("**Workspace version line**: `v") {
            lines.push(format!("**Workspace version line**: `v{}`", surface.version));
            workspace_seen = true;
        } else if line.starts_with("**Published crate surface**: ") {
            lines.push(format!(
                "**Published crate surface**: {} crates",
                surface.published_crate_count
            ));
            surface_seen = true;
        } else if line.starts_with("- Remaining work is operational: finish `v")
            && line.contains(" prep verification, then publish and record final channel receipts")
        {
            lines.push(format!(
                "- Remaining work is operational: finish `v{}` prep verification, then publish and record final channel receipts",
                surface.version
            ));
            remaining_seen = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !train_seen {
        bail!("status/release.md: current release train line not found");
    }
    if !workspace_seen {
        bail!("status/release.md: workspace version line not found");
    }
    if !surface_seen {
        bail!("status/release.md: published crate surface line not found");
    }
    if !remaining_seen {
        bail!("status/release.md: remaining blockers prep verification line not found");
    }
    Ok(restore_trailing_newline(content, &lines))
}

fn restore_trailing_newline(original: &str, lines: &[String]) -> String {
    let mut updated = lines.join("\n");
    if original.ends_with('\n') {
        updated.push('\n');
    }
    updated
}
