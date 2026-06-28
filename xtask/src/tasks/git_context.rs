//! Git command helpers for Windows-linked worktrees.

use color_eyre::eyre::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn git_output_with_mount_root(
    root: &Path,
    args: &[&str],
    windows_drive_mount_root: &Path,
) -> Result<Output> {
    let output = git_output(root, args, None)?;
    if output.status.success() {
        return Ok(output);
    }

    if let Some(context) = worktree_git_context_with_mount_root(root, windows_drive_mount_root)? {
        let fallback = git_output(&context.work_tree, args, Some(&context.git_dir))?;
        if fallback.status.success() {
            return Ok(fallback);
        }
    }

    Ok(output)
}

pub(crate) fn git_stdout_with_worktree_fallback(root: &Path, args: &[&str]) -> Result<String> {
    git_stdout_with_mount_root(root, args, default_windows_drive_mount_root())
}

pub(crate) fn git_stdout_with_mount_root(
    root: &Path,
    args: &[&str],
    windows_drive_mount_root: &Path,
) -> Result<String> {
    let output = git_output_with_mount_root(root, args, windows_drive_mount_root)?;
    if !output.status.success() {
        bail!("git {} failed with status {}", args.join(" "), output.status);
    }
    Ok(String::from_utf8(output.stdout)
        .context("git command returned non-UTF8 output")?
        .trim()
        .to_string())
}

fn git_output(root: &Path, args: &[&str], git_dir: Option<&Path>) -> Result<Output> {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    if let Some(git_dir) = git_dir {
        command.env("GIT_DIR", git_dir).env("GIT_WORK_TREE", root);
    }
    command.output().context("running git command")
}

struct WorktreeGitContext {
    git_dir: PathBuf,
    work_tree: PathBuf,
}

fn worktree_git_context_with_mount_root(
    root: &Path,
    windows_drive_mount_root: &Path,
) -> Result<Option<WorktreeGitContext>> {
    for work_tree in root.ancestors() {
        let git_file = work_tree.join(".git");
        if git_file.is_dir() {
            return Ok(None);
        }
        let contents = match fs::read_to_string(&git_file) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("reading {}", git_file.display()));
            }
        };
        let Some(raw_git_dir) = contents.trim().strip_prefix("gitdir:") else {
            return Ok(None);
        };
        return Ok(Some(WorktreeGitContext {
            git_dir: resolve_git_dir_with_mount_root(
                work_tree,
                raw_git_dir.trim(),
                windows_drive_mount_root,
            ),
            work_tree: work_tree.to_path_buf(),
        }));
    }
    Ok(None)
}

fn resolve_git_dir_with_mount_root(
    work_tree: &Path,
    raw_git_dir: &str,
    windows_drive_mount_root: &Path,
) -> PathBuf {
    let direct = PathBuf::from(raw_git_dir);
    if direct.is_absolute() || direct.exists() {
        return direct;
    }
    if let Some(translated) =
        translate_windows_git_dir_for_unix(raw_git_dir, windows_drive_mount_root)
    {
        return translated;
    }
    work_tree.join(raw_git_dir)
}

#[cfg(unix)]
pub(crate) fn default_windows_drive_mount_root() -> &'static Path {
    Path::new("/mnt")
}

#[cfg(not(unix))]
pub(crate) fn default_windows_drive_mount_root() -> &'static Path {
    Path::new("")
}

#[cfg(unix)]
fn translate_windows_git_dir_for_unix(raw: &str, mount_root: &Path) -> Option<PathBuf> {
    let mut chars = raw.chars();
    let drive = chars.next()?;
    if !drive.is_ascii_alphabetic() || chars.next()? != ':' {
        return None;
    }
    let rest = chars.as_str().trim_start_matches(['/', '\\']).replace('\\', "/");
    Some(mount_root.join(drive.to_ascii_lowercase().to_string()).join(rest))
}

