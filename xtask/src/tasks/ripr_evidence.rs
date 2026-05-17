//! PR-scoped RIPR evidence task wrappers.

use std::fs;
use std::path::Path;
use std::process::Command;

use color_eyre::eyre::{Result, WrapErr, bail};

use crate::utils::project_root;

const RIPR_PR_DIR: &str = "target/ripr/pr";
const RIPR_REVIEW_DIR: &str = "target/ripr/review";
const REPO_EXPOSURE_JSON: &str = "repo-exposure.json";
const REPO_EXPOSURE_MD: &str = "repo-exposure.md";
const COMMENTS_JSON: &str = "comments.json";
const COMMENTS_MD: &str = "comments.md";
const DEFAULT_HEAD: &str = "HEAD";

pub(crate) fn run_pr(check: bool) -> Result<()> {
    let workspace_root = project_root()?;
    let out_dir = workspace_root.join(RIPR_PR_DIR);

    if check {
        return check_pr_contract(&out_dir);
    }

    fs::create_dir_all(&out_dir).wrap_err("creating RIPR PR evidence directory")?;
    let base = resolve_base_ref(&workspace_root);
    run_ripr(
        &workspace_root,
        &["check", "--root", ".", "--base", base.as_str(), "--format", "repo-exposure-json"],
        &out_dir.join(REPO_EXPOSURE_JSON),
    )?;
    run_ripr(
        &workspace_root,
        &["check", "--root", ".", "--base", base.as_str(), "--format", "repo-exposure-md"],
        &out_dir.join(REPO_EXPOSURE_MD),
    )?;

    check_pr_contract(&out_dir)
}

pub(crate) fn run_review_comments(check: bool) -> Result<()> {
    let workspace_root = project_root()?;
    let out_dir = workspace_root.join(RIPR_REVIEW_DIR);
    let out_json = out_dir.join(COMMENTS_JSON);

    if check {
        return check_review_contract(&out_dir);
    }

    fs::create_dir_all(&out_dir).wrap_err("creating RIPR review guidance directory")?;
    let base = resolve_base_ref(&workspace_root);
    let ripr_bin = ripr_bin();
    let output = Command::new(&ripr_bin)
        .args(["review-comments", "--root", ".", "--base", &base, "--head", DEFAULT_HEAD, "--out"])
        .arg(&out_json)
        .current_dir(&workspace_root)
        .output()
        .wrap_err_with(|| format!("running {ripr_bin} review-comments"))?;

    if !output.status.success() {
        if output.status.code() == Some(137) || output.stderr.is_empty() {
            write_fallback_review(&out_dir)?;
            return check_review_contract(&out_dir);
        }
        bail!(
            "{ripr_bin} review-comments failed: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    check_review_contract(&out_dir)
}

fn resolve_base_ref(workspace_root: &Path) -> String {
    if let Ok(base_ref) = std::env::var("GITHUB_BASE_REF") {
        if !base_ref.trim().is_empty() {
            let candidate = format!("origin/{base_ref}");
            if git_ref_exists(workspace_root, &candidate) {
                return candidate;
            }
        }
    }

    for candidate in ["origin/master", "origin/main", "HEAD~1", "HEAD"] {
        if git_ref_exists(workspace_root, candidate) {
            return candidate.to_string();
        }
    }

    "HEAD".to_string()
}

fn git_ref_exists(workspace_root: &Path, candidate: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", candidate])
        .current_dir(workspace_root)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn write_fallback_review(out_dir: &Path) -> Result<()> {
    let json = serde_json::json!({
        "schema_version": 1,
        "status": "degraded",
        "reason": "ripr review-comments exited before producing guidance",
        "comments": [],
        "summary_only": [],
        "suppressed": [],
        "warnings": ["ripr review-comments exited before producing guidance"]
    });
    fs::write(out_dir.join(COMMENTS_JSON), format!("{}\n", serde_json::to_string_pretty(&json)?))
        .wrap_err("writing fallback RIPR review JSON")?;
    fs::write(
        out_dir.join(COMMENTS_MD),
        "# RIPR Review Guidance\n\nRIPR exited before producing review guidance.\n",
    )
    .wrap_err("writing fallback RIPR review Markdown")
}

fn run_ripr(workspace_root: &Path, args: &[&str], stdout_path: &Path) -> Result<()> {
    let ripr_bin = ripr_bin();
    let output = Command::new(&ripr_bin)
        .args(args)
        .current_dir(workspace_root)
        .output()
        .wrap_err_with(|| format!("running {ripr_bin} {}", args.join(" ")))?;

    if !output.status.success() {
        if output.status.code() == Some(137) || output.stderr.is_empty() {
            write_fallback_evidence(stdout_path, args)?;
            return Ok(());
        }
        bail!(
            "{ripr_bin} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    fs::write(stdout_path, &output.stdout)
        .wrap_err_with(|| format!("writing {}", stdout_path.display()))
}

fn write_fallback_evidence(stdout_path: &Path, args: &[&str]) -> Result<()> {
    let rendered = if args.contains(&"repo-exposure-json") {
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "status": "degraded",
            "reason": "ripr exited 137 before producing repository exposure JSON",
            "command": args.join(" ")
        }))?
    } else if args.contains(&"repo-exposure-md") {
        "# RIPR PR Evidence\n\nRIPR exited 137 before producing repository exposure Markdown.\n"
            .to_string()
    } else {
        bail!("ripr exited 137 for unsupported fallback command: {}", args.join(" "));
    };

    fs::write(stdout_path, format!("{rendered}\n"))
        .wrap_err_with(|| format!("writing fallback {}", stdout_path.display()))
}

fn check_pr_contract(out_dir: &Path) -> Result<()> {
    validate_json_file(&out_dir.join(REPO_EXPOSURE_JSON))?;
    validate_non_empty_file(&out_dir.join(REPO_EXPOSURE_MD))?;
    println!("ripr-pr: output contract is intact");
    Ok(())
}

fn check_review_contract(out_dir: &Path) -> Result<()> {
    validate_json_file(&out_dir.join(COMMENTS_JSON))?;
    validate_non_empty_file(&out_dir.join(COMMENTS_MD))?;
    println!("ripr-review-comments: output contract is intact");
    Ok(())
}

fn validate_json_file(path: &Path) -> Result<()> {
    let text = validate_non_empty_file(path)?;
    let _value: serde_json::Value = serde_json::from_str(&text)
        .wrap_err_with(|| format!("{} is not valid JSON", path.display()))?;
    Ok(())
}

fn validate_non_empty_file(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path).wrap_err_with(|| format!("reading {}", path.display()))?;
    if text.trim().is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(text)
}

fn ripr_bin() -> String {
    std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string())
}
