//! Emit the curated release-notes body for a given version.
//!
//! Reads `docs/releases/v<version>.md`, strips its YAML front-matter, and
//! writes the remaining markdown to stdout. Consumed by the Release workflow
//! (`.github/workflows/release.yml`) so the GitHub Release body mirrors the
//! committed per-release notes file instead of an auto-generated template.
//!
//! See issue #4340 for the motivation and RELEASE.md "Before publishing" for
//! the author-facing checklist.

use color_eyre::eyre::{Result, bail, eyre};
use std::path::PathBuf;
use std::{fs, io::Write};

/// Configuration for the `release-notes` task.
#[derive(Debug, Clone)]
pub struct Args {
    /// Semantic version string, with or without a leading `v` (e.g. `0.12.4`).
    /// Ignored when `file` is set.
    pub version: Option<String>,
    /// Repository root. `docs/releases/v<version>.md` is resolved under this.
    /// Defaults to the current working directory.
    pub root: PathBuf,
    /// Explicit path to a notes file. Bypasses version resolution when set;
    /// useful for callers with non-standard layouts and for tests.
    pub file: Option<PathBuf>,
}

/// CLI entry point: extract the body and print it to stdout.
pub fn run(args: Args) -> Result<()> {
    let body = extract(&args)?;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(body.as_bytes())?;
    if !body.ends_with('\n') {
        handle.write_all(b"\n")?;
    }
    Ok(())
}

/// Extract the curated notes body for the given arguments.
///
/// # Errors
/// - The notes file is missing (for both `version` and `file` modes).
/// - The notes file is empty, has unterminated YAML front-matter, or contains
///   only front-matter with no body.
/// - Neither `version` nor `file` is supplied, or the version string is not
///   `MAJOR.MINOR.PATCH`.
pub fn extract(args: &Args) -> Result<String> {
    let path = resolve_path(args)?;

    if !path.exists() {
        bail!(
            "release notes file not found: {}\n\
             hint: create the curated notes file before publishing.\n\
                   see RELEASE.md \"Before publishing\" checklist.",
            path.display()
        );
    }

    let contents = fs::read_to_string(&path)
        .map_err(|err| eyre!("failed to read {}: {err}", path.display()))?;

    if contents.trim().is_empty() {
        bail!("release notes file is empty: {}", path.display());
    }

    strip_front_matter(&contents).map_err(|err| eyre!("{}: {err}", path.display()))
}

fn resolve_path(args: &Args) -> Result<PathBuf> {
    if let Some(file) = &args.file {
        return Ok(file.clone());
    }

    let version = args
        .version
        .as_deref()
        .ok_or_else(|| eyre!("version argument required (e.g. 0.12.4) or --file must be set"))?;

    let normalized = version.strip_prefix('v').unwrap_or(version);
    validate_version(normalized)?;

    Ok(args.root.join("docs").join("releases").join(format!("v{normalized}.md")))
}

fn validate_version(version: &str) -> Result<()> {
    // MAJOR.MINOR.PATCH with an optional `-prerelease.identifier` suffix.
    let (core, _pre) = version.split_once('-').unwrap_or((version, ""));
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        bail!("invalid version: {version} (expected MAJOR.MINOR.PATCH)");
    }
    Ok(())
}

