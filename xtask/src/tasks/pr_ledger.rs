//! `cargo xtask pr-ledger generate` — PR reconciliation ledger generator.
//!
//! Shells to `gh pr list --json ...` for one or more repositories and emits:
//!
//! 1. Per-repo `<repo-slug>.json` skeleton ledger arrays under `--out` directory.
//! 2. A combined `pr-ledger.md` markdown summary table.
//!
//! Skeleton rows have `classification: "unclassified"` and empty `evidence: []`
//! so scouts can fill in classifications without starting from scratch.
//!
//! The output is a **worklist** — not a ground-truth ledger.  Scouts read it,
//! fill `classification` and `evidence`, then the validator (`cargo xtask agent
//! ledgers validate`) enforces correctness.
//!
//! # Schema
//!
//! Each row conforms to the ORCHESTRATION_ROLES.md builder/closer output schema:
//!
//! ```json
//! {
//!   "pr": "1234",
//!   "title": "fix: something (#1234)",
//!   "surface_guess": "xtask",
//!   "classification": "unclassified",
//!   "confidence": "low",
//!   "evidence": [],
//!   "cleanup_done": false,
//!   "known_gaps": [],
//!   "is_draft": false,
//!   "mergeable": "MERGEABLE",
//!   "head_ref": "feat/1234-thing",
//!   "author": "EffortlessSteven"
//! }
//! ```
//!
//! # Exit codes
//! - `0` — generation succeeded.
//! - `1` — error (gh not available, bad repo, write failure).

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Public config
// ---------------------------------------------------------------------------

/// Configuration for `pr-ledger generate`.
pub struct GenerateConfig {
    /// Repositories to query (owner/name format).
    pub repos: Vec<String>,
    /// Output directory for generated artifacts. Defaults to
    /// `target/reconciliation/`.
    pub out: PathBuf,
    /// Optional fixture JSON path (one per repo, for testing without live gh).
    /// When set, the fixture is used instead of shelling to gh.
    pub fixture: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

/// A single PR row in the reconciliation ledger.
///
/// Fields align with the ORCHESTRATION_ROLES.md output schema for builder/closer agents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LedgerRow {
    /// PR number as string (matches validator schema).
    pub pr: String,
    /// PR title.
    pub title: String,
    /// Best-guess surface area inferred from labels + title prefix.
    pub surface_guess: String,
    /// Classification — starts as "unclassified"; scouts fill in.
    pub classification: String,
    /// Confidence — starts as "low".
    pub confidence: String,
    /// Evidence citations — starts empty.
    pub evidence: Vec<String>,
    /// Cleanup done — starts false.
    pub cleanup_done: bool,
    /// Known gaps — starts empty.
    pub known_gaps: Vec<String>,
    /// Whether the PR is a draft.
    pub is_draft: bool,
    /// Mergeability status from GitHub.
    pub mergeable: String,
    /// Head branch ref name.
    pub head_ref: String,
    /// PR author login.
    pub author: String,
}

