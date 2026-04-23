//! Extract curated release-note bodies from `docs/releases/<tag>.md`.
//!
//! Release note files follow a fixed convention:
//!
//! ```text
//! ---
//! version: "X.Y.Z"
//! tag: "vX.Y.Z"
//! ...
//! ---
//!
//! # vX.Y.Z
//!
//! ## Summary
//! ...
//! ```
//!
//! This task reads such a file and emits the body (everything after the closing
//! `---` of the YAML frontmatter) so the release workflow can use it as the
//! `body_path` for the GitHub Release.
//!
//! The command is intentionally strict: if the file is missing, or the
//! frontmatter opens without closing, we fail loudly. A release without a
//! curated note file is a pipeline bug, not a silent fall-through to
//! auto-generated PR lists (see issue #4340).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, WrapErr, bail};

use crate::utils::project_root;

/// Strip a leading YAML frontmatter block (delimited by `---` lines) from
/// `content` and return the trailing body.
///
/// Behavior:
/// - If the first non-empty line is `---`, a frontmatter block is expected and
///   must terminate with another `---` line. Everything between is discarded.
/// - One blank line immediately after the closing `---` is also consumed so
///   the returned body starts at the first real content line.
/// - If no frontmatter is present, the whole content is returned unchanged.
/// - A frontmatter block that opens but never closes is a hard error.
pub fn extract_body(content: &str) -> Result<&str> {
    // Locate the first non-empty line — it must be `---` to open a frontmatter
    // block. If it's anything else, treat the whole file as body.
    let mut cursor = 0usize;
    let bytes = content.as_bytes();

    // Skip a single leading UTF-8 BOM if present.
    if content.starts_with('\u{feff}') {
        cursor += '\u{feff}'.len_utf8();
    }

    // Skip leading blank lines before the frontmatter delimiter.
    let scan_start = cursor;
    let mut pos = scan_start;
    while pos < bytes.len() {
        let line_end = find_line_end(content, pos);
        let line = &content[pos..line_end];
        if line.trim().is_empty() {
            pos = advance_past_newline(content, line_end);
            continue;
        }
        break;
    }

    if pos >= bytes.len() || !line_is_fence(content, pos) {
        // No frontmatter — return original content.
        return Ok(content);
    }

    // We're at the opening `---`. Advance past that line, then look for the
    // closing fence.
    let opening_end = find_line_end(content, pos);
    let mut scan = advance_past_newline(content, opening_end);

    while scan < bytes.len() {
        let line_end = find_line_end(content, scan);
        if line_is_fence(content, scan) {
            // Consume the closing fence line.
            let after_close = advance_past_newline(content, line_end);
            // Also consume one immediately-following blank line so the body
            // starts on the first real content line.
            let body_start = if after_close < bytes.len() {
                let next_end = find_line_end(content, after_close);
                if content[after_close..next_end].trim().is_empty() {
                    advance_past_newline(content, next_end)
                } else {
                    after_close
                }
            } else {
                after_close
            };
            return Ok(&content[body_start..]);
        }
        scan = advance_past_newline(content, line_end);
    }

    bail!("release notes frontmatter opened with `---` but never closed");
}

/// Return the byte offset of the end of the line starting at `start`
/// (pointing at the `\n`, the `\r`, or `content.len()` at EOF).
fn find_line_end(content: &str, start: usize) -> usize {
    match content[start..].find(['\n', '\r']) {
        Some(offset) => start + offset,
        None => content.len(),
    }
}

/// Return the byte offset just past the newline sequence at `end`
/// (handles `\n`, `\r`, `\r\n`).
fn advance_past_newline(content: &str, end: usize) -> usize {
    let bytes = content.as_bytes();
    if end >= bytes.len() {
        return end;
    }
    match bytes[end] {
        b'\r' if bytes.get(end + 1) == Some(&b'\n') => end + 2,
        b'\r' | b'\n' => end + 1,
        _ => end,
    }
}

/// Return `true` if the line at byte offset `start` is exactly `---`
/// (trimmed of trailing whitespace).
fn line_is_fence(content: &str, start: usize) -> bool {
    let end = find_line_end(content, start);
    content[start..end].trim_end() == "---"
}

/// Normalize a tag argument to the `vX.Y.Z` form used by the note filenames.
///
/// Accepts both `v0.12.4` and `0.12.4`; anything else is passed through
/// unchanged so obviously-wrong tags surface as "file not found" errors.
fn normalize_tag(tag: &str) -> String {
    let trimmed = tag.trim();
    if trimmed.starts_with('v') || trimmed.starts_with('V') {
        trimmed.to_string()
    } else {
        format!("v{trimmed}")
    }
}

/// Resolve the canonical note-file path for `tag` under `root`.
pub fn note_path(root: &Path, tag: &str) -> PathBuf {
    let normalized = normalize_tag(tag);
    root.join("docs").join("releases").join(format!("{normalized}.md"))
}