/// Strip a leading YAML front-matter block and trim leading blank lines.
///
/// A front-matter block is a `---` line at line 1, zero or more content lines,
/// and a closing `---` line. Files without front-matter are returned verbatim
/// (aside from blank-line trimming at the top).
///
/// # Errors
/// - Unterminated front-matter (opened with `---` but never closed).
/// - File has no body content after the front-matter.
fn strip_front_matter(input: &str) -> Result<String> {
    enum State {
        Start,
        FrontMatter,
        Body,
    }

    let mut state = State::Start;
    let mut body_lines: Vec<&str> = Vec::new();
    let mut seen_content = false;

    for (idx, line) in input.lines().enumerate() {
        match state {
            State::Start => {
                if idx == 0 && line == "---" {
                    state = State::FrontMatter;
                    continue;
                }
                state = State::Body;
                if !line.trim().is_empty() {
                    seen_content = true;
                    body_lines.push(line);
                }
                // else: swallow the leading blank line.
            }
            State::FrontMatter => {
                if line == "---" {
                    state = State::Body;
                }
                // front-matter contents are discarded either way.
            }
            State::Body => {
                if !seen_content {
                    if line.trim().is_empty() {
                        continue;
                    }
                    seen_content = true;
                }
                body_lines.push(line);
            }
        }
    }

    if matches!(state, State::FrontMatter) {
        bail!("release notes file has unterminated YAML front-matter");
    }
    if !seen_content {
        bail!("release notes file has no body content after front-matter");
    }

    // Preserve a trailing newline so downstream markdown parsers and shell
    // appends behave predictably.
    let mut out = body_lines.join("\n");
    out.push('\n');
    Ok(out)
}

