//! Worktree maintenance helpers used by local agent operations.

use crate::utils::project_root;
use color_eyre::eyre::{Result, bail};
use std::process::{Command, Stdio};
use std::str;

/// Prune stale worktrees and remove entries under `.claude/worktrees`.
pub fn cleanup() -> Result<()> {
    let root = project_root()?;

    let prune_status =
        Command::new("git").current_dir(&root).args(["worktree", "prune"]).status()?;
    if !prune_status.success() {
        bail!("failed to prune git worktrees");
    }

    let list_output = Command::new("git")
        .current_dir(&root)
        .args(["worktree", "list"])
        .stdout(Stdio::piped())
        .output()?;
    if !list_output.status.success() {
        bail!("failed to list git worktrees");
    }

    let list = str::from_utf8(&list_output.stdout)?;
    let stale = list
        .lines()
        .filter_map(|line| {
            if line.contains(".claude/worktrees/") { line.split_whitespace().next() } else { None }
        })
        .collect::<Vec<_>>();

    println!("Found {} agent worktrees", stale.len());

    if stale.is_empty() {
        println!("No stale worktrees to clean up");
        return Ok(());
    }

    println!("=== Removing stale worktrees ===");
    for worktree in stale {
        println!("Removing: {}", worktree);
        let remove_status = Command::new("git")
            .current_dir(&root)
            .args(["worktree", "remove", "--force", worktree])
            .status()?;
        if !remove_status.success() {
            std::fs::remove_dir_all(worktree)?;
        }
    }

    let final_prune_status =
        Command::new("git").current_dir(&root).args(["worktree", "prune"]).status()?;
    if !final_prune_status.success() {
        bail!("failed to prune git worktrees after cleanup");
    }

    println!("Cleanup complete");
    Ok(())
}