// ---------------------------------------------------------------------------
// Raw GitHub PR shape (what gh returns)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GhPr {
    pub number: u64,
    pub title: String,
    pub labels: Vec<GhLabel>,
    #[serde(rename = "isDraft")]
    pub is_draft: bool,
    pub mergeable: String,
    #[serde(rename = "headRefName")]
    pub head_ref_name: String,
    pub author: GhAuthor,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct GhAuthor {
    pub login: String,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn generate(config: GenerateConfig) -> Result<()> {
    fs::create_dir_all(&config.out)
        .with_context(|| format!("creating output directory {}", config.out.display()))?;

    let mut all_rows: Vec<(String, Vec<LedgerRow>)> = Vec::new();

    for repo in &config.repos {
        let prs = if let Some(ref fixture) = config.fixture {
            load_fixture(fixture)?
        } else {
            fetch_prs_from_gh(repo)?
        };

        let rows: Vec<LedgerRow> = prs.into_iter().map(|pr| shape_row(pr, repo)).collect();
        write_repo_json(&rows, repo, &config.out)?;
        all_rows.push((repo.clone(), rows));
    }

    write_summary_md(&all_rows, &config.out)?;

    println!(
        "pr-ledger generate: wrote {} repo(s) to {}",
        config.repos.len(),
        config.out.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// gh invocation
// ---------------------------------------------------------------------------

fn fetch_prs_from_gh(repo: &str) -> Result<Vec<GhPr>> {
    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--repo",
            repo,
            "--state",
            "open",
            "--limit",
            "200",
            "--json",
            "number,title,labels,isDraft,mergeable,headRefName,author",
        ])
        .output()
        .with_context(|| format!("running `gh pr list` for {repo}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        color_eyre::eyre::bail!("gh pr list failed for {repo}: {stderr}");
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let prs: Vec<GhPr> =
        serde_json::from_str(&raw).with_context(|| format!("parsing gh output for {repo}"))?;
    Ok(prs)
}

fn load_fixture(path: &Path) -> Result<Vec<GhPr>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))?;
    let prs: Vec<GhPr> =
        serde_json::from_str(&content).with_context(|| "parsing fixture JSON".to_string())?;
    Ok(prs)
}

// ---------------------------------------------------------------------------
// Row shaping
// ---------------------------------------------------------------------------

fn shape_row(pr: GhPr, _repo: &str) -> LedgerRow {
    let label_names: Vec<String> = pr.labels.iter().map(|l| l.name.clone()).collect();
    let surface_guess = infer_surface(&pr.title, &label_names);

    LedgerRow {
        pr: pr.number.to_string(),
        title: pr.title,
        surface_guess,
        classification: "unclassified".to_string(),
        confidence: "low".to_string(),
        evidence: Vec::new(),
        cleanup_done: false,
        known_gaps: Vec::new(),
        is_draft: pr.is_draft,
        mergeable: pr.mergeable,
        head_ref: pr.head_ref_name,
        author: pr.author.login,
    }
}

/// Infer the surface area from conventional commit type prefix in the title
/// or from labels when present.
fn infer_surface(title: &str, labels: &[String]) -> String {
    // Label-based hints take priority.
    for label in labels {
        let l = label.as_str();
        if l.contains("parser") {
            return "parser".to_string();
        }
        if l.contains("lsp") {
            return "lsp".to_string();
        }
        if l.contains("dap") {
            return "dap".to_string();
        }
        if l.contains("xtask") || l.contains("ci") {
            return "xtask".to_string();
        }
        if l.contains("docs") {
            return "docs".to_string();
        }
    }

    // Fall back to conventional-commit scope in the title.
    // Matches patterns like: `fix(parser): ...`, `feat(lsp): ...`, `xtask(agents): ...`
    if let Some(scope_start) = title.find('(') {
        if let Some(scope_end) = title[scope_start..].find(')') {
            let scope = &title[scope_start + 1..scope_start + scope_end];
            if !scope.is_empty() {
                return scope.to_string();
            }
        }
    }

    "unknown".to_string()
}

// ---------------------------------------------------------------------------
// Output writers
// ---------------------------------------------------------------------------

fn write_repo_json(rows: &[LedgerRow], repo: &str, out_dir: &Path) -> Result<()> {
    let slug = repo.replace('/', "-");
    let path = out_dir.join(format!("{slug}.json"));
    let json = serde_json::to_string_pretty(rows).context("serializing repo ledger JSON")?;
    fs::write(&path, format!("{json}\n"))
        .with_context(|| format!("writing repo ledger to {}", path.display()))?;
    println!("  wrote {}", path.display());
    Ok(())
}