// ---------------------------------------------------------------------------
// BDD scenarios — one test per observable behaviour. See issue #4340.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_notes(root: &Path, version: &str, content: &str) -> PathBuf {
        let dir = root.join("docs").join("releases");
        fs::create_dir_all(&dir).expect("create docs/releases");
        let path = dir.join(format!("v{version}.md"));
        fs::write(&path, content).expect("write notes");
        path
    }

    /// Given a curated notes file with YAML front-matter,
    /// When the extractor runs,
    /// Then the front-matter is stripped and the body starts on the first
    /// non-blank line (typically the `# v<version>` heading).
    #[test]
    fn front_matter_is_stripped_and_body_starts_on_first_non_blank_line() {
        let tmp = TempDir::new().unwrap();
        write_notes(
            tmp.path(),
            "1.2.3",
            "---\nversion: \"1.2.3\"\ntag: \"v1.2.3\"\n---\n\n# v1.2.3\n\nBody line.\n",
        );

        let body = extract(&Args {
            version: Some("1.2.3".into()),
            root: tmp.path().to_path_buf(),
            file: None,
        })
        .expect("extract succeeds");

        assert!(!body.contains("version: \"1.2.3\""), "front-matter leaked: {body}");
        assert_eq!(body.lines().next(), Some("# v1.2.3"));
        assert!(body.contains("Body line."));
    }

    /// Given a notes file with no front-matter,
    /// When the extractor runs,
    /// Then every non-leading-blank line is emitted verbatim.
    #[test]
    fn files_without_front_matter_are_emitted_verbatim() {
        let tmp = TempDir::new().unwrap();
        write_notes(tmp.path(), "2.0.0", "# v2.0.0\n\nNotes without front-matter.\n");

        let body = extract(&Args {
            version: Some("2.0.0".into()),
            root: tmp.path().to_path_buf(),
            file: None,
        })
        .expect("extract succeeds");

        assert_eq!(body.lines().next(), Some("# v2.0.0"));
        assert!(body.contains("Notes without front-matter."));
    }

    /// Given a missing notes file,
    /// When the extractor runs,
    /// Then it fails with a diagnostic mentioning the missing path.
    #[test]
    fn missing_file_fails_with_diagnostic_path() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("docs/releases")).unwrap();

        let err = extract(&Args {
            version: Some("9.9.9".into()),
            root: tmp.path().to_path_buf(),
            file: None,
        })
        .expect_err("missing file must error");

        let rendered = format!("{err:#}");
        assert!(rendered.contains("release notes file not found"), "{rendered}");
        assert!(rendered.contains("v9.9.9.md"), "{rendered}");
    }

    /// Given an unterminated YAML front-matter,
    /// When the extractor runs,
    /// Then it fails rather than silently emit an empty body.
    #[test]
    fn unterminated_front_matter_errors() {
        let tmp = TempDir::new().unwrap();
        write_notes(tmp.path(), "3.0.0", "---\nversion: \"3.0.0\"\n# body missing closing fence\n");

        let err = extract(&Args {
            version: Some("3.0.0".into()),
            root: tmp.path().to_path_buf(),
            file: None,
        })
        .expect_err("unterminated front-matter must error");

        assert!(format!("{err:#}").contains("unterminated"));
    }

    /// Given only front-matter and no body,
    /// When the extractor runs,
    /// Then it fails — we never publish an empty release body.
    #[test]
    fn front_matter_only_file_errors() {
        let tmp = TempDir::new().unwrap();
        write_notes(tmp.path(), "4.0.0", "---\nversion: \"4.0.0\"\n---\n");

        let err = extract(&Args {
            version: Some("4.0.0".into()),
            root: tmp.path().to_path_buf(),
            file: None,
        })
        .expect_err("empty body must error");

        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("no body content") || rendered.contains("empty"),
            "unexpected diagnostic: {rendered}"
        );
    }

    /// Given an invalid version argument,
    /// When the extractor runs,
    /// Then it fails with a version-validation diagnostic.
    #[test]
    fn invalid_version_errors() {
        let err = extract(&Args {
            version: Some("not-a-version".into()),
            root: PathBuf::from("."),
            file: None,
        })
        .expect_err("invalid version must error");

        assert!(format!("{err:#}").contains("invalid version"));
    }

    /// Given a `file` argument pointing at a non-standard path,
    /// When the extractor runs,
    /// Then that file is read directly without requiring the docs/releases layout.
    #[test]
    fn file_argument_bypasses_version_lookup() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom-notes.md");
        fs::write(
            &custom,
            "---\nversion: \"5.0.0\"\n---\n\n# v5.0.0\n\nFrom explicit --file path.\n",
        )
        .unwrap();

        let body = extract(&Args { version: None, root: PathBuf::new(), file: Some(custom) })
            .expect("extract succeeds");

        assert!(body.contains("From explicit --file path."));
        assert!(!body.contains("version: \"5.0.0\""));
    }

    /// Given a version prefixed with `v`,
    /// When the extractor runs,
    /// Then the leading `v` is tolerated (caller convenience).
    #[test]
    fn leading_v_prefix_on_version_is_tolerated() {
        let tmp = TempDir::new().unwrap();
        write_notes(tmp.path(), "6.1.2", "---\nversion: \"6.1.2\"\n---\n\n# v6.1.2\n\nBody.\n");

        let body = extract(&Args {
            version: Some("v6.1.2".into()),
            root: tmp.path().to_path_buf(),
            file: None,
        })
        .expect("extract succeeds");

        assert!(body.contains("# v6.1.2"));
    }

    /// A pre-release version is accepted (e.g. `1.0.0-rc.1`).
    #[test]
    fn prerelease_suffix_is_accepted() {
        let tmp = TempDir::new().unwrap();
        write_notes(
            tmp.path(),
            "1.0.0-rc.1",
            "---\nversion: \"1.0.0-rc.1\"\n---\n\n# v1.0.0-rc.1\n\nBody.\n",
        );

        let body = extract(&Args {
            version: Some("1.0.0-rc.1".into()),
            root: tmp.path().to_path_buf(),
            file: None,
        })
        .expect("extract succeeds");

        assert!(body.contains("# v1.0.0-rc.1"));
    }

    /// Neither `version` nor `file` → error (distinguishes from "file missing").
    #[test]
    fn missing_version_and_file_errors() {
        let err = extract(&Args { version: None, root: PathBuf::from("."), file: None })
            .expect_err("must error");
        assert!(format!("{err:#}").contains("version argument required"));
    }

    /// Strip trims repeated leading blank lines (defensive: multiple blanks
    /// after the closing `---` are collapsed so the body starts cleanly).
    #[test]
    fn strip_collapses_multiple_leading_blank_lines_after_frontmatter() {
        let body = strip_front_matter("---\nv: 1\n---\n\n\n\n# Heading\nBody.\n").unwrap();
        assert_eq!(body.lines().next(), Some("# Heading"));
    }

    /// Strip preserves internal blank lines and trailing newline.
    #[test]
    fn strip_preserves_internal_blank_lines() {
        let body = strip_front_matter("---\nv: 1\n---\n\n# H\n\nPara1.\n\nPara2.\n").unwrap();
        assert_eq!(body, "# H\n\nPara1.\n\nPara2.\n");
    }
}
