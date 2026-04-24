//! PR-driven release orchestration wrapper.
//!
//! This task delegates the existing shell flow in
//! `scripts/release-turnkey-pr.sh` while exposing the command through
//! `cargo xtask release-turnkey`.

use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

use crate::utils::project_root;

/// Configuration for the release turn-key wrapper task.
pub struct ReleaseTurnkeyConfig {
    pub version: Option<String>,
    pub positional_version: Option<String>,
    pub prerelease: bool,
    pub dry_run: bool,
    pub skip_crates: bool,
    pub skip_extension: bool,
    pub skip_docker: bool,
    pub base_branch: Option<String>,
    pub no_auto_merge: bool,
    pub no_wait_pr_merge: bool,
    pub no_wait_release: bool,
    pub workflow_timeout: Option<u64>,
}

impl ReleaseTurnkeyConfig {
    fn resolve_version(&self) -> Result<String> {
        match (self.version.as_deref(), self.positional_version.as_deref()) {
            (None, None) => bail!("release version is required"),
            (Some(v), None) => Ok(v.to_string()),
            (None, Some(v)) => Ok(v.to_string()),
            (Some(v), Some(p)) if v == p => Ok(v.to_string()),
            (Some(v), Some(p)) => {
                bail!("version mismatch: --version {v} does not match positional {p}")
            }
        }
    }

    fn build_args(&self, version: &str) -> Vec<String> {
        let mut args = vec!["--version".to_string(), version.to_string()];

        if self.prerelease {
            args.push("--prerelease".to_string());
        }
        if self.dry_run {
            args.push("--dry-run".to_string());
        }
        if self.skip_crates {
            args.push("--skip-crates".to_string());
        }
        if self.skip_extension {
            args.push("--skip-extension".to_string());
        }
        if self.skip_docker {
            args.push("--skip-docker".to_string());
        }
        if let Some(base_branch) = &self.base_branch {
            args.push("--base-branch".to_string());
            args.push(base_branch.clone());
        }
        if self.no_auto_merge {
            args.push("--no-auto-merge".to_string());
        }
        if self.no_wait_pr_merge {
            args.push("--no-wait-pr-merge".to_string());
        }
        if self.no_wait_release {
            args.push("--no-wait-release".to_string());
        }
        if let Some(workflow_timeout) = self.workflow_timeout {
            args.push("--workflow-timeout".to_string());
            args.push(workflow_timeout.to_string());
        }

        args
    }
}

/// Execute the shell release driver via the canonical script path.
pub fn run(config: ReleaseTurnkeyConfig) -> Result<()> {
    let root = project_root()?;
    let script = root.join("scripts").join("release-turnkey-pr.sh");

    let version = config.resolve_version()?;
    let args = config.build_args(&version);

    let status = Command::new("bash")
        .arg(&script)
        .args(&args)
        .current_dir(&root)
        .status()
        .with_context(|| format!("failed to run {}", script.display()))?;

    if !status.success() {
        bail!("release-turnkey command failed");
    }

    Ok(())
}