fn write_summary_md(all_rows: &[(String, Vec<LedgerRow>)], out_dir: &Path) -> Result<()> {
    let path = out_dir.join("pr-ledger.md");
    let mut buf = String::new();

    buf.push_str("# PR Reconciliation Ledger\n\n");
    buf.push_str(
        "> Generated by `cargo xtask pr-ledger generate`. \
         Classification and evidence columns are blank — fill in via scout.\n\n",
    );

    for (repo, rows) in all_rows {
        buf.push_str(&format!("## {repo}\n\n"));
        if rows.is_empty() {
            buf.push_str("_No open PRs._\n\n");
            continue;
        }
        buf.push_str("| PR | Title | Surface | Draft | Mergeable | Author |\n");
        buf.push_str("|----|-------|---------|-------|-----------|--------|\n");
        for row in rows {
            let draft_mark = if row.is_draft { "yes" } else { "no" };
            buf.push_str(&format!(
                "| #{pr} | {title} | {surface} | {draft} | {mergeable} | {author} |\n",
                pr = row.pr,
                title = md_escape(&row.title),
                surface = row.surface_guess,
                draft = draft_mark,
                mergeable = row.mergeable,
                author = row.author,
            ));
        }
        buf.push('\n');
    }

    fs::write(&path, &buf)
        .with_context(|| format!("writing summary markdown to {}", path.display()))?;
    println!("  wrote {}", path.display());
    Ok(())
}

