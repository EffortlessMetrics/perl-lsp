//! Guard user-facing install commands against release-surface drift.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

const REQUIRED_HOMEBREW_COMMAND: &str = "brew install effortlessmetrics/tap/perllsp";
const REQUIRED_TAP_COMMAND: &str = "brew tap effortlessmetrics/tap";
const UNQUALIFIED_HOMEBREW_COMMAND: &str = "brew install perllsp";
const RELEASE_CHOOSER_HEADING: &str = "Which file should I download?";

const SCAN_ROOTS: &[&str] = &[
    "README.md",
    "CHANGELOG.md",
    "RELEASE_HISTORY.md",
    "book/src",
    "docs",
    "vscode-extension",
    "crates/perl-lsp-rs-core/src/runtime/launcher/mod.rs",
];

const FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
    ("brew install perl-lsp", "retired Homebrew formula name"),
    ("brew tap effortlesssteven/tap", "retired Homebrew tap"),
    ("brew tap tree-sitter-perl/tap", "retired Homebrew tap"),
    ("cargo install perl-lsp-rs", "retired crates.io install command"),
    ("cargo install perl-lsp", "retired crates.io install command"),
    ("perl-lsp --stdio", "retired binary command"),
    ("perl-lsp --version", "retired binary command"),
    ("perl-lsp --health", "retired binary command"),
];

const REQUIRED_PATTERNS: &[(&str, &str)] = &[
    (REQUIRED_HOMEBREW_COMMAND, "owned Homebrew tap install command"),
    ("perllsp --stdio", "canonical LSP server command"),
    ("perllsp --version", "canonical version check"),
    ("perllsp --health", "canonical health check"),
    ("perl-lsp.linuxLibc", "VS Code Linux libc selector setting"),
];

#[derive(Debug)]
struct SourceFile {
    rel_path: PathBuf,
    text: String,
}

#[derive(Debug)]
struct Violation {
    location: String,
    message: String,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let files = collect_source_files(&root)?;
    let mut violations = Vec::new();

    check_forbidden_patterns(&files, &mut violations);
    check_required_patterns(&files, &mut violations);
    check_unqualified_homebrew(&files, &mut violations);
    check_release_note_choosers(&files, &mut violations);

    if violations.is_empty() {
        println!("Install surface check passed: {} active files scanned.", files.len());
        return Ok(());
    }

    eprintln!("INSTALL SURFACE VIOLATIONS:");
    eprintln!("{}", "=".repeat(60));
    for violation in &violations {
        eprintln!("  {}: {}", violation.location, violation.message);
    }
    eprintln!("{}", "=".repeat(60));
    bail!("install surface check failed with {} violation(s)", violations.len())
}

fn collect_source_files(root: &Path) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();

    for scan_root in SCAN_ROOTS {
        let path = root.join(scan_root);
        if path.is_file() {
            push_file(root, &path, &mut files)?;
            continue;
        }

        if !path.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&path).into_iter().filter_entry(|entry| {
            root_relative(root, entry.path()).map(|rel| !is_excluded_path(rel)).unwrap_or(true)
        }) {
            let entry = entry.with_context(|| format!("failed to walk {}", path.display()))?;
            let entry_path = entry.path();
            if entry_path.is_file() && is_scan_candidate(entry_path) {
                push_file(root, entry_path, &mut files)?;
            }
        }
    }

    files.sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
    Ok(files)
}

fn push_file(root: &Path, path: &Path, files: &mut Vec<SourceFile>) -> Result<()> {
    let rel_path =
        root_relative(root, path).map(Path::to_path_buf).unwrap_or_else(|| path.to_path_buf());

    if is_excluded_path(&rel_path) || !is_scan_candidate(path) {
        return Ok(());
    }

    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read install-surface file {}", path.display()))?;
    files.push(SourceFile { rel_path, text });
    Ok(())
}

fn root_relative<'a>(root: &Path, path: &'a Path) -> Option<&'a Path> {
    path.strip_prefix(root).ok()
}

fn is_scan_candidate(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("md" | "json" | "rs" | "sh" | "ts" | "js" | "yml" | "yaml" | "toml")
    )
}

