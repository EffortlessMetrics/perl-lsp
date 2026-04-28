use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::gate_receipts;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum ParserRatchetProfile {
    Pr,
    Nightly,
    Release,
}

impl ParserRatchetProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pr => "pr",
            Self::Nightly => "nightly",
            Self::Release => "release",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParserRatchetRunArgs {
    pub profile: ParserRatchetProfile,
    pub base: String,
    pub head: String,
    pub receipt: PathBuf,
    pub force_selected: bool,
}

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    schema_version: &'static str,
    check: &'static str,
    event: &'static str,
    profile: String,
    base_sha: String,
    head_sha: String,
    selected: bool,
    selection_reason: Vec<String>,
    verdict: &'static str,
    repro: Repro,
}

#[derive(Debug, Serialize)]
struct Repro {
    command: String,
}

pub fn run(args: ParserRatchetRunArgs) -> Result<()> {
    let base_sha = resolve_commit(&args.base)?;
    let head_sha = resolve_commit(&args.head)?;

    let selected = args.force_selected;
    let selection_reason = if args.force_selected {
        vec!["force-selected (scaffold only)".to_string()]
    } else {
        vec!["not selected by ci-scope".to_string()]
    };

    let receipt = ParserRatchetReceipt {
        schema_version: "1",
        check: "parser-ratchet",
        event: "local",
        profile: args.profile.as_str().to_string(),
        base_sha,
        head_sha,
        selected,
        selection_reason,
        verdict: "pass",
        repro: Repro {
            command: format!(
                "cargo xtask parser-ratchet run --profile {} --base {} --head {} --receipt {}{}",
                args.profile.as_str(),
                args.base,
                args.head,
                args.receipt.display(),
                if args.force_selected { " --force-selected" } else { "" }
            ),
        },
    };

    if let Some(parent) = args.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt directory {}", parent.display()))?;
    }

    let serialized = serde_json::to_string_pretty(&receipt)?;
    fs::write(&args.receipt, serialized)
        .with_context(|| format!("failed to write receipt {}", args.receipt.display()))?;

    if std::path::Path::new(".ci/receipts/registry.toml").exists() {
        gate_receipts::validate(&args.receipt, gate_receipts::OutputFormat::Human).map_err(
            |err| {
                color_eyre::eyre::eyre!(
                    "parser-ratchet receipt failed schema/registry validation: {}: {err}",
                    args.receipt.display()
                )
            },
        )?;
    }

    println!("Wrote parser-ratchet scaffold receipt: {}", args.receipt.display());
    Ok(())
}

fn resolve_commit(revision: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", revision])
        .output()
        .with_context(|| format!("failed to run git rev-parse for '{revision}'"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("unable to resolve revision '{revision}': {}", stderr.trim());
    }

    let sha = String::from_utf8(output.stdout)
        .with_context(|| format!("git rev-parse output was not valid UTF-8 for '{revision}'"))?;

    Ok(sha.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_file(path: &Path, contents: &str) -> Result<()> {
        fs::write(path, contents)
            .with_context(|| format!("failed to write fixture file {}", path.display()))
    }

    fn git_in(repo: &Path, args: &[&str]) -> Result<()> {
        let status = Command::new("git").args(args).current_dir(repo).status()?;
        if status.success() {
            Ok(())
        } else {
            bail!("git command failed in {}: git {:?}", repo.display(), args)
        }
    }

    struct CwdGuard {
        original: PathBuf,
    }

    impl CwdGuard {
        fn enter(path: &Path) -> Result<Self> {
            let original = std::env::current_dir()?;
            std::env::set_current_dir(path)
                .with_context(|| format!("failed to enter {}", path.display()))?;
            Ok(Self { original })
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    #[test]
    fn run_supports_detached_head_with_explicit_base_and_head() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();

        git_in(repo, &["init"])?;
        git_in(repo, &["config", "user.email", "parser-ratchet@example.com"])?;
        git_in(repo, &["config", "user.name", "Parser Ratchet Test"])?;

        write_file(&repo.join("fixture.txt"), "one\n")?;
        git_in(repo, &["add", "fixture.txt"])?;
        git_in(repo, &["commit", "-m", "first"])?;

        write_file(&repo.join("fixture.txt"), "two\n")?;
        git_in(repo, &["add", "fixture.txt"])?;
        git_in(repo, &["commit", "-m", "second"])?;

        git_in(repo, &["checkout", "--detach", "HEAD"])?;
        let _cwd_guard = CwdGuard::enter(repo)?;

        let receipt_path = repo.join("target/receipts/parser-ratchet.json");
        run(ParserRatchetRunArgs {
            profile: ParserRatchetProfile::Pr,
            base: "HEAD~1".to_string(),
            head: "HEAD".to_string(),
            receipt: receipt_path.clone(),
            force_selected: false,
        })?;

        let content = fs::read_to_string(&receipt_path)?;
        let receipt: serde_json::Value = serde_json::from_str(&content)?;

        assert_eq!(receipt.get("selected"), Some(&serde_json::Value::Bool(false)));
        assert_eq!(receipt.get("verdict").and_then(serde_json::Value::as_str), Some("pass"));
        assert_eq!(receipt.get("profile").and_then(serde_json::Value::as_str), Some("pr"));

        Ok(())
    }
}