/// Escape `|` characters in markdown table cells.
fn md_escape(s: &str) -> String {
    s.replace('|', "&#124;")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- surface inference ------------------------------------------------

    #[test]
    fn test_infer_surface_from_conventional_scope() -> Result<()> {
        assert_eq!(infer_surface("fix(parser): handle heredoc", &[]), "parser");
        assert_eq!(infer_surface("feat(lsp): add hover", &[]), "lsp");
        assert_eq!(infer_surface("xtask(agents): validate ledgers", &[]), "agents");
        Ok(())
    }

    #[test]
    fn test_infer_surface_from_labels() -> Result<()> {
        let labels = vec!["area/parser".to_string()];
        assert_eq!(infer_surface("unformatted title", &labels), "parser");
        Ok(())
    }

    #[test]
    fn test_infer_surface_labels_beat_scope() -> Result<()> {
        // Label wins over scope.
        let labels = vec!["area/lsp".to_string()];
        assert_eq!(infer_surface("fix(parser): thing", &labels), "lsp");
        Ok(())
    }

    #[test]
    fn test_infer_surface_unknown_fallback() -> Result<()> {
        assert_eq!(infer_surface("no scope or label here", &[]), "unknown");
        Ok(())
    }

    // ----- row shaping from canned gh JSON ----------------------------------

    fn make_gh_pr(number: u64, title: &str, labels: &[&str]) -> GhPr {
        GhPr {
            number,
            title: title.to_string(),
            labels: labels.iter().map(|l| GhLabel { name: l.to_string() }).collect(),
            is_draft: false,
            mergeable: "MERGEABLE".to_string(),
            head_ref_name: format!("feat/{number}-thing"),
            author: GhAuthor { login: "test-user".to_string() },
        }
    }

    #[test]
    fn test_shape_row_defaults() -> Result<()> {
        let pr = make_gh_pr(42, "fix(lsp): hover docs (#42)", &[]);
        let row = shape_row(pr, "EffortlessMetrics/perl-lsp-swarm");

        assert_eq!(row.pr, "42");
        assert_eq!(row.classification, "unclassified");
        assert_eq!(row.confidence, "low");
        assert!(row.evidence.is_empty());
        assert!(!row.cleanup_done);
        assert!(row.known_gaps.is_empty());
        assert_eq!(row.surface_guess, "lsp");
        Ok(())
    }

    #[test]
    fn test_shape_row_draft_preserved() -> Result<()> {
        let mut pr = make_gh_pr(7, "wip: draft (#7)", &[]);
        pr.is_draft = true;
        let row = shape_row(pr, "owner/repo");
        assert!(row.is_draft);
        Ok(())
    }

    #[test]
    fn test_shape_row_author_preserved() -> Result<()> {
        let pr = make_gh_pr(9, "feat(dap): thing (#9)", &[]);
        let row = shape_row(pr, "owner/repo");
        assert_eq!(row.author, "test-user");
        Ok(())
    }

    #[test]
    fn test_shape_row_mergeable_preserved() -> Result<()> {
        let mut pr = make_gh_pr(10, "fix(parser): thing (#10)", &[]);
        pr.mergeable = "CONFLICTING".to_string();
        let row = shape_row(pr, "owner/repo");
        assert_eq!(row.mergeable, "CONFLICTING");
        Ok(())
    }

    // ----- fixture-based end-to-end -----------------------------------------

    #[test]
    fn test_generate_from_fixture() -> Result<()> {
        use std::io::Write;

        let tmp = tempfile::tempdir().context("creating temp dir")?;

        // Write a small canned gh JSON fixture.
        let fixture_data = serde_json::json!([
            {
                "number": 101,
                "title": "feat(lsp): add definition provider (#101)",
                "labels": [{"name": "size/S"}],
                "isDraft": false,
                "mergeable": "MERGEABLE",
                "headRefName": "feat/101-def-provider",
                "author": {"login": "EffortlessSteven"}
            },
            {
                "number": 102,
                "title": "fix(parser): heredoc edge case (#102)",
                "labels": [{"name": "area/parser"}],
                "isDraft": true,
                "mergeable": "UNKNOWN",
                "headRefName": "fix/102-heredoc",
                "author": {"login": "bot"}
            }
        ]);

        let fixture_path = tmp.path().join("fixture.json");
        let mut f = fs::File::create(&fixture_path).context("creating fixture file")?;
        write!(f, "{}", serde_json::to_string_pretty(&fixture_data)?)?;

        let out_dir = tmp.path().join("out");

        generate(GenerateConfig {
            repos: vec!["EffortlessMetrics/perl-lsp-swarm".to_string()],
            out: out_dir.clone(),
            fixture: Some(fixture_path),
        })?;

        // Check repo JSON was written.
        let repo_json_path = out_dir.join("EffortlessMetrics-perl-lsp-swarm.json");
        assert!(repo_json_path.exists(), "repo JSON not found");

        let content = fs::read_to_string(&repo_json_path)?;
        let rows: Vec<LedgerRow> = serde_json::from_str(&content)?;

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pr, "101");
        assert_eq!(rows[0].classification, "unclassified");
        assert_eq!(rows[0].surface_guess, "lsp");
        assert!(!rows[0].is_draft);

        assert_eq!(rows[1].pr, "102");
        assert_eq!(rows[1].surface_guess, "parser"); // label wins
        assert!(rows[1].is_draft);

        // Check summary markdown was written.
        let md_path = out_dir.join("pr-ledger.md");
        assert!(md_path.exists(), "summary markdown not found");
        let md = fs::read_to_string(&md_path)?;
        assert!(md.contains("EffortlessMetrics/perl-lsp-swarm"));
        assert!(md.contains("#101"));
        assert!(md.contains("#102"));

        Ok(())
    }

    #[test]
    fn test_md_escape_pipes() -> Result<()> {
        let escaped = md_escape("title | with | pipes");
        assert!(!escaped.contains('|'));
        Ok(())
    }

    // ----- empty repo -------------------------------------------------------

    #[test]
    fn test_generate_empty_fixture() -> Result<()> {
        use std::io::Write;

        let tmp = tempfile::tempdir().context("creating temp dir")?;
        let fixture_path = tmp.path().join("empty.json");
        let mut f = fs::File::create(&fixture_path).context("creating fixture file")?;
        write!(f, "[]")?;

        let out_dir = tmp.path().join("out");
        generate(GenerateConfig {
            repos: vec!["owner/repo".to_string()],
            out: out_dir.clone(),
            fixture: Some(fixture_path),
        })?;

        let repo_json_path = out_dir.join("owner-repo.json");
        assert!(repo_json_path.exists());

        let content = fs::read_to_string(&repo_json_path)?;
        let rows: Vec<LedgerRow> = serde_json::from_str(&content)?;
        assert!(rows.is_empty());

        let md = fs::read_to_string(out_dir.join("pr-ledger.md"))?;
        assert!(md.contains("No open PRs"));

        Ok(())
    }
}