/// Run the `release-notes` task: read `docs/releases/<tag>.md`, strip its
/// frontmatter, and emit the body to stdout or the `--output` file.
///
/// `root` is optional; when `None`, falls back to [`project_root`]. Tests use
/// an explicit `root` so they can operate on a throwaway tempdir instead of
/// the repo's shipped notes.
pub fn run(tag: String, output: Option<PathBuf>, root: Option<PathBuf>) -> Result<()> {
    let root = match root {
        Some(r) => r,
        None => project_root()?,
    };
    let path = note_path(&root, &tag);
    if !path.exists() {
        bail!(
            "release notes file missing: {}\n\
             Every release must ship a curated note file. See RELEASE.md \
             \"Release History Updates\" for the template.",
            path.display()
        );
    }

    let content = fs::read_to_string(&path)
        .with_context_msg(|| format!("failed to read {}", path.display()))?;
    let body = extract_body(&content)
        .with_context_msg(|| format!("failed to parse frontmatter in {}", path.display()))?;

    match output {
        Some(dest) => {
            if let Some(parent) = dest.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent).with_context_msg(|| {
                    format!("failed to create parent dir {}", parent.display())
                })?;
            }
            fs::write(&dest, body)
                .with_context_msg(|| format!("failed to write {}", dest.display()))?;
            eprintln!("Wrote release body to {}", dest.display());
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle.write_all(body.as_bytes()).context("failed to write release body to stdout")?;
        }
    }

    Ok(())
}

// Small local trait so we can use `.with_context_msg(|| ...)` without pulling
// extra eyre traits into every caller — mirrors the pattern used elsewhere
// in xtask with `WrapErr`.
trait ResultExt<T> {
    fn with_context_msg<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: Into<color_eyre::eyre::Report>,
{
    fn with_context_msg<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.map_err(Into::into).wrap_err_with(|| f().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_body_strips_frontmatter() {
        let input =
            "---\nversion: \"1.2.3\"\ntag: \"v1.2.3\"\n---\n\n# v1.2.3\n\n## Summary\n\nhi\n";
        let body = extract_body(input).expect("extraction should succeed");
        assert_eq!(body, "# v1.2.3\n\n## Summary\n\nhi\n");
    }

    #[test]
    fn extract_body_handles_no_trailing_blank_line_after_fence() {
        let input = "---\nversion: \"1.2.3\"\n---\n# v1.2.3\nhi\n";
        let body = extract_body(input).expect("extraction should succeed");
        // No blank-line between closing fence and body — body starts immediately.
        assert_eq!(body, "# v1.2.3\nhi\n");
    }

    #[test]
    fn extract_body_handles_crlf_line_endings() {
        let input = "---\r\nversion: \"1.2.3\"\r\n---\r\n\r\n# v1.2.3\r\nhi\r\n";
        let body = extract_body(input).expect("extraction should succeed");
        assert_eq!(body, "# v1.2.3\r\nhi\r\n");
    }

    #[test]
    fn extract_body_skips_leading_bom() {
        let input = "\u{feff}---\nversion: \"1\"\n---\n\nbody\n";
        let body = extract_body(input).expect("extraction should succeed");
        assert_eq!(body, "body\n");
    }

    #[test]
    fn extract_body_returns_content_when_no_frontmatter() {
        let input = "# v1.2.3\n\nhello\n";
        let body = extract_body(input).expect("extraction should succeed");
        assert_eq!(body, input);
    }

    #[test]
    fn extract_body_errors_on_unterminated_frontmatter() {
        let input = "---\nversion: \"1.2.3\"\nno closing fence\n";
        let err = extract_body(input).expect_err("should fail on unterminated frontmatter");
        let msg = format!("{err:#}");
        assert!(msg.contains("never closed"), "message was: {msg}");
    }

    #[test]
    fn extract_body_tolerates_leading_blank_lines() {
        let input = "\n\n---\nversion: \"1\"\n---\n\nbody\n";
        let body = extract_body(input).expect("extraction should succeed");
        assert_eq!(body, "body\n");
    }

    #[test]
    fn extract_body_preserves_fence_inside_body() {
        // A `---` horizontal rule inside the body must not be mistaken for a
        // closing fence; only the *first* closing fence after the opener counts.
        let input = "---\nversion: \"1\"\n---\n\nIntro.\n\n---\n\nMore.\n";
        let body = extract_body(input).expect("extraction should succeed");
        assert_eq!(body, "Intro.\n\n---\n\nMore.\n");
    }

    #[test]
    fn normalize_tag_prepends_v_when_missing() {
        assert_eq!(normalize_tag("0.12.4"), "v0.12.4");
        assert_eq!(normalize_tag("v0.12.4"), "v0.12.4");
        assert_eq!(normalize_tag(" v0.12.4 "), "v0.12.4");
    }

    #[test]
    fn note_path_targets_docs_releases_file() {
        let path = note_path(Path::new("/repo"), "v1.2.3");
        assert_eq!(path, PathBuf::from("/repo/docs/releases/v1.2.3.md"));
        // Bare version gets the `v` prefix.
        let path = note_path(Path::new("/repo"), "1.2.3");
        assert_eq!(path, PathBuf::from("/repo/docs/releases/v1.2.3.md"));
    }

    #[test]
    fn extract_body_against_real_release_note() {
        // Snapshot-ish check against the real v0.12.4 note shipped in the repo.
        // The test only asserts structural invariants so it stays stable even
        // if the human-written prose is tweaked.
        let root = project_root().expect("project_root should resolve");
        let path = root.join("docs/releases/v0.12.4.md");
        let content = fs::read_to_string(&path).expect("v0.12.4 note must exist");
        let body = extract_body(&content).expect("extraction should succeed");

        assert!(!body.starts_with("---"), "frontmatter leaked into body: {body:.40?}");
        assert!(body.starts_with("# v0.12.4"), "body should start with H1 header: {body:.40?}");
        assert!(body.contains("## Summary"), "body should carry the summary heading");
    }
}
