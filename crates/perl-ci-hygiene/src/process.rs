use color_eyre::eyre::{Context, Result};
use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(crate) fn command_with_output(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if status != 0 {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(color_eyre::eyre::eyre!(
            "command '{command}' failed (exit {status}): {stderr}"
        ));
    }
    Ok(stdout)
}

pub(crate) fn command_with_output_all(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    let status = output.status.code().unwrap_or(1);
    if status != 0 {
        return Err(color_eyre::eyre::eyre!(
            "command '{command}' failed (exit {status}): {combined}"
        ));
    }
    Ok(combined)
}

pub(crate) fn command_with_input_with_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
    stdin_payload: &str,
) -> Result<(i32, String)> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = child.spawn().wrap_err_with(|| format!("running {command}"))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| color_eyre::eyre::eyre!("failed to open stdin for command {command}"))?;
        stdin
            .write_all(stdin_payload.as_bytes())
            .wrap_err_with(|| format!("writing to stdin for {command}"))?;
    }
    let output = child.wait_with_output().wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    Ok((status, combined))
}

pub(crate) fn command_output_with_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<(i32, String)> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok((status, stdout))
}

pub(crate) fn command_timed_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<(i32, Duration)> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::null()).stderr(Stdio::null());
    let start = Instant::now();
    let status = child.status().wrap_err_with(|| format!("running {command}"))?;
    let elapsed = start.elapsed();
    Ok((status.code().unwrap_or(1), elapsed))
}

pub(crate) fn command_with_output_allow_empty_match(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    if status != 0 && status != 1 {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(color_eyre::eyre::eyre!(
            "command '{command}' failed (exit {status}): {stderr}"
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn command_with_output_allow_failure(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn command_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<i32> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    let status = child.status().wrap_err_with(|| format!("running {command}"))?;
    Ok(status.code().unwrap_or(1))
}

pub(crate) fn command_status_strict(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<()> {
    let status = command_status(repo_root, command, args, env_vars)?;
    if status != 0 {
        return Err(color_eyre::eyre::eyre!("{command} failed with code {status}"));
    }
    Ok(())
}

pub(crate) fn command_exists(command: &str) -> bool {
    #[cfg(windows)]
    let suffixes: &[&str] = &[".exe", ".cmd", ".bat", ""];
    #[cfg(not(windows))]
    let suffixes: &[&str] = &[""];
    env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .any(|dir| suffixes.iter().any(|ext| dir.join(format!("{command}{ext}")).exists()))
}

pub(crate) fn command_output_lines(output: &str) -> Vec<String> {
    output.lines().map(str::trim).filter(|line| !line.is_empty()).map(ToString::to_string).collect()
}
