use anyhow::Result;
use std::{
    fs,
    path::Path,
    process::{Command as StdCommand, Stdio},
};

/// Initialize a minimal git repo in `dir` with one initial commit.
pub fn init_git_repo(dir: &Path) -> Result<()> {
    git_cmd(&["init", "-b", "master"], Some(dir)).or_else(|_| git_cmd(&["init"], Some(dir)))?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(dir))?;
    git_cmd(&["config", "user.name", "Test"], Some(dir))?;
    // Rename branch to master if needed (older git).
    let _ = git_cmd(&["checkout", "-b", "master"], Some(dir));
    Ok(())
}

/// Run a git command in `cwd`. Returns an error if the command fails.
pub fn git_cmd(args: &[&str], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = StdCommand::new("git");
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("git {:?} failed with {:?}", args, status.code());
    }
    Ok(())
}

/// Stage and commit a set of files in `repo`.
pub fn add_and_commit(repo: &Path, files: &[(&str, &str)], message: &str) -> Result<()> {
    for (name, content) in files {
        let path = repo.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
    }
    git_cmd(&["add", "."], Some(repo))?;
    git_cmd(&["commit", "-m", message], Some(repo))?;
    Ok(())
}