#[cfg(not(unix))]
fn translate_windows_git_dir_for_unix(_raw: &str, _mount_root: &Path) -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn worktree_git_context_finds_git_file_in_ancestor() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let nested = repo.join("xtask/src");
        let git_dir = temp.path().join("git/worktrees/repo");
        fs::create_dir_all(&nested)?;
        fs::create_dir_all(&git_dir)?;
        fs::write(repo.join(".git"), format!("gitdir: {}\n", git_dir.display()))?;

        let context =
            worktree_git_context_with_mount_root(&nested, default_windows_drive_mount_root())?
                .ok_or("missing worktree git context")?;

        assert_eq!(context.git_dir, git_dir);
        assert_eq!(context.work_tree, repo);
        Ok(())
    }

    #[test]
    fn resolve_git_dir_uses_worktree_for_relative_paths() -> TestResult {
        let temp = tempfile::tempdir()?;
        let work_tree = temp.path().join("repo");
        let expected = work_tree.join(".git/worktrees/repo");

        assert_eq!(
            resolve_git_dir_with_mount_root(
                &work_tree,
                ".git/worktrees/repo",
                default_windows_drive_mount_root()
            ),
            expected
        );
        Ok(())
    }

    #[test]
    fn worktree_git_context_returns_none_without_git_file() -> TestResult {
        let temp = tempfile::tempdir()?;

        assert!(
            worktree_git_context_with_mount_root(temp.path(), default_windows_drive_mount_root())?
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn worktree_git_context_stops_at_malformed_nearest_git_file() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let nested = repo.join("nested");
        let git_dir = temp.path().join("repo.git");
        fs::create_dir_all(&nested)?;
        fs::create_dir_all(&git_dir)?;
        fs::write(repo.join(".git"), format!("gitdir: {}\n", git_dir.display()))?;
        fs::write(nested.join(".git"), "not a gitdir\n")?;

        assert!(
            worktree_git_context_with_mount_root(&nested, default_windows_drive_mount_root())?
                .is_none()
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn translate_windows_git_dir_for_unix_maps_drive_paths() -> TestResult {
        assert_eq!(
            translate_windows_git_dir_for_unix("H:/Code/Rust2/repo/.git", Path::new("/mnt"))
                .ok_or("drive path did not translate")?,
            PathBuf::from("/mnt/h/Code/Rust2/repo/.git")
        );
        assert!(translate_windows_git_dir_for_unix("relative/.git", Path::new("/mnt")).is_none());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn resolve_git_dir_translates_windows_drive_paths_on_unix() -> TestResult {
        let temp = tempfile::tempdir()?;

        assert_eq!(
            resolve_git_dir_with_mount_root(
                temp.path(),
                "H:/Code/Rust2/repo/.git",
                Path::new("/mnt")
            ),
            PathBuf::from("/mnt/h/Code/Rust2/repo/.git")
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn git_stdout_fallback_handles_windows_gitdir_with_mount_root() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let mount_root = temp.path().join("mnt");
        let git_dir = mount_root.join("z/repo.git");
        fs::create_dir_all(&repo)?;
        run_git(&repo, &["init"])?;
        run_git(&repo, &["config", "user.email", "agent@example.invalid"])?;
        run_git(&repo, &["config", "user.name", "Agent Test"])?;
        fs::write(repo.join("covered.rs"), "fn covered() {}\n")?;
        run_git(&repo, &["add", "covered.rs"])?;
        run_git(&repo, &["commit", "-m", "initial"])?;
        let head = run_git(&repo, &["rev-parse", "HEAD"])?.trim().to_string();

        fs::create_dir_all(git_dir.parent().ok_or("git dir missing parent")?)?;
        fs::rename(repo.join(".git"), &git_dir)?;
        fs::write(repo.join(".git"), "gitdir: Z:/repo.git\n")?;

        assert_eq!(git_stdout_with_mount_root(&repo, &["rev-parse", "HEAD"], &mount_root)?, head);
        let diff = git_output_with_mount_root(
            &repo,
            &["diff", "--unified=0", "HEAD...HEAD"],
            &mount_root,
        )?;
        assert!(diff.status.success());
        Ok(())
    }

    fn run_git(repo: &Path, args: &[&str]) -> TestResult<String> {
        let output = Command::new("git").args(args).current_dir(repo).output()?;
        if !output.status.success() {
            return Err(format!("git {:?} failed with status {}", args, output.status).into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }
}