fn is_excluded_path(rel_path: &Path) -> bool {
    rel_path.starts_with(".git")
        || rel_path.starts_with("target")
        || rel_path.starts_with("book/book")
        || rel_path.starts_with("docs/reference/archive")
        || rel_path.starts_with("docs/issues")
        || rel_path.starts_with("vscode-extension/node_modules")
        || rel_path.starts_with("vscode-extension/out")
        || rel_path.starts_with("vscode-extension/dist")
}

fn check_forbidden_patterns(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for file in files {
        for (line_no, line) in file.text.lines().enumerate() {
            for &(pattern, reason) in FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(line_violation(
                        file,
                        line_no + 1,
                        format!("found {reason}: `{pattern}`"),
                    ));
                }
            }
        }
    }
}

fn check_required_patterns(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for &(pattern, description) in REQUIRED_PATTERNS {
        if !files.iter().any(|file| file.text.contains(pattern)) {
            violations.push(global_violation(format!("missing {description}: `{pattern}`")));
        }
    }
}

fn check_unqualified_homebrew(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for file in files {
        let lines: Vec<&str> = file.text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains(UNQUALIFIED_HOMEBREW_COMMAND) {
                continue;
            }

            if has_nearby_tap_command(&lines, index) {
                continue;
            }

            violations.push(line_violation(
                file,
                index + 1,
                format!(
                    "unqualified `{UNQUALIFIED_HOMEBREW_COMMAND}` must be paired with `{REQUIRED_TAP_COMMAND}` in the same install flow"
                ),
            ));
        }
    }
}

fn has_nearby_tap_command(lines: &[&str], install_index: usize) -> bool {
    let start = install_index.saturating_sub(3);
    lines[start..install_index].iter().any(|line| line.contains(REQUIRED_TAP_COMMAND))
}

fn check_release_note_choosers(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for file in files {
        if !requires_release_chooser(&file.rel_path, &file.text) {
            continue;
        }

        if file.text.contains(RELEASE_CHOOSER_HEADING) {
            continue;
        }

        violations.push(Violation {
            location: file.rel_path.display().to_string(),
            message: format!(
                "release notes with GNU/musl Linux assets must include `{RELEASE_CHOOSER_HEADING}`"
            ),
        });
    }
}

fn requires_release_chooser(rel_path: &Path, text: &str) -> bool {
    if !rel_path.starts_with("docs/releases") {
        return false;
    }

    if !text.contains("unknown-linux-gnu") && !text.contains("unknown-linux-musl") {
        return false;
    }

    release_version(rel_path).is_some_and(|version| version >= (0, 12, 4))
}

fn release_version(rel_path: &Path) -> Option<(u64, u64, u64)> {
    let file_stem = rel_path.file_stem()?.to_str()?;
    let version = file_stem.strip_prefix('v')?;
    let numeric = version.split_once('-').map(|(base, _suffix)| base).unwrap_or(version);
    let mut parts = numeric.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

fn line_violation(file: &SourceFile, line_no: usize, message: String) -> Violation {
    Violation { location: format!("{}:{line_no}", file.rel_path.display()), message }
}

fn global_violation(message: String) -> Violation {
    Violation { location: "install surface".to_string(), message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn unqualified_brew_install_requires_nearby_tap() -> Result<()> {
        let allowed = vec!["brew tap effortlessmetrics/tap", "brew install perllsp"];
        let rejected = vec!["brew update", "brew install perllsp"];

        assert!(has_nearby_tap_command(&allowed, 1));
        assert!(!has_nearby_tap_command(&rejected, 1));
        Ok(())
    }

    #[test]
    fn release_notes_need_chooser_from_v0_12_4_forward() -> Result<()> {
        let old_release = Path::new("docs/releases/v0.12.3.md");
        let current_release = Path::new("docs/releases/v0.12.4.md");
        let text = "perllsp-0.12.4-x86_64-unknown-linux-gnu.tar.gz";

        assert!(!requires_release_chooser(old_release, text));
        assert!(requires_release_chooser(current_release, text));
        Ok(())
    }

    #[test]
    fn rc_release_version_uses_base_semver() -> Result<()> {
        let version = release_version(Path::new("docs/releases/v0.13.0-rc1.md"));

        assert_eq!(version, Some((0, 13, 0)));
        Ok(())
    }
}
